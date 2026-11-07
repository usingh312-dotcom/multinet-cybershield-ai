# Threat Model (Phase 1 — foundation)

## Assets
- Host network configuration integrity, credentials/secrets, local telemetry/DB, audit trail, user trust decisions.

## Trust boundaries
- UI (untrusted input) ↔ Tauri IPC ↔ Rust core ↔ (future) privileged platform agent/drivers.
- Local app ↔ future relay/cloud APIs.
- Third-party security tool outputs (integrations) treated as untrusted until validated.

## Key threats & mitigations (planned/foundation)
| Threat | Foundation control |
|---|---|
| Unauthorized network/firewall changes | `SecurityPolicy` requires approval for defensive/system actions; **foundation executes no mutations** |
| Arbitrary command execution via AI/UI | Policy denies shell-like actions; AI traits return stub guidance only |
| Secret leakage | No plaintext API keys/passwords in DB schema; add secure storage abstraction (OS keychain/DPAPI) before storing secrets |
| Telemetry spoofing / fake metrics | UI shows no production metrics; future events require `source` + validation |
| Privilege escalation | Least-privilege design; platform code isolated under `platform/` with explicit boundaries |
| Supply-chain / integration risk | Integrations are placeholders; verify compatibility and signatures before enablement |

## Residual risk
High until platform backends, sandboxing, update signing, and verified collectors are implemented and tested.
