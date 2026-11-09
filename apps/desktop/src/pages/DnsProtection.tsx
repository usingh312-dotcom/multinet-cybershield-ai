import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue } from "../components/ui/controls";
import { DataTable, type DataTableColumn } from "../components/ui/DataTable";
import { useSystemStore } from "../store/systemStore";
import type { DnsEvent } from "../types/securityIntelligence";
import {
  IconActivity, IconBell, IconCopilot, IconDns, IconReports, IconShield, IconThreatIntel
} from "../components/ui/icons";

const statusCards = [
  { title: "DNS Protection Engine", value: "Not initialized", hint: "The DNS protection engine exists as typed interfaces; no policy applies and no enforcement runs.", icon: <IconShield size={14} /> },
  { title: "DNS Monitoring", value: "Monitoring not active", hint: "Monitors are interface-level stubs; no resolver is observed or queried.", icon: <IconActivity size={14} /> },
  { title: "Threat Domain Detection", value: "No DNS events", hint: "Detection requires a verified event source — nothing detects or claims domains.", icon: <IconThreatIntel size={14} /> },
  { title: "DNS Policy Status", value: "Policy not configured", hint: "No policy exists; no DNS setting of this machine is read, planned or changed.", icon: <IconDns size={14} /> },
  { title: "DNS Query Collection", value: "Integration pending", hint: "Collection attaches to a verified collector once the DNS module integrates.", icon: <IconBell size={14} /> },
  { title: "Last Analysis", value: "No analysis performed", hint: "Analysis runs on collector history only; nothing is analyzed today.", icon: <IconReports size={14} /> }
] as const;

const activityPanels = [
  { title: "Total DNS queries", value: "Data unavailable", hint: "Query counters arrive from the collector — never interpolated client-side." },
  { title: "Unique queried domains", value: "Data unavailable", hint: "Domain sets are computed from real query logs only." },
  { title: "Suspicious domains", value: "Data unavailable", hint: "Suspicion requires source-backed classification; nothing is labeled today." },
  { title: "Blocked domains", value: "Data unavailable", hint: "Blocking is a policy action — none exists in this build." },
  { title: "Query latency", value: "Not measured", hint: "Latency is measured on the collector clock — never simulated." }
] as const;

const dnsEventColumns: DataTableColumn<DnsEvent>[] = [
  { id: "domain", header: "Domain", render: (row) => <span className="font-medium text-ink-primary">{row.domain}</span> },
  { id: "queryType", header: "Query type", render: (row) => row.queryType },
  { id: "client", header: "Client", render: (row) => row.clientReference },
  { id: "resolver", header: "Resolver", render: (row) => row.resolverReference },
  { id: "reputation", header: "Reputation", render: (row) => row.reputation },
  { id: "timestamp", header: "Timestamp", render: (row) => row.timestamp },
  { id: "status", header: "Status", render: (row) => row.status },
  {
    id: "action",
    header: "Action",
    align: "right",
    render: () => <Button disabled disabledReason="Opening a domain requires a verified event and policy-gated workflow.">Open</Button>
  }
];

const allowlistColumns: DataTableColumn<{ domain: string; category: string; source: string; addedAt: string; expiry: string; status: string }>[] = [
  { id: "domain", header: "Domain", render: (row) => row.domain },
  { id: "category", header: "Category", render: (row) => row.category },
  { id: "source", header: "Source", render: (row) => row.source },
  { id: "addedDate", header: "Added date", render: (row) => row.addedAt },
  { id: "expiry", header: "Expiry", render: (row) => row.expiry },
  { id: "status", header: "Status", render: (row) => row.status }
];

const threatCategories = [
  { name: "Known malicious domain", description: "Domains flagged malicious by verified intelligence sources.", icon: <IconThreatIntel size={14} /> },
  { name: "Phishing domain", description: "Credential-harvesting and impersonation domains.", icon: <IconBell size={14} /> },
  { name: "Malware distribution", description: "Domains serving or staging known malware.", icon: <IconReports size={14} /> },
  { name: "Command-and-control domain", description: "C2 infrastructure actively used by malware.", icon: <IconActivity size={14} /> },
  { name: "Suspicious DNS tunneling", description: "Data-exfiltration over DNS query patterns.", icon: <IconDns size={14} /> },
  { name: "DNS anomaly", description: "Statistical deviations from baseline query behaviour.", icon: <IconActivity size={14} /> }
] as const;

const policyRows = [
  { name: "Malicious domain protection", description: "Block domains with source-backed malicious reputation.", reason: "Requires a configured policy engine and human approval." },
  { name: "Suspicious domain detection", description: "Flag domains with low-confidence suspicious signals.", reason: "Detection requires integrated intelligence feeds." },
  { name: "Allowlist management", description: "Explicitly trusted domains, source-attributed.", reason: "Editing rules requires policy validation and approval." },
  { name: "Blocklist management", description: "Explicitly blocked domains, source-attributed.", reason: "Editing rules requires policy validation and approval." },
  { name: "DNS query anomaly detection", description: "Baseline + deviation analysis over the query log.", reason: "Anomaly models need a verified collection baseline first." },
  { name: "DNS tunneling detection", description: "Detect exfil patterns hidden inside DNS queries.", reason: "Tunneling detection requires collector history and a scoring engine." },
  { name: "Policy update frequency", description: "Cadence for re-validating policy data against sources.", reason: "Scheduling requires a working policy refresh pipeline." }
] as const;

const responseActions = [
  { label: "Investigate domain", reason: "Opening an investigation requires a verified DNS event; none exists." },
  { label: "Add to proposed blocklist", reason: "Adding entries requires validated evidence and human approval." },
  { label: "Add to proposed allowlist", reason: "Allowlisting requires validated evidence and human approval." },
  { label: "Create incident", reason: "Incident creation needs a confirmed domain finding with evidence." }
] as const;

export default function DnsProtectionPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const bridgeOk = Boolean(foundation || database);
  const readinessVariation = loading ? "warn" : bridgeOk ? "idle" : "idle";
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "DNS protection not initialized";

  // Verified DNS event/policy records only — none exist in this build. No domains,
  // clients, resolvers or timestamps are inserted.
  const dnsEvents: DnsEvent[] = [];
  const allowlist: Array<{ domain: string; category: string; source: string; addedAt: string; expiry: string; status: string }> = [];
  const blocklist: typeof allowlist = [];

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="DNS Protection"
          subtitle="Monitor DNS security, domain reputation and protective policy readiness."
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error ? (
              <>Backend bridge unavailable — {error}. DNS status below reflects the static frontend only.</>
            ) : (
              <>
                DNS protection is <strong className="font-semibold text-ink-primary">not initialized</strong>. This screen is the design surface for the future DNS security
                module — no resolvers are queried, no packets are captured and no DNS setting is read or modified. All policies stay disabled until a backend exists.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="DNS protection status" description="Truthful readiness — no protection is claimed without a working engine." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {statusCards.map((c) => (
            <StatusCard key={c.title} title={c.title} value={c.value} variation="idle" hint={c.hint} icon={c.icon} />
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="DNS activity overview" description="Prepared panels — all metrics unavailable until real collection exists." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
          {activityPanels.map((c) => (
            <div key={c.title} className="rounded-xl3 border border-charcoal-600 bg-elevated shadow-panel p-3.5 flex flex-col gap-2">
              <span className="text-[11px] uppercase tracking-wider text-ink-muted">{c.title}</span>
              <span className="text-base font-semibold text-ink-primary">{c.value}</span>
              <p className="text-[10px] leading-4 text-ink-secondary">{c.hint}</p>
            </div>
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="DNS events" description="Verified DNS query events from the collector. Nothing is fabricated." action={<Badge tone="neutral">no collection attached</Badge>} />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <DataTable
            columns={dnsEventColumns}
            rows={dnsEvents}
            rowKey={(row) => row.id}
            minWidth={1000}
            caption="DNS events — verified collector records only"
            emptyState={
              <EmptyState
                icon={<IconDns size={18} />}
                title="No DNS events available. DNS monitoring integration is pending."
                description="Domains, client references, resolver references, query types and timestamps are never fabricated while the collector is detached. Query types A, AAAA, CNAME, MX, TXT, NS and PTR will populate from the verified feed."
              />
            }
          />
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Domain threat analysis" description="Investigation template for a real, verified domain finding." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="mb-3 flex items-center justify-between">
              <span className="text-sm font-semibold text-ink-primary">Domain</span>
              <Badge tone="accent">awaiting selection</Badge>
            </div>
            <KeyValue label="Domain" value="Not available" />
            <KeyValue label="Reputation" value="Not available" />
            <KeyValue label="Threat category" value="Not available" />
            <KeyValue label="Intelligence source" value="Not available" />
            <KeyValue label="Confidence" value="Not available" />
            <KeyValue label="First seen" value="Not available" />
            <KeyValue label="Last seen" value="Not available" />
            <KeyValue label="DNS query history" value="Not available" />
            <KeyValue label="Associated security events" value="Not available" />
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">Selecting a domain populates this panel from verified collector and intelligence data only. Reputation is never invented.</p>
          </div>
        </div>
        <div>
          <SectionHeader title="DNS protection policies" description="Planned controls — every rule stays disabled until a backend exists." />
          <div className="grid gap-2.5">
            {policyRows.map((p) => (
              <div key={p.name} className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="text-xs font-semibold text-ink-primary">{p.name}</div>
                    <div className="text-[11px] text-ink-secondary">{p.description}</div>
                  </div>
                  <span className="flex shrink-0 items-center gap-2">
                    <Badge tone="neutral">disabled</Badge>
                    <Button disabled disabledReason={p.reason}>Enable</Button>
                  </span>
                </div>
              </div>
            ))}
            <p className="mt-1 text-[10px] leading-4 text-ink-muted">All policies require a validated backend, policy validation (SecurityPolicy / HumanApprovalPolicy) and explicit human approval before any DNS behavior changes. No system DNS configuration is read or modified by this build.</p>
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="DNS allowlist" description="Trusted domains — source-attributed. Nothing is saved or enforced." action={<Badge tone="neutral">empty</Badge>} />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
            <DataTable
              columns={allowlistColumns}
              rows={allowlist}
              rowKey={(row, index) => `allowlist-${index}`}
              minWidth={680}
              caption="DNS allowlist — empty until policy integration"
              emptyState={
                <EmptyState
                  icon={<IconDns size={18} />}
                  title="Allowlist is empty"
                  description="Allowlist entries (domain, category, source, added date, expiry, status) appear only after the DNS policy integration exists. Nothing is persisted or enforced in this build."
                />
              }
            />
          </div>
        </div>
        <div>
          <SectionHeader title="DNS blocklist" description="Blocked domains — source-attributed. Nothing is saved or enforced." action={<Badge tone="neutral">empty</Badge>} />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
            <DataTable
              columns={allowlistColumns}
              rows={blocklist}
              rowKey={(row, index) => `blocklist-${index}`}
              minWidth={680}
              caption="DNS blocklist — empty until policy integration"
              emptyState={
                <EmptyState
                  icon={<IconDns size={18} />}
                  title="Blocklist is empty"
                  description="Blocklist entries appear only after the DNS policy integration exists with an approved source. No domain is blocked by this build — never are rules created or applied."
                />
              }
            />
          </div>
        </div>
      </section>

      <section>
        <SectionHeader title="DNS threat categories" description="Future detection capabilities — none have detected anything." action={<Badge tone="neutral">no detections</Badge>} />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {threatCategories.map((c) => (
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
        <SectionHeader title="DNS response actions" description="Approval-gated workflow surface." />
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
              DNS changes require validated evidence and human approval
            </div>
            <p className="mt-1 text-[11px] leading-4 text-ink-secondary">DNS changes require a validated finding, policy validation (SecurityPolicy / HumanApprovalPolicy) and explicit human approval. AI must never directly modify system DNS configuration — this build performs none of those actions.</p>
          </div>
        </div>
      </section>
    </PageContainer>
  );
}
