# MultiNet CyberShield AI — Beta Installation & Rollback Guide
## Version 0.1.0-beta.1 (Milestone NM-07.0)

This guide provides instructions for deploying, updating, repairing, and rolling back MultiNet CyberShield AI during the controlled beta evaluation.

---

### 1. System Requirements

- **Operating System:** Windows 10 (64-bit, Build 19041 or higher) or Windows 11 (64-bit).
- **Architecture:** x86_64 (AMD64).
- **Privileges:** Administrator access required for installation and Windows Service management.
- **Hardware:**
  - Minimum 4 GB RAM.
  - 100 MB available disk storage for application binaries and state database.
  - Two or more active network adapters (e.g. Ethernet + Wi-Fi, or Wi-Fi + Cellular hotspot) to test multi-path bonding.

---

### 2. Standard Installation (Interactive GUI)

1. Download `MultiNetCyberShield-v0.1.0-beta.1-Windows-x64.zip` and extract its contents to a local directory.
2. Verify the package checksum against `checksums.sha256`:
   ```powershell
   Get-FileHash MultiNetCyberShield-Setup.msi -Algorithm SHA256
   ```
3. Double-click `MultiNetCyberShield-Setup.msi`.
4. When prompted by User Account Control (UAC), click **Yes** to grant installation privileges.
5. The installer completes the following setup automatically:
   - Stages program binaries into `C:\Program Files\MultiNet CyberShield AI`.
   - Stages `wintun.dll` driver into the `drivers\` subfolder.
   - Registers and starts the Windows Service `MultiNetNetMerge`.
   - Creates Start Menu and Desktop shortcuts for the user application.
6. Launch **MultiNet CyberShield AI** from the Desktop shortcut or Start Menu.

---

### 3. Enterprise Silent Deployment & Unattended Testing

To install quietly without user interface prompts (ideal for automated CI or lab testing):

```powershell
# Unattended Installation with Verbose Logging:
msiexec.exe /i MultiNetCyberShield-Setup.msi /qn /l*v "C:\Temp\MultiNet-Install.log"
```

Verify that the service is running:
```powershell
Get-Service -Name "MultiNetNetMerge"
```

Expected status: `Running`.

---

### 4. Package Repair & Integrity Re-validation

If any files, drivers, or service registry keys become corrupted or deleted:

```powershell
# Repair existing installation:
msiexec.exe /fa MultiNetCyberShield-Setup.msi /qn /l*v "C:\Temp\MultiNet-Repair.log"
```

The repair operation validates checksums of all installed files, restores missing assets, and re-registers the SCM service without affecting the user state database.

---

### 5. In-Place Upgrades

To upgrade to a subsequent beta build:

1. Obtain the updated installer (e.g., `MultiNetCyberShield-Setup-v0.1.0-beta.2.msi`).
2. Execute the installer directly:
   ```powershell
   msiexec.exe /i MultiNetCyberShield-Setup-v0.1.0-beta.2.msi /qn /l*v "C:\Temp\MultiNet-Upgrade.log"
   ```
3. Windows Installer performs a transactional `MajorUpgrade`:
   - Gracefully stops the active `MultiNetNetMerge` service.
   - Updates executable binaries and drivers in `C:\Program Files\MultiNet CyberShield AI`.
   - Preserves user configuration and SQLite database in `%APPDATA%\ai.multinet.cybershield\`.
   - Automatically restarts the updated service and verifies health.

---

### 6. Uninstallation & Clean Removal

To remove MultiNet CyberShield AI:

```powershell
# Interactive removal via Windows Settings -> Installed Apps, or CLI:
msiexec.exe /x MultiNetCyberShield-Setup.msi /qn /l*v "C:\Temp\MultiNet-Uninstall.log"
```

During uninstallation:
- The `MultiNetNetMerge` service is stopped and removed from the Windows Service Control Manager.
- All files in `C:\Program Files\MultiNet CyberShield AI` are deleted.
- Desktop and Start Menu shortcuts are deleted.
- **User Data Policy:** The user database located in `%APPDATA%\ai.multinet.cybershield\multinet-cybershield-ai.db` is intentionally preserved to prevent data loss. If a 100% clean wipe is desired, manually delete the `%APPDATA%\ai.multinet.cybershield` folder.

---

### 7. Disaster Recovery & Manual Route Rollback

MultiNet CyberShield AI includes automated disaster recovery. If a crash or abrupt reboot occurs while bonding was active:

1. **Automatic Recovery:**
   When the service starts, it automatically detects any orphaned capture routes (`0.0.0.0/1` and `128.0.0.0/1`) and restores weak-host settings from `%TEMP%\net-merge-service-weakhost.txt`.

2. **Manual Emergency Route Cleanup (CLI):**
   If testing without the service running, routes can be manually restored via elevated PowerShell:
   ```powershell
   # Remove any lingering Wintun capture routes:
   route delete 0.0.0.0 mask 128.0.0.0
   route delete 128.0.0.0 mask 128.0.0.0

   # Restore default weak-host settings on physical NICs:
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

