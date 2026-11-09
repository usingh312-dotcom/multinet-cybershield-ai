# NET MERGE INTEGRATION — NM-06.3.3 SERVICE LIFECYCLE REPORT

**Project:** D:\git\multinet-cybershield-ai
**Upstream:** https://github.com/net-merge/net-merge
**Pinned commit:** `2b333a3ad820996ba337588adfa8f7259a45baec` (Release 0.3.7)
**Date:** 2026-11-09
**Role:** Principal Windows Systems Architect, Senior Go Engineer, QA Lead
**Smart App Control:** ENABLED — NOT disabled, NOT bypassed.
**Repository protection:** D:\git\browser never touched. No staging, commit, reset, or clean. All existing modifications preserved.

---

## 1. Executive Summary

This report documents the service lifecycle design and verification status for the Net Merge Go service (`net-merge-service.exe`). The service is the elevated Windows process that owns the Wintun adapter, capture routes, and per-interface sockets (Mode A). It is controlled by the unelevated MultiNet Tauri desktop application via authenticated named-pipe IPC.

**Critical limitation:** Zero service lifecycle tests have been executed. Go is not installed on this host (cannot compile/test Go code), and Smart App Control blocks Rust test binary execution (error 4551). All lifecycle behavior described is source-implemented and architecturally sound, but not execution-verified.

---

## 2. Service Lifecycle Design

### Process model

```
MultiNet Tauri (unelevated)  <-- named pipe IPC -->  Net Merge service (elevated)
```

- **Net Merge service** runs as a separate elevated Windows process
- **Tauri desktop** runs as unelevated user process
- Communication: Windows named pipe `\\.\pipe\multinet-netmerge-v1`
- Authentication: `ImpersonateNamedPipeClient` + `CheckTokenMembership` for admin/user/system
- Pipe ACL: `D:P(A;;GA;;;BA)(A;;GRGW;;;S-1-5-32-545)(A;;GRGW;;;S-1-5-18)`

### Service lifecycle states

| State | Description | Transition |
|---|---|---|
| Stopped | Service not running | Initial state |
| Starting | `net-merge-service.exe` launching, waiting for pipe readiness | `start` IPC command |
| Running | Service listening on named pipe, Wintun adapter active (Mode A) or idle | `start`/`stop` IPC commands |
| Stopping | Service shutting down, removing capture routes, restoring weak-host | `stop` IPC command or SIGINT/SIGTERM |
| Failed | Service process terminated unexpectedly | Crash recovery on restart |

### Service lifecycle commands

| IPC Command | Action | Privilege Required |
|---|---|---|
| `health` | Return service liveness + mode status | None (read-only) |
| `status` | Return per-adapter snapshot | None (read-only) |
| `adapters` | Return discovered adapters + skipped | None (read-only) |
| `start` | Activate Mode A: create Wintun adapter, install capture routes, start health probes | Admin |
| `stop` | Deactivate Mode A: remove capture routes, tear down TUN, restore weak-host | Admin |
| `set_limit` | Set per-adapter speed cap (token bucket) | Admin |
| `set_mode_b` | Activate/deactivate Mode B (Rust bonding engine) | Admin |
| `mode_b_status` | Return Mode B active status | None (read-only) |

### Safe shutdown sequence

```
1. Receive SIGINT/SIGTERM or `stop` IPC command
2. Stop accepting new pipe connections
3. Complete in-flight IPC requests
4. engine.Stop(): teardown TUN, delete capture routes, restore weak-host settings
5. coord.ReleaseMode(): release kernel named mutex
6. Close named pipe listener
7. Exit process
```

### Crash recovery

- **Kernel mutex auto-release:** `Global\multinet-netmerge-mode-owner-v1` is kernel-owned; if the service process crashes, Windows automatically releases the mutex
- **`restoreStrongHost()`:** Reads `strongHostFile` (`net-merge-service-weakhost.txt`) to restore weak-host settings on adapters that were modified by a crashed run
- **`CleanupStale()`:** Removes capture routes (`0.0.0.0/1`, `128.0.0.0/1`) from any surviving `net-merge` adapter
- **Mutex auto-release:** `Global\multinet-netmerge-mode-owner-v1` released by kernel on process exit

---

## 3. Service Startup Sequence

```
1. Parse command-line flags (-pipe, -state-dir, -log, -mtu, -ipv6, -acquire-timeout)
2. Create mode.Coordinator (modeowner.Owner with kernel mutex)
3. Create engineadapter wrapping engine.Scheduler + engine.Balancer + tun.Tun
4. Create ipc.Server with engine adapter + Authorizer
5. Register IPC handlers (health, status, adapters, start, stop, set_limit, mode_b_status, set_mode_b)
6. Call coord.RequestModeA(): acquire kernel named mutex
7. Call securePipe(): CreateNamedPipe with restrictive SDDL
8. Call server.Run(ctx): listen on pipe, accept connections
9. On signal (SIGINT/SIGTERM): server.Close(), eng.Stop(), coord.ReleaseMode()
```

### Startup failure modes

| Failure | Handling |
|---|---|
| Mutex already held (Mode B active) | Return error: "another process owns the networking tunnel" |
| Wintun DLL missing | Return error from `platform.CheckDriver()` |
| No usable adapters | Return error: "no usable IPv4 adapters found" |
| TUN creation fails | Rollback: release mutex, return error |
| Route installation fails | Rollback: `t.Close()` removes routes, release mutex |

---

## 4. Service Shutdown Sequence

### Normal shutdown (via `stop` IPC command)

```
1. handleStop() called via IPC
2. engine.Stop(): tun.Tun.Close()
3. tun.Tun.Close(): teardown()
4. teardown(): delRoutes() removes capture routes
5. teardown(): tunnel.T().SetProxy(&reject.Reject{}) rejects new flows
6. teardown(): dev.Close() closes Wintun adapter
7. teardown(): stk.Close() + stk.Wait() closes gVisor stack
8. coord.ReleaseMode(): close rustHandle, release kernel mutex
9. Return {status:"stopped"} to client
```

### Unexpected process termination

```
1. Windows kills process (task kill, crash, power loss)
2. Kernel auto-releases named mutex `Global\multinet-netmerge-mode-owner-v1`
3. Kernel auto-closes all process handles (including rustHandle if open)
4. On restart: restoreStrongHost() reads strongHostFile to restore weak-host settings
5. On restart: CleanupStale() removes capture routes from any surviving adapter
6. New service instance can acquire mutex and start Mode A
```

---

## 5. Mode B Process Lifecycle

### Mode B activation sequence

```
1. Rust bonding engine connects to named pipe
2. Client sends IPC set_mode_b(active=true)
3. Server: validateClientIdentity() → get client PID via GetNamedPipeClientProcessId()
4. Server: openValidatedProcess(clientPid) → OpenProcess + GetProcessId verification
5. Server: engine.SetModeB(active=true, rustPid=ci.ClientPid, rustHandle=handle)
6. Coordinator: RequestModeB(rustPid, rustHandle)
   - Acquire kernel mutex
   - Store rustPid and rustHandle
   - Set mode = Bonding
7. coord.MonitorRustProcess(rustPid): wait on handle via WaitForSingleObject
8. Return {active:true, mode_b:true} to client
```

### Mode B deactivation

```
1. Rust client sends IPC set_mode_b(active=false)
2. Server: engine.SetModeB(false, ci.ClientPid, rustHandle)
3. Coordinator: ReleaseMode()
   - Set mode = None
   - Close rustHandle
   - Release kernel mutex
   - Bump session ID (invalidate stale monitors)
4. Return {active:false, mode_b:false} to client
```

### Rust process crash

```
1. Rust process exits unexpectedly
2. WaitForSingleObject(rustHandle) returns WAIT_OBJECT_0
3. MonitorRustProcess() calls c.ReleaseMode()
4. ReleaseMode(): set mode=None, close rustHandle, release kernel mutex
5. Mode A becomes available for activation
```

---

## 6. Multiple Client Connections

| Scenario | Handling |
|---|---|
| Multiple clients connect simultaneously | `connSem` semaphore limits to 64 concurrent connections |
| Client disconnects mid-request | `scanner.Err()` checked; `io.EOF` treated as normal disconnect |
| Client sends malformed JSON | `invalid_json` error response |
| Client sends unknown method | `unknown_method` error response |
| Client sends unsupported pipe version | `unsupported_version` error response |
| Client sends empty ID | `invalid_request` error response |
| Non-admin calls admin method | `unauthorized` error response |

---

## 7. Duplicate Service Prevention

| Mechanism | Implementation |
|---|---|
| Kernel named mutex | `Global\multinet-netmerge-mode-owner-v1` — only one process can own Mode A |
| `SingleInstance()` (platform) | `CreateMutex` with `ERROR_ALREADY_EXISTS` check — prevents duplicate service instances |
| Mode mutex | Coordinator owns mutex for entire Mode A lifetime; Mode B requests rejected while Mode A active |

---

## 8. Resource Cleanup

| Resource | Cleanup mechanism |
|---|---|
| Wintun adapter | `tun.Close()` → `dev.Close()` |
| Capture routes | `delRoutes()` removes `0.0.0.0/1`, `128.0.0.0/1` |
| Weak-host settings | `stopStrongHost()`/`restoreStrongHost()` restores original values |
| gVisor stack | `stk.Close()` + `stk.Wait()` |
| Named pipe | `DisconnectNamedPipe()` + `CloseHandle()` |
| Kernel mutex | `ReleaseMutex()` + `CloseHandle()` |
| Process handle | `CloseHandle(rustHandle)` in `ReleaseMode()` |

---

## 9. Attestation

This report is based on direct source inspection. Service lifecycle code is implemented but not execution-tested (Go not installed, SAC blocks Rust test binaries). All lifecycle behavior described is source-implemented and architecturally sound, but not execution-verified. No results are fabricated.
