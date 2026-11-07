# Backend Audit (Phase 1)

## Scope
Foundation-only audit of the **planned** MultiNet backend components as represented by crate/module boundaries.

## Findings
| Area | Current state | Risk | Next step |
|---|---|---|---|
| Interface discovery | Trait + mock | Platform APIs/privilege differences (macOS/Windows) | Read-only discovery impls with explicit permissions |
| Metrics | Typed structs, no collectors | Misleading metrics if simulated | Only ingest verified counters/probes; label source |
| Routing/virtual IF | Trait methods stubbed; `configure_routes` rejected in mock | System destabilization, privilege escalation | Dry-run planner + rollback + human approval + OS-specific implementation |
| Bonding/tunnel | Mock reports initialized/no paths | Security/MTU/rekey complexity | Design protocol with `tunnel-security` crate; threat model per path |
| Relay services | Binaries placeholders only | Authz/abuse, multi-tenant risk | Define mTLS/auth, quotas, audit, rate limits before coding |

## Non-goals verified
- No default route changes, no interface rebind side effects, no tunnel started.
