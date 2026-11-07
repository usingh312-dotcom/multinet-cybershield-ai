# Security Policy (Phase 1)

## Principles
1. **Least privilege** — components request minimal capabilities; platform effects isolated.
2. **Explicit permission boundaries** — Tauri capabilities minimal; OS actions require dedicated gated paths.
3. **Human approval for every defensive action** — firewall/routing/DNS/block/quarantine/network changes require explicit approval; foundation records intent but **does not execute** system mutations.
4. **No arbitrary shell / no direct OS security mutation by AI** — enforced by policy validation (deny shell); AI provider is advisory-only stubs in Phase 1.
5. **Auditability** — local audit events for deny/approval/rollback intent; extend to immutable storage in later phases.
6. **Rollback** — network/security changes must be reversible; rollback API present, production rollback implemented with platform code.
7. **Structured event validation** — telemetry validates required fields; reject/flag malformed inputs.

## Data handling
- No passwords or plaintext API keys in SQLite. Use OS-backed secret storage before introducing credentials.

## Incident response actions (future)
All automated responses default to **recommendation + approval queue**; never silent destructive actions.
