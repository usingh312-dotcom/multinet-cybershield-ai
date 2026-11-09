import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue } from "../components/ui/controls";
import { DataTable, type DataTableColumn } from "../components/ui/DataTable";
import { ActivityTimeline } from "../components/ui/widgets";
import { useSystemStore } from "../store/systemStore";
import type { IncidentRecord } from "../types/incidents";
import {
  IconActivity, IconBell, IconCopilot, IconDatabase,
  IconIncident, IconReports, IconSearch, IconShield
} from "../components/ui/icons";

const readinessCards = [
  { title: "Incident Response Engine", value: "Not initialized", hint: "The incident engine exists as typed interfaces; no detector opens incidents and no playbook runs.", icon: <IconShield size={14} /> },
  { title: "Open Incidents", value: "No incident data", hint: "Incident feeds require a verified detection backend — nothing is opened, counted or synthesized.", icon: <IconIncident size={14} /> },
  { title: "Critical Incidents", value: "No incident data", hint: "Severity classification arrives from a verified engine; a missing feed is never zero threats.", icon: <IconBell size={14} /> },
  { title: "Investigations", value: "No investigations available", hint: "Investigation state is bound to verified incidents — none exist in this build.", icon: <IconSearch size={14} /> },
  { title: "Pending Approvals", value: "No approval requests", hint: "Defensive responses require evidence + policy validation + your explicit approval.", icon: <IconCopilot size={14} /> },
  { title: "Last Incident", value: "No incident history", hint: "No incident has been recorded; history appears only after integration.", icon: <IconReports size={14} /> }
] as const;

const overviewPanels = [
  { title: "Incident queue", value: "Empty", hint: "Queue populates from verified detectors only." },
  { title: "Investigation status", value: "Not available", hint: "Investigation workflow binds to verified incidents." },
  { title: "Severity distribution", value: "Not available", hint: "Distribution is computed from real incident records." },
  { title: "Response readiness", value: "Not initialized", hint: "Response readiness requires the incident engine and approval pipeline." },
  { title: "Recent incident activity", value: "No activity", hint: "Activity is timestamped by the audit/engine clock only." }
] as const;

const incidentColumns: DataTableColumn<IncidentRecord>[] = [
  { id: "incidentId", header: "Incident ID", render: (row) => <span className="font-mono text-ink-primary">{row.id}</span> },
  { id: "title", header: "Title", render: (row) => <span className="font-medium text-ink-primary">{row.title}</span> },
  { id: "severity", header: "Severity", render: (row) => row.severity },
  { id: "category", header: "Category", render: (row) => row.category },
  { id: "source", header: "Source", render: (row) => row.source },
  { id: "createdAt", header: "Created at", render: (row) => row.createdAt },
  { id: "assignedTo", header: "Assigned to", render: (row) => row.assignedTo ?? "Unassigned" },
  { id: "status", header: "Investigation status", render: (row) => row.status },
  {
    id: "actions",
    header: "Actions",
    align: "right",
    render: () => <Button disabled disabledReason="Opening an incident requires a verified incident record and policy-gated workflow.">Open</Button>
  }
];

const evidenceStreams = [
  { name: "Network event references", description: "Flow/connection records tied to the incident from the network collector." },
  { name: "Endpoint event references", description: "Collector findings (process, integrity, malware) attached to the incident." },
  { name: "DNS event references", description: "Resolver events corroborating the incident, when DNS integration lands." },
  { name: "Threat intelligence references", description: "Source-attributed matches used during investigation." },
  { name: "Audit log references", description: "Immutable audit records for every proposed and approved action." },
  { name: "Evidence integrity hashes", description: "Hash verification for each attached evidence object." }
] as const;

const workflowSteps = [
  "Detection", "Incident creation", "Evidence collection", "Investigation",
  "AI recommendation", "Policy validation", "Human approval", "Authorized response",
  "Audit logging", "Resolution"
] as const;

const timelinePreview = [
  "Incident created", "Evidence collected", "Investigation started", "Severity reviewed",
  "Action proposed", "Approval requested", "Action approved", "Response completed", "Incident resolved"
] as const;

const responseActions = [
  { label: "Assign investigation", reason: "Assignment requires a verified incident record and an onboarded analyst identity." },
  { label: "Add investigation note", reason: "Notes attach to verified incidents only; none exists in this build." },
  { label: "Escalate incident", reason: "Escalation routes through the approval pipeline; no incident exists to escalate." },
  { label: "Request endpoint scan", reason: "Scans execute only against a verified, onboarded agent." },
  { label: "Propose endpoint isolation", reason: "Containment is a proposal requiring evidence, policy validation and explicit approval." },
  { label: "Propose firewall block", reason: "Blocking is a proposal requiring evidence, policy validation and explicit approval." },
  { label: "Resolve incident", reason: "Resolution requires a verified incident and a completed, audited response." }
] as const;

export default function IncidentResponsePage() {
  const { foundation, database, error, loading } = useSystemStore();
  const bridgeOk = Boolean(foundation || database);
  const readinessVariation = loading ? "warn" : "idle";
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "Incident response not initialized";

  // Verified incident records only — none exist in this build. No incident,
  // identifier, timestamp, address or assignee is ever invented client-side.
  const incidents: IncidentRecord[] = [];

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Incident Response"
          subtitle="Investigate security incidents, review evidence and coordinate defensive response."
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error ? (
              <>Backend bridge unavailable — {error}. Incident status below reflects the static frontend only.</>
            ) : (
              <>
                Incident response is <strong className="font-semibold text-ink-primary">not initialized</strong>. This screen is the design surface for the future
                incident module — no incident is opened, no evidence is collected, no device is contained and no rule is applied. All actions stay disabled until a verified backend and approval architecture exist.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Incident readiness" description="Truthful readiness — incidents are never claimed without a working engine." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {readinessCards.map((c) => (
            <StatusCard key={c.title} title={c.title} value={c.value} variation="idle" hint={c.hint} icon={c.icon} />
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Incident overview" description="Operations workspace — every panel unavailable until verified records exist." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
          {overviewPanels.map((p) => (
            <div key={p.title} className="rounded-xl3 border border-charcoal-600 bg-elevated shadow-panel p-3.5 flex flex-col gap-2">
              <span className="text-[11px] uppercase tracking-wider text-ink-muted">{p.title}</span>
              <span className="text-base font-semibold text-ink-primary">{p.value}</span>
              <p className="text-[10px] leading-4 text-ink-secondary">{p.hint}</p>
            </div>
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Incident queue" description="Verified incidents from the detection backend. Nothing is fabricated." action={<Badge tone="neutral">no incident source</Badge>} />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <DataTable<IncidentRecord>
            columns={incidentColumns}
            rows={incidents}
            rowKey={(row) => row.id}
            minWidth={1100}
            caption="Incident queue — verified engine records only"
            emptyState={
              <EmptyState
                icon={<IconIncident size={18} />}
                title="No incidents available. Security event and incident response integration is pending."
                description="Incident IDs, titles, severities, categories, sources, timestamps, assignees and statuses are never invented client-side. Future lifecycle states: New, Investigating, Contained, Resolved, False positive — populating exclusively from the verified detection backend."
              />
            }
          />
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Incident detail" description="Per-incident investigation workspace — populates from a verified record only." action={<Badge tone="accent">awaiting selection</Badge>} />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <KeyValue label="Incident summary" value="Not available" />
            <KeyValue label="Severity and classification" value="Not assessed" />
            <KeyValue label="Event source" value="Not available" />
            <KeyValue label="Affected assets" value="Not available" />
            <KeyValue label="Evidence references" value="Not available" />
            <KeyValue label="Investigation notes" value="Not available" />
            <KeyValue label="Activity timeline" value="Not available" />
            <KeyValue label="Current response status" value="Not available" />
            <KeyValue label="Recommended next steps" value="Not available" />
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">Selecting an incident populates this panel from verified engine and audit data only. Recommendations are never generated without verified evidence and an authorized provider.</p>
          </div>
        </div>
        <div>
          <SectionHeader title="Incident timeline" description="Chronological record of the incident lifecycle." action={<Badge tone="neutral">no history</Badge>} />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="mb-3 flex flex-wrap gap-1.5">
              {timelinePreview.map((e) => (
                <Badge key={e} tone="neutral">{e}</Badge>
              ))}
            </div>
            <ActivityTimeline
              items={[]}
              emptyState={
                <EmptyState
                  icon={<IconActivity size={18} />}
                  title="No incident activity recorded"
                  description="Lifecycle events appear only after the incident engine and audit pipeline integrate. Nothing is simulated to fill this timeline."
                />
              }
            />
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Evidence management" description="Referenced evidence only — nothing is uploaded, read or scanned." action={<Badge tone="neutral">no evidence</Badge>} />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="grid gap-2.5">
              {evidenceStreams.map((s) => (
                <div key={s.name} className="flex items-center justify-between gap-3 rounded-lg border border-charcoal-600 bg-charcoal-900/60 p-3">
                  <div className="flex items-center gap-2.5">
                    <span className="rounded-lg border border-charcoal-600 bg-charcoal-800 p-1.5 text-accent-cyan"><IconDatabase size={13} /></span>
                    <div>
                      <div className="text-xs font-semibold text-ink-primary">{s.name}</div>
                      <div className="text-[10px] text-ink-secondary">{s.description}</div>
                    </div>
                  </div>
                  <Badge tone="warn">pending</Badge>
                </div>
              ))}
            </div>
            <div className="mt-3 rounded-lg border border-charcoal-600 bg-charcoal-900/60 p-3 text-[11px] leading-4 text-ink-secondary">
              No evidence attached. Evidence collection integration is pending. No file is uploaded, read or scanned; no hash is computed or invented by this build.
            </div>
          </div>
        </div>
        <div>
          <SectionHeader title="Incident response workflow" description="End-to-end lifecycle — target architecture, not a running system." action={<Badge tone="warn">preview — not operational</Badge>} />
          <div className="rounded-xl2 border border-amber-900/30 bg-amber-900/5 p-4">
            <div className="flex items-center gap-2">
              <IconShield size={14} className="text-amber-400" />
              <span className="text-xs font-semibold text-amber-200">Incident Response Architecture Preview — Not Operational</span>
            </div>
            <ol className="mt-3 space-y-1.5">
              {workflowSteps.map((step, index) => (
                <li key={step} className="flex items-start gap-2.5 text-xs leading-5 text-ink-secondary">
                  <span className="mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-md border border-charcoal-600 bg-charcoal-800 font-mono text-[10px] text-ink-muted">{index + 1}</span>
                  {step}
                </li>
              ))}
            </ol>
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">Every defensive response in this chain requires validated evidence, structured SecurityPolicy checks, your explicit approval, authorized backend execution, an audit record and rollback where technically feasible. No approval request exists in this build.</p>
          </div>
        </div>
      </section>

      <section>
        <SectionHeader title="Response actions" description="Approval-gated workflow surface — nothing executes." />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
          <div className="flex flex-wrap gap-2">
            {responseActions.map((a) => (
              <Button key={a.label} icon={<IconCopilot size={12} />} disabled disabledReason={a.reason}>
                {a.label}
              </Button>
            ))}
          </div>
          <div className="mt-3 flex items-start gap-2 rounded-lg border border-amber-900/40 bg-amber-900/10 p-3">
            <IconBell size={14} className="mt-0.5 shrink-0 text-amber-300" />
            <span className="text-[11px] leading-4 text-amber-200">
              Any defensive response requires validated evidence, structured SecurityPolicy checks, your explicit approval, authorized backend execution, an audit record and rollback where technically feasible.
              This build never isolates a device, blocks an IP address or modifies firewall rules.
            </span>
          </div>
        </div>
      </section>
    </PageContainer>
  );
}
