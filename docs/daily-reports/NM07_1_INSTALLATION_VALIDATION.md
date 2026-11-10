# MultiNet CyberShield AI — Milestone NM-07.1
# MSI Installation, Lifecycle & Repair Qualification Report

**Document ID:** `NM07_1_INSTALLATION_VALIDATION`  
**Date:** 2026-11-10  
**Status:** QUALIFIED / CI VERIFIED  
**Author:** Principal Windows Installer Engineer & Release QA Lead  
**Repository:** `usingh312-dotcom/multinet-cybershield-ai`  
**Branch:** `master`  
**Milestone:** NM-07.1 (Controlled Beta Runtime Qualification)

---

## 1. Executive Summary

Milestone NM-07.1 establishes end-to-end qualification of the Windows Installer (MSI) lifecycle for MultiNet CyberShield AI. Unlike previous milestones that verified only compile-time PE headers and package archive packaging, NM-07.1 executes real silent installation, file placement verification, service registration, component repair, and clean uninstallation in an ephemeral Windows environment.

---

## 2. Test Execution & Coverage Status Matrix

| Stage | Test Name | Method / Target | Environment | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Stage 1** | Pre-Execution Sanitation | SCM query & cleanup | CI (Windows Server 2025) | **CI tested** |
| **Stage 2** | Silent Clean Installation | `msiexec.exe /i ... /qn /l*v` | CI (Windows Server 2025) | **CI tested** |
| **Stage 3** | File & Asset Placement | AMD64 PE verification | Local / CI | **Locally tested / CI tested** |
| **Stage 4** | Wintun Dual Placement | Root & `drivers\` placement | Local / CI | **Locally tested / CI tested** |
| **Stage 5** | SCM Service Registration | `MultiNetNetMerge` in SCM | CI (Windows Server 2025) | **CI tested** |
| **Stage 6** | MSI Component Repair | Simulated file deletion + `/fa` | CI (Windows Server 2025) | **CI tested** |
| **Stage 7** | Clean Uninstallation | `msiexec.exe /x ... /qn /l*v` | CI (Windows Server 2025) | **CI tested** |
| **Stage 8** | Orphan & State Audit | Service, route & file cleanup | CI (Windows Server 2025) | **CI tested** |
| **Stage 9** | Physical Dual-NIC Bonding | Real hardware failover | Physical dual-NIC lab | **Physical hardware tested** |

---

## 3. WiX v5 Packaging Architecture (`MultiNetCyberShield.wxs`)

### 3.1. Target Directory Structure
```
C:\Program Files\MultiNet CyberShield AI\
├── multinet-cybershield-ai.exe   (Tauri v2 GUI Shell, AMD64)
├── net-merge-service.exe         (Go Background Network Service, AMD64)
├── wintun.dll                    (WireGuard LLC Wintun Driver, AMD64)
└── drivers\
    └── wintun.dll                (Identical WireGuard Driver Copy)
```

### 3.2. Service Installation Configuration
```xml
<ServiceInstall Id="ServiceInstaller"
                Type="ownProcess"
                Vital="yes"
                Name="MultiNetNetMerge"
                DisplayName="MultiNet Net-Merge Service"
                Description="Manages virtual network adapter bonding, tunnel exclusivity, and packet routing for MultiNet CyberShield AI."
                Start="auto"
                Account="LocalSystem"
                ErrorControl="normal" />

<ServiceControl Id="ServiceController"
                Name="MultiNetNetMerge"
                Start="install"
                Stop="both"
                Remove="uninstall"
                Wait="yes" />
```

---

## 4. Test Harness Implementation (`packaging/test-installer-lifecycle.ps1`)

The test harness exercises the complete lifecycle through silent Windows Installer commands:
1. **Clean Installation:** `msiexec.exe /i MultiNetCyberShield-Setup.msi /qn /l*v msi-install.log` (Validates exit code 0).
2. **Key File Verification:** Confirms `multinet-cybershield-ai.exe`, `net-merge-service.exe`, `wintun.dll`, and `drivers\wintun.dll` exist with non-zero length and valid PE headers.
3. **Service Registration:** Queries `Get-Service -Name "MultiNetNetMerge"`.
4. **Repair Simulation:** Intentionally deletes `drivers\wintun.dll`, then executes `msiexec.exe /fa MultiNetCyberShield-Setup.msi /qn /l*v msi-repair.log`, verifying that the key file is restored.
5. **Clean Removal:** Executes `msiexec.exe /x MultiNetCyberShield-Setup.msi /qn /l*v msi-uninstall.log`, verifying SCM service deletion, directory cleanup, and zero orphaned routes.
