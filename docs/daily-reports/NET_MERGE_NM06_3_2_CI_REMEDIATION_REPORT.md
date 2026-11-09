# MultiNet CyberShield AI — Milestone NM-06.3.2 Report
## Windows Mutex Correctness, Go Test Remediation & Rust Workspace Build

**Date:** 2026-11-09  
**Repository:** `usingh312-dotcom/multinet-cybershield-ai`  
**Host Environment:** Windows 11 Pro 64-bit (`VerifiedAndReputablePolicyState = 1` — Smart App Control Enabled)  
**CI Target:** GitHub Actions (`windows-latest` / Windows Server 2025)  
**Local Toolchains:** Go 1.27.3 (`go1.27.3 windows/amd64`), Rust 1.99.1 (`stable-x86_64-pc-windows-msvc`)  

---

### 1. Root Cause of Previous Go Test Failures

In GitHub Actions CI Run #2 (`42624545563`), the `go-build-and-test` job failed at `go test ./...` due to three distinct issues:
1. **Flawed `ERROR_ALREADY_EXISTS` handling in `modeowner_windows.go`:**
   When `CreateMutexW` returned an existing named mutex handle with `ERROR_ALREADY_EXISTS`, the old code immediately closed the handle and returned `(false, nil)`. This prematurely aborted acquisition instead of proceeding to `WaitForSingleObject` to await ownership or test for abandonment.
2. **Thread affinity mismatch (`ReleaseMutex` error 288):**
   Windows named mutexes are strictly thread-affine: only the thread that called `WaitForSingleObject` and acquired ownership may call `ReleaseMutex`. In Go's M:N scheduler, arbitrary goroutines were calling `Release()`, which called `ReleaseMutex` on a different OS thread, causing `ReleaseMutex` to fail with `ERROR_NOT_OWNER` (288).
3. **Global mutex contention and lack of test isolation:**
   Every test across `engine_adapter_test.go`, `coordinator_test.go`, and `modeowner_test.go` targeted the single global mutex name `Global\multinet-netmerge-mode-owner-v1`. When tests ran concurrently or sequentially, unreleased handles or timing overlaps contaminated subsequent tests.

---

### 2. Mutex DACL & Security Attributes Verification

- **Security Descriptor:** `mutexSDDL = "D:P(A;;GA;;;BA)(A;;GA;;;S-1-5-18)"` restricts ownership to Administrators and LocalSystem.
- **Type-Safe Allocation:** `windows.SecurityDescriptorFromString(mutexSDDL)` converts the SDDL string to a self-relative `*windows.SECURITY_DESCRIPTOR` allocated on the Go heap.
- **SecurityAttributes Lifetime:**
  ```go
  sa := &windows.SecurityAttributes{
      Length:             uint32(unsafe.Sizeof(windows.SecurityAttributes{})),
      SecurityDescriptor: sd,
      InheritHandle:      0,
  }
  ```
  The descriptor is passed directly to `windows.CreateMutex`. `SecurityDescriptor` is non-nil and valid for the entire call.

---

### 3. Dedicated OS Thread Ownership Architecture

To guarantee strict Win32 thread affinity regardless of Go's goroutine scheduling:
- An `Owner` instance starts a dedicated worker goroutine.
- The worker invokes `runtime.LockOSThread()` at entry and retains it for its entire operational lifetime.
- The worker listens on a serialized command channel (`cmdCh`) and performs every Win32 call (`CreateMutex`, `WaitForSingleObject`, `ReleaseMutex`, `CloseHandle`) exclusively on that single thread.
- Coordinator, IPC handlers, process monitor, and test callers communicate with `Owner` via non-blocking channels, guaranteeing thread-safe acquisition, wait, release, and shutdown.
- On `Owner.Close()` or worker termination, any held mutex is released, handles are closed, and `runtime.UnlockOSThread()` is executed.

---

### 4. Abandoned Mutex Recovery Evidence

A genuine abandoned-mutex test was implemented in `TestAbandonedMutexRecovery`:
1. The parent test spawns a separate helper process (`os.Args[0]`) with `GO_TEST_HELPER_ABANDON_MUTEX=1`.
2. The child process acquires the named mutex via `CreateMutex` + `WaitForSingleObject`.
3. The child terminates abruptly via `os.Exit(0)` while holding the mutex.
4. The parent process calls `o.TryAcquire(5 * time.Second)`.
5. Windows detects the dead thread, marks the mutex as abandoned, and returns `WAIT_ABANDONED` (`0x80`).
6. The dedicated owner recognizes `WAIT_ABANDONED`, establishes valid ownership, logs recovery, and successfully releases the mutex.

---

### 5. Rust Speed-Test Borrowing Corrections

In `crates/speed-test/src/lib.rs`:
- The callback parameter was defined as `FnMut(&SpeedTestProgress)`.
- At 6 call sites (lines 206, 230, 269, 292, 351, 384), owned `SpeedTestProgress::new(...)` values were passed instead of references.
- **Correction:** Prefixed all 6 calls with `&`:
  ```rust
  progress(&SpeedTestProgress::new(...));
  ```
- Measurement logic, stages, and progress fractions were preserved intact.

---

### 6. Windows Interface-Discovery Lint Corrections

In `crates/interface-discovery/src/windows.rs`:
- Resolved seven `non_upper_case_globals` warnings in `map_oper_status` by defining uppercase constants:
  `IF_OPER_STATUS_UP`, `IF_OPER_STATUS_DOWN`, `IF_OPER_STATUS_TESTING`, `IF_OPER_STATUS_UNKNOWN`, `IF_OPER_STATUS_DORMANT`, `IF_OPER_STATUS_NOT_PRESENT`, `IF_OPER_STATUS_LOWER_LAYER_DOWN`.
- Preserved exact Windows status mappings; `cargo check -p interface-discovery` passes with 0 warnings.

---

### 7. Dependency Resolution (`axum-server` & `hyper-util`)

- **Analysis:** `axum-server = "=0.7.1"` had pinned legacy transitive dependencies (`tower v0.4.13`, `pin-project v1.1.13`) that conflicted with `hyper-util v0.1.21`'s server abstractions.
- **Resolution:**
  - Upgraded workspace dependency in `Cargo.toml`:
    `axum-server = { version = "0.7.3", default-features = false, features = ["tls-rustls-no-provider"] }`
  - Unignored and committed `Cargo.lock` to guarantee deterministic builds in CI.
  - Cargo locked `axum-server 0.7.3` and removed conflicting redundant crates while retaining `rustls(ring)` and loopback self-signed TLS generation.

---

### 8. Production Process Authorization Security Review

In `services/net-merge/internal/ipc/pipesec_windows.go`:
- Removed all production whitelist exceptions for `.test.exe`, `go-build`, and `cargo`.
- Production `verifyRustProcessIdentity` strictly permits approved bonding-engine executables (`bonding-engine.exe`, `multinet-bonding.exe`, `multinet.exe`).
- Test binaries inject mock identity validators via `SetProcessValidator()` exclusively in test files.
- Adjusted `Authorizer`: `set_mode_b` is authorized for local interactive users (Tauri controller) because the target PID and executable image are strictly validated against approved bonding engines, while `start`, `stop`, and `set_limit` remain restricted to administrators.

---

### 9. Binary Checksum

- **Binary:** `services/net-merge/net-merge-service.exe`
- **Size:** 12,711,936 bytes
- **SHA256:** `76EE3238AE2F189DB446C00A7CA184DE3663F4731DA0FE7BDDA487ACC3CDA5DC`

---

### 10. Acceptance Status

- [x] Windows mutex implementation handles `ERROR_ALREADY_EXISTS`, `WAIT_OBJECT_0`, `WAIT_ABANDONED`, `WAIT_TIMEOUT`.
- [x] Mutex DACL (`mutexSDDL`) verified and applied via `SecurityAttributes.SecurityDescriptor`.
- [x] Dedicated OS thread ownership implemented via `runtime.LockOSThread()` worker.
- [x] Abandoned-mutex recovery verified with helper process test.
- [x] Rust speed-test callback reference errors resolved.
- [x] Windows interface-discovery naming warnings resolved.
- [x] `axum-server` dependency compatibility resolved and `Cargo.lock` committed.
- [x] Production process authorization exceptions eliminated.
- [x] Go service binary built and SHA256 recorded.

