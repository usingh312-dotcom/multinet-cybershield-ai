# MultiNet CyberShield AI

**Connect. Protect. Detect.** — desktop foundation (v0.1.0, Phase 1).

## What this is (Phase 1)
A cross-platform desktop application foundation (macOS/Windows targets) with **three independent subsystems** and shared, typed Rust interfaces:

1. **MultiNet Network Engine** — interface discovery/metrics/routing/bonding abstractions (stubs/mocks only; **no network configuration changes**).
2. **CyberShield Security Engine** — event collection/findings/status abstractions (stubs/mocks only; **no OS security changes**).
3. **AI Intelligence Engine** — analyze/explain/recommend abstractions (mock only; **no arbitrary shell, no direct OS security mutations**).

## Repo layout (foundation)
- `apps/desktop/` — Tauri 2 shell + React/TypeScript/Tailwind UI (navigation placeholders; **no fake production metrics**).
- `crates/` — Rust workspace crates (core traits, policy, telemetry, audit, bonding/security/AI mocks, stubs for planned modules).
- `platform/` — macOS Network Extension and Windows Wintun/service **architecture placeholders**.
- `relay/` — future bonding server/session manager/security API placeholders.
- `integrations/` — evaluation placeholders (Suricata/Zeek/Wazuh/CrowdSec/ClamAV/YARA). **No native desktop support assumed.**
- `database/sqlite/migrations/` — initial schema migrations.
- `docs/` — architecture, networking/security notes, daily report.

## Safety guarantees (Phase 1)
- No routing/firewall/DNS/OS security settings modified.
- No unverified networking or security features implemented.
- Human approval required for defensive actions (policy foundation); foundation **executes no system mutations**.
- Local audit logging abstractions present; secrets must not be stored in plaintext (no secrets committed).

## Build / test (adapt to environment)
From repo root (requires Rust toolchain + Node/npm; Tauri platform deps vary by OS):
```bash
npm --prefix apps/desktop install
npm --prefix apps/desktop run build
cargo check --workspace   # if all members are present/toolchain installed
cargo test --workspace
npm --prefix apps/desktop run tauri dev
```

## Status
See `IMPLEMENTATION_STATUS.md` and `docs/daily-reports/DAY_01.md`.
