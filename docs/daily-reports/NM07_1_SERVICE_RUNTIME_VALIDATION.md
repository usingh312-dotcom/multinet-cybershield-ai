# MultiNet CyberShield AI — Milestone NM-07.1
# Service Runtime & Windows SCM Integration Report

**Document ID:** `NM07_1_SERVICE_RUNTIME_VALIDATION`  
**Date:** 2026-11-10  
**Status:** QUALIFIED / CI VERIFIED  
**Author:** Senior Windows Systems Engineer & Go Systems Architect  
**Repository:** `usingh312-dotcom/multinet-cybershield-ai`  
**Branch:** `master`  
**Milestone:** NM-07.1 (Controlled Beta Runtime Qualification)

---

## 1. Executive Summary

The MultiNet Net-Merge Service (`net-merge-service.exe`) is the privileged core of MultiNet CyberShield AI on Windows. It manages the Wintun adapter, network interface routing, tunnel exclusivity, and packet interception. This report validates the service's execution under the Windows Service Control Manager (SCM), named pipe IPC server initialization, disaster self-healing, and graceful termination.

---

## 2. Test Execution & Coverage Status Matrix

| Subsystem | Requirement | Method | Environment | Status |
| :--- | :--- | :--- | :--- | :--- |
| **SCM Runner** | `svc.IsWindowsService()` dispatching | Automated SCM invocation | CI (Windows Server 2025) | **CI tested** |
| **CLI Actions** | `-action start/stop/status` | Process command line | Local / CI | **Locally tested / CI tested** |
| **Named Pipe IPC** | `\\.\pipe\multinet-cybershield-ipc` | IPC listener creation | Local / CI | **Locally tested / CI tested** |
| **SDDL Pipe Security** | Admin/LocalSystem/Owner DACL | `pipesec_windows.go` audit | Local / CI | **Locally tested / CI tested** |
| **Disaster Recovery** | `RunDisasterRecovery()` self-heal | Crash file restoration | Local / CI | **Locally tested / CI tested** |
| **Graceful Shutdown** | SIGTERM / SCM Stop event | Route cleanup & socket release | Local / CI | **Locally tested / CI tested** |

---

## 3. SCM Service Architecture

### 3.1. Dual-Mode Entry Point (`services/net-merge/main.go`)
- **Service Mode:** Automatically detected via `svc.IsWindowsService()`. Dispatches `svcrunner.RunService()` which hooks `svc.Handler` (Execute method handling `svc.Stop`, `svc.Shutdown`, `svc.Interrogate`).
- **Console Mode:** Invoked with `-console` or when run interactively in a developer terminal.
- **Service Management Mode:** Invoked with `-action install|uninstall|start|stop|status` to manage the SCM configuration programmatically.

### 3.2. SCM Identification Constants (`internal/svcrunner/service_windows.go`)
- **Service Name:** `MultiNetNetMerge`
- **Display Name:** `MultiNet Net-Merge Service`
- **Account:** `LocalSystem`
- **Startup Type:** `Automatic` (`Start="auto"`)

---

## 4. Disaster Recovery & Route Protection

On startup, `net-merge-service.exe` executes `svcrunner.RunDisasterRecovery()` before accepting connections:
1. **Crash File Check:** Inspects `%TEMP%\net-merge-service-weakhost.txt`. If present, parses saved interface index and weak-host parameters, issuing `netsh interface <fam> set interface interface=<id> weakhostsend=<val> forwarding=<val>` to restore original physical settings.
2. **Orphaned Route Sweep:** Scans the Windows routing table for lingering `0.0.0.0/1` and `128.0.0.0/1` routes and deletes them if no active Wintun session is running.
3. **Graceful Engine Shutdown:** Releases kernel mode mutex (`Global\multinet-netmerge-mode-owner-v1`), removes capture routes, and deletes temporary tracking state.

