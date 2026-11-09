# Smart App Control / Code Integrity Build Blocker — Diagnostic Report

- Date: 2026-11-09
- Repository: `D:\git\multinet-cybershield-ai`
- Scope: Diagnostics Final Implementation Sprint (Traffic Analytics + Speed Test)
- Status: **Rust compilation, tests and desktop runtime verification are BLOCKED by host code integrity enforcement.**

## 1. Summary

Windows Code Integrity is enforcing policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}` on this host.
Every newly generated unsigned Rust build-script executable, proc-macro DLL, test binary and
application executable is blocked at process creation (os error 4551). Signed Windows binaries
continue to run normally. The user directed that host security must not be modified or bypassed,
so all Rust build/test/runtime work is paused and marked blocked.

Windows Smart App Control is enabled: `HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy`
`VerifiedAndReputablePolicyState = 1` (enforcement). The session is not elevated.

## 2. Toolchain and host inventory

| Item | Value |
| --- | --- |
| rustc | 1.99.1 (58cf8f9c3 2026-10-13), LLVM 23.1.1 |
| cargo | 1.99.1 (5f94df478 2026-08-27) |
| rustup home | `C:\Users\utkar\.rustup` |
| Toolchains installed | `stable-x86_64-pc-windows-msvc` only (active, default) |
| Installed targets | `x86_64-pc-windows-msvc` only |
| Host triple | `x86_64-pc-windows-msvc` |
| OS | Windows 11 Home Single Language, build 26200 |
| Elevation | Not administrator |
| SAC state | `VerifiedAndReputablePolicyState = 1` |
| CI policy ID | `{0283ac0f-fff1-49ae-ada1-8a933130cad6}` |
| Event log source | `Microsoft-Windows-CodeIntegrity/Operational` |

`cargo` and `rustc` themselves are signed and start; the failure is at execution of
freshly linked unsigned artifacts produced by the toolchain.

## 3. Code Integrity evidence (events 3077 / 3033)

Events were readable without elevation. Representative entries (all on 2026-11-09):

| Time | Blocked executable | Loading process |
| --- | --- | --- |
| 11:11:51 | `C:\Users\utkar\AppData\Local\Temp\opencode\wdac-test\hello.exe` | `powershell.exe` |
| 11:11:05 | `target\debug\build\native-tls-ef76e8cc0053a851\build-script-build.exe` | `cargo.exe` |
| 11:11:02 | `target\debug\build\rustls-0c038394348bef9d\build-script-build.exe` | `cargo.exe` |
| 11:08:11 | `target\debug\build\fs-err-007ae85ed31a0194\build-script-build.exe` | `cargo.exe` |
| 11:08:09 | `target\debug\build\rustls-66af0fd4f903e1a2\build-script-build.exe` | `cargo.exe` |
| 10:58:27 | `target\debug\build\getrandom-faab77a30411d148\build-script-build.exe` | `powershell.exe` |
| 10:58:24 | `target\debug\build\getrandom-af1a3439ada8d873\build-script-build.exe` | `powershell.exe` |
| 10:58:22 | `target\debug\build\getrandom-0eea6aa5ba946a34\build-script-build.exe` | `powershell.exe` |
| 10:58:20 | `target\debug\build\serde_core-f40a17f0b285e972\build-script-build.exe` | `powershell.exe` |
| 10:58:10 | `target\debug\build\serde_core-54a931ad03f6ade6\build-script-build.exe` | `powershell.exe` |
| 10:58:07 | `target\debug\build\serde_json-8b6f917403521add\build-script-build.exe` | `powershell.exe` |
| 10:58:02 | `target\debug\build\serde_json-18fe016d6652bf76\build-script-build.exe` | `powershell.exe` |
| 10:57:58 | `target\debug\build\serde_json-042964430169ba52\build-script-build.exe` | `powershell.exe` |
| 10:57:55 | `target\debug\build\fs-err-007ae85ed31a0194\build-script-build.exe` | `powershell.exe` |
| 10:56:00 | `target\debug\build\serde_core-7bc7a59e3a39049c\build-script-build.exe` | `cargo.exe` |
| 10:55:58 | `target\debug\build\getrandom-faab77a30411d148\build-script-build.exe` | `cargo.exe` |
| 10:55:53 | `target\debug\deps\rustversion-cd39c03b22dbbaa9.dll` (proc macro) | `rustc.exe` |
| 2026-11-08 15:09 | `target\debug\build\multinet-cybershield-ai-desktop-e5a3ed5a69a8df2b\build-script-build.exe` | `cargo.exe` |
| 2026-11-08 14:00 | `target\debug\build\multinet-cybershield-ai-desktop-77d0799a671016af\build-script-build.exe` | `cargo.exe` |

Each block emits a 3077 (policy violation) and 3033 (audit/signing level) pair.
The desktop application executable built earlier still starts because it predates the current
enforcement state; any freshly linked executable is blocked.

## 4. Affected toolchain components

Blocked at execution:

- All Rust build scripts compiled during this session, including `serde_core`, `serde_json`,
  `getrandom`, `fs-err`, `native-tls`, `rustls` and the Tauri desktop build script.
- Proc-macro DLLs loaded by `rustc` (for example `rustversion-*.dll`).
- Any newly linked unit-test binary, integration-test binary, standalone service binary
  (`speedtest-server`) and the rebuilt desktop executable.

Not affected:

- `cargo` and `rustc` process startup.
- Signed Microsoft tools (used only to confirm the policy, never to bypass it).
- Pre-existing signed/allowlisted binaries and already-built artifacts.

Consequence: `cargo check`, `cargo test`, `cargo build`, hardware FFI tests and live desktop
verification cannot run on this host until code integrity permits the outputs.

## 5. Repository state (preserved)

- Git working tree preserved: 63 changed/untracked entries, 0 staged. No commits, reset, clean
  or stash was performed.
- No source file was reverted. `Cargo.lock` remains as resolved for the new dependencies.
- Generated `target\` artifacts only: signed-OS-binary stand-ins created earlier in this session
  were deleted (12 files), seeded `out\private.rs` files from the same stand-in attempt were
  removed, and the temporary probe directory was deleted. No further bypass artifacts remain.

### 5.1 Unverified source changes made this session

These files exist but have **not** been compiled or tested:

- `crates/network-monitor/src/lib.rs` — real `GetIfTable2` counters, rate derivation, reset
  handling, unit tests.
- `crates/speed-test/src/lib.rs` (new crate) — HTTPS client, latency/jitter/loss, download,
  upload, cancellation.
- `relay/speedtest-server/src/main.rs` (new crate) — standalone HTTPS service, `/health`,
  `/ping`, `/download`, `/upload`, size/concurrency/timeout limits.
- `database/sqlite/migrations/003_diagnostics.sql` — `traffic_samples`,
  `speed_test_results`, indexes.
- `apps/desktop/src-tauri/src/diagnostics.rs` — sampler task, history/peak queries, speed test
  run store and commands.
- `apps/desktop/src-tauri/src/main.rs` — module wiring, `IpcError` constructors, command
  registration, sampler startup.
- `apps/desktop/src-tauri/src/db.rs` — migration-count assertions still expect 2/17 and must be
  updated to 3/19 before tests can pass.
- `Cargo.toml`, `crates/*/Cargo.toml`, `relay/speedtest-server/Cargo.toml` — workspace members
  and dependencies (`reqwest` native-tls, `axum-server =0.7.1` tls-rustls-no-provider,
  `rustls` ring, `rcgen`, `tower-http`).
- Frontend Diagnostics pages were **not** yet rewritten; `TrafficAnalytics.tsx` and
  `SpeedTest.tsx` remain the placeholder versions.

## 6. Supported resolutions (in order of preference)

1. **Build on a host/VM where code integrity permits the toolchain outputs.**
   Use a dedicated Windows development VM (Hyper-V/Windows Sandbox) or machine whose Smart App
   Control is off or whose WDAC policy allows the Rust toolchain output directory. This is the
   supported path for a non-administrator session and requires no change to this host.
2. **Controlled CI.** Run `cargo check --workspace`, `cargo test --workspace` and
   `npm run build` on a Windows CI runner, then perform runtime hardware tests on a VM with
   code integrity configured to allow the artifacts. Artifacts from CI remain unsigned and will
   still be blocked on this host unless it is allowlisted.
3. **Policy owner action.** If this machine is managed, the WDAC/SAC policy owner can add a
   signed-publisher rule or a supplemental policy that trusts the organization's code-signing
   certificate and the build output path. This must be done through supported administration
   tooling; it was not attempted here.
4. **Signing.** Sign build outputs with a trusted code-signing certificate under a policy that
   accepts that publisher. A user-profile self-signed certificate is not a supported fix for
   SAC and was explicitly not used.
5. **Host setting.** Disabling Smart App Control through Windows Security is possible only with
   administrator action and is explicitly out of scope per the user's directive.

## 7. Verification status

| Check | Status |
| --- | --- |
| `cargo check --workspace` | BLOCKED (build scripts, code integrity 4551) |
| `cargo test --workspace` | BLOCKED |
| Windows hardware counter test (`network-monitor`) | BLOCKED |
| HTTPS speed test against the local service | BLOCKED |
| Desktop runtime inspection / screenshots | BLOCKED for rebuilt binaries |
| `npm run build` | Not run after the blocker; frontend pages not yet rewritten |
| Diagnostics completion report | Not written; Diagnostics is NOT complete |

## 8. Recommended next steps

1. Restore a supported build environment (option 1 or 2 above) before resuming Rust work.
2. When Rust execution is available, first run `cargo check --workspace` and fix the unverified
   code in section 5.1, then update the migration assertions in `db.rs` (2/17 -> 3/19).
3. Build the standalone `speedtest-server`, run a real HTTPS download/upload against it, and
   verify interface binding from the desktop app.
4. Only after those checks pass, rewrite the Traffic Analytics and Speed Test pages and write
   `docs/daily-reports/DIAGNOSTICS_COMPLETION_REPORT_2026-11-09.md` with evidence.
5. The requested localhost dashboard restyle remains a frontend-only task and can proceed
   without Rust execution once the design source at `http://localhost:5500/` is available.
