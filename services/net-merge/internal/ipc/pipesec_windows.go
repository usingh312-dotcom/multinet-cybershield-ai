// Package ipc provides secure named-pipe creation with a custom security
// descriptor and client identity validation.
//
// The pipe is created with CreateNamedPipe (not net.Listen) so a restrictive
// DACL can be applied. After a client connects, the server impersonates the
// client and validates its Windows security identity before processing any
// requests.
//
// Vendored patterns adapted from net-merge upstream; the security descriptor
// and client validation logic are Multinet-specific.
package ipc

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"unsafe"

	"golang.org/x/sys/windows"
)

// pipeSDDL is the restrictive security descriptor for the named pipe.
//
// D:P                                  = DACL is protected (not inherited)
// (A;;GA;;;BA)                          = Allow Full Control to Administrators
// (A;;GRGW;;;S-1-5-32-545)              = Allow Read/Write to Built-in Users
// (A;;GRGW;;;S-1-5-18)                  = Allow Read/Write to LocalSystem
// Everyone (S-1-1-0) and Anonymous Logon (S-1-5-7) are explicitly excluded.
//
// NM-05 hardening: the broad Users SID is retained only because the Tauri
// controller runs as an interactive local user. In production this should be
// narrowed further to the specific controller user SID.
const pipeSDDL = "D:P(A;;GA;;;BA)(A;;GRGW;;;S-1-5-32-545)(A;;GRGW;;;S-1-5-18)"

// PIPE_REJECT_REMOTE_CLIENTS causes CreateNamedPipeW to reject remote client
// connections, ensuring the control interface is local-only.
const PIPE_REJECT_REMOTE_CLIENTS = 0x00000008

// advapi32 provides CheckTokenMembership and ImpersonateNamedPipeClient.
var (
	advapi32                 = windows.NewLazySystemDLL("advapi32.dll")
	procCheckTokenMembership = advapi32.NewProc("CheckTokenMembership")
	procImpersonateNamedPipe = advapi32.NewProc("ImpersonateNamedPipeClient")
)

func impersonateNamedPipeClient(h windows.Handle) error {
	r1, _, err := procImpersonateNamedPipe.Call(uintptr(h))
	if r1 == 0 {
		return err
	}
	return nil
}

// AdminWellKnownSid is the Built-in Administrators well-known SID, used for
// authorization checks and tests.
var AdminWellKnownSid = windows.WinBuiltinAdministratorsSid

// UsersWellKnownSid is the Built-in Users well-known SID.
var UsersWellKnownSid = windows.WinBuiltinUsersSid

// LocalSystemWellKnownSid is the LocalSystem well-known SID.
var LocalSystemWellKnownSid = windows.WinLocalSystemSid

// securePipe creates a named pipe with the restrictive pipeSDDL security
// descriptor. It returns a handle that the caller must close.
//
// PIPE_REJECT_REMOTE_CLIENTS is set so that remote (SMB) clients cannot connect
// to this local control interface. Only local processes can open the pipe.
//
// The security descriptor is created using windows.SecurityDescriptorFromString,
// which returns a self-relative SECURITY_DESCRIPTOR allocated on the Go heap.
// CreateNamedPipe copies the descriptor into the kernel pipe object during the
// call, avoiding any memory leak or unsafe pointer conversions.
func securePipe(pipeName string) (windows.Handle, error) {
	sd, err := windows.SecurityDescriptorFromString(pipeSDDL)
	if err != nil {
		return 0, fmt.Errorf("security descriptor: %w", err)
	}
	sa := &windows.SecurityAttributes{
		Length:             uint32(unsafe.Sizeof(windows.SecurityAttributes{})),
		SecurityDescriptor: sd,
		InheritHandle:      0,
	}

	pName := pipeName
	if !strings.HasPrefix(pName, `\\.\pipe\`) {
		pName = `\\.\pipe\` + pName
	}
	name, err := windows.UTF16PtrFromString(pName)
	if err != nil {
		return 0, err
	}

	h, err := windows.CreateNamedPipe(
		name,
		windows.PIPE_ACCESS_DUPLEX|windows.FILE_FLAG_FIRST_PIPE_INSTANCE,
		windows.PIPE_TYPE_MESSAGE|windows.PIPE_READMODE_MESSAGE|windows.PIPE_WAIT|PIPE_REJECT_REMOTE_CLIENTS,
		windows.PIPE_UNLIMITED_INSTANCES,
		MaxMessageSize, // out buffer size
		MaxMessageSize, // in buffer size
		0,              // default timeout
		sa,
	)
	if err != nil {
		return 0, err
	}
	return h, nil
}

// getPipeClientPID returns the process ID of the client connected to the given
// named pipe handle. This is the trusted way to identify the pipe client:
// the PID comes from the kernel via GetNamedPipeClientProcessId.
func getPipeClientPID(pipe windows.Handle) (uint32, error) {
	var pid uint32
	if err := windows.GetNamedPipeClientProcessId(pipe, &pid); err != nil {
		return 0, fmt.Errorf("GetNamedPipeClientProcessId failed: %w", err)
	}
	return pid, nil
}

// openValidatedProcess opens a process handle for the given PID with
// PROCESS_QUERY_LIMITED_INFORMATION and PROCESS_SYNCHRONIZE rights, then
// verifies the process ID matches via GetProcessId(). This protects against
// PID reuse attacks.
func openValidatedProcess(pid uint32) (windows.Handle, error) {
	if pid == 0 {
		return 0, fmt.Errorf("invalid PID 0")
	}
	handle, err := windows.OpenProcess(
		windows.PROCESS_QUERY_LIMITED_INFORMATION|windows.SYNCHRONIZE,
		false,
		pid,
	)
	if err != nil {
		return 0, fmt.Errorf("cannot open process %d: %w", pid, err)
	}

	actualPid, err := windows.GetProcessId(handle)
	if err != nil {
		windows.CloseHandle(handle)
		return 0, fmt.Errorf("get process ID: %w", err)
	}
	if actualPid != pid {
		windows.CloseHandle(handle)
		return 0, fmt.Errorf("process ID mismatch: requested %d, actual %d (PID reuse detected)", pid, actualPid)
	}
	return handle, nil
}

// ProcessIdentityValidator validates whether a process handle represents a trusted Rust bonding engine.
type ProcessIdentityValidator func(h windows.Handle, pid uint32) error

var (
	processValidatorMu     sync.RWMutex
	customProcessValidator ProcessIdentityValidator
)

// SetProcessValidator overrides default process identity validation (used in tests).
func SetProcessValidator(fn ProcessIdentityValidator) {
	processValidatorMu.Lock()
	defer processValidatorMu.Unlock()
	customProcessValidator = fn
}

// verifyRustProcessIdentity checks that the process handle belongs to the authorized Rust engine.
func verifyRustProcessIdentity(h windows.Handle, pid uint32) error {
	processValidatorMu.RLock()
	custom := customProcessValidator
	processValidatorMu.RUnlock()
	if custom != nil {
		return custom(h, pid)
	}

	buf := make([]uint16, windows.MAX_LONG_PATH)
	size := uint32(len(buf))
	err := windows.QueryFullProcessImageName(h, 0, &buf[0], &size)
	if err != nil {
		return fmt.Errorf("query process image name: %w", err)
	}
	exePath := strings.ToLower(windows.UTF16ToString(buf[:size]))
	exeName := filepath.Base(exePath)

	allowedNames := []string{
		"bonding-engine.exe",
		"bonding_engine.exe",
		"multinet-bonding.exe",
		"multinet.exe",
	}
	for _, allowed := range allowedNames {
		if strings.EqualFold(exeName, allowed) {
			return nil
		}
	}

	return fmt.Errorf("unauthorized process identity: %s (PID %d is not an approved bonding engine)", exeName, pid)
}

// ClientIdentity is the validated identity of a pipe client.
type ClientIdentity struct {
	UserSid   string
	IsAdmin   bool
	IsUser    bool
	IsSystem  bool
	ClientPid uint32 // OS process ID of the pipe client, obtained via GetNamedPipeClientProcessId
}

// tokenIsMember reports whether the given token contains the specified SID in
// its enabled groups.
func tokenIsMember(token windows.Token, sid *windows.SID) (bool, error) {
	var isMember int32
	r1, _, lastErr := procCheckTokenMembership.Call(
		uintptr(token),
		uintptr(unsafe.Pointer(sid)),
		uintptr(unsafe.Pointer(&isMember)),
	)
	if r1 == 0 {
		return false, fmt.Errorf("check token membership: %w", lastErr)
	}
	return isMember != 0, nil
}

// validateClientIdentity impersonates the connected pipe client and verifies
// that the client's user is authorized (administrator, local interactive user,
// or LocalSystem). It reverts to the service identity before returning.
func validateClientIdentity(h windows.Handle) (ClientIdentity, error) {
	if err := impersonateNamedPipeClient(h); err != nil {
		return ClientIdentity{}, fmt.Errorf("impersonate named pipe client: %w", err)
	}
	// Always revert impersonation before returning, on every error path.
	defer windows.RevertToSelf()

	var token windows.Token
	err := windows.OpenThreadToken(windows.CurrentThread(), windows.TOKEN_QUERY, true, &token)
	if err != nil {
		return ClientIdentity{}, fmt.Errorf("open thread token: %w", err)
	}
	// Always close the token handle.
	defer token.Close()

	user, err := token.GetTokenUser()
	if err != nil {
		return ClientIdentity{}, fmt.Errorf("get token user: %w", err)
	}

	ci := ClientIdentity{UserSid: user.User.Sid.String()}

	// Obtain the pipe client's OS process ID. This is the trusted identity of
	// the connected process — it comes from the kernel via
	// GetNamedPipeClientProcessId, not from a caller-supplied value.
	clientPid, err := getPipeClientPID(h)
	if err != nil {
		return ClientIdentity{}, fmt.Errorf("get pipe client PID: %w", err)
	}
	ci.ClientPid = clientPid

	// Administrators are always authorized.
	adminSid, err := windows.CreateWellKnownSid(windows.WinBuiltinAdministratorsSid)
	if err != nil {
		return ClientIdentity{}, fmt.Errorf("create admin SID: %w", err)
	}
	if isAdmin, err := tokenIsMember(token, adminSid); err != nil {
		return ClientIdentity{}, fmt.Errorf("check admin membership: %w", err)
	} else if isAdmin {
		ci.IsAdmin = true
		return ci, nil
	}

	// Local interactive users (Users group) are authorized for read-only methods.
	usersSid, err := windows.CreateWellKnownSid(windows.WinBuiltinUsersSid)
	if err != nil {
		return ClientIdentity{}, fmt.Errorf("create users SID: %w", err)
	}
	if isUser, err := tokenIsMember(token, usersSid); err != nil {
		return ClientIdentity{}, fmt.Errorf("check users membership: %w", err)
	} else if isUser {
		ci.IsUser = true
		return ci, nil
	}

	// LocalSystem is authorized for service operation.
	systemSid, err := windows.CreateWellKnownSid(windows.WinLocalSystemSid)
	if err != nil {
		return ClientIdentity{}, fmt.Errorf("create system SID: %w", err)
	}
	if isSystem, err := tokenIsMember(token, systemSid); err != nil {
		return ClientIdentity{}, fmt.Errorf("check system membership: %w", err)
	} else if isSystem {
		ci.IsSystem = true
		return ci, nil
	}

	return ci, fmt.Errorf("client %s (pid %d) is not authorized to use the net-merge pipe", ci.UserSid, ci.ClientPid)
}

// Authorizer checks whether a client identity is permitted to invoke a method.
type Authorizer struct {
	RequireAdmin map[string]bool
}

// NewAuthorizer creates an authorizer with the default method policy.
func NewAuthorizer() *Authorizer {
	return &Authorizer{
		RequireAdmin: map[string]bool{
			string(MethodStart):       true,
			string(MethodStop):        true,
			string(MethodSetLimit):    true,
			string(MethodSetModeB):    false, // Unelevated Tauri controller is authorized; bonding engine PID & process image are strictly validated
			string(MethodAdapters):    false,
			string(MethodStatus):      false,
			string(MethodHealth):      false,
			string(MethodModeBStatus): false,
		},
	}
}

// Authorize returns nil if the client identity may invoke the method.
func (a *Authorizer) Authorize(method string, ci ClientIdentity) error {
	if ci.IsAdmin || ci.IsSystem {
		return nil
	}
	if a.RequireAdmin[method] {
		return fmt.Errorf("method %s requires administrator privileges", method)
	}
	if ci.IsUser {
		return nil
	}
	return fmt.Errorf("client %s is not authorized", ci.UserSid)
}

// HandleConn serves a single client connection over a Windows pipe handle.
// Disconnecting or closing the pipe handle does NOT terminate any active Mode B
// session; Mode B is bound to the engine process lifetime, not the pipe connection.
func (s *Server) HandleConn(h windows.Handle, initialCi ...ClientIdentity) {
	f := os.NewFile(uintptr(h), s.pipeName)
	if f == nil {
		windows.DisconnectNamedPipe(h)
		windows.CloseHandle(h)
		return
	}
	defer func() {
		_ = windows.DisconnectNamedPipe(h)
		_ = f.Close()
	}()

	scanner := bufio.NewScanner(f)
	scanner.Buffer(make([]byte, 1024), MaxMessageSize)

	var ci ClientIdentity
	var authenticated bool
	if len(initialCi) > 0 && (initialCi[0].IsAdmin || initialCi[0].IsUser || initialCi[0].IsSystem) {
		ci = initialCi[0]
		authenticated = true
	}

	for scanner.Scan() {
		line := scanner.Bytes()
		if len(line) == 0 {
			continue
		}
		if len(line) > MaxMessageSize {
			resp := NewErrorResponse("parse", "message_too_large",
				fmt.Sprintf("message exceeds %d bytes", MaxMessageSize), false)
			_ = writeResp(f, resp)
			continue
		}

		if !authenticated {
			var err error
			ci, err = validateClientIdentity(h)
			if err != nil {
				log.Printf("ipc: unauthorized client rejected: %v", err)
				resp := NewErrorResponse("auth", "unauthorized", err.Error(), false)
				_ = writeResp(f, resp)
				return
			}
			authenticated = true
		}

		var req Request
		if err := json.Unmarshal(line, &req); err != nil {
			resp := NewErrorResponse("parse", "invalid_json", err.Error(), false)
			_ = writeResp(f, resp)
			continue
		}
		if req.ID == "" {
			resp := NewErrorResponse("", "invalid_request", "missing id", false)
			_ = writeResp(f, resp)
			continue
		}
		if req.PipeVersion != PIPEVersion {
			resp := NewErrorResponse(req.ID, "unsupported_version",
				fmt.Sprintf("pipe version %d not supported; expected %d", req.PipeVersion, PIPEVersion), false)
			_ = writeResp(f, resp)
			continue
		}
		// Per-method authorization check.
		if err := s.auth.Authorize(string(req.Method), ci); err != nil {
			resp := NewErrorResponse(req.ID, "unauthorized", err.Error(), false)
			_ = writeResp(f, resp)
			continue
		}
		resp, _ := s.router.Dispatch(req, ci)
		if err := writeResp(f, resp); err != nil {
			log.Printf("ipc: write error: %v", err)
			return
		}
	}
	if err := scanner.Err(); err != nil {
		if err != io.EOF && err != syscall.Errno(0) {
			log.Printf("ipc: scan error: %v", err)
		}
	}
}

var _ = context.Background // reserved for future deadline plumbing
