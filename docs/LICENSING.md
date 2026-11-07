# Licensing Notes (Phase 1 foundation)

This repository foundation is marked `license = "Proprietary"` in the Rust workspace pending a final licensing decision by the project owner.

## Third-party components (planned/evaluated — not bundled as integrated runtime in Phase 1)
Before integrating any of the following, verify license compatibility, redistribution terms, and desktop-platform support:

| Component | Typical license (verify upstream) | Note |
|---|---|---|
| Suricata | GPLv2 | Network IDS; deployment model/compatibility varies |
| Zeek | BSD-style | Network analysis framework; often server-oriented |
| Wazuh | GPLv2 / Apache components (verify) | Agent/manager architecture; desktop fit must be validated |
| CrowdSec | MIT (core, verify) | Bouncer/agent model; verify components used |
| ClamAV | GPLv2 | AV engine; resource/UX implications on desktop |
| YARA | BSD 3-Clause | Pattern matching; rule licensing separate |

## Dependencies
Tauri/React/Rust crates introduce their own licenses; generate and review SBOM/NOTICE artifacts before release.
