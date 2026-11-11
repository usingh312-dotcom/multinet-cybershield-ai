# MultiNet CyberShield AI — Milestone NM-08.0.5
# Desktop Runtime & Native Diagnostics Integration Acceptance Report

**Document ID:** `NM08_0_5_DESKTOP_RUNTIME_ACCEPTANCE`
**Date:** 2026-11-11
**Status:** PASS — Native Runtime and IPC Verified
**Author:** Principal Tauri 2 Architect, Senior Rust Engineer, React/TypeScript Engineer, Windows Runtime Engineer and QA Lead
**Repository:** `usingh312-dotcom/multinet-cybershield-ai` (`D:\git\multinet-cybershield-ai`)
**Branch:** `feature/nm-08.0-benchmark-lab`
**Baseline Commit:** `ef484d30cc97deba4fcd8bf75aabce14cc46eb69`

---

## 1. Executive Summary

The desktop application's "native IPC bridge issue" was **not** a broken bridge. The
Tauri IPC channel worked. The reported symptom had **three independent root
causes**, none of which was visible to a build-only or CI-only review:

| # | Root cause | Severity | Symptom |
|---|---|---|---|
| **R1** | The frontend called `invoke()` with no runtime guard. Opened in a plain browser, every native command threw `TypeError: Cannot read properties of undefined (reading 'invoke')`. | High | "Backend bridge unavailable" in every browser preview |
| **R2** | `app.manage(Arc<DiagnosticsState>)` stored state under `TypeId<Arc<DiagnosticsState>>`, but the commands requested `State<'_, DiagnosticsState>`. The lookup missed. | **Critical** | **6 of 9 diagnostics commands failed at runtime in the shipped application** |
| **R3** | The release binary was built without the `custom-protocol` feature, so it embedded no frontend and pointed at `http://localhost:1420`. | **Critical** | The shipped release executable rendered an empty window on any machine without the Vite dev server |

R2 and R3 are the reason the GitHub build passing did **not** mean the installed
desktop application worked. Both were invisible to compilation and to CI, because
CI built the binary, checked that the file existed, and never launched it.

**Verdict: `PASS — Native Runtime and IPC Verified`.** The native application was
launched in both the development and the production configuration, and both
received real Rust backend responses for all nine diagnostics commands.

---

## 2. Phase 1 — Runtime Diagnosis

### 2.1 Executable naming verification

`apps/desktop/src-tauri/Cargo.toml` declares the package
`multinet-cybershield-ai-desktop` but an explicit binary target:

```toml
[[bin]]
name = "multinet-cybershield-ai"
path = "src/main.rs"
```

| Artefact | Path | Verified |
|---|---|---|
| Cargo binary target name | `multinet-cybershield-ai` | `cargo metadata` confirms a single `bin` target of that name |
| Debug executable | `target/debug/multinet-cybershield-ai.exe` | 19 379 712 bytes, produced and launched |
| Release executable | `target/release/multinet-cybershield-ai.exe` | 13 932 544 bytes, produced and launched |
| CI expectation | `target/release/multinet-cybershield-ai.exe` | Matches — the CI path was already correct |
| Packaging expectation | `packaging/staging/multinet-cybershield-ai.exe` | Matches |

**Executable naming was correct.** No rename was required. A stale
`multinet-cybershield-ai-desktop.exe` (11 511 2026) existed in `target/debug`
from an earlier build and was never referenced by any script.

### 2.2 Tauri configuration

`apps/desktop/src-tauri/tauri.conf.json` — product `MultiNet CyberShield AI`,
identifier `ai.multinet.cybershield`, window title
`MultiNet CyberShield AI — Connect. Protect. Detect.`, `devUrl`
`http://localhost:1420`, `frontendDist` `../dist`, CSP permitting
`ipc:`/`http://ipc.localhost`/`http://localhost:1420`.

`apps/desktop/src-tauri/capabilities/default.json` — `core:default` only, bound to
window `main`. No shell, filesystem or network plugin permission was granted, and
none was added.

### 2.3 The release build produced no usable application

This was the decisive finding of Phase 1.

`apps/desktop/src-tauri/Cargo.toml` declared `tauri = { version = "2", features = [] }`.
The Tauri CLI enables `tauri/custom-protocol` for release builds; a plain
`cargo build --release` does **not**. The CI workflow used exactly that plain
cargo invocation.

Consequence, verified empirically on the binary produced by
`cargo build --release -p multinet-cybershield-ai-desktop`:

```
size=13727232
contains_1420: True          <- points at the Vite dev server
contains_index_html: True
contains_dist_asset: False    <- no compiled frontend embedded
```

Launching that binary on a machine with nothing on port 1420 produced:

```
TARGET http://tauri.localhost/ | localhost
"url": "chrome-error://chromewebdata/",
"bodyText": "Hmmm… can't reach this page\n\nlocalhost refused to connect. … ERR_CONNECTION_REFUSED",
"invokeError": "Origin header is not a valid URL"
```

The application never loaded its frontend and therefore could never reach the
bridge. The window existed; the application did not.

---

## 3. Phase 2 — Native Bridge

### 3.1 Runtime detection

`apps/desktop/src/api.ts` now uses the supported Tauri 2 probe:

```typescript
import { invoke, isTauri } from "@tauri-apps/api/core";
```

- `detectNativeRuntime()` returns `{ isTauri, previewMessage }`.
- `isNativeRuntime()` is the boolean form.
- `window.__TAURI__` is **not** used anywhere: the project does not enable
  `withGlobalTauri`, and the report confirms `isTauriGlobal: false` in the real
  application while `isTauriInternals: true`.

The runtime is probed on render (`useNativeRuntime()`) rather than cached in a
store, because the host — not the store — decides the answer.

### 3.2 Refusing to dispatch outside the desktop runtime

Every native call goes through `invokeNative(command, args)`:

1. If `isTauri()` is false, the call is **refused before dispatch**. No IPC
   message is sent at all.
2. Otherwise the call is dispatched through the imported `invoke`.
3. On failure the exact command name and the exact reason are captured in a
   `NativeCommandError` carrying `command`, `reason`, `code`, `retryable` and
   `isRuntimeUnavailable`.
4. A failed command never resolves with fallback data.

The backend's structured `IpcError` payload (`{ code, message, retryable }`) is
unpacked, so the UI reports `speed test server must use HTTPS` rather than a
stringified JSON blob.

### 3.3 Browser-preview message

`BROWSER_PREVIEW_MESSAGE`:

> Browser preview: the native MultiNet bridge is unavailable, so live network,
> traffic and speed-test data cannot be read. Run the desktop application
> (`npm run tauri:dev`) to query the real Rust backend.

Every diagnostics screen renders this instead of an empty table when the host is
a plain browser.

### 3.4 Verified command responses (native runtime)

All four commands required by the mission were dispatched through the real IPC
channel and returned real Rust responses:

| Command | Result |
|---|---|
| `get_foundation_status` | `{"product":"MultiNet CyberShield AI","version":"0.1.0","phase":"Phase 1 - Foundation and Architecture","note":"No live network/security operations, no fake metrics, no system mutations."}` |
| `get_database_status` | `{"db_file":"…\\multinet-cybershield-ai.db","migrations_applied":3,"latest_version":3,"foreign_keys_enabled":true}` |
| `list_network_interfaces` | 7 adapters with real names, kinds, IPv4/IPv6 addresses and MAC addresses |
| `get_incidents` / `get_security_events` | `[]` — real empty results, no synthesized rows |

---

## 4. Phase 3 — Diagnostics Backend Audit

### 4.1 Findings

| Command | Parameter names | Serialization | Behaviour | Verdict |
|---|---|---|---|---|
| `list_network_interfaces` | none | `InterfaceDto`, camelCase | Correct | **Fixed** — was unusable (R2) |
| `get_traffic_monitor_status` | none | camelCase | Correct | **Fixed** — was unusable (R2) |
| `set_traffic_monitor_running` | `running: bool` | camelCase | Correct | **Fixed** — was unusable (R2) |
| `get_traffic_snapshot` | none | camelCase | **Broken on a fresh install** | **Fixed** (see 4.2) |
| `get_traffic_history` | `interface_id`, `seconds` | camelCase | **Bucketing wrong** | **Fixed** (see 4.3) |
| `start_speed_test` | `request: StartSpeedTestRequest` | camelCase | Correct, fail-closed | **Fixed** — was unusable (R2) |
| `get_speed_test_status` | `id: String` | camelCase | Correct | **Fixed** — was unusable (R2) |
| `cancel_speed_test` | `id: String` | camelCase | Correct | **Fixed** — was unusable (R2) |
| `get_speed_test_history` | none | camelCase | Correct | Correct |

### 4.2 R2 — `Arc<DiagnosticsState>` / `DiagnosticsState` type mismatch

`main.rs` managed the state as `Arc<DiagnosticsState>`:

```rust
let diagnostics = Arc::new(diagnostics::DiagnosticsState::new());
app.manage(diagnostics);
```

Tauri's `StateManager` is keyed by the exact `TypeId` of the managed value.
`Arc<DiagnosticsState>` and `DiagnosticsState` are different types, so every
command declaring `state: tauri::State<'_, DiagnosticsState>` missed the lookup
and failed with:

```
state not managed for field `state` on command `get_traffic_snapshot`.
You must call `.manage()` before using this command
```

**Six of the nine diagnostics commands were dead in the shipped application.**
Confirmed against the original binary before the fix:

```
"get_traffic_monitor_status": { "ok": false, "error": "state not managed …" }
"get_traffic_snapshot":       { "ok": false, "error": "state not managed …" }
```

**Fix:** the commands now request `tauri::State<'_, Arc<DiagnosticsState>>`,
matching the type that is actually managed. The shared `Arc` is required: the
background traffic sampler holds a clone of the same state.

### 4.3 `get_traffic_snapshot` failed on a fresh installation

The peak query was:

```sql
SELECT COALESCE(MAX(COALESCE(rx_rate_bps, 0) + COALESCE(tx_rate_bps, 0)), 0) AS peak
```

On a machine with no recorded sample the aggregate is `NULL`, so SQLite returned
the integer literal `0`. `sqlx` refuses to decode an `INTEGER` column into an
`f64`:

```
error occurred while decoding column "peak": mismatched types; Rust type `f64`
(as SQL type `REAL`) is not compatible with SQL type `INTEGER`
```

The command therefore failed exactly when a new installation opened the Traffic
Analytics screen. **Fix:** the literals are `0.0`, so the fallback is a REAL.
Regression test: `traffic_snapshot_command_works_before_the_first_sample_is_recorded`.

### 4.4 History bucketing divided by the wrong quantity

```rust
let bucket = (seconds / 600).max(1);
// …
GROUP BY CAST(strftime('%s', sampled_at) AS INTEGER) / $3   // $3 = bucket
```

`$3` was the **number of buckets**, but the `GROUP BY` needs the bucket **size**.
A one-hour window therefore produced 600 one-second groups instead of six
ten-minute groups, and the `LIMIT 4000` was consumed by near-duplicate rows.
**Fix:** the bucket size (600 s) is the grouping divisor and the bucket count
only decides whether aggregation happens at all. Regression test:
`traffic_history_aggregates_into_fixed_ten_minute_buckets`.

### 4.5 The traffic sampler and the adapter inventory disagreed

`GetIfTable2` (used by the sampler) reports NDIS filter and QoS pseudo-interfaces
(`…-WFP 802.3 MAC Layer LightWeight Filter-0000`, `…-QoS Packet Scheduler-0000`,
`Teredo Tunneling Pseudo-Interface`, `6to4 Adapter`, …). `GetAdaptersAddresses`
(used by the inventory) does not. The two screens therefore reported **41
interfaces** and **7 adapters** for the same machine.

**Fix:** the sampler now restricts itself to the discovered adapter set when the
discovery call produced a result, so both screens describe the same adapters.
When discovery fails no filter is applied, so no measurement is ever lost.
Verified after the fix: 6 sampled adapters + 1 loopback (deliberately excluded)
= the 7 discovered adapters. Regression test:
`traffic_snapshot_only_reports_real_adapters`.

### 4.6 Speed-test fail-closed behaviour (unchanged, verified)

- A plaintext `http://` server is rejected: `speed test server must use HTTPS`.
- `allowSelfSignedLoopback` is rejected for any non-loopback host.
- Transfer size, ping sample count and timeout are bounded.
- A failed measurement is reported as a failure with the backend's message; no
  result value is ever shown in its place.

---

## 5. Phase 4 — Frontend Integration

### 5.1 API layer (`apps/desktop/src/api.ts`)

- `invokeNative()` — the single guarded dispatch path.
- `NativeCommandError` — command name, reason, code, retryable, runtime flag.
- `detectNativeRuntime()` / `isNativeRuntime()` / `BROWSER_PREVIEW_MESSAGE`.
- Typed wrappers for all nine diagnostics commands, mirroring the Rust DTOs
  exactly (every diagnostics DTO uses `#[serde(rename_all = "camelCase")]`):
  `listNetworkInterfaces`, `getTrafficMonitorStatus`, `setTrafficMonitorRunning`,
  `getTrafficSnapshot`, `getTrafficHistory`, `startSpeedTest`,
  `getSpeedTestStatus`, `cancelSpeedTest`, `getSpeedTestHistory`.

### 5.2 State (`apps/desktop/src/store/diagnosticsStore.ts`)

A dedicated store holds the interface inventory, traffic snapshot, traffic
history, monitor status, speed-test status and speed-test history, each with its
own `idle | loading | ready | error` phase and a structured error carrying the
failing command name.

### 5.3 Network Manager

Live adapter inventory from `list_network_interfaces`: name, description, type,
physical/virtual classification, up/down state, IPv4 and IPv6 addresses, MAC,
MTU, link speed and interface index. A working **Refresh interfaces** action
re-runs the command. Loading, empty, error and success states are distinct, and
a per-adapter inspector is populated from the selected row only.

### 5.4 Traffic Analytics

Live snapshot and recorded history from `get_traffic_snapshot` /
`get_traffic_history`: session totals, current download/upload rates, peak
bandwidth and the sampler status, all with the timestamps the backend returned.
Time-range buttons (5 min / 15 min / 1 h / 24 h) re-query the backend. The
chart renders only real samples; with no samples the empty state is shown and no
series is generated.

### 5.5 Speed Test

Start, live progress polling (1 s), status, cancellation and history are all
bound to the real commands. The socket is bound to the selected adapter by
passing that adapter's IPv4 address as `interfaceLocalIp`. The server URL is
operator-supplied (HTTPS only) — no endpoint is invented. A failed start or a
failed measurement is reported verbatim; the gauge and result cards stay empty.

---

## 6. Phase 5 — Security and Reliability Preservation

| Requirement | Status | Evidence |
|---|---|---|
| Fail-closed behaviour preserved | Held | `start_speed_test` rejects plaintext, remote self-signed certs, oversized transfers and out-of-range parameters; `get_traffic_history` rejects windows outside 60…604 800 s |
| No route or interface mutation | Held | No command writes a route, address or adapter configuration; the only writes are to the local SQLite history tables |
| No fabricated adapters or telemetry | Held | `list_network_interfaces` returns OS data; the sampler now reports exactly the discovered adapters; the frontend never synthesizes a row |
| No silent fallback to another adapter | Held | The speed test binds the socket to the explicitly selected address; when none is selected the operating system default route applies and the UI says so |
| No credentials or private network data in logs | Held | No new logging was introduced; the existing `tracing::warn!` calls log error strings only |
| Five bonded E2E scenarios remain BLOCKED | Held | Untouched — they still require the NM-08.1 relay infrastructure |
| No plugin permissions added | Held | `capabilities/default.json` still grants `core:default` only |

---

## 7. Phase 6 — Automated Tests

### 7.1 Separation of the two test projects

`apps/desktop/vitest.config.ts` defines two projects:

| Project | Environment | What it proves |
|---|---|---|
| `browser` | jsdom | The guard works: no native command is dispatched outside the desktop runtime; failures surface their command name; empty and error states render; no measurement is invented |
| `native` | node | The real desktop executable starts and creates its database |

The **native Tauri IPC integration tests** are Rust tests
(`apps/desktop/src-tauri/tests/native_ipc.rs`) that drive the real
`#[tauri::command]` implementations through Tauri's own IPC dispatch path
(`tauri::test::get_ipc_response`) against the real SQLite database. No command
behaviour is re-implemented or stubbed.

### 7.2 Test inventory

| # | Required test | Where | Result |
|---|---|---|---|
| 1 | Browser runtime detection | `src/test/browser/bridge.test.ts` | PASS |
| 2 | Native Tauri IPC smoke test | `src-tauri/tests/native_ipc.rs` | PASS |
| 3 | Foundation status invocation | `native_ipc.rs` | PASS |
| 4 | Database status invocation | `native_ipc.rs` | PASS |
| 5 | Interface inventory command integration | `native_ipc.rs` | PASS |
| 6 | Traffic snapshot integration | `native_ipc.rs` | PASS |
| 7 | Speed test lifecycle | `native_ipc.rs` | PASS |
| 8 | Empty-state and error-state frontend tests | `src/test/browser/pages.test.tsx` | PASS |
| 9 | Regression: no fake network measurements displayed | `pages.test.tsx` | PASS |

### 7.3 Results

```
apps/desktop (vitest)
  Test Files  3 passed (3)
  Tests      17 passed (17)

apps/desktop/src-tauri (cargo test)
  test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo test --workspace
  57 test binaries — 0 failures
```

New regression tests that pin the defects found in this milestone:

- `traffic_snapshot_command_works_before_the_first_sample_is_recorded` (§4.3)
- `traffic_history_aggregates_into_fixed_ten_minute_buckets` (§4.4)
- `traffic_snapshot_only_reports_real_adapters` (§4.5)
- `speed_test_start_rejects_a_non_https_server` (§4.6)
- `speed_test_start_rejects_self_signed_certificates_for_remote_servers` (§4.6)

### 7.4 A Windows-specific test-harness blocker

Tauri links `TaskDialogIndirect` from `comctl32.dll`, which exists only in the
v6 flavour of that library. The v6 flavour is loaded through the application
manifest, and the manifest is attached only to the shipped executable — not to
the test harnesses Cargo builds. Without it the loader resolved the v5 library
and every test binary aborted with `STATUS_ENTRYPOINT_NOT_FOUND` before a single
assertion ran.

`apps/desktop/src-tauri/build.rs` now embeds the Common Controls v6 manifest into
the **test** targets only (`cargo:rustc-link-arg-tests`), mirroring the
workaround the Tauri project applies to its own suite
(tauri-apps/tauri#4383). The shipped executable is unaffected — it already
receives the manifest through the Windows resource file that `tauri_build`
compiles, and embedding it twice produced a duplicate-resource link error.

---

## 8. Phase 7 — Windows Runtime Verification

### 8.1 Native development launch — `npm run tauri:dev`

| Item | Value |
|---|---|
| Command | `npm run tauri:dev` (Tauri CLI 2.12.4) |
| Dev server | Vite 5.4.21 on `http://localhost:1420` |
| Executable | `target/debug/multinet-cybershield-ai.exe` (19 379 712 bytes) |
| Window title | `MultiNet CyberShield AI — Connect. Protect. Detect.` |
| Document URL | `http://localhost:1420/` |
| Native runtime detection | `IPC_BRIDGE_READY true` — `isTauri()` returns true |
| `window.__TAURI__` | absent (correct — `withGlobalTauri` is not enabled) |
| `window.__TAURI_INTERNALS__` | present |
| Database | created under `MULTINET_DATA_DIR`, 3 migrations applied |

All nine diagnostics commands returned `ok: true` against the live Rust backend.

### 8.2 Production release executable

| Item | Value |
|---|---|
| Build | `cargo build --release -p multinet-cybershield-ai-desktop --features custom-protocol` |
| Executable path | `D:\git\multinet-cybershield-ai\target\release\multinet-cybershield-ai.exe` |
| Size | 13 932 544 bytes |
| Document URL | `http://tauri.localhost/` — **embedded assets, no dev server** |
| Port 1420 listeners during the run | **0** |
| `window.__TAURI__` / `__TAURI_INTERNALS__` | absent / present |
| `get_foundation_status` | real product contract |
| `get_database_status` | `migrations_applied: 3`, `foreign_keys_enabled: true` |
| `list_network_interfaces` | 7 real adapters |
| `get_traffic_monitor_status` | `running: true`, `source: GetIfTable2` |
| `get_traffic_snapshot` | real sample with `sampledAt`, interfaces and rates |
| `get_traffic_history` | real recorded samples |
| `get_speed_test_history` / `get_incidents` / `get_security_events` | `[]` — truthful empty results |

### 8.3 Screens observed in the production application

**Network Manager** — "AVAILABLE INTERFACES 7 / 3 physical", "ACTIVE CONNECTIONS
3", "NETWORK MONITORING STATUS Running", and the live table:

```
INTERFACE          TYPE      STATUS  IPV4 / IPV6                       DOWNLOAD  UPLOAD  INDEX
Speedify           Other     Up      10.202.0.2, fd0c:…                 9 Kbps    4 Kbps  42
Ethernet           Ethernet  Down    169.254.111.138, fe80::…           —         —       3
Local Area Conn* 1 Wi-Fi     Down    169.254.66.10, fe80::…             —         —       4
```

**Traffic Analytics** — "TOTAL DOWNLOAD 334.83 KiB", "TOTAL UPLOAD 308.07 KiB",
"CURRENT DOWNLOAD RATE 5.27 Mbps", "CURRENT UPLOAD RATE 6.97 Mbps", "PEAK
BANDWIDTH 12.24 Mbps" — all computed by the Rust backend from the interface
byte counters.

**Speed Test** — engine idle, gauge empty, result cards empty, and the adapter
selector populated from the live inventory. No value is displayed before a
measurement completes.

### 8.4 Windows runtime log

The production application was driven through the WebView2 DevTools Protocol
(`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port`) with the
console attached:

```
CONSOLE:
  (no console errors, no uncaught exceptions)
```

The application reported "All systems operational / No issues detected" and
"Core online" in its status panel.

---

## 9. Outstanding Issues and Blocked Scenarios

### 9.1 Blocked (unchanged)

The five physical bonded end-to-end scenarios remain **BLOCKED** and were not
touched: `bonded_e2e_2wan`, `bonded_e2e_failover`, `bonded_e2e_3wan`,
`bonded_e2e_reassembly`, `bonded_e2e_return`. They require the NM-08.1 relay
infrastructure and physical multi-WAN hardware.

The five physical speed-test scenarios against a remote relay remain **BLOCKED**
for the same reason. The local loopback speed-test server
(`relay/speedtest-server`) is available and the frontend is wired to it, but no
end-to-end run against it is claimed here: the server was not started as part of
this verification.

### 9.2 Open items

| Item | Impact | Recommendation |
|---|---|---|
| The desktop runtime qualification harness (`packaging/test-desktop-runtime.ps1`) still only proves that the process starts and that the database file appears. It does not prove the window renders the frontend or that the IPC bridge answers. | The exact class of defect that R2 and R3 slipped through remains possible for future regressions. | Extend the harness to assert an embedded-asset marker in the executable and to drive a live IPC round-trip, as done manually in this milestone. |
| `packaging/build-installer.ps1` and `package-beta.ps1` accept `target/release/multinet-cybershield-ai.exe` without checking that it was built with `custom-protocol`. | A binary built by the old command would still be packaged. | The CI build step and its new asset-embedding assertion now guarantee the correct artefact; the packaging scripts could repeat the check. |
| `apps/desktop/src/api.ts` still exposes the legacy `getDatabaseStatus` contract with `snake_case` field names (`db_file`, `migrations_applied`), unlike the camelCase diagnostics DTOs. | Cosmetic inconsistency only; the frontend type mirrors the Rust struct exactly. | Deliberately left unchanged to avoid destabilising the existing contract. |
| The route inspector is not implemented by the diagnostics backend; Network Manager reports "Not reported" for the default route. | No route data is displayed. | Requires a new read-only backend command; out of scope for this milestone. |

### 9.3 Environment note (not a repository defect)

The build machine enforces a Windows Application Control policy
(`Policy ID {0283ac0f-fff1-49ae-ada1-8a933130cad6}`, status *Enforced*) that
blocks unsigned binaries that have not yet accumulated reputation. This made
`cargo build --release` fail intermittently with
`An Application Control policy has blocked this file. (os error 4551)` until the
artefact was re-signed by the platform policy. The same policy blocked the
`serde_json` build script once, which left a stale `Precalculated` fingerprint
whose cached build-script output had been corrupted; the affected cache
directories were purged and the workspace rebuilt cleanly. No repository source
file was involved, and CI is unaffected.

---

## 10. Changes Made

### 10.1 Rust — backend

| File | Change |
|---|---|
| `apps/desktop/src-tauri/src/lib.rs` | **New.** The command surface, the structured `IpcError`, `build_app()` and `run()`. Splitting the crate into a library and a thin binary makes the native IPC integration tests possible. |
| `apps/desktop/src-tauri/src/main.rs` | Reduced to a thin entry point calling `multinet_cybershield_ai_desktop::run()`. |
| `apps/desktop/src-tauri/src/diagnostics.rs` | **R2** — state type corrected in six commands. **§4.3** — REAL-typed peak fallback. **§4.4** — bucket-size grouping. **§4.5** — sampler restricted to discovered adapters. |
| `apps/desktop/src-tauri/Cargo.toml` | `custom-protocol` feature; `[lib]` target; dev-dependencies for the IPC tests. |
| `apps/desktop/src-tauri/build.rs` | Common Controls v6 manifest for the test targets (§7.4). |
| `apps/desktop/src-tauri/windows-app-manifest.xml` | **New.** The manifest itself. |
| `apps/desktop/src-tauri/tests/native_ipc.rs` | **New.** 18 native IPC integration tests. |

### 10.2 TypeScript — frontend

| File | Change |
|---|---|
| `apps/desktop/src/api.ts` | Guarded `invokeNative`, `NativeCommandError`, runtime detection, structured-error unpacking, typed wrappers for all nine diagnostics commands. |
| `apps/desktop/src/store/diagnosticsStore.ts` | **New.** Diagnostics state with per-slice loading/empty/error phases. |
| `apps/desktop/src/store/systemStore.ts` | Runtime probe extracted into `useNativeRuntime()`; reset helper for tests. |
| `apps/desktop/src/lib/format.ts` | **New.** Byte/rate/timestamp formatting for real measurements. |
| `apps/desktop/src/pages/NetworkManager.tsx` | Live adapter inventory, refresh, per-adapter inspector, real counters. |
| `apps/desktop/src/pages/TrafficAnalytics.tsx` | Live snapshot and history, real rates and timestamps, time-range selection. |
| `apps/desktop/src/pages/SpeedTest.tsx` | Start/progress/cancel/history, adapter binding, honest failure reporting. |
| `apps/desktop/src/components/ui/BandwidthChart.tsx` | Renders the backend sample series. |
| `apps/desktop/src/test/**` | **New.** Browser-preview tests, native runtime test, shared setup. |
| `apps/desktop/vitest.config.ts` | **New.** The two separated test projects. |
| `apps/desktop/package.json`, `package-lock.json` | Vitest 4.1.12, jsdom 27, Testing Library, `@types/node`. No new audit findings (the pre-existing `esbuild`/`vite` advisories are unchanged). |
| `apps/desktop/tsconfig.json` | Node and test type definitions. |

### 10.3 CI

`.github/workflows/windows-build.yml`:

- The release build now uses `--features custom-protocol`.
- A new step asserts that the release binary embeds the compiled frontend.
- A new **Desktop Frontend Tests** job runs the browser-preview tests, the
  native frontend runtime test and the TypeScript type check against the real
  release artefact.
- The native Tauri IPC integration tests run as their own step.

---

## 11. Verification Commands

```powershell
# Native Tauri IPC integration tests (real commands, real SQLite)
cargo test -p multinet-cybershield-ai-desktop --test native_ipc

# Whole workspace
cargo test --workspace

# Frontend
cd apps/desktop
npm ci
npm run build
npm run test:browser
npm run test:native
npx tsc -b --pretty false

# Native development launch
npm run tauri:dev

# Production release executable with embedded assets
cd ../..
cargo build --release -p multinet-cybershield-ai-desktop --features custom-protocol
.\target\release\multinet-cybershield-ai.exe
```

---

## 12. Final Acceptance

```
PASS — Native Runtime and IPC Verified
```

The native application starts in both the development and the production
configuration, loads its own frontend in the production configuration with no
dev server present, and receives real Rust backend responses for all nine
diagnostics commands and both foundation commands. Three production defects
(R1, R2, R3) were found and fixed, each of them invisible to compilation and to
the previous CI pipeline, and each of them is now pinned by a regression test.
