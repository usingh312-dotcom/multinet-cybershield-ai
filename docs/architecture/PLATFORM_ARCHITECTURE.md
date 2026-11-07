# Platform Architecture (Phase 1)

## macOS — Network Extension (architecture only)
- **Target capability**: packet tunnel / filter / DNS proxy / app proxy classes as evaluated; requires Apple entitlements, provisioning, and user-approved system extension install.
- **Foundation status**: `platform/macos/network-extension/` placeholder only. **No extension, no entitlements, no network configuration.**
- **Risks**: notarization/signing, user consent, MDM vs consumer install, App Store constraints.

## Windows — IP Helper + Wintun + Service (architecture only)
- **IP Helper APIs**: interface enumeration/metrics/routes (read-focused initially) via documented Windows APIs; privileged operations require elevation/service boundary design.
- **Wintun**: user-mode TUN adapter for future tunnel/bonding dataplane; requires driver/adapter lifecycle and admin considerations.
- **Windows Service**: privileged agent boundary for approved operations (future); strict IPC + approval gating.
- **Foundation status**: placeholders under `platform/windows/*`. **No driver install, no service registration, no route/firewall changes.**

## Cross-platform rule
All platform effects flow through `NetworkPlatform`/policy gates; Phase 1 provides **no production platform backends**.
