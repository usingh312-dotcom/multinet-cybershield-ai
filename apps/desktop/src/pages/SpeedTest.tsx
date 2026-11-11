import { useEffect, useMemo, useRef, useState } from "react";
import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { StatusIndicator, type StatusVariation } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue } from "../components/ui/controls";
import { SpeedTestResultCard, Speedometer, SpeedTestStage } from "../components/ui/Speedometer";
import { InterfaceUsageTable } from "../components/ui/analytics";
import { EmptyState } from "../components/ui/feedback";
import { IconShield, IconSpeed } from "../components/ui/icons";
import { useNativeRuntime, useSystemStore } from "../store/systemStore";
import { useDiagnosticsStore } from "../store/diagnosticsStore";
import type { BackendInterface } from "../api";
import { formatMbps, formatMs, formatPercent, formatTimestamp } from "../lib/format";

const DEFAULT_SERVER_URL = "https://127.0.0.1:9443";
const DOWNLOAD_BYTES = 8 * 1024 * 1024;
const UPLOAD_BYTES = 4 * 1024 * 1024;
const PING_SAMPLES = 8;
const TIMEOUT_SECS = 60;

const stageOrder = [
  { id: "prepare", title: "Preparing connection", description: "Selects the target interface and validates the environment" },
  { id: "latency", title: "Measuring latency", description: "Baseline round-trip timing from verified sources" },
  { id: "jitter", title: "Measuring jitter", description: "Variation statistics over the sampling window" },
  { id: "download", title: "Download test", description: "Dataset transfer against a configured test server" },
  { id: "upload", title: "Upload test", description: "Dataset transfer against a configured test server" },
  { id: "report", title: "Final report", description: "Verified result set written to local history" }
] as const;

/** Maps the Rust `SpeedTestStage` value to the pipeline stage identifiers. */
function stageKey(stage: string | undefined): string {
  switch (stage) {
    case "preparing":
      return "prepare";
    case "latency":
      return "latency";
    case "download":
      return "download";
    case "upload":
      return "upload";
    case "finalizing":
    case "complete":
      return "report";
    case "cancelled":
    case "failed":
      return "report";
    default:
      return "prepare";
  }
}

export default function SpeedTestPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const { isTauri: nativeRuntime, previewMessage } = useNativeRuntime();
  const {
    interfaces,
    refreshInterfaces,
    speedRunId,
    speedStatus,
    speedState,
    speedError,
    speedHistory,
    speedHistoryState,
    refreshSpeedHistory,
    runSpeedTest,
    pollSpeedTest,
    cancelCurrentSpeedTest
  } = useDiagnosticsStore();

  const [serverUrl, setServerUrl] = useState(DEFAULT_SERVER_URL);
  const [selectedInterfaceId, setSelectedInterfaceId] = useState<string>("");
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    void refreshInterfaces();
    void refreshSpeedHistory();
    return () => {
      if (timerRef.current !== null) window.clearInterval(timerRef.current);
    };
  }, [refreshInterfaces, refreshSpeedHistory]);

  // Poll the running test until it reaches a terminal state.
  useEffect(() => {
    if (!speedRunId || !speedStatus || speedStatus.state !== "running") return;
    if (timerRef.current !== null) window.clearInterval(timerRef.current);
    timerRef.current = window.setInterval(() => {
      void pollSpeedTest(speedRunId);
    }, 1000);
    return () => {
      if (timerRef.current !== null) window.clearInterval(timerRef.current);
      timerRef.current = null;
    };
  }, [speedRunId, speedStatus, pollSpeedTest]);

  const selectedInterface = useMemo<BackendInterface | null>(
    () => interfaces.find((iface) => iface.id === selectedInterfaceId) ?? null,
    [interfaces, selectedInterfaceId]
  );

  // The socket is bound to the local address of the chosen adapter so the
  // operating system routes the measurement through it.
  const selectedLocalIp = selectedInterface?.ipv4Addresses[0] ?? null;

  const report = speedStatus?.report ?? null;
  const running = speedStatus?.state === "running";
  const progress = speedStatus?.progress ?? null;

  const stageStates = useMemo(() => {
    const current = stageKey(progress?.stage);
    const currentIndex = stageOrder.findIndex((s) => s.id === current);
    const terminal = speedStatus?.state === "completed" || speedStatus?.state === "failed" || speedStatus?.state === "cancelled";
    return stageOrder.map((stage, index) => {
      if (terminal) return "done" as const;
      if (index < currentIndex) return "done" as const;
      if (index === currentIndex) return "active" as const;
      return "idle" as const;
    });
  }, [progress, speedStatus]);

  const bridgeOk = Boolean(foundation || database);
  const readinessVariation: StatusVariation = !bridgeOk
    ? loading
      ? "warn"
      : "idle"
    : database
      ? "ok"
      : "warn";
  const headerLabel = !bridgeOk
    ? loading
      ? "Connecting…"
      : "Backend unavailable"
    : speedStatus
      ? `Speed engine ${speedStatus.state}`
      : "Speed engine ready";

  const historyRows = speedHistory.map((entry) => ({
    cells: [
      <span key="date" className="text-ink-secondary">{formatTimestamp(entry.completedAt)}</span>,
      <span key="iface" className="font-mono text-ink-secondary">
        {entry.interfaceLocalIp ?? entry.interfaceId ?? "—"}
      </span>,
      <span key="down" className="font-mono">{formatMbps(entry.downloadMbps)}</span>,
      <span key="up" className="font-mono">{formatMbps(entry.uploadMbps)}</span>,
      <span key="ping" className="font-mono">{formatMs(entry.pingMs)}</span>,
      <Badge key="status" tone="ok">completed</Badge>
    ]
  }));

  const start = async () => {
    await runSpeedTest({
      serverUrl: serverUrl.trim(),
      interfaceId: selectedInterfaceId || null,
      interfaceLocalIp: selectedLocalIp,
      downloadBytes: DOWNLOAD_BYTES,
      uploadBytes: UPLOAD_BYTES,
      pingSamples: PING_SAMPLES,
      timeoutSecs: TIMEOUT_SECS,
      allowSelfSignedLoopback: true
    });
  };

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Internet Speed Test"
          subtitle="Measure connection performance and network quality."
          actions={<StatusIndicator variation={readinessVariation} label={headerLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {!nativeRuntime ? (
              <>{previewMessage}</>
            ) : error ? (
              <>Backend bridge unavailable — {error}. The speed test never fabricates results or runs client-side.</>
            ) : speedError ? (
              <>
                Native command <code className="font-mono">{speedError.command}</code> failed — {speedError.reason}.{" "}
                No result value is shown in place of a failed measurement.
              </>
            ) : (
              <>
                Measurements are produced by the Rust <code className="font-mono">speed-test</code> engine:{" "}
                <code className="font-mono">start_speed_test</code> binds the socket to the selected adapter,
                transfers real bytes against the configured HTTPS server and returns the verified report. A
                failed or unreachable server is reported as a failure — no result is invented.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div>
          <SectionHeader title="Speed test" description="Live gauge driven by the backend report." />
          <div className="flex min-h-[340px] items-center justify-center rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-6">
            <div className="flex flex-col items-center gap-5">
              <Speedometer
                valueMbps={report ? report.downloadMbps : null}
                caption="Connection speed"
                pending={!report}
                pendingReason={
                  running
                    ? "A measurement is running — the gauge fills when the report completes."
                    : "No completed measurement yet. No numbers are animated or faked."
                }
              />
              <div className="grid w-full max-w-2xl grid-cols-2 gap-3 sm:grid-cols-4">
                <SpeedTestResultCard label="Download" value={report ? report.downloadMbps.toFixed(2) : "—"} sub="Mbps" />
                <SpeedTestResultCard label="Upload" value={report ? report.uploadMbps.toFixed(2) : "—"} sub="Mbps" />
                <SpeedTestResultCard label="Latency" value={report ? report.pingMs.toFixed(2) : "—"} sub="ms" />
                <SpeedTestResultCard label="Jitter" value={report ? report.jitterMs.toFixed(2) : "—"} sub="ms" />
              </div>
            </div>
          </div>
        </div>

        <div className="flex flex-col gap-4">
          <div>
            <SectionHeader title="Test configuration" description="Bound to the live interface inventory." />
            <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
              <div className="space-y-3">
                <div>
                  <label className="mb-1 block text-xs font-medium text-ink-secondary" htmlFor="speed-server-url">
                    Test server
                  </label>
                  <input
                    id="speed-server-url"
                    type="text"
                    value={serverUrl}
                    onChange={(e) => setServerUrl(e.target.value)}
                    disabled={!nativeRuntime || running}
                    className="h-8 w-full rounded-lg border border-charcoal-600 bg-field px-2.5 text-xs text-ink-primary disabled:cursor-not-allowed disabled:text-ink-muted"
                  />
                  <p className="mt-1 text-[10px] leading-4 text-ink-muted">
                    HTTPS only. A self-signed certificate is accepted for loopback servers only.
                  </p>
                </div>
                <div>
                  <label className="mb-1 block text-xs font-medium text-ink-secondary" htmlFor="speed-interface">
                    Network interface
                  </label>
                  <select
                    id="speed-interface"
                    value={selectedInterfaceId}
                    onChange={(e) => setSelectedInterfaceId(e.target.value)}
                    disabled={!nativeRuntime || running}
                    className="h-8 w-full appearance-none rounded-lg border border-charcoal-600 bg-field px-2.5 text-xs text-ink-primary disabled:cursor-not-allowed disabled:text-ink-muted"
                  >
                    <option value="">Automatic (system default route)</option>
                    {interfaces.map((iface) => (
                      <option key={iface.id} value={iface.id}>
                        {iface.name} — {iface.description}
                      </option>
                    ))}
                  </select>
                  <p className="mt-1 text-[10px] leading-4 text-ink-muted">
                    {selectedInterface
                      ? `The socket binds to ${selectedLocalIp ?? "the adapter's primary address"}.`
                      : "No adapter selected — the operating system picks the default route."}
                  </p>
                </div>
                <KeyValue label="Download payload" value={`${(DOWNLOAD_BYTES / 1024 / 1024).toFixed(0)} MiB`} />
                <KeyValue label="Upload payload" value={`${(UPLOAD_BYTES / 1024 / 1024).toFixed(0)} MiB`} />
                <KeyValue label="Ping samples" value={String(PING_SAMPLES)} />
                <KeyValue label="Timeout" value={`${TIMEOUT_SECS}s`} />
              </div>
            </div>
          </div>
          <div>
            <SectionHeader title="Test stages" description="Live stage reported by the backend." />
            <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
              <ol className="space-y-3">
                {stageOrder.map((s, i) => (
                  <li key={s.id}>
                    <SpeedTestStage index={i + 1} title={s.title} description={s.description} state={stageStates[i]} />
                  </li>
                ))}
              </ol>
              {progress?.detail && (
                <div className="mt-3 text-[11px] text-ink-secondary">{progress.detail}</div>
              )}
              <div className="mt-3 flex items-center gap-2 text-[10px] text-ink-muted">
                <span className="text-amber-300/90"><IconShield size={12} /></span>
                {running
                  ? "A measurement is in progress — the stage indicator reflects the live backend progress."
                  : "Stage list reflects the last backend stage only. No test is indicated as running."}
              </div>
            </div>
          </div>
        </div>
      </section>

      <section>
        <SectionHeader title="Results summary" description="Every value comes from the verified backend report." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
          <SpeedTestResultCard label="Download speed" value={report ? report.downloadMbps.toFixed(2) : "Not"} sub={report ? "Mbps" : "measured (Mbps)"} />
          <SpeedTestResultCard label="Upload speed" value={report ? report.uploadMbps.toFixed(2) : "Not"} sub={report ? "Mbps" : "measured (Mbps)"} />
          <SpeedTestResultCard label="Ping" value={report ? report.pingMs.toFixed(2) : "Not"} sub={report ? "ms" : "measured (ms)"} />
          <SpeedTestResultCard label="Jitter" value={report ? report.jitterMs.toFixed(2) : "Not"} sub={report ? "ms" : "measured (ms)"} />
          <SpeedTestResultCard label="Packet loss" value={report ? report.lossPct.toFixed(2) : "Not"} sub={report ? "%" : "measured (%)"} />
        </div>
        {report && (
          <div className="mt-3 rounded-xl2 border border-charcoal-600 bg-elevated p-3 shadow-panel">
            <div className="grid gap-2 text-[11px] text-ink-secondary sm:grid-cols-2 lg:grid-cols-4">
              <span>Server: {report.serverName}</span>
              <span>Observed client: {report.serverObservedClientIp}</span>
              <span>Bound address: {report.interfaceLocalIp ?? "system default"}</span>
              <span>Duration: {report.durationMs} ms</span>
              <span>Started: {formatTimestamp(report.startedAt)}</span>
              <span>Completed: {formatTimestamp(report.completedAt)}</span>
              <span>Bytes: {(report.downloadBytes / 1024 / 1024).toFixed(2)} MiB down / {(report.uploadBytes / 1024 / 1024).toFixed(2)} MiB up</span>
              <span>Probes: {report.pingReceived}/{report.pingSamples} received</span>
            </div>
          </div>
        )}
        {speedStatus?.state === "failed" && speedStatus.error && (
          <div className="mt-3 rounded-xl border border-red-900/60 bg-red-900/20 px-3 py-2 text-[11px] text-red-200">
            The backend reported a failed measurement: {speedStatus.error}
          </div>
        )}
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Test history" description="Verified records written by the backend." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
            <InterfaceUsageTable
              headers={["Date", "Interface", "Download", "Upload", "Ping", "Test status"]}
              rows={historyRows}
              emptyState={
                <EmptyState
                  icon={<IconSpeed size={18} />}
                  title="No speed tests recorded"
                  description="History is written locally per successful verified run. Completed runs never appear before the engine exists."
                />
              }
            />
          </div>
        </div>
        <div>
          <SectionHeader title="Speed test controls" description="Drives the real Rust measurement engine." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated p-4 shadow-panel">
            <KeyValue
              label="Engine status"
              value={
                <Badge tone={speedStatus ? (speedStatus.state === "completed" ? "ok" : speedStatus.state === "failed" ? "danger" : "warn") : "neutral"}>
                  {speedStatus ? speedStatus.state : "idle"}
                </Badge>
              }
            />
            <KeyValue
              label="Dataset provenance"
              value={
                <Badge tone={speedHistory.length > 0 ? "ok" : "neutral"}>
                  {speedHistory.length > 0 ? `${speedHistory.length} recorded runs` : "no local history yet"}
                </Badge>
              }
            />
            <div className="mt-4 flex flex-wrap items-center gap-2">
              <Button
                variant="primary"
                onClick={() => void start()}
                disabled={!nativeRuntime || running || speedState === "loading"}
                disabledReason={
                  !nativeRuntime
                    ? "The native bridge is unavailable in a browser preview."
                    : running
                      ? "A measurement is already running."
                      : "Starting the measurement engine."
                }
              >
                Start Test
              </Button>
              <Button
                onClick={() => void cancelCurrentSpeedTest()}
                disabled={!nativeRuntime || !running}
                disabledReason={running ? "Cancelling the running measurement." : "Nothing is running to cancel."}
              >
                Cancel Test
              </Button>
              <Button
                onClick={() => void refreshSpeedHistory()}
                disabled={!nativeRuntime || speedHistoryState === "loading"}
                disabledReason="Reloading the recorded run history."
              >
                View History
              </Button>
              <span className="text-[11px] leading-4 text-ink-muted">
                {running
                  ? "The backend is transferring real bytes against the configured server."
                  : "Execution is never simulated. A start requires a reachable HTTPS test server."}
              </span>
            </div>
          </div>
        </div>
      </section>
    </PageContainer>
  );
}
