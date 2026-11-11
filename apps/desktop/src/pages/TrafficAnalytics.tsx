import { lazy, Suspense, useEffect } from "react";
import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { StatusCard, StatusIndicator, type StatusVariation } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice } from "../components/ui/controls";
import { InterfaceUsageTable, ProtocolDistribution } from "../components/ui/analytics";
import { EmptyState } from "../components/ui/feedback";
import { IconActivity, IconNetworkManager } from "../components/ui/icons";
import { useNativeRuntime, useSystemStore } from "../store/systemStore";
import { useDiagnosticsStore } from "../store/diagnosticsStore";
import {
  formatBitsPerSecond,
  formatBytes,
  formatClock,
  formatTimestamp
} from "../lib/format";

// Recharts travels in this lazy chunk so the shell and other screens never download it.
const LazyBandwidthChart = lazy(() =>
  import("../components/ui/BandwidthChart").then((m) => ({ default: m.BandwidthChart }))
);

const historyWindows = [
  { label: "Last 5 minutes", seconds: 300 },
  { label: "Last 15 minutes", seconds: 900 },
  { label: "Last 1 hour", seconds: 3600 },
  { label: "Last 24 hours", seconds: 86400 }
] as const;

const chartEmptyState = (
  <EmptyState
    icon={<IconActivity size={18} />}
    title="No traffic data to chart"
    description="Throughput history renders from the GetIfTable2 sampler feed only. No sample series, smoothing or synthetic spikes are displayed while the monitor is offline."
  />
);

export default function TrafficAnalyticsPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const { isTauri: nativeRuntime, previewMessage } = useNativeRuntime();
  const {
    snapshot,
    snapshotState,
    snapshotError,
    history,
    historyState,
    historyError,
    historySeconds,
    monitor,
    refreshTraffic,
    refreshTrafficHistory,
    setHistoryWindow
  } = useDiagnosticsStore();

  useEffect(() => {
    void refreshTraffic();
    void refreshTrafficHistory();
  }, [refreshTraffic, refreshTrafficHistory]);

  const bridgeOk = Boolean(foundation || database);
  const readinessVariation: StatusVariation = !bridgeOk
    ? loading
      ? "warn"
      : "idle"
    : database
      ? "ok"
      : "warn";
  const readinessLabel = readinessVariation === "ok"
    ? "Monitor reporting"
    : bridgeOk
      ? "Backend reachable"
      : loading
        ? "Connecting…"
        : "Backend unavailable";

  const interfaceRows = (snapshot?.interfaces ?? []).map((entry) => ({
    cells: [
      <span key="name" className="font-semibold text-ink-primary">{entry.name}</span>,
      <span key="kind" className="text-ink-secondary">{entry.kind}</span>,
      <span key="down" className="font-mono">{formatBitsPerSecond(entry.rxBps)}</span>,
      <span key="up" className="font-mono">{formatBitsPerSecond(entry.txBps)}</span>,
      <span key="total" className="font-mono">
        {formatBytes(entry.sessionRxBytes + entry.sessionTxBytes)}
      </span>,
      <StatusIndicator
        key="status"
        variation={entry.isUp ? "ok" : "idle"}
        label={entry.isUp ? "Up" : "Down"}
      />
    ]
  }));

  const peakBps = snapshot?.peakCombinedBps ?? null;

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
            {!nativeRuntime ? (
              <>{previewMessage}</>
            ) : error ? (
              <>Backend bridge unavailable — {error}.</>
            ) : snapshotError ? (
              <>
                Native command <code className="font-mono">{snapshotError.command}</code> failed — {snapshotError.reason}.{" "}
                No traffic value is fabricated.
              </>
            ) : (
              <>
                Traffic figures come from the live Windows <code className="font-mono">GetIfTable2</code>{" "}
                sampler, which writes one row per interface every second into the local SQLite history.
                Rates are computed by the Rust backend from the interface byte counters — no series is
                generated, smoothed or estimated in the browser.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Traffic overview" description="Totals and rates read from the live sampler." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          <StatusCard
            title="Total Download"
            value={formatBytes(snapshot?.sessionRxBytes ?? null)}
            sub="since session start"
            variation={snapshot ? "ok" : "idle"}
            hint="Session receive bytes accumulated by the sampler."
            icon={<IconActivity size={14} />}
          />
          <StatusCard
            title="Total Upload"
            value={formatBytes(snapshot?.sessionTxBytes ?? null)}
            sub="since session start"
            variation={snapshot ? "ok" : "idle"}
            hint="Session transmit bytes accumulated by the sampler."
            icon={<IconActivity size={14} />}
          />
          <StatusCard
            title="Current Download Rate"
            value={formatBitsPerSecond(snapshot?.physicalRxBps ?? null)}
            variation={snapshot?.physicalRxBps != null ? "ok" : "idle"}
            hint="Sum of the per-interface receive rates of physical adapters."
          />
          <StatusCard
            title="Current Upload Rate"
            value={formatBitsPerSecond(snapshot?.physicalTxBps ?? null)}
            variation={snapshot?.physicalTxBps != null ? "ok" : "idle"}
            hint="Sum of the per-interface transmit rates of physical adapters."
          />
          <StatusCard
            title="Peak Bandwidth"
            value={formatBitsPerSecond(peakBps)}
            variation={peakBps ? "ok" : "idle"}
            hint="Maximum combined rate recorded in the local history table."
          />
          <StatusCard
            title="Active Interfaces"
            value={snapshot ? String(snapshot.interfaces.length) : "Unavailable"}
            variation={snapshot ? "ok" : "idle"}
            hint="Interfaces the sampler is currently reporting."
          />
        </div>
      </section>

      <section>
        <SectionHeader
          title="Bandwidth history"
          description="Download & upload throughput over time."
          action={
            <div className="flex items-center gap-2">
              <Badge tone="accent">GetIfTable2 feed</Badge>
              <Button
                onClick={() => void refreshTrafficHistory()}
                disabled={historyState === "loading" || !nativeRuntime}
                disabledReason={
                  !nativeRuntime
                    ? "The native bridge is unavailable in a browser preview."
                    : "Reloading the recorded sample history."
                }
              >
                Reload history
              </Button>
            </div>
          }
        />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
          <div className="mb-3 flex flex-wrap items-center gap-2">
            <span className="text-[10px] uppercase tracking-wider text-ink-muted">Time range</span>
            {historyWindows.map((w) => (
              <Button
                key={w.label}
                variant="ghost"
                onClick={() => void setHistoryWindow(w.seconds)}
                disabled={!nativeRuntime}
                disabledReason="The native bridge is unavailable in a browser preview."
                className={historySeconds === w.seconds ? "text-accent-cyan" : undefined}
              >
                {w.label}
              </Button>
            ))}
          </div>
          {historyState === "error" && historyError ? (
            <div className="mb-3 rounded-xl border border-red-900/60 bg-red-900/20 px-3 py-2 text-[11px] text-red-200">
              Native command <code className="font-mono">{historyError.command}</code> failed — {historyError.reason}.
            </div>
          ) : null}
          {history.length === 0 ? (
            chartEmptyState
          ) : (
            <Suspense fallback={chartEmptyState}>
              <LazyBandwidthChart data={history} emptyState={chartEmptyState} height={320} />
            </Suspense>
          )}
          {history.length > 0 && (
            <p className="mt-3 text-[10px] text-ink-muted">
              {history.length} verified samples between {formatTimestamp(history[0]?.sampledAt)} and{" "}
              {formatTimestamp(history[history.length - 1]?.sampledAt)}.
            </p>
          )}
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Interface usage" description="Per-interface totals from the live sampler." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
            <InterfaceUsageTable
              headers={["Interface", "Connection type", "Download", "Upload", "Session usage", "Status"]}
              rows={interfaceRows}
              emptyState={
                <EmptyState
                  icon={<IconNetworkManager size={18} />}
                  title="No interface counters yet"
                  description="The sampler has not produced a per-interface sample yet. Wi-Fi names, device identifiers and IP addresses are never invented."
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
        <SectionHeader title="Sampler status" description="Reported by the Rust diagnostics backend." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <StatusCard
            title="Monitor state"
            value={monitor ? (monitor.running ? "Running" : "Stopped") : snapshot ? (snapshot.monitorActive ? "Running" : "Stopped") : "Unavailable"}
            variation={monitor?.running ? "ok" : "idle"}
            hint="Whether the GetIfTable2 sampler thread is active."
          />
          <StatusCard
            title="Last sample age"
            value={monitor?.lastSampleAgeMs != null ? `${monitor.lastSampleAgeMs} ms` : "Unavailable"}
            variation="idle"
            hint="Milliseconds since the most recent sample was written."
          />
          <StatusCard
            title="Sample source"
            value={monitor?.source ?? snapshot?.source ?? "Unavailable"}
            variation="idle"
            hint="The Windows API that produced the counters."
          />
          <StatusCard
            title="Total RX / TX"
            value={
              snapshot ? `${formatBytes(snapshot.totalRxBytes)} / ${formatBytes(snapshot.totalTxBytes)}` : "Unavailable"
            }
            variation="idle"
            hint="Absolute adapter byte counters."
          />
        </div>
        {snapshot && (
          <p className="mt-3 text-[10px] text-ink-muted">
            Snapshot taken at {formatTimestamp(snapshot.sampledAt)} ({formatClock(snapshot.sampledAt)}).
          </p>
        )}
      </section>
    </PageContainer>
  );
}
