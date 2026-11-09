# MultiNet CyberShield AI — Milestone NM-06.4 Architecture Audit
## Production Hardening & Windows Release Engineering

**Project:** MultiNet CyberShield AI  
**Repository:** `usingh312-dotcom/multinet-cybershield-ai`  
**Branch:** `master`  
**Milestone:** NM-06.4  
**Date:** November 9, 2026  
**Auditor:** Principal Software Architect, Windows Platform Engineer & Security Lead  

---

## 1. Executive Summary

Milestone **NM-06.4** ("Production Hardening & Windows Release Engineering") transforms the validated MultiNet CyberShield AI application into a professionally installable, upgradeable, resilient, and securely distributable Windows product.

Milestones NM-06.1 through NM-06.3.3 proved the core networking algorithms, multi-path bonding, IPC protocol, mutual exclusivity, and fault-recovery in test harnesses and CI. NM-06.4 addresses the production gap: delivering a verified Windows Service running under the Service Control Manager (SCM), a production MSI installer (WiX Toolset), Authenticode signing pipelines, network route disaster recovery, structured Event Log observability, and Software Bill of Materials (SBOM) generation.

---

## 2. Component Inventory & Architecture Audit

### 2.1 Go Networking Service (`services/net-merge`)
- **Binary:** `net-merge-service.exe` (Go 1.27.3, Windows amd64).
- **Core Functionality:**
  - Manages the Wintun virtual network adapter (Mode A) and host capture routes (`0.0.0.0/1` and `128.0.0.0/1`).
  - Acts as the authoritative Mode Coordinator managing cross-process tunnel exclusion mutex `Global\multinet-netmerge-mode-owner-v1`.
  - Exposes secured local named pipe `\\.\pipe\multinet-netmerge-v1` with restrictive SDDL (`D:(A;;GA;;;BA)(A;;GA;;;SY)(A;;GRGW;;;BU)`).
  - Authenticates pipe clients via `ImpersonateNamedPipeClient` + `CheckTokenMembership`.
  - Monitors unelevated Rust bonding engine processes via `PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE` and `WaitForSingleObject`.
- **Identified Production Gaps:**
  1. *SCM Integration:* `main.go` currently executes only as an interactive CLI process using `flag.Parse()` and console signals (`signal.NotifyContext`). When launched by the Windows Service Control Manager (`services.exe`), SCM expects the process to register via `StartServiceCtrlDispatcherW` within 30 seconds. Without this, starting the service via SCM times out with Error 1053.
  2. *Windows Event Log:* Logging currently outputs to `stderr` or a plain text file. A production Windows service must write operational events (Service Start, Stop, Mode A/B transitions, authentication rejections, crash recoveries) to the Windows Application Event Log under source `MultiNetNetMerge`.
  3. *Service Installation & Recovery Flags:* Missing built-in `-install`, `-uninstall`, `-service`, and `-console` CLI commands to register SCM service configs, delayed auto-start, and restart-on-failure actions.

### 2.2 Rust Workspace (`crates/*`, `relay/*`)
- **Workspace:** 25 crates managed via Cargo resolver 2.
- **Key Modules:**
  - `bonding-engine`: Multi-path packet scheduler with deficit round-robin and weighted failover.
  - `tunnel-protocol`: Versioned framing, sequence reordering, HMAC-SHA256 packet authentication.
  - `netmerge-client`: Async Rust IPC client connecting to `\\.\pipe\multinet-netmerge-v1`.
  - `interface-discovery`: Windows IP Helper API (`GetAdaptersAddresses`, `GetIfEntry2`) and NDIS link status monitoring.
  - `speed-test`: Loopback diagnostics and throughput benchmarking with timeout controls.
- **Identified Production Gaps:**
  1. *Service Dependency Detection:* Rust client must handle SCM service startup latency gracefully when starting at boot.
  2. *Executable Metadata:* Rust crates require version resource files (`.rc`) embeding product name, company, version, and copyright for Windows Explorer properties.

### 2.3 Tauri Desktop Application (`apps/desktop`)
- **Shell:** Tauri v2 (`ai.multinet.cybershield`).
- **Frontend:** React + TypeScript + Vite + Tailwind CSS.
- **Backend:** `src-tauri` with SQLite persistence across 3 migrations (19 tables) storing adapter metrics, security events, and speed test historical samples.
- **Identified Production Gaps:**
  1. *User Data Path:* Database must reside under `%APPDATA%\MultiNetCyberShield` (or `%LOCALAPPDATA%`), guaranteeing that MSI upgrades and uninstalls do not wipe user diagnostic logs unless explicitly requested.
  2. *Unelevated Execution:* The desktop UI runs unelevated under standard user context and communicates with the elevated background service purely via the authenticated named pipe.

### 2.4 Driver & Network Safety Architecture
- **Wintun Driver (`wintun.dll`):**
  - Requires signed 64-bit `wintun.dll` placed next to `net-merge-service.exe`.
  - Creates virtual network interface with static GUID `{3f0d5a7e-9b1c-4e52-8a47-1c6b2d9e5f30}`.
- **Route & Adapter Recovery:**
  - During Mode A, host default traffic is captured using pair-routes `0.0.0.0/1` and `128.0.0.0/1`.
  - Weak-host send and forwarding are toggled to strong-host on physical interfaces.
  - State file `%TEMP%\net-merge-service-weakhost.txt` records original interface states.
- **Identified Production Gaps:**
  1. *Disaster Recovery Hook:* If Windows reboots ungracefully during active capture, or if the service binary is abruptly terminated, network routes must automatically restore on startup.
  2. *Uninstallation Cleanliness:* The MSI uninstaller must invoke a cleanup action ensuring no orphaned routes or virtual adapters remain.

---

## 3. Production Architecture Blueprint

```
+---------------------------------------------------------------------------------+
|                                Windows Host OS                                  |
+---------------------------------------------------------------------------------+
|                                                                                 |
|  [ Interactive User Session ]                  [ Windows Service Control Manager ]
|  +-----------------------------------+         +------------------------------+ |
|  | Tauri Desktop GUI (Unelevated)    |         | MultiNetNetMerge Service     | |
|  | multinet-cybershield-ai.exe       |         | (LocalSystem / Elevated)     | |
|  | - React/TS Dashboard              |         | net-merge-service.exe        | |
|  | - SQLite (%APPDATA%)              |         | - golang.org/x/sys/windows/svc|
|  | - NetMergeClient (Rust)           |         | - SCM Handler (Start/Stop)   | |
|  +-----------------+-----------------+         | - Event Log Provider         | |
|                    |                           | - Disaster Route Recovery    | |
|                    | Local Named Pipe          +---------------+--------------+ |
|                    | \\.\pipe\multinet-netmerge-v1             |                |
|                    +===========================================+                |
|                                                                |                |
|                                                +---------------+--------------+ |
|                                                | Wintun Adapter & Route Table | |
|                                                | - wintun.dll                 | |
|                                                | - 0.0.0.0/1, 128.0.0.0/1     | |
|                                                | - Strong-Host Enforcement    | |
|                                                +------------------------------+ |
+---------------------------------------------------------------------------------+
|                          Windows Installer Package                              |
|                          MultiNetCyberShield.msi                                |
|  - WiX Toolset v5 / v4 perMachine Deployment                                   |
|  - SCM Service Registration (Auto-Start + ServiceRecoveryActions)              |
|  - Wintun Driver Staging & Desktop/Start Menu Shortcuts                         |
|  - Upgrade & Rollback (MajorUpgrade, Data Preservation)                        |
+---------------------------------------------------------------------------------+
```

---

## 4. Remediation Plan by Milestone Phase

1. **Phase 2 (Installer):** Author WiX project definition (`packaging/wix/MultiNetCyberShield.wxs`) declaring per-machine installation, desktop/start-menu shortcuts, service control, and driver staging.
2. **Phase 3 (Service Hardening):** Implement `golang.org/x/sys/windows/svc` handler in `services/net-merge`:
   - Dual-mode execution: runs under SCM when invoked as a service, or CLI when run with `-console`.
   - Windows Event Log integration logging under source `MultiNetNetMerge`.
   - Built-in service installation, removal, start, and stop management CLI.
   - Crash restart actions configuration (restart service on 1st, 2nd, 3rd failures).
3. **Phase 4 (Security & Signing):** Document Authenticode certificate requirements, Smart App Control policies, and signing validation scripts.
4. **Phase 5 (Network Safety):** Implement disaster route backup and cleanup on uninstallation and service recovery.
5. **Phase 6 (Upgrade & Rollback):** Configure upgrade GUIDs and verify configuration and database persistence across upgrades.
6. **Phase 7 (Observability):** Add structured logging, Windows Event Log publisher, and health export commands.
7. **Phase 8 (Supply Chain & SBOM):** Generate dependency inventory, CycloneDX SBOM, and release checksum manifest.
8. **Phase 9 & 10 (Release Testing & Acceptance):** Verify full build in GitHub Actions CI, document results, and generate the final NM-06.4 reports.
