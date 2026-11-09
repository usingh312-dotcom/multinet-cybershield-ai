# MultiNet CyberShield AI — Milestone NM-06.4 Upgrade & Rollback Report
## Major Upgrade Strategy, Database Migration Safety & Recovery

**Project:** MultiNet CyberShield AI  
**Repository:** `usingh312-dotcom/multinet-cybershield-ai`  
**Branch:** `master`  
**Milestone:** NM-06.4  
**Date:** November 9, 2026  
**Auditor:** Principal Software Architect & QA Automation Lead  

---

## 1. Executive Summary

Milestone **NM-06.4** defines and verifies the upgrade, migration, and rollback procedures for MultiNet CyberShield AI. Upgrades from earlier versions must be smooth, unattended, non-destructive to user data, and fully recoverable if the new version fails to initialize.

---

## 2. Upgrade Architecture

### 2.1 Major Upgrade Identification
- **Upgrade Code:** `{4E8D7F91-C8A2-4D3B-98F1-2B8C67D3A109}` (Constant across all product versions).
- **Product ID:** Generated dynamically per build (enabling Windows Installer Major Upgrade detection).
- **Rule in WiX:**
  ```xml
  <MajorUpgrade DowngradeErrorMessage="A newer version of MultiNet CyberShield AI is already installed."
                AllowSameVersionUpgrades="yes" />
  ```

### 2.2 Upgrade Execution Sequence
```
+-------------------------------------------------------------+
| 1. Installer Initialization & Version Detection              |
|    - FindRelatedProducts queries registry for UpgradeCode   |
|    - If Target Version < Installed Version: Halt (Downgrade) |
|    - If Target Version >= Installed Version: Proceed         |
+------------------------------+------------------------------+
                               |
                               v
+-------------------------------------------------------------+
| 2. Existing Service Graceful Shutdown                        |
|    - SCM issues SERVICE_CONTROL_STOP to MultiNetNetMerge    |
|    - Service closes active named pipe, releases routes       |
|    - Service terminates within 30-second SCM timeout        |
+------------------------------+------------------------------+
                               |
                               v
+-------------------------------------------------------------+
| 3. Atomic Binary Replacement                                |
|    - Old binaries in Program Files are unlinked             |
|    - Updated net-merge-service.exe and GUI are copied       |
|    - wintun.dll driver updated if version incremented       |
+------------------------------+------------------------------+
                               |
                               v
+-------------------------------------------------------------+
| 4. Service Re-registration & Startup                         |
|    - Service config updated in SCM                          |
|    - SCM issues StartService                                |
|    - New service initializes disaster recovery & named pipe |
+------------------------------+------------------------------+
                               |
                               v
+-------------------------------------------------------------+
| 5. Post-Upgrade Health Verification                          |
|    - Named pipe ping with method "health"                   |
|    - Verification of Mode Coordinator availability          |
|    - If healthy: Commit upgrade transaction                 |
|    - If failed: Trigger MSI Kernel Rollback                 |
+-------------------------------------------------------------+
```

---

## 3. Database Migration Safety & Configuration Preservation

### 3.1 Separation of Program and Data Files
All user diagnostic data, metrics, security logs, and configuration settings are strictly separated from application binaries:
- **Program Files:** `C:\Program Files\MultiNet CyberShield AI\` (Read-only for standard users; overwritten during upgrades).
- **User Data:** `%APPDATA%\ai.multinet.cybershield\multinet-cybershield-ai.db` (User-writable; completely isolated from MSI file actions).

### 3.2 SQLx Migration Strategy
The SQLite database schema evolves via versioned SQL migrations embedded into the desktop binary via `sqlx::migrate!`:
- `001_foundation.sql`: Core schema, settings, audit logs.
- `002_security_infrastructure.sql`: Incidents, threat intelligence, security policies.
- `003_diagnostics.sql`: Traffic samples, network interfaces, speed test results.

### 3.3 Transactional Safety Rules
1. **Write-Ahead Logging (WAL):** Database uses `PRAGMA journal_mode=WAL`, enabling concurrent readers during service writes.
2. **Atomic Migration Runs:** Each migration runs inside a single database transaction. If any migration step fails, the transaction rolls back, leaving the database at the prior valid version.
3. **No Drop Policies:** Production migrations never drop historical tables. Deprecated columns remain nullable to preserve backward compatibility.

---

## 4. Disaster Recovery & Rollback Scenarios

### Scenario A: Binary Fails to Start After Upgrade
- **Trigger:** Upgraded service binary crashes on launch due to missing system dependency.
- **Handling:** SCM fails to receive `SERVICE_RUNNING` within timeout. The MSI installation engine intercepts the failure and initiates rollback:
  - SCM configuration is reverted to prior version.
  - Previous binary files are restored from backup cache.
  - Prior service is restarted and verified.

### Scenario B: Host Reboots During Active Capture (Mode A)
- **Trigger:** Power loss while Wintun capture routes `0.0.0.0/1` and `128.0.0.0/1` were active.
- **Handling:** On next boot, `net-merge-service.exe` executes `svcrunner.RunDisasterRecovery()`:
  - Scans routing table for orphaned routes pointing to inactive virtual adapters.
  - Deletes orphaned capture routes.
  - Reads `%TEMP%\net-merge-service-weakhost.txt` and restores physical adapter weak-host/forwarding settings.
  - Host internet connectivity is restored automatically before user login.

---

## 5. Summary Matrix

| Failure Mode | Impact | Recovery Mechanism | Verified Result |
|---|---|---|---|
| **Downgrade Attempt** | User attempts installing older MSI | Blocked by `MajorUpgrade` rule with error dialog | PASS |
| **Aborted Upgrade** | Power cut mid-installation | MSI transaction rollback restores previous files | PASS |
| **Database Corruption Risk** | Schema migration interrupted | SQLite WAL journal ensures zero partial transactions | PASS |
| **Orphaned Routes After Crash** | Lost internet access | Automatic `RunDisasterRecovery` on service startup | PASS |

