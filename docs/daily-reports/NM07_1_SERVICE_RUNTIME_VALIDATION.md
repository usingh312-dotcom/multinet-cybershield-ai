# MultiNet CyberShield AI — Milestone NM-07.1 / NM-07.1.1
# Service Runtime & Windows SCM Integration Report

**Document ID:** `NM07_1_SERVICE_RUNTIME_VALIDATION`  
**Date:** 2026-11-10  
**Status:** QUALIFIED / CI VERIFIED  
**Author:** Senior Windows Systems Engineer & Go Systems Architect  
**Repository:** `usingh312-dotcom/multinet-cybershield-ai`  
**Branch:** `master`  
**Milestone:** NM-07.1 / NM-07.1.1 (Runtime Test Integrity & Final Beta Qualification)

---

## 1. Executive Summary

The MultiNet Net-Merge Service (`net-merge-service.exe`) is the privileged core of MultiNet CyberShield AI on Windows. It manages the Wintun adapter, network interface routing, tunnel exclusivity, and packet interception.

In NM-07.1.1, the service test harnesses were updated to:
1. Verify SCM registration with `LocalSystem` account and exact binary path matching.
2. Establish a genuine `NamedPipeClientStream` connection to the actual endpoint `\\.\pipe\multinet-netmerge-v1`.
3. Dispatch a live `health` request and validate JSON response schema (`ok: true`, `pipe_version: 1`).
4. Support explicit CLI disaster recovery via `-action recover`.

---

## 2. Test Execution & Coverage Status Matrix by Environment

| Subsystem | Requirement | Method | Environment | Status |
| :--- | :--- | :--- | :--- | :--- |
| **SCM Runner** | `svc.IsWindowsService()` dispatching | Automated SCM invocation | Cloud CI (Windows Server 2025) | **CI tested** |
| **CLI Actions** | `-action start/stop/status/recover` | Process command line | Local / Cloud CI | **Locally tested / CI tested** |
| **Named Pipe IPC** | `\\.\pipe\multinet-netmerge-v1` | `NamedPipeClientStream` live query | Cloud CI / VM | **CI tested / VM tested** |
| **SDDL Pipe Security** | Admin/LocalSystem/Owner DACL | `pipesec_windows.go` audit | Local / Cloud CI | **Locally tested / CI tested** |
| **Disaster Recovery** | `RunDisasterRecovery()` self-heal | Crash file restoration via CLI | Local / Cloud CI | **Locally tested / CI tested** |
| **Graceful Shutdown** | SIGTERM / SCM Stop event | Route cleanup & socket release | Cloud CI / VM | **CI tested / VM tested** |

---

## 3. SCM Service Architecture

### 3.1. Dual-Mode Entry Point (`services/net-merge/main.go`)
- **Service Mode:** Automatically detected via `svc.IsWindowsService()`. Dispatches `svcrunner.RunService()` which hooks `svc.Handler` (Execute method handling `svc.Stop`, `svc.Shutdown`, `svc.Interrogate`).
- **Console Mode:** Invoked with `-console` or when run interactively in a developer terminal.
- **Service Management Mode:** Invoked with `-action install|uninstall|start|stop|status|recover` to manage the SCM configuration programmatically or trigger recovery on demand.

### 3.2. SCM Identification Constants (`internal/svcrunner/service_windows.go`)
- **Service Name:** `MultiNetNetMerge`
- **Display Name:** `MultiNet Net-Merge Service`
- **Account:** `LocalSystem`
- **Startup Type:** `Automatic` (`Start="auto"`)
- **Named-Pipe Endpoint:** `\\.\pipe\multinet-netmerge-v1`

---

## 4. Live IPC Verification & Protocol Contract

During installer qualification, after verifying that the service is registered and running under SCM:
1. `System.IO.Pipes.NamedPipeClientStream` connects to endpoint `multinet-netmerge-v1`.
2. A JSON message is formatted and sent:
   ```json
   {"id":"lifecycle-health-chk","pipe_version":1,"method":"health"}
   ```
3. The service receives the request, identifies the client process via `GetNamedPipeClientProcessId()`, routes the method through `internal/ipc/router.go`, and responds:
   ```json
   {"id":"lifecycle-health-chk","ok":true,"result":{"status":"healthy","pipe_version":1}}
   ```
4. The test asserts that the response is received within 5,000ms and contains `pipe_version: 1` and `ok: true`.
