# System Architecture (Phase 1)

## Engines
```mermaid
flowchart TB
  subgraph UI[Desktop App — Tauri 2 + React]
    NAV[Navigation / Dashboards]
    COP[AI Copilot UI placeholder]
  end

  subgraph CORE[Shared Rust Core Interfaces]
    NP[NetworkPlatform]
    BE[BondingEngine]
    SE[SecurityEngine]
    SP[SecurityPolicy + Approvals]
    AI[AIProvider]
    AUD[Audit Logger]
    TEL[Telemetry Validation]
  end

  subgraph NET[MultiNet Network Engine]
    IF[Interface Discovery]
    NM[Network Monitor]
    RT[Routing Engine]
    BN[Bonding Engine]
    TN[Tunnel Protocol/Security]
  end

  subgraph SEC[CyberShield Security Engine]
    IDS[Intrusion Detection]
    EPM[Endpoint Monitor]
    TI[Threat Intelligence]
    DNSP[DNS Protection]
    FW[AI Firewall / Rules Mgmt]
    IR[Incident Response]
  end

  subgraph AIE[AI Intelligence Engine]
    ORCH[AI Orchestrator]
    EXPL[Explain/Recommend]
  end

  subgraph DATA[Data]
    SQL[(SQLite — migrations)]
  end

  UI --> CORE
  CORE --> NET
  CORE --> SEC
  CORE --> AIE
  SP --> AUD
  NET --> TEL
  SEC --> TEL
  TEL --> SQL
  AUD --> SQL
  AI -. never executes shell / OS security mutations .-> SP
```

## Phase 1 reality
- Traits compile as the contract; **mock/stub** implementations only in tests/foundation.
- No platform drivers, no packet capture, no IDS runtime, no route/firewall mutation, no cloud/relay execution.
