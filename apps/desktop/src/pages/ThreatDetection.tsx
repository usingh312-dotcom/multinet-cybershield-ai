import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue, SelectField } from "../components/ui/controls";
import { DataTable, type DataTableColumn } from "../components/ui/DataTable";
import { useSystemStore } from "../store/systemStore";
import type { SecurityEvent } from "../types/securityEvents";
import {
  IconAlert, IconBell, IconCopilot, IconDns, IconEndpoint, IconFirewall, IconNetworkManager,
  IconReports, IconShield, IconThreatDetection, IconThreatIntel
} from "../components/ui/icons";

const severityCards = [
  { severity: "Critical", hint: "Severity assignment happens in the engine. Until a feed exists, critical is shown as data unavailable — never as zero." },
  { severity: "High", hint: "A missing event feed must never be read as the absence of high-severity threats." },
  { severity: "Medium", hint: "Counts appear only from the verified detector feed." },
  { severity: "Low", hint: "Counts appear only from the verified detector feed." },
  { severity: "Informational", hint: "Informational records arrive with the engine's audit trail." }
] as const;

const classificationCategories = [
  { name: "Network anomaly", description: "Traffic-shape deviations over verified metrics", icon: <IconNetworkManager size={14} /> },
  { name: "Suspicious DNS activity", description: "Resolver-level patterns from the DNS module", icon: <IconDns size={14} /> },
  { name: "Endpoint activity", description: "Process and device behavior signals", icon: <IconEndpoint size={14} /> },
  { name: "Firewall events", description: "Policy-relevant firewall records", icon: <IconFirewall size={14} /> },
  { name: "Threat intelligence matches", description: "Indicator hits from curated feeds", icon: <IconThreatIntel size={14} /> }
] as const;

const responseActions = [
  { label: "Investigate", reason: "Opening an investigation requires a real event; none exists while the engine is detached." },
  { label: "Mark as reviewed", reason: "Review workflow applies to verified events only." },
  { label: "Resolve", reason: "Resolution requires validated evidence from the detection engine." },
  { label: "Escalate", reason: "Escalation targets exist once the response module is integrated." },
  { label: "Create incident", reason: "Incident creation needs a confirmed event with evidence references." }
] as const;

const eventColumns: DataTableColumn<SecurityEvent>[] = [
  { id: "event", header: "Event", render: (row) => <span className="font-medium text-ink-primary">{row.title}</span> },
  { id: "severity", header: "Severity", render: (row) => <Badge tone="neutral">{row.severity}</Badge> },
  { id: "category", header: "Category", render: (row) => row.category },
  { id: "source", header: "Source", render: (row) => row.source },
  { id: "detectedAt", header: "Detected at", render: (row) => row.timestamp },
  { id: "confidence", header: "Confidence", align: "right", render: (row) => `${Math.round(row.confidence * 100)}%` },
  { id: "status", header: "Status", render: (row) => row.status },
  {
    id: "action",
    header: "Action",
    align: "right",
    render: () => <Button disabled disabledReason="Security action execution requires validated evidence, policy checks and human approval.">Open</Button>
  }
];

export default function ThreatDetectionPage() {
  const { foundation, database, error, loading } = useSystemStore();

  const feedVariation = loading ? "warn" : "idle" as const;
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "Detection engine not attached";

  // Real SecurityEvent records only — the engine never runs in this build, so the
  // table is intentionally empty and stays honest. No fabricated events are seeded.
  const events: SecurityEvent[] = [];

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Threat Detection"
          subtitle="Security event monitoring and threat investigation workspace."
          actions={<StatusIndicator variation={feedVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error ? (
              <>Backend bridge unavailable — {error}. Detection status below reflects the static frontend only.</>
            ) : (
              <>
                The detection engine is <strong className="font-semibold text-ink-primary">not attached</strong>. This workspace is the design surface for the future
                SecurityEvent feed — no detector runs, no event is produced and no threat count is claimed until that integration exists.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Threat monitoring status" description="Every state below is a truthful unavailability statement." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <StatusCard title="Detection engine readiness" value="Not initialized" variation="idle" icon={<IconShield size={14} />} hint="The engine loop has never been started in this build." />
          <StatusCard title="Last monitoring initialization" value="Never" variation="idle" icon={<IconAlert size={14} />} hint="No initialization has been performed, so no initialization time can be reported." />
          <StatusCard title="Security event feed" value="Data unavailable" variation="idle" icon={<IconBell size={14} />} hint="A missing feed is displayed as unavailable — it is never treated as '0 threats'." />
          <StatusCard title="Event collection availability" value="Integration pending" variation="idle" icon={<IconReports size={14} />} hint="Event persistence attaches to the database once the engine writes records." />
        </div>
      </section>

      <section>
        <SectionHeader
          title="Severity overview"
          description="Event counts by engine-assigned severity. All values are data unavailable while no feed exists."
          action={<Badge tone="neutral">no event feed</Badge>}
        />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
          {severityCards.map((s) => (
            <div key={s.severity} className="rounded-xl3 border border-charcoal-600 bg-elevated shadow-panel p-3 flex flex-col gap-2">
              <div className="flex items-center justify-between gap-2">
                <span className="text-[11px] uppercase tracking-wider text-ink-muted">{s.severity}</span>
                <IconAlert size={13} className="text-ink-muted" />
              </div>
              <div className="text-lg font-semibold text-ink-primary">Data unavailable</div>
              <p className="text-[11px] leading-4 text-ink-secondary">{s.hint}</p>
            </div>
          ))}
        </div>
      </section>

      <section>
        <SectionHeader
          title="Threat events"
          description="Verified SecurityEvent records only. The engine is not producing events in this build."
        />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <DataTable
            columns={eventColumns}
            rows={events}
            rowKey={(row) => row.id}
            minWidth={980}
            caption="Threat events — verified records only"
            emptyState={
              <EmptyState
                icon={<IconThreatDetection size={18} />}
                title="No security events available. Detection engine integration is pending."
                description="Addresses, domains, process identifiers and timestamps are never fabricated while the engine is detached. The table populates exclusively from the future SecurityEvent feed."
              />
            }
          />
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Detection filters" description="Filtering applies once real events exist." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <SelectField label="Severity" options={["Critical", "High", "Medium", "Low", "Informational"]} disabled disabledReason="Severity filtering requires an available event feed." />
              <SelectField label="Event category" options={["Network anomaly", "Endpoint activity", "Suspicious DNS activity", "Firewall events", "Threat intelligence matches"]} disabled disabledReason="Category filtering requires an available event feed." />
              <SelectField label="Source" options={["Detector source"]} disabled disabledReason="No detector is registered, so no source can be selected." />
              <SelectField label="Investigation status" options={["New", "Investigating", "Resolved", "False positive"]} disabled disabledReason="Status filtering requires persisted investigation state." />
              <SelectField label="Date range" options={["Last 24 hours", "Last 7 days", "Last 30 days"]} disabled disabledReason="Time filtering requires engine-produced timestamps." />
            </div>
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">
              Controls are disabled because filtering depends entirely on the unavailable event feed. They enable automatically when verified events arrive.
            </p>
          </div>
        </div>
        <div>
          <SectionHeader
            title="Threat classification"
            description="Planned category labels for future detections."
            action={<Badge tone="neutral">no detections occurred</Badge>}
          />
          <div className="grid gap-2.5">
            {classificationCategories.map((c) => (
              <div key={c.name} className="flex items-center justify-between gap-3 rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3">
                <div className="flex items-center gap-2.5">
                  <span className="rounded-lg border border-charcoal-600 bg-charcoal-900 p-1.5 text-accent-cyan">{c.icon}</span>
                  <div>
                    <div className="text-xs font-semibold text-ink-primary">{c.name}</div>
                    <div className="text-[10px] text-ink-muted">{c.description}</div>
                  </div>
                </div>
                <Badge tone="neutral">no data</Badge>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Threat investigation" description="Detail template for a real, verified event." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="mb-3 flex items-center justify-between">
              <span className="text-sm font-semibold text-ink-primary">Event</span>
              <Badge tone="accent">awaiting selection</Badge>
            </div>
            <KeyValue label="Event overview" value="Populated from a selected SecurityEvent" />
            <KeyValue label="Severity and confidence" value="Engine-assigned only" />
            <KeyValue label="Source information" value="Not available" />
            <KeyValue label="Evidence references" value="Not available" />
            <KeyValue label="Detection explanation" value="Not available" />
            <KeyValue label="Timeline" value="Not recorded" />
            <KeyValue label="Investigation status" value="Awaiting event" />
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">
              Selecting an event from the table populates this panel from the verified record only. No incident is ever synthesized to demonstrate this view.
            </p>
          </div>
        </div>
        <div>
          <SectionHeader title="Response actions" description="Approval-gated workflow surface." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="flex flex-wrap gap-2">
              {responseActions.map((a) => (
                <Button key={a.label} icon={<IconCopilot size={12} />} disabled disabledReason={a.reason}>
                  {a.label}
                </Button>
              ))}
            </div>
            <div className="mt-4 rounded-lg border border-amber-900/40 bg-amber-900/10 p-3">
              <div className="flex items-center gap-2 text-xs font-semibold text-amber-300">
                <IconBell size={13} />
                Human approval is mandatory
              </div>
              <p className="mt-1 text-[11px] leading-4 text-ink-secondary">
                Security action execution requires validated evidence, policy checks (SecurityPolicy / HumanApprovalPolicy)
                and explicit human approval. AI never autonomously blocks IPs, terminates processes or changes firewall rules.
              </p>
            </div>
          </div>
        </div>
      </section>
    </PageContainer>
  );
}
