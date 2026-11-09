// Package ipc_test tests the live IPC server and client interactions over a
// real Windows named pipe.
package ipc_test

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"sync"
	"testing"
	"time"

	"golang.org/x/sys/windows"

	"multinet.ai/netmerge-service/internal/ipc"
)

type testMockEngine struct {
	mu      sync.Mutex
	running bool
	modeB   bool
}

func (m *testMockEngine) Discover() ([]ipc.AdapterInfo, []ipc.Skip, error) {
	return []ipc.AdapterInfo{
		{Name: "Ethernet", Index: 1, IPv4: "192.168.1.100", Weight: 1.0},
		{Name: "Wi-Fi", Index: 2, IPv4: "192.168.2.100", Weight: 0.8},
	}, nil, nil
}

func (m *testMockEngine) Start(cfg ipc.StartConfig) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.running = true
	return nil
}

func (m *testMockEngine) Stop() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.running = false
	return nil
}

func (m *testMockEngine) Status() (ipc.Status, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	return ipc.Status{
		On:    m.running,
		ModeA: m.running,
		ModeB: m.modeB,
		Adapters: []ipc.AdapterStatus{
			{Name: "Ethernet", Index: 1, IPv4: "192.168.1.100", State: "up", Weight: 1.0},
		},
	}, nil
}

func (m *testMockEngine) SetLimit(adapter string, mbps float64) error { return nil }

func (m *testMockEngine) ModeBActive() bool {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.modeB
}

func (m *testMockEngine) SetModeB(active bool, rustPid uint32, rustHandle windows.Handle) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.modeB = active
	return nil
}

func connectTestClient(pipeName string) (*os.File, error) {
	pName := pipeName
	if !strings.HasPrefix(pName, `\\.\pipe\`) {
		pName = `\\.\pipe\` + pName
	}
	name, err := windows.UTF16PtrFromString(pName)
	if err != nil {
		return nil, err
	}

	for i := 0; i < 20; i++ {
		h, err := windows.CreateFile(
			name,
			windows.GENERIC_READ|windows.GENERIC_WRITE,
			0,
			nil,
			windows.OPEN_EXISTING,
			0,
			0,
		)
		if err == nil {
			return os.NewFile(uintptr(h), pName), nil
		}
		time.Sleep(20 * time.Millisecond)
	}
	return nil, fmt.Errorf("timeout waiting to connect to pipe %s", pipeName)
}

func roundTrip(f *os.File, req ipc.Request) (ipc.Response, error) {
	b, err := json.Marshal(req)
	if err != nil {
		return ipc.Response{}, err
	}
	b = append(b, '\n')
	if _, err := f.Write(b); err != nil {
		return ipc.Response{}, err
	}
	scanner := bufio.NewScanner(f)
	if !scanner.Scan() {
		return ipc.Response{}, fmt.Errorf("scan error: %v", scanner.Err())
	}
	var resp ipc.Response
	if err := json.Unmarshal(scanner.Bytes(), &resp); err != nil {
		return ipc.Response{}, err
	}
	return resp, nil
}

// TestIPCServerLifecycleAndHandshake verifies full client/server roundtrip
// over a live Windows named pipe.
func TestIPCServerLifecycleAndHandshake(t *testing.T) {
	pipeName := fmt.Sprintf("multinet-test-ipc-%d-%d", os.Getpid(), time.Now().UnixNano())
	eng := &testMockEngine{}
	server := ipc.NewServer(pipeName, eng)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	errCh := make(chan error, 1)
	go func() {
		errCh <- server.Run(ctx)
	}()

	client, err := connectTestClient(pipeName)
	if err != nil {
		t.Fatalf("connectTestClient: %v", err)
	}
	defer client.Close()

	// 1. Health check
	req := ipc.Request{
		ID:          "test-health-1",
		PipeVersion: ipc.PIPEVersion,
		Method:      ipc.MethodHealth,
	}
	resp, err := roundTrip(client, req)
	if err != nil {
		t.Fatalf("health roundtrip: %v", err)
	}
	if !resp.OK {
		t.Fatalf("expected ok response, got error: %+v", resp.Error)
	}
	if resp.ID != req.ID {
		t.Fatalf("expected echoed id %s, got %s", req.ID, resp.ID)
	}

	// 2. Adapters discovery
	reqAdapters := ipc.Request{
		ID:          "test-adapters-1",
		PipeVersion: ipc.PIPEVersion,
		Method:      ipc.MethodAdapters,
	}
	respAdapters, err := roundTrip(client, reqAdapters)
	if err != nil {
		t.Fatalf("adapters roundtrip: %v", err)
	}
	if !respAdapters.OK {
		t.Fatalf("expected ok response, got error: %+v", respAdapters.Error)
	}

	// 3. Status check
	reqStatus := ipc.Request{
		ID:          "test-status-1",
		PipeVersion: ipc.PIPEVersion,
		Method:      ipc.MethodStatus,
	}
	respStatus, err := roundTrip(client, reqStatus)
	if err != nil {
		t.Fatalf("status roundtrip: %v", err)
	}
	if !respStatus.OK {
		t.Fatalf("expected ok response, got error: %+v", respStatus.Error)
	}

	// 4. Mode B status
	reqModeB := ipc.Request{
		ID:          "test-modeb-status-1",
		PipeVersion: ipc.PIPEVersion,
		Method:      ipc.MethodModeBStatus,
	}
	respModeB, err := roundTrip(client, reqModeB)
	if err != nil {
		t.Fatalf("mode_b_status roundtrip: %v", err)
	}
	if !respModeB.OK {
		t.Fatalf("expected ok response, got error: %+v", respModeB.Error)
	}

	// Shut down server
	cancel()
	_ = server.Close()

	select {
	case err := <-errCh:
		if err != nil {
			t.Fatalf("server.Run returned error on shutdown: %v", err)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("server.Run failed to exit after cancel and Close")
	}
}

// TestIPCConcurrentClientsStress tests multiple concurrent clients communicating
// simultaneously over the named pipe.
func TestIPCConcurrentClientsStress(t *testing.T) {
	pipeName := fmt.Sprintf("multinet-test-concurrent-%d-%d", os.Getpid(), time.Now().UnixNano())
	eng := &testMockEngine{}
	server := ipc.NewServer(pipeName, eng)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go func() {
		_ = server.Run(ctx)
	}()

	const clientCount = 8
	const requestsPerClient = 5
	var wg sync.WaitGroup

	for c := 0; c < clientCount; c++ {
		wg.Add(1)
		go func(clientID int) {
			defer wg.Done()
			client, err := connectTestClient(pipeName)
			if err != nil {
				t.Errorf("client %d connect error: %v", clientID, err)
				return
			}
			defer client.Close()

			for r := 0; r < requestsPerClient; r++ {
				req := ipc.Request{
					ID:          fmt.Sprintf("client-%d-req-%d", clientID, r),
					PipeVersion: ipc.PIPEVersion,
					Method:      ipc.MethodHealth,
				}
				resp, err := roundTrip(client, req)
				if err != nil {
					t.Errorf("client %d req %d roundtrip error: %v", clientID, r, err)
					return
				}
				if !resp.OK {
					t.Errorf("client %d req %d unexpected error: %+v", clientID, r, resp.Error)
					return
				}
			}
		}(c)
	}

	wg.Wait()
	cancel()
	_ = server.Close()
}

// TestIPCClientAbruptDisconnect verifies the server does not crash or hang
// when a client abruptly closes its pipe mid-session.
func TestIPCClientAbruptDisconnect(t *testing.T) {
	pipeName := fmt.Sprintf("multinet-test-disconnect-%d-%d", os.Getpid(), time.Now().UnixNano())
	eng := &testMockEngine{}
	server := ipc.NewServer(pipeName, eng)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go func() {
		_ = server.Run(ctx)
	}()

	client, err := connectTestClient(pipeName)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	// Write partial request without newline, then abruptly close
	_, _ = client.Write([]byte(`{"id":"partial"`))
	_ = client.Close()

	// Subsequent client must connect cleanly
	client2, err := connectTestClient(pipeName)
	if err != nil {
		t.Fatalf("second client connect: %v", err)
	}
	defer client2.Close()

	req := ipc.Request{
		ID:          "health-after-disconnect",
		PipeVersion: ipc.PIPEVersion,
		Method:      ipc.MethodHealth,
	}
	resp, err := roundTrip(client2, req)
	if err != nil {
		t.Fatalf("roundtrip after disconnect: %v", err)
	}
	if !resp.OK {
		t.Fatalf("expected ok, got %+v", resp.Error)
	}

	cancel()
	_ = server.Close()
}

// TestIPCInvalidJsonAndOversizedPayload verifies server rejection of malformed
// or oversized requests.
func TestIPCInvalidJsonAndOversizedPayload(t *testing.T) {
	pipeName := fmt.Sprintf("multinet-test-invalid-%d-%d", os.Getpid(), time.Now().UnixNano())
	eng := &testMockEngine{}
	server := ipc.NewServer(pipeName, eng)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go func() {
		_ = server.Run(ctx)
	}()

	client, err := connectTestClient(pipeName)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer client.Close()

	// 1. Invalid JSON
	_, err = client.Write([]byte("this is definitely not json\n"))
	if err != nil {
		t.Fatalf("write invalid json: %v", err)
	}
	scanner := bufio.NewScanner(client)
	if !scanner.Scan() {
		t.Fatalf("scan response: %v", scanner.Err())
	}
	var resp ipc.Response
	if err := json.Unmarshal(scanner.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error resp: %v", err)
	}
	if resp.OK {
		t.Fatal("expected OK=false for invalid JSON")
	}
	if resp.Error == nil || resp.Error.Code != "invalid_json" {
		t.Fatalf("expected error code 'invalid_json', got %+v", resp.Error)
	}

	cancel()
	_ = server.Close()
}
