import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue } from "../components/ui/controls";
import { DataTable, type DataTableColumn } from "../components/ui/DataTable";
import { useSystemStore } from "../store/systemStore";
import type { ThreatIndicator, ThreatIntelligenceFeed } from "../types/securityIntelligence";
import {
  IconBell, IconReports, IconSearch, IconShield, IconThreatIntel
} from "../components/ui/icons";

const readinessCards = [
  { title: "Intelligence Engine", value: "Not initialized", hint: "The intelligence engine exists as typed interfaces; no collector runs and no feed is ingested.", icon: <IconShield size={14} /> },
  { title: "Feed Connectivity", value: "No feeds configured", hint: "No intelligence provider has been configured, contacted or authenticated.", icon: <IconBell size={14} /> },
  { title: "Last Feed Update", value: "Unavailable", hint: "No feed has ever synchronized, so no update time can be reported.", icon: <IconReports size={14} /> },
  { title: "Indicator Database", value: "No intelligence data", hint: "The indicator store is empty; nothing is queried or claimed until feeds exist.", icon: <IconThreatIntel size={14} /> }
] as const;

const summaryCards = [
  { title: "Total Indicators", value: "Data unavailable", hint: "Indicator counts appear only from verified feed ingestion — never invented." },
  { title: "Malicious Indicators", value: "Data unavailable", hint: "A missing feed must never be read as zero malicious indicators." },
  { title: "Suspicious Indicators", value: "Data unavailable", hint: "Suspicion classification requires source-assigned confidence." },
  { title: "Trusted Indicators", value: "Data unavailable", hint: "Allowlisting requires verified source reputation." },
  { title: "Active Feeds", value: "No feeds configured", hint: "Feeds enable only through the feed management panel once integration exists." },
  { title: "Feed Freshness", value: "Unknown", hint: "Freshness is a source-reported signal; it is never estimated client-side." }
] as const;

const indicatorColumns: DataTableColumn<ThreatIndicator>[] = [
  { id: "indicator", header: "Indicator", render: (row) => <span className="font-medium text-ink-primary">{row.value}</span> },
  { id: "type", header: "Indicator type", render: (row) => row.type },
  { id: "reputation", header: "Reputation", render: (row) => row.reputation },
  { id: "severity", header: "Severity", render: (row) => row.severity },
  { id: "confidence", header: "Confidence", align: "right", render: (row) => `${Math.round(row.confidence * 100)}%` },
  { id: "source", header: "Intelligence source", render: (row) => row.source },
  { id: "lastUpdated", header: "Last updated", render: (row) => row.lastSeen ?? "Not reported" },
  { id: "status", header: "Status", render: (row) => row.status },
  {
    id: "actions",
    header: "Actions",
    align: "right",
    render: () => <Button disabled disabledReason="Actions require a validated feed item and policy-gated workflow.">Open</Button>
  }
];

const feedColumns: DataTableColumn<ThreatIntelligenceFeed>[] = [
  { id: "name", header: "Feed name", render: (row) => row.name },
  { id: "provider", header: "Provider", render: (row) => row.provider },
  { id: "feedType", header: "Feed type", render: (row) => row.feedType },
  { id: "connectionStatus", header: "Connection status", render: (row) => row.connectionStatus },
  { id: "lastSync", header: "Last sync", render: (row) => row.lastSynchronizedAt ?? "Never" },
  { id: "freshness", header: "Freshness", render: (row) => row.freshness },
  { id: "reliability", header: "Reliability", align: "right", render: (row) => (row.reliability == null ? "Not published" : `${Math.round(row.reliability * 100)}%`) },
  {
    id: "enabled",
    header: "Enabled",
    align: "right",
    render: () => <Button disabled disabledReason="Feed enablement requires provider integration and human approval.">Disabled</Button>
  }
];

const reputationCards = [
  { reputation: "Malicious", tones: "border-red-900/60 bg-red-900/10 text-red-300", description: "Source-backed malicious reputation and corroborated confidence. Nothing is flagged until feeds exist." },
  { reputation: "Suspicious", tones: "border-amber-900/60 bg-amber-900/10 text-amber-300", description: "Elevated concern from one or more intelligence sources. Requires corroboration before response." },
  { reputation: "Unknown", tones: "border-charcoal-600 bg-charcoal-800 text-ink-secondary", description: "No source evidence yet. Unknown is NOT safe and is never treated as trusted." },
  { reputation: "Trusted", tones: "border-emerald-900/60 bg-emerald-900/10 text-emerald-300", description: "Source-verified benign reputation with proven freshness and corroboration." }
] as const;

const evaluationCandidates = [
  { name: "MISP", provider: "MISP Project", note: "Open-source threat sharing platform — planned evaluation candidate, no instance configured." },
  { name: "AbuseIPDB", provider: "AbuseIPDB", note: "IP reputation database — candidate for future integration, no API key held by this build." },
  { name: "VirusTotal", provider: "VirusTotal", note: "File/URL/domain reputation — aspirational provider, no connection, no terms accepted." },
  { name: "AlienVault OTX", provider: "AT&T Cybersecurity", note: "Open threat exchange — candidate only, nothing synchronized or ingested." }
] as const;

const lookupPlaceholderOptions = ["IPv4", "IPv6", "Domain", "URL", "File hash"] as const;

export default function ThreatIntelligencePage() {
  const { foundation, database, error, loading } = useSystemStore();
  const bridgeOk = Boolean(foundation || database);
  const readinessVariation = loading ? "warn" : bridgeOk ? "idle" : "idle";
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "Intelligence engine not initialized";

  // Real feed records only — no provider is configured in this build, so the table
  // is intentionally empty. No domains, addresses, hashes or URLs are inserted.
  const indicators: ThreatIndicator[] = [];
  const feeds: ThreatIntelligenceFeed[] = [];

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Threat Intelligence"
          subtitle="Understand threat indicators, intelligence sources and reputation signals."
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error ? (
              <>Backend bridge unavailable — {error}. Intelligence status below reflects the static frontend only.</>
            ) : (
              <>
                The intelligence engine is <strong className="font-semibold text-ink-primary">not initialized</strong>. This screen is the design surface for the future
                indicator feed — no provider is connected, no reputation lookup is performed and no indicator count is claimed until a verified feed exists.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Intelligence readiness" description="Actual integration readiness — honest states only." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {readinessCards.map((c) => (
            <StatusCard key={c.title} title={c.title} value={c.value} variation="idle" hint={c.hint} icon={c.icon} />
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Intelligence summary" description="Indicators and feeds from verified intelligence only." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {summaryCards.map((c) => (
            <StatusCard key={c.title} title={c.title} value={c.value} variation="idle" hint={c.hint} />
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Threat indicator workspace" description="Verified indicators from configured feeds. Nothing is fabricated." action={<Badge tone="neutral">no feeds configured</Badge>} />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <DataTable
            columns={indicatorColumns}
            rows={indicators}
            rowKey={(row) => row.id}
            minWidth={1100}
            caption="Threat indicators — verified feed records only"
            emptyState={
              <EmptyState
                icon={<IconThreatIntel size={18} />}
                title="No threat indicators available. Intelligence feed integration is pending."
                description="Indicator addresses, domains, URLs, file hashes, confidences and source names are never invented. Rows populate exclusively from the future feed ingestion pipeline."
              />
            }
          />
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Indicator detail" description="Investigation template for a real indicator." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="mb-3 flex items-center justify-between">
              <span className="text-sm font-semibold text-ink-primary">Indicator</span>
              <Badge tone="accent">awaiting selection</Badge>
            </div>
            <KeyValue label="Indicator value" value="Not available" />
            <KeyValue label="Indicator type" value="Not available" />
            <KeyValue label="Reputation classification" value="Not available" />
            <KeyValue label="Severity" value="Not available" />
            <KeyValue label="Confidence" value="Not available" />
            <KeyValue label="Intelligence source" value="Not available" />
            <KeyValue label="First seen" value="Not available" />
            <KeyValue label="Last seen" value="Not available" />
            <KeyValue label="Evidence references" value="Not available" />
            <KeyValue label="Related security events" value="Not available" />
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">Selecting an indicator populates this panel from verified feed data only. No threat evidence is ever synthesized.</p>
          </div>
        </div>
        <div>
          <SectionHeader title="Indicator search" description="Design surface for future lookup." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="flex items-center gap-2 rounded-lg border border-charcoal-600 bg-field px-3 py-2 text-xs text-ink-muted">
              <IconSearch size={13} />
              <input disabled placeholder="Search IP address, domain, URL or file hash…" aria-label="Indicator search" className="h-7 w-full bg-transparent text-ink-primary placeholder:text-ink-muted focus:outline-none" />
              <span className="text-[10px] uppercase tracking-wider">Feed required</span>
            </div>
            <div className="mt-2.5 flex flex-wrap items-center gap-2">
              {lookupPlaceholderOptions.map((opt) => <Badge key={opt} tone="neutral">{opt}</Badge>)}
            </div>
            <div className="mt-3 rounded-lg border border-amber-900/40 bg-amber-900/10 p-3 text-[11px] leading-4 text-amber-300">
              No live reputation lookup is connected. Queries are <strong>never</strong> transmitted to external services; the lookup action stays disabled until an approved provider is integrated.
            </div>
            <div className="mt-3">
              <Button disabled disabledReason="Reputation lookup requires an integrated provider and an explicit user action.">Look up</Button>
            </div>
          </div>
        </div>
      </section>

      <section>
        <SectionHeader title="Intelligence feed management" description="Provider configuration panel — nothing connects until integrated." />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <DataTable
            columns={feedColumns}
            rows={feeds}
            rowKey={(row) => row.id}
            minWidth={980}
            caption="Intelligence feeds — configuration surface, none configured"
            emptyState={
              <EmptyState
                icon={<IconThreatIntel size={18} />}
                title="Threat intelligence provider integration is pending."
                description="Feed fields (name, provider, type, connection status, last sync, freshness, reliability, enable/disable) populate only after a provider is integrated through this panel — with human approval and explicit consent before any outbound call."
              />
            }
          />
        </div>
      </section>

      <section>
        <SectionHeader title="Intelligence sources" description="Future providers — evaluation candidates only, no integrations implied." action={<Badge tone="neutral">no integrations</Badge>} />
        <div className="grid gap-2.5 sm:grid-cols-2 xl:grid-cols-4">
          {evaluationCandidates.map((c) => (
            <div key={c.name} className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3.5">
              <div className="flex items-center gap-2 text-xs font-semibold text-ink-primary">
                <span className="rounded-lg border border-charcoal-600 bg-charcoal-900 p-1.5 text-accent-cyan"><IconThreatIntel size={13} /></span>
                {c.name}
              </div>
              <div className="mt-0.5 text-[10px] text-ink-muted">{c.provider}</div>
              <p className="mt-1.5 text-[11px] leading-4 text-ink-secondary">{c.note}</p>
              <div className="mt-2"><Badge tone="warn">evaluation candidate</Badge></div>
            </div>
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Reputation classification" description="Labels require source-backed evidence and confidence — Unknown is not safe." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {reputationCards.map((r) => (
            <div key={r.reputation} className={`rounded-xl3 border p-3.5 shadow-panel ${r.tones}`}>
              <div className="text-[11px] uppercase tracking-wider opacity-90">{r.reputation}</div>
              <p className="mt-1.5 text-[11px] leading-4 text-ink-secondary">{r.description}</p>
            </div>
          ))}
        </div>
      </section>
    </PageContainer>
  );
}
