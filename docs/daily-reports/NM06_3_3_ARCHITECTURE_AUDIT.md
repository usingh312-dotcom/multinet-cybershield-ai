# MultiNet CyberShield AI — Milestone NM-06.3.3 Architecture Audit

**Project:** MultiNet CyberShield AI  
**Repository:** `usingh312-dotcom/multinet-cybershield-ai`  
**Branch:** `master`  
**Milestone:** NM-06.3.3 Architecture & Production Integration Audit  
**Date:** November 9, 2026  
**Auditor:** Principal Systems Architect, Senior Go/Rust Systems Engineer, Security Architect  

---

## 1. Executive Summary

This architecture audit evaluates the complete MultiNet CyberShield AI networking architecture across the Go service, Rust workspace, Tauri desktop application, IPC layer, and network bonding engines.

### Core Components Audited:
1. **MultiNet Tauri Desktop Application (`apps/desktop/src-tauri`)**: Unelevated desktop shell built on Tauri 2. Controls Net Merge via IPC, hosts diagnostics background tasks, SQLite migrations, and UI controls.
2. **Net Merge Go Service (`services/net-merge`)**: Elevated Windows service running with LocalSystem / Administrator rights. Manages Wintun virtual adapters, capture routes (`0.0.0.0/1`, `128.0.0.0/1`), token-bucket speed limits, and connection-level load balancing (Mode A).
3. **Rust Bonding Engine (`crates/bonding-engine`)**: Packet-level UDP bonding engine implementing Weighted Deficit Round Robin (WDRR) scheduling, PathManager capacity/loss/latency tracking, and authenticated HMAC-SHA256 framing (Mode B).
4. **Named-Pipe IPC Layer (`services/net-merge/internal/ipc`, `crates/netmerge-client`)**: Versioned JSON contract over a secured Windows named pipe (`\\.\pipe\multinet-netmerge-v1`) with client process identity validation.
5. **Mode A / Mode B Mutual Exclusion (`services/net-merge/internal/mode`, `internal/modeowner`)**: Kernel named mutex (`Global\multinet-netmerge-mode-owner-v1`) guaranteeing that connection-level load balancing and packet-level bonding are never active simultaneously.

---

## 2. Go Service Architecture (`services/net-merge/`)

### 2.1 Module Layout & Responsibilities

```
services/net-merge/
├── go.mod                          # Module: multinet.ai/netmerge-service (Go 1.27.3)
├── go.sum
├── main.go                         # Windows service entry point & flag parsing
└── internal/
    ├── engine/                     # Upstream connection balancer
    │   ├── balancer.go             # TCP/UDP flow relay + failover
    │   ├── limit.go                # Per-adapter token bucket rate caps
    │   ├── probe.go                # TCP health probes (1.1.1.1 / 8.8.8.8)
    │   └── sched.go                # Weighted least-busy-flows scheduler
    ├── engineadapter/              # IPC-facing adapter
    │   └── engine_adapter.go       # Implements ipc.Engine interface
    ├── ipc/                        # Named-pipe IPC server & security
    │   ├── contracts.go            # Request/Response/Method contracts
    │   ├── pipesec_windows.go      # CreateNamedPipeW + SDDL + client validation
    │   ├── server.go               # Named-pipe accept loop, router, auth
    │   └── server_integration_test.go # End-to-end pipe integration tests
    ├── lifecycle/                  # Service start/stop/readiness lifecycle
    ├── logx/                       # Leveled logging
    ├── mode/                       # Mode A / Mode B mutual exclusion coordinator
    │   ├── coordinator.go          # Authoritative coordinator & process monitor
    │   ├── coordinator_test.go     # Coordinator unit & lifecycle tests
    │   └── reliability_test.go     # Rapid switching & fault-injection tests
    ├── modeowner/                  # Dedicated OS thread Windows mutex owner
    │   ├── modeowner_windows.go    # CreateMutexW + LockOSThread worker loop
    │   └── modeowner_test.go       # Mutex acquisition & abandoned mutex tests
    ├── nics/                       # Adapter discovery & traffic counters
    │   ├── nics.go                 # DiscoverNicsDetailed via GetAdaptersAddresses
    │   ├── info_windows.go         # Gateway families, interface types
    │   └── counters_windows.go     # Octet counters via GetIfEntry2Ex
    └── tun/                        # Wintun virtual network adapter
        ├── tun.go                  # Tun lifecycle + gVisor userland TCP/IP stack
        ├── tun_windows.go          # Route management & weak-host configuration
        └── forward.go              # TCP/UDP forwarders over gVisor stack
```

---

## 3. Rust Workspace Architecture (`crates/` and `relay/`)

| Crate | Purpose | Key Modules |
|---|---|---|
| `bonding-engine` | Packet-level UDP bonding engine | `scheduler.rs` (WDRR), `path_manager.rs`, `metrics.rs` |
| `tunnel-protocol` | Authenticated framing | `framing.rs` (HMAC-SHA256, replay protection) |
| `netmerge-client` | Named-pipe client for Go service | `lib.rs` (NetMergeClient), `contracts.rs`, `pipe.rs` |
| `interface-discovery` | Adapter discovery on Windows | `windows.rs` (`GetAdaptersAddresses`) |
| `network-monitor` | Real-time traffic rate tracker | `lib.rs` (`GetIfTable2` counters) |
| `speed-test` | Multi-stream HTTPS speed test | `lib.rs` (transfer engine, progress callbacks) |
| `apps/desktop/src-tauri` | Tauri 2 desktop application | `main.rs`, `diagnostics.rs`, `db.rs` |
| `relay/speedtest-server` | Standalone HTTPS speed test server | `main.rs` (Axum + Rustls + ring) |

---

## 4. Key Architectural Findings & Defect Resolutions

### 4.1 Named-Pipe Path Concatenation Bug
- **Location:** `services/net-merge/internal/ipc/pipesec_windows.go:93`
- **Issue:** `securePipe()` previously concatenated `\\.\pipe\` unconditionally. When called with the standard flag default `ipc.DefaultPipeName` (`\\.\pipe\multinet-netmerge-v1`), it produced `\\.\pipe\\\.\pipe\multinet-netmerge-v1`, which fails `CreateNamedPipeW`.
- **Resolution:** Added path prefix normalization check `if !strings.HasPrefix(pName, "\\\\.\\pipe\\")`.

### 4.2 Server Accept Loop Exit on Close
- **Location:** `services/net-merge/internal/ipc/server.go:130`
- **Issue:** When `Server.Close()` closed the pipe handle, the blocking `ConnectNamedPipe` unblocked with an error, but the loop would log the error and attempt to recreate the pipe unless `s.closed` was checked.
- **Resolution:** Added `isClosed()` thread-safe check and `windows.CancelIoEx` invocation on shutdown.

### 4.3 Missing `Clone` Derivation on IPC Client Handle
- **Location:** `crates/netmerge-client/src/lib.rs:54`
- **Issue:** `NetMergeClient` wrapped `Arc<NetMergeClientInner>` but lacked `#[derive(Clone)]`, preventing cloning across concurrent desktop tasks.
- **Resolution:** Added `#[derive(Clone)]` to `NetMergeClient`.

---

## 5. Security Architecture & Boundary Isolation

1. **Privilege Boundary:**
   - The desktop application runs unelevated (standard user context).
   - The Go service runs elevated and enforces strict named-pipe DACL: `D:P(A;;GA;;;BA)(A;;GRGW;;;S-1-5-32-545)(A;;GRGW;;;S-1-5-18)`.
   - Anonymous and network clients are rejected (`PIPE_REJECT_REMOTE_CLIENTS`).
2. **Client Process Identity Binding:**
   - The Go service determines client PID via `GetNamedPipeClientProcessId(pipe)`.
   - The PID is validated via `OpenProcess` and `GetProcessId` before registering Mode B ownership.
3. **Role-Based IPC Authorization:**
   - Standard users can execute read-only methods (`health`, `status`, `adapters`, `mode_b_status`).
   - Only Administrators and LocalSystem can execute privileged methods (`start`, `stop`, `set_limit`, `set_mode_b`).
