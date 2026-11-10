# MultiNet CyberShield AI — Beta Installation & Rollback Guide
## Version 0.1.0-beta.1 (Milestone NM-07.1)

This guide provides instructions for deploying, updating, repairing, and rolling back MultiNet CyberShield AI during the controlled beta evaluation.

---

### 1. System Requirements & Runtime Prerequisites

- **Operating System:** Windows 10 (64-bit, Build 19041 or higher) or Windows 11 (64-bit).
- **Architecture:** x86_64 (AMD64).
- **Privileges:** Administrator access required for installation and Windows Service management.
- **Runtime Prerequisites:**
  - **Microsoft Edge WebView2 Runtime:** Required by the Tauri desktop user interface. Pre-installed by default on Windows 10 (recent updates) and all Windows 11 builds. If missing on a stripped enterprise image, download and install the Evergreen Bootstrapper from [Microsoft WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
  - **Network Adapters:** Two or more active network adapters (e.g., Ethernet + Wi-Fi, or Wi-Fi + Cellular tethering) to evaluate active multi-path bonding.
- **Hardware:**
  - Minimum 4 GB RAM.
  - 100 MB available disk storage for application binaries, driver components, and SQLite state database.

---

### 2. Code Signing & Smart App Control (SAC) Advisory

- **Driver Integrity:** The kernel virtual network adapter driver (`wintun.dll` v0.14.1) is authentically signed by **WireGuard LLC** with a valid Authenticode signature and RFC 3161 timestamp.
- **First-Party Binaries:** In accordance with open pre-release qualification standards, first-party executables (`multinet-cybershield-ai.exe`, `net-merge-service.exe`, and `MultiNetCyberShield-Setup.msi`) are truthfully distributed as **Unsigned Beta** artifacts.
- **Windows Smart App Control / Defender SmartScreen:**
  - On Windows 11 systems where Smart App Control (SAC) is enabled in strict enforcement mode, unsigned binaries without established cloud reputation may be intercepted upon download or execution.
  - **Security Best Practice:** Do **not** disable core Windows security features. In controlled enterprise testing or evaluation labs, test installations should be conducted on evaluation systems configured for pre-release software testing or where file exceptions for the staging directory are applied in accordance with organizational security policies.
  - Full EV (Extended Validation) Authenticode code signing with hardware HSM backing is scheduled for the formal General Availability (GA) production release gate.

---

### 3. Standard Installation (Interactive GUI)

1. Download `MultiNetCyberShield-v0.1.0-beta.1-Windows-x64.zip` and extract its contents to a local folder.
2. Verify package integrity against `checksums.sha256`:
   ```powershell
   Get-FileHash MultiNetCyberShield-Setup.msi -Algorithm SHA256
   ```
3. Launch `MultiNetCyberShield-Setup.msi`.
4. When prompted by User Account Control (UAC), accept elevation to grant installation privileges.
5. The installer completes the following setup transactionally:
   - Copies `net-merge-service.exe`, `multinet-cybershield-ai.exe`, and `wintun.dll` into `C:\Program Files\MultiNet CyberShield AI`.
   - Copies an intentional duplicate of `wintun.dll` into `C:\Program Files\MultiNet CyberShield AI\drivers\` for legacy loader compatibility.
   - Registers the Windows Service `MultiNetNetMerge` in the Service Control Manager (SCM) configured for `Automatic` startup under `LocalSystem`.
   - Starts `MultiNetNetMerge` automatically.
   - Installs Start Menu and Desktop shortcuts for **MultiNet CyberShield AI**.
   - Creates the application data directory `C:\ProgramData\MultiNetCyberShield` for system-wide configuration.

---

### 4. Enterprise Silent Deployment & Unattended Testing

To install quietly without UI prompts (ideal for automated CI or lab testing):

```powershell
# Unattended Installation with Verbose Logging:
msiexec.exe /i MultiNetCyberShield-Setup.msi /qn /l*v "C:\Temp\MultiNet-Install.log"
```

Verify that the service is registered and running:
```powershell
Get-Service -Name "MultiNetNetMerge"
```

Expected output:
```
Status   Name               DisplayName
------   ----               -----------
Running  MultiNetNetMerge   MultiNet Net-Merge Service
```

---

### 5. Package Repair & Integrity Re-validation

If any files, drivers, or service configurations become damaged or deleted:

```powershell
# Execute repair of existing installation:
msiexec.exe /fa MultiNetCyberShield-Setup.msi /qn /l*v "C:\Temp\MultiNet-Repair.log"
```

The repair operation:
- Verifies key-path files against the embedded MSI cabinet.
- Re-stages missing or corrupted binaries (`net-merge-service.exe`, `multinet-cybershield-ai.exe`, `wintun.dll`).
- Re-registers the SCM service and shortcuts.
- Preserves all local databases and user settings in `%APPDATA%\ai.multinet.cybershield\`.

---

### 6. In-Place Upgrades

To upgrade to a subsequent beta build:

1. Obtain the updated installer package (e.g. `MultiNetCyberShield-Setup.msi`).
2. Execute the installer directly:
   ```powershell
   msiexec.exe /i MultiNetCyberShield-Setup.msi /qn /l*v "C:\Temp\MultiNet-Upgrade.log"
   ```
3. Windows Installer performs a transactional `MajorUpgrade`:
   - Gracefully stops the active `MultiNetNetMerge` service.
   - Updates executable binaries and drivers in `C:\Program Files\MultiNet CyberShield AI`.
   - Preserves user configuration and SQLite database in `%APPDATA%\ai.multinet.cybershield\`.
   - Restarts the updated service and verifies health.

---

### 7. Uninstallation & Clean Removal

To remove MultiNet CyberShield AI:

```powershell
# Unattended clean removal:
msiexec.exe /x MultiNetCyberShield-Setup.msi /qn /l*v "C:\Temp\MultiNet-Uninstall.log"
```

During uninstallation:
- The `MultiNetNetMerge` service is stopped and removed from the Service Control Manager.
- All installed files in `C:\Program Files\MultiNet CyberShield AI` and `drivers\` are deleted.
- Start Menu folder and Desktop shortcuts are removed.
- **User Data Preservation:** The user SQLite database located in `%APPDATA%\ai.multinet.cybershield\multinet-cybershield-ai.db` is intentionally preserved to prevent data loss. If a complete purge is desired, delete `%APPDATA%\ai.multinet.cybershield`.

---

### 8. Disaster Recovery & Network Route Rollback

MultiNet CyberShield AI includes built-in automated disaster recovery. If a power interruption or abrupt crash occurs while network bonding or tunnel capture was active:

1. **Automatic Self-Healing on Service Startup:**
   When `MultiNetNetMerge` initializes, it automatically runs `RunDisasterRecovery()`:
   - Scans and deletes any orphaned capture routes (`0.0.0.0/1` and `128.0.0.0/1`).
   - Reads `%TEMP%\net-merge-service-weakhost.txt` and restores original physical interface weak-host and forwarding flags.

2. **Manual Emergency Route Restoration (CLI):**
   If testing outside the service lifecycle, network state can be manually reset via elevated PowerShell:
   ```powershell
   # Remove lingering Wintun capture routes:
   route delete 0.0.0.0 mask 128.0.0.0
   route delete 128.0.0.0 mask 128.0.0.0

   # Restore default weak-host settings on physical network adapters:
   Get-NetIPInterface | Set-NetIPInterface -WeakHostReceive Disabled -WeakHostSend Disabled
   ```

3. **Standalone Service Management CLI:**
   ```powershell
   cd "C:\Program Files\MultiNet CyberShield AI"
   
   # Check service status:
   .\net-merge-service.exe -action status

   # Stop service:
   .\net-merge-service.exe -action stop

   # Start service:
   .\net-merge-service.exe -action start
   ```
