import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue, SelectField } from "../components/ui/controls";
import { DataTable, type DataTableColumn } from "../components/ui/DataTable";
import { useSystemStore } from "../store/systemStore";
import {
  IconActivity, IconArrowRight, IconDns, IconEndpoint, IconFirewall, IconIncident,
  IconReports, IconShield, IconThreatIntel
} from "../components/ui/icons";

const overviewCards = [
  { title: "Available Reports", value: "No reports available", hint: "Reports render only from persisted, verified records. None exist today — no count is fabricated.", icon: <IconReports size={14} /> },
  { title: "Security Events", value: "Data unavailable", hint: "No event feed is attached, so event totals are unavailable — never presented as zero threats.", icon: <IconActivity size={14} /> },
  { title: "Open Incidents", value: "Incident data not initialized", hint: "The incident lifecycle has no backing engine in this build.", icon: <IconIncident size={14} /> },
  { title: "Resolved Incidents", value: "Incident data not initialized", hint: "Resolved lifecycle records appear once the incident module integrates.", icon: <IconIncident size={14} /> },
  { title: "Last Report Generated", value: "No report generated", hint: "No report has ever been generated in this build.", icon: <IconReports size={14} /> }
] as const;

type ReportCategory = {
  name: string;
  description: string;
  availability: "Integration pending" | "No data source";
  icon: React.ReactNode;
  generateReason: string;
};

const reportCategories: ReportCategory[] = [
  { name: "Security Overview Report", description: "Engine readiness, coverage and event posture at a glance.", availability: "Integration pending", icon: <IconShield size={15} />, generateReason: "Requires the verified SecurityEngine data surface." },
  { name: "Network Security Report", description: "Traffic, interfaces, routes and inspection outcomes.", availability: "Integration pending", icon: <IconActivity size={15} />, generateReason: "Requires the TrafficMetrics collector feed." },
  { name: "Threat Intelligence Report", description: "Indicators, feed posture and reputation signals.", availability: "Integration pending", icon: <IconThreatIntel size={15} />, generateReason: "Requires integrated threat intelligence feeds." },
  { name: "DNS Security Report", description: "Domain reputation, policy outcomes and DNS anomalies.", availability: "Integration pending", icon: <IconDns size={15} />, generateReason: "Requires the DNS protection module." },
  { name: "Endpoint Security Report", description: "Process and device behavior over verified collectors.", availability: "Integration pending", icon: <IconEndpoint size={15} />, generateReason: "Requires endpoint collectors and an audit engine." },
  { name: "Incident Response Report", description: "Incidents opened, investigated, resolved or closed.", availability: "No data source", icon: <IconIncident size={15} />, generateReason: "Requires the Incident Response lifecycle backend." },
  { name: "Firewall Activity Report", description: "Policy changes and enforcement outcomes.", availability: "No data source", icon: <IconFirewall size={15} />, generateReason: "Requires the integrated firewall manager." },
  { name: "Executive Security Summary", description: "Leadership-ready posture narrative across all modules.", availability: "No data source", icon: <IconReports size={15} />, generateReason: "Requires verified datasets from every module." }
];

const executiveSections = [
  { name: "Security readiness", description: "Engine, database and policy readiness line-items." },
  { name: "Network protection", description: "Bonded paths, inspection posture and resolver health." },
  { name: "Endpoint protection", description: "Process and device coverage over verified collectors." },
  { name: "Threat landscape", description: "Correlated intelligence and incident-weighted severity." },
  { name: "Incident summary", description: "Lifecycle counts, MTTR — sourced only from audit records." },
  { name: "Recommended actions", description: "Approval-gated, priority-ranked defensive recommendations." }
] as const;

type ReportRecord = {
  id: string;
  name: string;
  category: string;
  generatedAt: string;
  reportingPeriod: string;
  source: string;
  status: string;
};

const reportColumns: DataTableColumn<ReportRecord>[] = [
  { id: "reportName", header: "Report name", render: (row) => row.name },
  { id: "category", header: "Category", render: (row) => row.category },
  { id: "generatedAt", header: "Generated at", render: (row) => row.generatedAt },
  { id: "reportingPeriod", header: "Reporting period", render: (row) => row.reportingPeriod },
  { id: "source", header: "Source", render: (row) => row.source },
  { id: "status", header: "Status", render: (row) => row.status },
  {
    id: "actions",
    header: "Actions",
    align: "right",
    render: () => <Button disabled disabledReason="Viewing a report requires a generated, verified report record.">View</Button>
  }
];

const filterDisabledReasons = {
  category: "Report category filtering requires generated report records.",
  period: "Reporting-period filtering requires verified reporting-period metadata.",
  status: "Status filtering requires report-lifecycle state.",
  source: "Source filtering requires a configured, verified report source."
};

export default function SecurityReportsPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const bridgeOk = Boolean(foundation || database);
  const readinessVariation = loading ? "warn" : bridgeOk ? "idle" : "idle";
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "No verified dataset connected";

  // Verified report records only — none exist in this build. No report counts,
  // incident counts, event statistics or generation timestamps are inserted.
  const reports: ReportRecord[] = [];

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Security Reports"
          subtitle="Review security posture, threat activity and incident intelligence."
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error ? (
              <>Backend bridge unavailable — {error}. Report availability below reflects the static frontend only.</>
            ) : (
              <>
                Security reporting is <strong className="font-semibold text-ink-primary">not yet available</strong>. Reports render only from verified, persisted datasets —
                nothing is generated, and no findings or counts are claimed. Export actions stay disabled until a secure report pipeline exists.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Reports overview" description="Posture metrics from verified datasets only — never fabricated." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
          {overviewCards.map((c) => (
            <StatusCard key={c.title} title={c.title} value={c.value} variation="idle" hint={c.hint} icon={c.icon} />
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Report categories" description="Publication-grade categories — each shows its honest availability." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {reportCategories.map((c) => (
            <div key={c.name} className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3.5 flex flex-col gap-2">
              <div className="flex items-center justify-between gap-2">
                <span className="flex items-center gap-2 text-xs font-semibold text-ink-primary">
                  <span className="rounded-lg border border-charcoal-600 bg-charcoal-900 p-1.5 text-accent-cyan">{c.icon}</span>
                  {c.name}
                </span>
                <Badge tone="warn">{c.availability}</Badge>
              </div>
              <p className="text-[11px] leading-4 text-ink-secondary">{c.description}</p>
              <div className="mt-auto pt-1.5">
                <Button disabled disabledReason={c.generateReason}>Generate report</Button>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Report template" description="Layout contract for a real report generation." action={<Badge tone="neutral">template — not renderable</Badge>} />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <KeyValue label="Report title" value="Not generated" />
            <KeyValue label="Description" value="Populated from report template" />
            <KeyValue label="Reporting period" value="Not applicable — no verified data" />
            <KeyValue label="Data source" value="Not connected" />
            <KeyValue label="Report status" value="Not ready — no verified dataset" />
            <KeyValue label="Generation timestamp" value="Not available" />
            <KeyValue label="Findings summary" value="Not available" />
            <KeyValue label="Evidence references" value="Not available" />
            <KeyValue label="Recommendations" value="Not available" />
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">The template renders once a verified backend dataset exists. Generation never fabricates findings, incidents or timelines.</p>
          </div>
        </div>
        <div>
          <SectionHeader title="Report history" description="Verified report records only." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
            <DataTable
              columns={reportColumns}
              rows={reports}
              rowKey={(row) => row.id}
              minWidth={780}
              caption="Report history — generated report records only"
              emptyState={
                <EmptyState
                  icon={<IconReports size={18} />}
                  title="No security reports have been generated yet."
                  description="Report records appear after the reporting pipeline integrates and a verified dataset exists. No report name, timestamp, finding or incident is invented to fill this table."
                />
              }
            />
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Report filters" description="Filtering applies to persisted report records only." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <SelectField label="Report category" options={["Security Overview", "Network Security", "Threat Intelligence", "DNS Security", "Endpoint Security", "Incident Response", "Firewall Activity", "Executive Summary"]} disabled disabledReason={filterDisabledReasons.category} />
              <SelectField label="Reporting period" options={["Last 7 days", "Last 30 days", "Last 90 days", "Custom range"]} disabled disabledReason={filterDisabledReasons.period} />
              <SelectField label="Generation status" options={["Completed", "Failed", "Scheduled", "Superseded"]} disabled disabledReason={filterDisabledReasons.status} />
              <SelectField label="Data source" options={["Security Engine", "Traffic Metrics", "Threat Intelligence", "DNS Module", "Incident Engine"]} disabled disabledReason={filterDisabledReasons.source} />
            </div>
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">Filters enable automatically once generated report records exist. They never pre-filter invented rows.</p>
          </div>
        </div>
        <div>
          <SectionHeader title="Export" description="Secure export formats for verified reports." />
          <div className="grid gap-3 sm:grid-cols-3">
            {[
              { format: "PDF", hint: "Paginated leadership / board-ready layout.", disabledReason: "Export requires a generated, verified report and a hardened renderer." },
              { format: "CSV", hint: "Row-level data for audit and analysis tooling.", disabledReason: "Export requires a generated, verified report and a hardened serializer." },
              { format: "JSON", hint: "Machine-readable report artifact for pipelines.", disabledReason: "Export requires a generated, verified report and a hardened serializer." }
            ].map((f) => (
              <div key={f.format} className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4 flex flex-col gap-2">
                <div className="text-sm font-semibold text-ink-primary">{f.format}</div>
                <p className="text-[11px] leading-4 text-ink-secondary">{f.hint}</p>
                <Button className="mt-auto" disabled disabledReason={f.disabledReason}>Export {f.format}</Button>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section>
        <SectionHeader
          title="Executive summary preview"
          description="Intended report structure — visual preview only."
          action={<Badge tone="warn">report preview — no verified security dataset connected</Badge>}
        />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-5">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-charcoal-600/70 pb-4">
            <div className="flex items-center gap-3">
              <span className="rounded-lg border border-accent-blue/30 bg-accent-blue/10 p-2 text-accent-cyan"><IconReports size={17} /></span>
              <div>
                <div className="text-sm font-semibold text-ink-primary">MultiNet CyberShield AI — Executive Security Summary</div>
                <div className="text-[11px] text-ink-secondary">Prepared for leadership · period: not applicable (no dataset) · distribution: restricted</div>
              </div>
            </div>
            <Badge tone="neutral">placeholder — no data</Badge>
          </div>
          <div className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {executiveSections.map((s) => (
              <div key={s.name} className="rounded-lg border border-charcoal-600 bg-charcoal-900/60 p-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-[11px] font-semibold uppercase tracking-wider text-ink-muted">{s.name}</span>
                  <IconArrowRight size={12} className="text-ink-muted" />
                </div>
                <p className="mt-1.5 text-[11px] leading-4 text-ink-secondary">{s.description}</p>
                <div className="mt-2 text-[10px] uppercase tracking-wider text-ink-muted">no verified dataset connected</div>
              </div>
            ))}
          </div>
        </div>
      </section>
    </PageContainer>
  );
}
