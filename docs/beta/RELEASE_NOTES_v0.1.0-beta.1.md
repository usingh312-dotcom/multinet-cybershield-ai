# MultiNet CyberShield AI — Release Notes
## Version 0.1.0-beta.1 (Milestone NM-07.0)

**Release Tag:** `v0.1.0-beta.1`  
**Target Platform:** Windows 10 (Build 19041+) / Windows 11 (x64)  
**Distribution File:** `MultiNetCyberShield-v0.1.0-beta.1-Windows-x64.zip`  
**MSI Package:** `MultiNetCyberShield-Setup.msi`  
**License:** Proprietary / MultiNet Beta Testing Agreement  

---

### 1. Overview & Beta Objectives

MultiNet CyberShield AI `v0.1.0-beta.1` is the first controlled external beta distribution of the intelligent multi-link network protection and bonding platform. 

This release invites qualified network engineers, security professionals, and beta testers to validate:
1. Dynamic multi-link network bonding across physical interfaces (Wi-Fi, Ethernet, Cellular LTE/5G).
2. Mode A (Policy-Based Multipath) and Mode B (Wintun Virtual Adapter Bonding) switching reliability.
3. Windows Service Control Manager (SCM) resilience and disaster recovery route restoration.
4. Seamless installation, upgrade, repair, and uninstallation workflows via Windows Installer (MSI).

---

### 2. What's New in v0.1.0-beta.1

#### High-Performance Network Bonding (Rust Core)
- **Deficit Round-Robin Multipath Scheduler:** Dynamically balances traffic across heterogeneous network adapters according to link capacity, latency, and packet loss metrics.
- **Wintun Kernel Driver Integration:** High-throughput userspace TUN driver staging with zero packet drops under high-bandwidth saturation.
- **Dynamic Interface Discovery:** Real-time detection of adapter connect/disconnect events with sub-second failover.

#### Hardened Windows Architecture (Go & SCM)
- **Authoritative Service Control Manager Runner:** Elevated background Windows Service (`MultiNetNetMerge`) with dual-mode console/service execution.
- **Strict Mutex Coexistence:** Exclusive kernel mutex (`Global\multinet-netmerge-mode-owner-v1`) preventing Mode A and Mode B concurrency violations.
- **Disaster Recovery Route Restorer:** Startup and crash recovery mechanism ensuring `0.0.0.0/1` and `128.0.0.0/1` routes are purged and weak-host settings restored to prevent loss of internet connectivity.

#### Security & Least-Privilege Separation
- **DACL-Protected Named Pipe IPC:** Restrictive Security Descriptor Definition Language (SDDL) allowing only local Administrators (`BA`) and Authenticated Users (`AU`), while forbidding non-admin impersonation.
- **Process Token Identity Validation:** Real-time caller PID validation and client executable signature checking.
- **Unelevated GUI / Elevated Engine Separation:** Desktop UI operates under standard user privileges; elevated capabilities are isolated within the SCM service.

#### Enterprise Packaging & Observability
- **WiX Toolset v5 MSI Package:** Single-file MSI installer with perMachine scope, desktop & start menu shortcuts, and automatic service registration.
- **Windows Event Log Publishing:** Structured event logging to the Windows Application log under source `MultiNetNetMerge`.
- **Software Bill of Materials:** Full CycloneDX v1.5 SBOM (`sbom.json`) and software dependency inventory mapping all transitive components.

---

### 3. Known Limitations & Operating Guidance

1. **Smart App Control (SAC) on Windows 11:**
   - Beta binaries are digitally built in clean GitHub Actions runners.
   - On Windows 11 systems with Smart App Control (SAC) enabled in enforcement mode, uncataloged beta binaries without an Extended Validation (EV) certificate enrolled in the Microsoft Trusted Root program may trigger an alert.
   - For beta testing, ensure your workstation allows beta evaluation or whitelist the installer directory. Production commercial releases will feature full EV Authenticode signatures.
2. **UAC Elevation:**
   - Installation and service installation require administrative elevation.
   - The desktop client GUI executes as a standard unelevated user.
3. **Driver Prerequisites:**
   - The verified `wintun.dll` driver library is automatically staged into the `drivers\` directory by the MSI installer.

---

### 4. Privacy & Zero-Telemetry Commitment

MultiNet CyberShield AI operates under a **Strict Privacy & Zero-Telemetry** policy:
- **No Personal Data Collection:** The application does not collect, transmit, or store personally identifiable information (PII).
- **No Packet Inspection Storage:** Payload data traversing the bonding tunnel is processed strictly in-memory and never logged to disk or external servers.
- **Sanitized Diagnostic Bundles:** Diagnostic reports generated for beta troubleshooting redact IP addresses, MAC addresses, and environment identifiers prior to export.

---

### 5. Issue Reporting & Beta Support

If you encounter issues during testing, please submit a report:
- **GitHub Issues:** Submit reports at `https://github.com/usingh312-dotcom/multinet-cybershield-ai/issues`.
- **Diagnostic Export:** Run `net-merge-service.exe -action status` and collect logs from the Windows Event Viewer (`Application` log, Source: `MultiNetNetMerge`).
