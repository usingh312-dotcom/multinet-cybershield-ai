import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue, SelectField } from "../components/ui/controls";
import { DataTable, type DataTableColumn } from "../components/ui/DataTable";
import { ActivityTimeline } from "../components/ui/widgets";
import { useSystemStore } from "../store/systemStore";
import type { FirewallRule } from "../types/endpointFirewall";
import {
  IconActivity, IconBell, IconCheck, IconCopilot, IconFirewall, IconReports, IconShield
} from "../components/ui/icons";

const readinessCards = [
  { title: "Firewall Manager", value: "Not initialized", hint: "The firewall manager exists as typed interfaces; no manager runs and no rule is evaluated.", icon: <IconShield size={14} /> },
  { title: "Platform Firewall", value: "Platform integration pending", hint: "Windows Firewall / packet-filter integration is not implemented or verified in this build.", icon: <IconFirewall size={14} /> },
  { title: "Active Rules", value: "Rules unavailable", hint: "No rule source exists yet; rule inventory arrives exclusively from the platform integration.", icon: <IconActivity size={14} /> },
  { title: "Pending Proposals", value: "No proposals", hint: "AI proposals require an authorized provider and verified evidence — neither exists.", icon: <IconCopilot size={14} /> },
  { title: "Last Policy Review", value: "No policy review performed", hint: "Policy review runs against real rule state; nothing has been reviewed.", icon: <IconReports size={14} /> },
  { title: "Firewall Monitoring", value: "Monitoring not active", hint: "No verdict, block or allow telemetry is collected in this build.", icon: <IconBell size={14} /> }
] as const;

const overviewCapabilities = [
  { name: "Inbound policy", description: "Rules governing unsolicited inbound traffic to this endpoint.", icon: <IconFirewall size={14} /> },
  { name: "Outbound policy", description: "Rules governing which processes may initiate outbound connections.", icon: <IconActivity size={14} /> },
  { name: "Application rules", description: "Per-executable network entitlements, scoped by signature and path.", icon: <IconShield size={14} /> },
  { name: "Port rules", description: "Protocol/port constraints for inbound and outbound traffic.", icon: <IconFirewall size={14} /> },
  { name: "Network zones", description: "Trusted, public and restricted network profile classification.", icon: <IconActivity size={14} /> },
  { name: "Rule priority", description: "Deterministic ordering when multiple rules match the same flow.", icon: <IconReports size={14} /> }
] as const;

const firewallRuleColumns: DataTableColumn<FirewallRule>[] = [
  { id: "name", header: "Rule name", render: (row) => <span className="font-medium text-ink-primary">{row.name}</span> },
  { id: "direction", header: "Direction", render: (row) => row.direction },
  { id: "protocol", header: "Protocol", render: (row) => row.protocol.toUpperCase() },
  { id: "source", header: "Source", render: (row) => row.source },
  { id: "destination", header: "Destination", render: (row) => row.destination },
  { id: "port", header: "Port", render: (row) => row.port },
  { id: "action", header: "Action", render: (row) => row.action },
  { id: "priority", header: "Priority", render: (row) => row.priority },
  { id: "status", header: "Status", render: (row) => row.status }
];

const approvalStages = [
  "Security event detected by a verified engine",
  "AI analyzes verified evidence",
  "Firewall rule proposal is generated",
  "SecurityPolicy validates the proposal",
  "User reviews rationale and impact",
  "User explicitly approves or rejects",
  "Authorized backend executes the approved action",
  "Audit log records the result",
  "Rollback is available where supported"
] as const;

const safetyPolicy = [
  "AI never directly modifies system firewall rules",
  "All rules require structured validation",
  "Explicit human approval is mandatory",
  "Privileged execution belongs to an authorized backend",
  "Every approved operation must be auditable",
  "Rollback must be supported where technically feasible"
] as const;

const fieldInputClass =
  "h-8 w-full rounded-lg border border-charcoal-600/60 bg-charcoal-800/40 px-2.5 text-xs text-ink-muted disabled:cursor-not-allowed";

export default function AiFirewallPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const bridgeOk = Boolean(foundation || database);
  const readinessVariation = loading ? "warn" : bridgeOk ? "idle" : "idle";
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "AI firewall not initialized";

  // Verified platform rule records only — none exist in this build. No rule,
  // address, port or priority is ever invented client-side.
  const firewallRules: FirewallRule[] = [];

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="AI Firewall"
          subtitle="Review firewall policies and defensive recommendations with human approval."
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error ? (
              <>Backend bridge unavailable — {error}. Firewall status below reflects the static frontend only.</>
            ) : (
              <>
                The AI Firewall is <strong className="font-semibold text-ink-primary">not initialized</strong>. This screen is the design surface for the future
                firewall policy module — no rule is read from or applied to the platform firewall, and no AI proposal exists. All actions stay disabled until a verified backend and approval architecture exist.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Firewall readiness" description="Truthful readiness — protection is never claimed without a working engine." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {readinessCards.map((c) => (
            <StatusCard key={c.title} title={c.title} value={c.value} variation="idle" hint={c.hint} icon={c.icon} />
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Firewall overview" description="Future capabilities — nothing is read from or written to the platform firewall." action={<Badge tone="neutral">platform integration pending</Badge>} />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {overviewCapabilities.map((c) => (
            <div key={c.name} className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3.5">
              <div className="flex items-center justify-between gap-2">
                <span className="flex items-center gap-2 text-xs font-semibold text-ink-primary">
                  <span className="rounded-lg border border-charcoal-600 bg-charcoal-900 p-1.5 text-accent-cyan">{c.icon}</span>
                  {c.name}
                </span>
                <Badge tone="warn">integration pending</Badge>
              </div>
              <p className="mt-1.5 text-[11px] leading-4 text-ink-secondary">{c.description}</p>
            </div>
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Firewall rules" description="Rules from the platform integration once attached. Nothing is invented." action={<Badge tone="neutral">no rule source</Badge>} />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <DataTable<FirewallRule>
            columns={firewallRuleColumns}
            rows={firewallRules}
            rowKey={(row) => row.id}
            minWidth={1000}
            caption="Firewall rules — platform-verified records only"
            emptyState={
              <EmptyState
                icon={<IconFirewall size={18} />}
                title="No firewall rules available. Platform integration is pending."
                description="Rule names, directions, protocols, sources, destinations, ports, actions and priorities are never invented client-side. Rows populate exclusively from the verified platform firewall integration."
              />
            }
          />
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="AI recommendation" description="Structured proposal template — empty until provider and evidence exist." action={<Badge tone="neutral">unavailable</Badge>} />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="mb-3 flex items-center justify-between gap-3">
              <span className="text-sm font-semibold text-ink-primary">Recommendation</span>
              <Badge tone="accent">not generated</Badge>
            </div>
            <KeyValue label="Recommendation title" value="Not available" />
            <KeyValue label="Security rationale" value="Not available" />
            <KeyValue label="Evidence references" value="Not available" />
            <KeyValue label="Proposed rule" value="Not available" />
            <KeyValue label="Risk level" value="Not available" />
            <KeyValue label="Potential impact" value="Not available" />
            <KeyValue label="Reversibility" value="Not available" />
            <KeyValue label="Approval status" value="No pending proposal" />
            <div className="mt-3 rounded-lg border border-charcoal-600 bg-charcoal-900/60 p-3">
              <div className="text-[11px] leading-4 text-ink-secondary">
                AI recommendations are unavailable until an authorized AI provider and verified security evidence source are connected.
              </div>
            </div>
          </div>
        </div>

        <div>
          <SectionHeader title="Human approval workflow" description="No rule executes without explicit human approval." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <ol className="space-y-2">
              {approvalStages.map((stage, index) => (
                <li key={stage} className="flex items-start gap-2.5 text-xs leading-5 text-ink-secondary">
                  <span className="mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-md border border-charcoal-600 bg-charcoal-800 font-mono text-[10px] text-ink-muted">{index + 1}</span>
                  {stage}
                </li>
              ))}
            </ol>
            <div className="mt-3 flex items-start gap-2 rounded-lg border border-amber-900/40 bg-amber-900/10 p-3">
              <IconBell size={14} className="mt-0.5 shrink-0 text-amber-300" />
              <span className="text-[11px] leading-4 text-amber-200">Integration pending — no proposal exists, so Approve / Reject stay disabled. Approval requires a verified engine finding, structured SecurityPolicy validation and your explicit decision. Execution belongs to the authorized backend only.</span>
            </div>
            <div className="mt-3 flex flex-wrap gap-2">
              <Button disabled disabledReason="No proposal exists to approve — requires the AI provider, verified evidence and policy validation.">Approve</Button>
              <Button disabled disabledReason="No proposal exists to reject — requires a pending proposal under review.">Reject</Button>
              <Button disabled disabledReason="Applying a rule executes on the platform firewall — requires an approved proposal and the authorized backend.">Apply Rule</Button>
            </div>
          </div>
        </div>
      </section>

      <section>
        <SectionHeader title="Rule builder preview" description="Integration preview — every field disabled, nothing is persisted or applied." action={<Badge tone="neutral">disabled preview</Badge>} />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            <div>
              <label htmlFor="fw-rule-name" className="mb-1 block text-xs font-medium text-ink-secondary">Rule name</label>
              <input id="fw-rule-name" type="text" disabled aria-disabled="true" title="Rule creation requires the platform integration and an approved proposal." placeholder="Rule name (pending integration)" className={fieldInputClass} />
            </div>
            <SelectField label="Direction" options={["Inbound", "Outbound"]} disabled disabledReason="Direction is fixed by the approved proposal, never typed freehand." hint="Options shown for shape only — integration pending." />
            <SelectField label="Protocol" options={["TCP", "UDP", "ICMP", "Any"]} disabled disabledReason="Protocol is derived from verified evidence and the approved proposal." hint="Options shown for shape only — integration pending." />
            <div>
              <label htmlFor="fw-source" className="mb-1 block text-xs font-medium text-ink-secondary">Source</label>
              <input id="fw-source" type="text" disabled aria-disabled="true" title="Source is bound by the approved proposal — never invented or typed ad hoc." placeholder="Source (pending integration)" className={fieldInputClass} />
            </div>
            <div>
              <label htmlFor="fw-destination" className="mb-1 block text-xs font-medium text-ink-secondary">Destination</label>
              <input id="fw-destination" type="text" disabled aria-disabled="true" title="Destination is bound by the approved proposal — never invented or typed ad hoc." placeholder="Destination (pending integration)" className={fieldInputClass} />
            </div>
            <div>
              <label htmlFor="fw-port" className="mb-1 block text-xs font-medium text-ink-secondary">Port</label>
              <input id="fw-port" type="text" disabled aria-disabled="true" title="Port scope comes from the approved proposal and policy validation." placeholder="Port (pending integration)" className={fieldInputClass} />
            </div>
            <SelectField label="Action" options={["Allow", "Block"]} disabled disabledReason="Action is part of the approved proposal — never chosen ad hoc." hint="Options shown for shape only — integration pending." />
            <SelectField label="Priority" options={["1 — Highest", "50 — Normal", "100 — Lowest"]} disabled disabledReason="Priority ordering is computed by policy validation, not set manually." hint="Options shown for shape only — integration pending." />
            <div>
              <label htmlFor="fw-expiration" className="mb-1 block text-xs font-medium text-ink-secondary">Expiration</label>
              <input id="fw-expiration" type="text" disabled aria-disabled="true" title="Expiration is set by policy for time-boxed proposals — integration pending." placeholder="Expiration (pending integration)" className={fieldInputClass} />
            </div>
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <Button disabled disabledReason="Rule builder is an integration preview — nothing is validated, persisted or applied.">Save draft</Button>
            <Button disabled disabledReason="Applying a rule requires an approved proposal and the authorized backend.">Apply rule</Button>
            <p className="text-[10px] leading-4 text-ink-muted">Disabled preview only. No rule is created, ordered, persisted or applied by this build.</p>
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Firewall activity" description="Future event timeline." action={<Badge tone="neutral">no events</Badge>} />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <ActivityTimeline
              items={[]}
              emptyState={
                <EmptyState
                  icon={<IconFirewall size={18} />}
                  title="No firewall events recorded"
                  description="Rule changes, policy approvals, blocked connections, allow decisions and rollback events appear only after the platform integration and audit pipeline exist. Nothing is simulated to fill this timeline."
                />
              }
            />
          </div>
        </div>
        <div>
          <SectionHeader title="AI firewall safety policy" description="Non-negotiable boundaries — enforced by architecture, not convention." />
          <div className="rounded-xl2 border border-amber-900/30 bg-amber-900/5 p-4">
            <div className="flex items-center gap-2">
              <IconShield size={14} className="text-amber-400" />
              <span className="text-xs font-semibold text-amber-200">AI firewall operating rules</span>
            </div>
            <ul className="mt-2.5 space-y-1.5">
              {safetyPolicy.map((rule) => (
                <li key={rule} className="flex items-start gap-2 text-[11px] leading-4 text-ink-secondary">
                  <IconCheck size={12} className="mt-0.5 shrink-0 text-amber-400" aria-hidden="true" />
                  {rule}
                </li>
              ))}
            </ul>
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">These boundaries hold regardless of provider, model or evidence source. Any future backend that relaxes them fails review.</p>
          </div>
        </div>
      </section>
    </PageContainer>
  );
}
