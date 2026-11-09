# Third-party notices for services/net-merge

This service integrates the Net Merge multi-WAN engine (upstream) and several
open-source libraries. Upstream attribution is preserved verbatim below.

## net-merge (upstream)
- Source: https://github.com/net-merge/net-merge
- Pinned commit: 2b333a3ad820996ba337588adfa8f7259a45baec (Release 0.3.7)
- License: MIT (Copyright 2026 Rohit Mahto)
- Notice: See services/net-merge/LICENSE and the upstream LICENSE file.
- Attribution: "net-merge by Rohit Mahto" — preserved in source headers.

## Wintun (golang.zx2c4.com/wintun)
- Prebuilt, signed DLL from WireGuard LLC (v0.14.1)
- Fetched at install time via scripts/fetch-wintun.ps1 with SHA-256 and
  Authenticode signature verification.
- Not part of this repository; licensed separately by WireGuard LLC.

## gVisor (gvisor.dev/gvisor)
- License: Apache 2.0

## tun2socks (github.com/xjasonlyu/tun2socks/v2)
- License: MIT

## golang.org/x/sys
- License: BSD-style (see individual files)

Multinet CyberShield AI (Rust crates, this repository) is Proprietary
licensed and is a separate work that *controls* this service over a local
IPC boundary; it does not modify net-merge source.
