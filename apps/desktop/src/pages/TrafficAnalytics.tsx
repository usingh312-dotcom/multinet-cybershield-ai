import { lazy, Suspense } from "react";
import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { StatusCard, StatusIndicator, type StatusVariation } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, SelectField } from "../components/ui/controls";
import { InterfaceUsageTable, NetworkQualityCards, ProtocolDistribution } from "../components/ui/analytics";
import { EmptyState } from "../components/ui/feedback";
import { IconActivity, IconAnalytics, IconEndpoint, IconNetworkManager } from "../components/ui/icons";
import { useSystemStore } from "../store/systemStore";
import type { TrafficMetrics } from "../types/telemetry";

// Recharts travels in this lazy chunk so the shell and other screens never download it.
const LazyBandwidthChart = lazy(() =>
  import("../components/ui/BandwidthChart").then((m) => ({ default: m.BandwidthChart }))
);

const timeRanges = ["Last 5 minutes", "Last 15 minutes", "Last 1 hour", "Last 24 hours", "Last 7 days"] as const;

const insightBlocks = [
  { title: "Daily bandwidth usage", description: "Aggregates verified collector samples over a rolling day." },
  { title: "Weekly usage trends", description: "Opens after seven consecutive days of collector history exist." },
  { title: "Interface comparison", description: "Contrasts per-adapter totals from the same verified feed." },
  { title: "Peak usage periods", description: "Flags verified intervals exceeding your configured thresholds." }
] as const;

const chartEmptyState = (
  <EmptyState
    icon={<IconActivity size={18} />}
    title="No traffic data to chart"
    description="Throughput history renders from the TrafficMetrics collector feed only. No sample series, smoothing or synthetic spikes are displayed while collectors are offline."
  />
);

export default function TrafficAnalyticsPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const bridgeOk = Boolean(foundation || database);
  const readinessVariation: StatusVariation = !bridgeOk ? (loading ? "warn" : "idle") : database ? "ok" : "warn";
  const readinessLabel = readinessVariation === "ok" ? "Monitoring not initialized" : bridgeOk ? "Backend reachable" : loading ? "Connecting…" : "Backend unavailable";

  const data: TrafficMetrics[] = []; // Real collector feed only — intentionally empty until integration.

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Traffic Analytics"
          subtitle="Understand network activity, bandwidth utilization and connection performance."
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error ? (
              <>Backend bridge unavailable — {error}.</>
            ) : (
              <>Traffic monitoring is <strong className="font-semibold text-ink-primary">not initialized</strong>. Charts, totals and quality scores populate exclusively from the future TrafficMetrics collector — random or placeholder series are never rendered as measurements.</>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Traffic overview" description="Totals and rates stay empty until telemetry exists." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          <StatusCard title="Total Download" value="No traffic data" variation="idle" hint="Byte counters bind to real interface statistics." icon={<IconAnalytics size={14} />} />
          <StatusCard title="Total Upload" value="No traffic data" variation="idle" hint="Byte counters bind to real interface statistics." />
          <StatusCard title="Current Download Rate" value="Not measured" variation="idle" hint="Rate calculation runs on the collector clock." />
          <StatusCard title="Current Upload Rate" value="Not measured" variation="idle" hint="Rate calculation runs on the collector clock." />
          <StatusCard title="Peak Bandwidth" value="Not measured" variation="idle" hint="Peaks are computed from verified history only." />
          <StatusCard title="Active Interfaces" value="Discovery pending" variation="idle" hint="Interface enumeration arrives with platform integration." />
        </div>
      </section>

      <section>
        <SectionHeader
          title="Bandwidth history"
          description="Download & upload throughput over time."
          action={
            <Badge tone="accent"  >TrafficMetrics feed</Badge>
          }
        />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
          <div className="mb-3 flex flex-wrap items-center gap-2">
            <span className="text-[10px] uppercase tracking-wider text-ink-muted">Time range</span>
            {timeRanges.map((r) => (
              <Button key={r} variant="ghost" disabled disabledReason={`Historical telemetry is unavailable — the ${r} window fills when the collector runs.`}>
                {r}
              </Button>
            ))}
          </div>
          {data.length === 0 ? (
            chartEmptyState
          ) : (
            <Suspense fallback={chartEmptyState}>
              <LazyBandwidthChart data={data} emptyState={chartEmptyState} height={320} />
            </Suspense>
          )}
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Interface usage" description="Per-interface totals come from verified collectors." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
            <InterfaceUsageTable
              headers={["Interface", "Connection type", "Download", "Upload", "Total usage", "Status"]}
              rows={[]}
              emptyState={
                <EmptyState
                  icon={<IconNetworkManager size={18} />}
                  title="Interface discovery pending"
                  description="No adapters have been enumerated by this build. Wi-Fi names, device identifiers and IP addresses are never invented."
                />
              }
            />
          </div>
        </div>
        <div>
          <SectionHeader title="Protocol distribution" description="Classification is a collector feature." />
          <ProtocolDistribution />
        </div>
      </section>

      <section>
        <SectionHeader title="Connection quality" description="Derived strictly from verified measurements." />
        <NetworkQualityCards quality={null} />
      </section>

      <section>
        <SectionHeader title="Usage insights" description="Analytics remain unavailable without continuous verified telemetry." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {insightBlocks.map((b) => (
            <div key={b.title} className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3.5">
              <div className="flex items-center gap-2 text-xs font-semibold text-ink-primary">
                <span className="rounded-lg border border-charcoal-600 bg-charcoal-900 p-1.5 text-accent-cyan"><IconEndpoint size={13} /></span>
                {b.title}
              </div>
              <p className="mt-1.5 text-[11px] leading-4 text-ink-secondary">{b.description}</p>
              <div className="mt-2"><Badge tone="warn">integration pending</Badge></div>
            </div>
          ))}
        </div>
      </section>
    </PageContainer>
  );
}
