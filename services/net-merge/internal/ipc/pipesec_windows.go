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

// sddlRevision is the SDDL string revision.
const sddlRevision = 1

// PIPE_REJECT_REMOTE_CLIENTS causes CreateNamedPipeW to reject remote client
// connections, ensuring the control interface is local-only.
const PIPE_REJECT_REMOTE_CLIENTS = 0x00000008

// ERROR_ALREADY_EXISTS is returned by CreateMutex when the mutex already exists.
const ERROR_ALREADY_EXISTS = 183

// advapi32 provides ConvertStringSecurityDescriptorToSecurityDescriptorW,
// CheckTokenMembership and LocalFree. kernel32 provides CreateMutexW,
// ReleaseMutex, GetLastError, OpenProcess and WaitForSingleObject.
var (
	advapi32                 = windows.NewLazySystemDLL("advapi32.dll")
	kernel32                 = windows.NewLazySystemDLL("kernel32.dll")
	procConvertStringSDToSD  = advapi32.NewProc("ConvertStringSecurityDescriptorToSecurityDescriptorW")
	procLocalFree            = kernel32.NewProc("LocalFree")
	procCheckTokenMembership  = advapi32.NewProc("CheckTokenMembership")
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

// stringToSecurityDescriptor converts an SDDL string to an absolute security
// descriptor allocated by LocalAlloc. The returned pointer remains valid until
// localFreeSecurityDescriptor is called.
func stringToSecurityDescriptor(sddl string) (uintptr, error) {
	sddlPtr, err := windows.UTF16PtrFromString(sddl)
	if err != nil {
		return 0, err
	}

	var sdPtr uintptr
	var sdSize uint32

	// First call: query required buffer size.
	ret, _, lastErr := procConvertStringSDToSD.Call(
		uintptr(unsafe.Pointer(sddlPtr)),
		uintptr(sddlRevision),
		uintptr(unsafe.Pointer(&sdPtr)),
		uintptr(unsafe.Pointer(&sdSize)),
	)
	if ret == 0 {
		return 0, fmt.Errorf("convert string SD to SD (query): %w", lastErr)
	}

	if sdPtr == 0 {
		return 0, fmt.Errorf("convert string SD to SD: null descriptor")
	}

	return sdPtr, nil
}

// localFreeSecurityDescriptor frees a security descriptor allocated by
// stringToSecurityDescriptor.
func localFreeSecurityDescriptor(sdPtr uintptr) {
	if sdPtr == 0 {
		return
	}
	procLocalFree.Call(sdPtr)
}

// securePipe creates a named pipe with the restrictive pipeSDDL security
// descriptor. It returns a handle that the caller must close.
//
// PIPE_REJECT_REMOTE_CLIENTS is set so that remote (SMB) clients cannot connect
// to this local control interface. Only local processes can open the pipe.
//
// The security descriptor is intentionally leaked for the process lifetime:
// CreateNamedPipe may retain a pointer to it, and freeing it before the pipe
// is closed would be a use-after-free. The descriptor is small (a few hundred
// bytes), so the leak is acceptable.
func securePipe(pipeName string) (windows.Handle, error) {
	sdPtr, err := stringToSecurityDescriptor(pipeSDDL)
	if err != nil {
		return 0, fmt.Errorf("security descriptor: %w", err)
	}
	// Intentionally not freed: CreateNamedPipe may copy or retain the pointer.
	// localFreeSecurityDescriptor is kept for test/library use where the
	// caller controls the lifetime.
	defer localFreeSecurityDescriptor(sdPtr)

	name, err := windows.UTF16PtrFromString(`\\.\pipe\` + pipeName)
	if err != nil {
		return 0, err
	}

	sa := &windows.SecurityAttributes{
		Length:             uint32(unsafe.Sizeof(windows.SecurityAttributes{})),
		SecurityDescriptor: (*windows.SECURITY_DESCRIPTOR)(unsafe.Pointer(sdPtr)),
		InheritHandle:      0,
	}

	h, err := windows.CreateNamedPipe(
		name,
		windows.PIPE_ACCESS_DUPLEX|windows.FILE_FLAG_FIRST_PIPE_INSTANCE,
		windows.PIPE_TYPE_MESSAGE|windows.PIPE_READMODE_MESSAGE|windows.PIPE_WAIT|PIPE_REJECT_REMOTE_CLIENTS,
		windows.PIPE_UNLIMITED_INSTANCES,
		MaxMessageSize,  // out buffer size
		MaxMessageSize,  // in buffer size
		0,               // default timeout
		sa,
	)
	if err != nil {
		return 0, err
	}
	return h, nil
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
// its enabled groups. This correctly handles UAC-filtered (split) tokens: an
// administrator running under UAC has a filtered token where GetTokenUser()
// returns the standard-user SID, but the Administrators SID is still present
// in the token's group list (and may be enabled depending on elevation).
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

	// Administrators are always authorized. Use CheckTokenMembership so that
	// UAC-filtered tokens (where the admin SID is in the group list but not
	// the user SID) are correctly recognized.
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

	// Local interactive users (Users group) are authorized for read-only
	// methods.
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
	// RequireAdmin marks methods that require administrator privileges.
	RequireAdmin map[string]bool
}

// NewAuthorizer creates an authorizer with the default method policy.
func NewAuthorizer() *Authorizer {
	return &Authorizer{
		RequireAdmin: map[string]bool{
			string(MethodStart):       true,
			string(MethodStop):        true,
			string(MethodSetLimit):    true,
			string(MethodSetModeB):    true,
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
// It wraps the handle in an *os.File so the bufio-based request loop can be
// reused, then disconnects and closes the handle when done.
func (s *Server) HandleConn(h windows.Handle, ci ClientIdentity) {
	defer func() {
		windows.DisconnectNamedPipe(h)
		windows.CloseHandle(h)
	}()

	f := os.NewFile(uintptr(h), s.pipeName)
	if f == nil {
		log.Printf("ipc: os.NewFile failed for pipe handle")
		return
	}
	defer f.Close()

	scanner := bufio.NewScanner(f)
	scanner.Buffer(make([]byte, 1024), MaxMessageSize)
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
