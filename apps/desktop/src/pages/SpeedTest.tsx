import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { StatusIndicator, type StatusVariation } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue, SelectField } from "../components/ui/controls";
import { SpeedTestResultCard, Speedometer, SpeedTestStage } from "../components/ui/Speedometer";
import { InterfaceUsageTable } from "../components/ui/analytics";
import { EmptyState } from "../components/ui/feedback";
import { IconShield, IconSpeed } from "../components/ui/icons";
import { useSystemStore } from "../store/systemStore";
import type { SpeedTestStageId } from "../types/telemetry";

const stages: Array<{ id: SpeedTestStageId; title: string; description: string }> = [
  { id: "prepare", title: "Preparing connection", description: "Selects the target interface and validates the environment" },
  { id: "latency", title: "Measuring latency", description: "Baseline round-trip timing from verified sources" },
  { id: "jitter", title: "Measuring jitter", description: "Variation statistics over the sampling window" },
  { id: "download", title: "Download test", description: "Dataset transfer against a configured test server" },
  { id: "upload", title: "Upload test", description: "Dataset transfer against a configured test server" },
  { id: "report", title: "Final report", description: "Verified result set written to local history" }
];

export default function SpeedTestPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const bridgeOk = Boolean(foundation || database);
  const readinessVariation: StatusVariation = !bridgeOk ? (loading ? "warn" : "idle") : database ? "ok" : "warn";
  const headerLabel = !bridgeOk ? (loading ? "Connecting…" : "Backend unavailable") : "Speed engine integration pending";

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
            {error ? (
              <>Backend bridge unavailable — {error}. The speed test never fabricates results or runs client-side.</>
            ) : (
              <><strong className="font-semibold text-ink-primary">Speed test engine integration pending.</strong> This is a design preview: no test executes, no probe traffic is sent and no result value is produced until the backend engine exists.</>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div>
          <SectionHeader title="Speed test" description="Central gauge becomes live only with a running engine." />
          <div className="flex min-h-[340px] items-center justify-center rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-6">
            <div className="flex flex-col items-center gap-5">
              <Speedometer valueMbps={null} caption="Connection speed" pending pendingReason="Speed test engine integration pending — no numbers are animated or faked." />
              <div className="grid w-full max-w-2xl grid-cols-2 gap-3 sm:grid-cols-4">
                <SpeedTestResultCard label="Download" value="—" sub="Not measured" />
                <SpeedTestResultCard label="Upload" value="—" sub="Not measured" />
                <SpeedTestResultCard label="Latency" value="—" sub="Not measured" />
                <SpeedTestResultCard label="Jitter" value="—" sub="Not measured" />
              </div>
            </div>
          </div>
        </div>

        <div className="flex flex-col gap-4">
          <div>
            <SectionHeader title="Test configuration" description="Disabled until backend capability exists." />
            <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
              <div className="space-y-3">
                <SelectField label="Network interface" options={["Primary interface"]} disabled disabledReason="Interface discovery integration pending." />
                <SelectField label="Test server" options={["Not configured"]} disabled disabledReason="Test servers are provisioned by the backend; no fake addresses are shown." hint="No invented server endpoints are listed." />
                <SelectField label="Test duration" options={["Standard"]} disabled disabledReason="Duration profiles arrive with the test engine." />
                <SelectField label="Test mode" options={["Download & upload"]} disabled disabledReason="Selection requires a live engine." />
              </div>
            </div>
          </div>
          <div>
            <SectionHeader title="Test stages" description="Conceptual pipeline; nothing is running." />
            <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
              <ol className="space-y-3">
                {stages.map((s, i) => (
                  <li key={s.id}>
                    <SpeedTestStage index={i + 1} title={s.title} description={s.description} state="idle" />
                  </li>
                ))}
              </ol>
              <div className="mt-3 flex items-center gap-2 text-[10px] text-ink-muted">
                <span className="text-amber-300/90"><IconShield size={12} /></span>
                Stage list is illustrative only — no test is indicated as running.
              </div>
            </div>
          </div>
        </div>
      </section>

      <section>
        <SectionHeader title="Results summary" description="Every value reads “Not measured” until a verified run completes." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
          <SpeedTestResultCard label="Download speed" value="Not" sub="measured (Mbps)" />
          <SpeedTestResultCard label="Upload speed" value="Not" sub="measured (Mbps)" />
          <SpeedTestResultCard label="Ping" value="Not" sub="measured (ms)" />
          <SpeedTestResultCard label="Jitter" value="Not" sub="measured (ms)" />
          <SpeedTestResultCard label="Packet loss" value="Not" sub="measured (%)" />
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Test history" description="Verified SpeedTestResult records only." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
            <InterfaceUsageTable
              headers={["Date", "Interface", "Download", "Upload", "Ping", "Test status"]}
              rows={[]}
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
          <SectionHeader title="Speed test controls" description="Explicitly disabled in this phase." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated p-4 shadow-panel">
            <KeyValue label="Engine status" value={<Badge tone="warn">integration pending</Badge>} />
            <KeyValue label="Dataset provenance" value={<Badge tone="neutral">no local history yet</Badge>} />
            <div className="mt-4 flex flex-wrap items-center gap-2">
              <Button variant="primary" disabled disabledReason="Speed test engine integration pending — no test traffic is generated.">Start Test</Button>
              <Button disabled disabledReason="Nothing is running to cancel.">Cancel Test</Button>
              <Button disabled disabledReason="History populates after verified runs are recorded.">View History</Button>
              <span className="text-[11px] leading-4 text-ink-muted">Reason: backend integration pending. Execution is never simulated.</span>
            </div>
          </div>
        </div>
      </section>
    </PageContainer>
  );
}
