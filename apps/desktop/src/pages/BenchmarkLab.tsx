import React, { useState } from "react";
import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { StatusIndicator } from "../components/ui/feedback";
import { IntegrationNotice } from "../components/ui/controls";
import { EmptyState } from "../components/ui/feedback";
import { IconShield, IconDatabase, IconActivity } from "../components/ui/icons";

interface SyntheticScenario {
  id: string;
  name: string;
  tier: "SYNTHETIC" | "LOOPBACK" | "BONDED_E2E";
  status: "PASS" | "FAIL" | "BLOCKED";
  throughput: string;
  latency: string;
  description: string;
  reason?: string;
}

const illustrativeScenarios: SyntheticScenario[] = [
  {
    id: "synth_sched_single_path",
    name: "Single-Path Scheduler Baseline",
    tier: "SYNTHETIC",
    status: "PASS",
    throughput: "100.0 Mbps (Configured)",
    latency: "12.0 ms (Target)",
    description: "Algorithmic validation of deficit round-robin with single path.",
  },
  {
    id: "synth_sched_dual_equal",
    name: "Dual Equal-Capacity Paths Distribution",
    tier: "SYNTHETIC",
    status: "PASS",
    throughput: "200.0 Mbps (Configured)",
    latency: "20.0 ms (Target)",
    description: "Balanced 50/50 packet and byte scheduling across equal paths.",
  },
  {
    id: "synth_sched_tri_asymmetric",
    name: "Three-Path Asymmetric Capacity Share",
    tier: "SYNTHETIC",
    status: "PASS",
    throughput: "680.0 Mbps (Configured Aggregate)",
    latency: "14.5 ms (Target)",
    description: "Proportional scheduling across Fiber (500M), Starlink (120M), and 5G (60M).",
  },
  {
    id: "synth_sched_degraded_shedding",
    name: "Degraded Path Dynamic Traffic Shedding",
    tier: "SYNTHETIC",
    status: "PASS",
    throughput: "Dynamic",
    latency: "Adaptive",
    description: "Automatic reduction of packet allocation to high-loss/high-latency path.",
  },
  {
    id: "loopback_udp_framing",
    name: "Loopback Authenticated Framing & Wire Sockets",
    tier: "LOOPBACK",
    status: "PASS",
    throughput: "Local Wire",
    latency: "< 0.5 ms",
    description: "Local 127.0.0.1 UDP transport with canonical wire framing and HMAC-SHA256 authentication.",
  },
  {
    id: "bonded_e2e_2wan",
    name: "Two-WAN Single-Flow Throughput Aggregation",
    tier: "BONDED_E2E",
    status: "BLOCKED",
    throughput: "—",
    latency: "—",
    description: "Real multi-WAN single TCP/UDP flow throughput aggregation.",
    reason: "BLOCKED: Remote relay ingress (relay/bonding-server) is a foundation placeholder. End-to-end multi-WAN bonding requires deployed relay ingress and return-path routing (scheduled for NM-08.1+).",
  },
  {
    id: "bonded_e2e_failover",
    name: "Single-Flow Failover Continuity",
    tier: "BONDED_E2E",
    status: "BLOCKED",
    throughput: "—",
    latency: "—",
    description: "Uninterrupted TCP transfer across physical link disconnect.",
    reason: "BLOCKED: TCP flow migration requires active proxy relay terminating the connection (scheduled for NM-08.1+).",
  },
];

export default function BenchmarkLabPage() {
  const [selectedTab, setSelectedTab] = useState<"results" | "models" | "competitor" | "cli">("results");

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="MultiNet Benchmark Lab"
          subtitle="Empirical multi-tier measurement, impairment simulation, and qualification platform."
          actions={<StatusIndicator variation="ok" label="Lab Engine Ready" />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            <strong className="font-semibold text-ink-primary">Evidence Tier Separation Active:</strong> All measurements are partitioned into strict evidence tiers (<code>SYNTHETIC</code>, <code>LOOPBACK</code>, <code>LIVE_INTERFACE</code>, <code>BONDED_E2E</code>, <code>COMPETITOR_EXTERNAL</code>). Synthetic scheduler decisions are never presented as physical WAN speed. Bonded E2E tests are formally marked <strong>BLOCKED</strong> until remote relay and reassembly planes are deployed.
          </IntegrationNotice>
        </div>
      </section>

      {/* Navigation Pills */}
      <div className="flex gap-2 border-b border-charcoal-600 pb-3 mt-4">
        <button
          onClick={() => setSelectedTab("results")}
          className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors ${
            selectedTab === "results"
              ? "bg-primary text-black font-bold"
              : "bg-surface hover:bg-elevated text-ink-secondary"
          }`}
        >
          Active Run Results
        </button>
        <button
          onClick={() => setSelectedTab("models")}
          className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors ${
            selectedTab === "models"
              ? "bg-primary text-black font-bold"
              : "bg-surface hover:bg-elevated text-ink-secondary"
          }`}
        >
          Illustrative Reference Models
        </button>
        <button
          onClick={() => setSelectedTab("competitor")}
          className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors ${
            selectedTab === "competitor"
              ? "bg-primary text-black font-bold"
              : "bg-surface hover:bg-elevated text-ink-secondary"
          }`}
        >
          Speedify Comparison Protocol
        </button>
        <button
          onClick={() => setSelectedTab("cli")}
          className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors ${
            selectedTab === "cli"
              ? "bg-primary text-black font-bold"
              : "bg-surface hover:bg-elevated text-ink-secondary"
          }`}
        >
          CLI & Automation
        </button>
      </div>

      {selectedTab === "results" && (
        <section className="mt-4 space-y-4">
          <SectionHeader
            title="Benchmark Run Results"
            description="Live execution results loaded from local netbench-cli run artifacts."
          />

          <div className="rounded-xl border border-charcoal-600 bg-elevated p-8 flex flex-col justify-center items-center text-center">
            <EmptyState
              icon={<IconActivity size={28} />}
              title="No Benchmark Run Loaded"
              description="Execute 'netbench-cli run' to run the deterministic test suite, or 'netbench-cli baseline --enable-live-network-tests' to measure a live interface baseline."
            />
            <div className="mt-4">
              <button
                onClick={() => setSelectedTab("models")}
                className="px-4 py-2 rounded-lg text-xs font-semibold bg-charcoal-800 hover:bg-charcoal-700 text-ink-primary border border-charcoal-600"
              >
                View Illustrative Reference Models →
              </button>
            </div>
          </div>
        </section>
      )}

      {selectedTab === "models" && (
        <section className="mt-4 space-y-4">
          <SectionHeader
            title="Illustrative Reference Models"
            description="Algorithmic specification models and target scenarios. These values represent simulated mathematical constraints, not live physical WAN throughput."
          />

          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel overflow-hidden">
            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs text-ink-primary">
                <thead className="border-b border-charcoal-600 bg-charcoal-800/50 text-[11px] uppercase tracking-wider text-ink-secondary">
                  <tr>
                    <th className="p-3">Scenario Name</th>
                    <th className="p-3">Evidence Tier</th>
                    <th className="p-3">Status</th>
                    <th className="p-3">Capacity Model</th>
                    <th className="p-3">Latency Model</th>
                    <th className="p-3">Technical Details & Architectural Rationale</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-charcoal-600/60 font-sans">
                  {illustrativeScenarios.map((sc) => (
                    <tr key={sc.id} className="hover:bg-charcoal-700/30">
                      <td className="p-3 font-medium text-white">{sc.name}</td>
                      <td className="p-3">
                        <span className="inline-block px-2 py-0.5 rounded text-[10px] font-bold tracking-wider bg-sky-950/60 border border-sky-600/40 text-sky-400">
                          {sc.tier}
                        </span>
                      </td>
                      <td className="p-3">
                        {sc.status === "PASS" ? (
                          <span className="inline-block px-2 py-0.5 rounded text-[10px] font-bold bg-green-950/60 border border-green-600/40 text-green-400">
                            PASS
                          </span>
                        ) : (
                          <span className="inline-block px-2 py-0.5 rounded text-[10px] font-bold bg-amber-950/60 border border-amber-600/40 text-amber-400">
                            BLOCKED
                          </span>
                        )}
                      </td>
                      <td className="p-3">{sc.throughput}</td>
                      <td className="p-3">{sc.latency}</td>
                      <td className="p-3 max-w-xs">
                        <p className="text-ink-secondary text-[11px]">{sc.description}</p>
                        {sc.reason && (
                          <p className="mt-1 text-[11px] font-medium text-amber-400 bg-amber-950/40 p-1.5 rounded border border-amber-800/40">
                            {sc.reason}
                          </p>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </section>
      )}

      {selectedTab === "competitor" && (
        <section className="mt-4 space-y-4">
          <SectionHeader
            title="Apples-to-Apples Competitor Protocol"
            description="Strict fairness constraints for external Speedify measurement comparisons."
          />

          <div className="grid gap-3 sm:grid-cols-2">
            <div className="rounded-xl border border-charcoal-600 bg-elevated p-4">
              <h4 className="text-xs font-bold uppercase tracking-wider text-sky-400 mb-2">Fairness Guardrails</h4>
              <ul className="text-xs space-y-2 text-ink-secondary">
                <li>• <strong>Identical Platform:</strong> Same host, OS platform, CPU architecture, and RAM.</li>
                <li>• <strong>Matching WANs:</strong> Identical active adapters and physical uplinks (within 15% baseline tolerance).</li>
                <li>• <strong>Matching Topology:</strong> Single-interface baselines cannot be compared to multi-WAN bonded tunnels.</li>
                <li>• <strong>Contemporaneous:</strong> Standalone baselines measured within the same active testing window.</li>
                <li>• <strong>Sample Rigor:</strong> Minimum of 5 valid, non-negative, finite measured repetitions per mode.</li>
                <li>• <strong>No Falsification:</strong> Synthetic and loopback data are strictly forbidden from competitor comparison.</li>
              </ul>
            </div>

            <div className="rounded-xl border border-charcoal-600 bg-elevated p-4 flex flex-col justify-center items-center text-center">
              <EmptyState
                icon={<IconShield size={24} />}
                title="No Competitor Import Yet"
                description="Import Speedify measurement records using 'netbench-cli competitor-import --input <file.csv>' to evaluate comparability."
              />
            </div>
          </div>
        </section>
      )}

      {selectedTab === "cli" && (
        <section className="mt-4 space-y-4">
          <SectionHeader
            title="Operator CLI Workflows"
            description="Command-line tools for automated CI qualification and reproducible benchmarking."
          />

          <div className="rounded-xl border border-charcoal-600 bg-elevated p-4 font-mono text-xs text-sky-300 space-y-3">
            <div>
              <p className="text-ink-secondary font-sans font-semibold mb-1"># Run Deterministic Synthetic Suite (Seed 42)</p>
              <p className="bg-charcoal-900 p-2 rounded border border-charcoal-700">cargo run -p netbench-cli -- run --profile synthetic-3wan --seed 42 --output artifacts/benchmark/runs</p>
            </div>
            <div>
              <p className="text-ink-secondary font-sans font-semibold mb-1"># Run Single-Interface Live Baseline (Opt-In Required)</p>
              <p className="bg-charcoal-900 p-2 rounded border border-charcoal-700">cargo run -p netbench-cli -- baseline --server-url https://speedtest.example.com --enable-live-network-tests --output artifacts/benchmark/runs</p>
            </div>
            <div>
              <p className="text-ink-secondary font-sans font-semibold mb-1"># Import Competitor Measurements</p>
              <p className="bg-charcoal-900 p-2 rounded border border-charcoal-700">cargo run -p netbench-cli -- competitor-import --input speedify-sample.csv --output artifacts/benchmark/competitor</p>
            </div>
            <div>
              <p className="text-ink-secondary font-sans font-semibold mb-1"># Compare MultiNet Run With Competitor Record</p>
              <p className="bg-charcoal-900 p-2 rounded border border-charcoal-700">cargo run -p netbench-cli -- compare --multi-net artifacts/benchmark/runs/&lt;run_id&gt; --competitor artifacts/benchmark/competitor/&lt;record_id&gt;.json --output artifacts/benchmark/comparisons</p>
            </div>
          </div>
        </section>
      )}
    </PageContainer>
  );
}
