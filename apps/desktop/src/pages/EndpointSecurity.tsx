import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue } from "../components/ui/controls";
import { DataTable, type DataTableColumn } from "../components/ui/DataTable";
import { ActivityTimeline } from "../components/ui/widgets";
import { useSystemStore } from "../store/systemStore";
import type { EndpointDevice } from "../types/endpointFirewall";
import {
  IconActivity, IconBell, IconCopilot, IconEndpoint, IconIncident, IconReports, IconSearch, IconShield
} from "../components/ui/icons";

const statusCards = [
  { title: "Endpoint Security Engine", value: "Not initialized", hint: "The endpoint engine exists as typed interfaces; no collector runs and no scan executes.", icon: <IconShield size={14} /> },
  { title: "Endpoint Monitoring", value: "Monitoring not active", hint: "No endpoint collector or agent heartbeat exists in this build.", icon: <IconActivity size={14} /> },
  { title: "Process Monitoring", value: "No endpoint data", hint: "Process visibility requires an authorized, verified collector — none is attached.", icon: <IconSearch size={14} /> },
  { title: "Malware Detection", value: "Scanner not configured", hint: "File and process scanning is unimplemented; no sample, quarantine or result exists.", icon: <IconReports size={14} /> },
  { title: "File Integrity Monitoring", value: "Not initialized", hint: "Hash verification and change tracking require the integrated backend.", icon: <IconShield size={14} /> },
  { title: "Last Security Scan", value: "No scan performed", hint: "No scan has ever run in this build.", icon: <IconReports size={14} /> }
] as const;

const summaryPanels = [
  { title: "Managed endpoints", value: "Data unavailable", hint: "Device inventory arrives only from verified agents — never enumerated locally." },
  { title: "Healthy endpoints", value: "Data unavailable", hint: "Health classification requires real posture telemetry." },
  { title: "At-risk endpoints", value: "Data unavailable", hint: "Risk requires findings; none exist yet." },
  { title: "Critical findings", value: "Data unavailable", hint: "A missing feed must never be read as zero critical findings." },
  { title: "Active monitoring", value: "Data unavailable", hint: "Monitoring status arrives from an attached agent collector." },
  { title: "Last scan", value: "Not available", hint: "No scan record exists in this build." }
] as const;

const endpointColumns: DataTableColumn<EndpointDevice>[] = [
  { id: "device", header: "Device", render: (row) => <span className="font-medium text-ink-primary">{row.hostname}</span> },
  { id: "os", header: "Operating system", render: (row) => row.operatingSystem },
  { id: "agentStatus", header: "Agent status", render: (row) => row.agentStatus },
  { id: "monitoringStatus", header: "Monitoring status", render: (row) => row.monitoringStatus },
  { id: "lastSeen", header: "Last seen", render: (row) => row.lastSeen ?? "Not reported" },
  {
    id: "riskLevel",
    header: "Risk level",
    render: (row) => row.riskLevel === "unknown" ? <span className="text-ink-muted">Unknown</span> : row.riskLevel
  },
  {
    id: "actions",
    header: "Actions",
    align: "right",
    render: () => <Button disabled disabledReason="Opening an endpoint requires a verified agent and policy-gated workflow.">Open</Button>
  }
];

const threatCategories = [
  { name: "Malware detection", description: "Signature and behavior-based verdicts from a verified scanner.", icon: <IconReports size={14} /> },
  { name: "Suspicious process behavior", description: "Deviation scoring over observed process behavior.", icon: <IconActivity size={14} /> },
  { name: "Unauthorized persistence", description: "Autorun, service, and startup-footprint anomalies.", icon: <IconShield size={14} /> },
  { name: "File integrity changes", description: "Hash drift over policy-protected paths.", icon: <IconBell size={14} /> },
  { name: "Suspicious network connections", description: "Outbound/inbound connection anomalies.", icon: <IconActivity size={14} /> },
  { name: "Privilege escalation indicators", description: "Token, integrity-level and credential-access signals.", icon: <IconIncident size={14} /> }
] as const;

const fimPolicies = [
  { name: "Protected paths", description: "Paths monitored for hash drift and content change.", reason: "Requires an authorized watch policy and approval flow." },
  { name: "Change history", description: "Hashed snapshots across change events.", reason: "Requires collector integration and audit storage." },
  { name: "Hash verification", description: "SHA-256/integrity validation on watch cycles.", reason: "Requires a verified integrity engine." },
  { name: "Algorithm policy", description: "Hash algorithm selection for verification.", reason: "Requires an approved policy bundle." }
] as const;

const responseActions = [
  { label: "Investigate endpoint", reason: "An investigation requires a verified endpoint finding; none exists." },
  { label: "Request scan", reason: "Scans execute only against a verified, onboarded agent." },
  { label: "Isolate endpoint", reason: "Containment affects real machines — requires evidence and explicit approval." },
  { label: "Release isolation", reason: "Release requires the same validated evidence + approval path." },
  { label: "View evidence", reason: "Evidence lives in the future evidence store, populated by a verified engine." }
] as const;

export default function EndpointSecurityPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const bridgeOk = Boolean(foundation || database);
  const readinessVariation = loading ? "warn" : bridgeOk ? "idle" : "idle";
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "Endpoint security not initialized";

  const devices: EndpointDevice[] = [];

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Endpoint Security"
          subtitle="Monitor endpoint protection readiness, device activity and security posture."
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error ? (
              <>Backend bridge unavailable — {error}. Endpoint status below reflects the static frontend only.</>
            ) : (
              <>
                Endpoint security is <strong className="font-semibold text-ink-primary">not initialized</strong>. This screen is the design surface for the future endpoint module —
                no process is enumerated, no file is scanned and no device is isolated. All actions stay disabled until a verified backend exists.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Endpoint protection status" description="Truthful readiness — protection is never claimed without a working engine." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {statusCards.map((c) => (
            <StatusCard key={c.title} title={c.title} value={c.value} variation="idle" hint={c.hint} icon={c.icon} />
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Endpoint summary" description="Inventory from verified agents only — nothing is enumerated locally." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {summaryPanels.map((c) => (
            <StatusCard key={c.title} title={c.title} value={c.value} variation="idle" hint={c.hint} />
          ))}
        </div>
      </section>

      <section>
        <SectionHeader title="Endpoint inventory" description="Registered endpoints from verified agents. Nothing is invented." action={<Badge tone="neutral">no agents onboarded</Badge>} />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <DataTable<EndpointDevice>
            columns={endpointColumns}
            rows={devices}
            rowKey={(row) => row.id}
            minWidth={900}
            caption="Endpoint inventory — verified agent records only"
            emptyState={
              <EmptyState
                icon={<IconEndpoint size={18} />}
                title="No endpoint devices registered. Endpoint agent integration is pending."
                description="Hostnames, operating systems, agent heartbeats, MAC addresses, IPs, last-seen timestamps and risk classifications are never invented. Rows populate exclusively from the verified agent collector."
              />
            }
          />
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Process monitoring" description="Prepared surface — no process is enumerated in this build." action={<Badge tone="neutral">no collector</Badge>} />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <EmptyState
              icon={<IconEndpoint size={18} />}
              title="Process visibility unavailable"
              description="Name, identifier, publisher, path, signature, network activity and reputation populate once the verified collector attaches — no process list is read from this machine. No user activity or credential data is collected."
            />
            <div className="mt-3 grid gap-2 sm:grid-cols-2">
              {[
                "Process name", "Process identifier", "Publisher", "File path",
                "Signature status", "Network activity", "Reputation", "Risk assessment"
              ].map((f) => (
                <div key={f} className="rounded-lg border border-charcoal-600 bg-charcoal-900/60 p-2.5">
                  <div className="text-[10px] uppercase tracking-wider text-ink-muted">{f}</div>
                  <div className="mt-0.5 text-[11px] text-ink-secondary">Awaiting collector — not available</div>
                </div>
              ))}
            </div>
          </div>
        </div>
        <div>
          <SectionHeader title="Endpoint findings" description="Future categories — none have detected anything." action={<Badge tone="neutral">no detections</Badge>} />
          <div className="grid gap-2.5">
            {threatCategories.map((c) => (
              <div key={c.name} className="flex items-center justify-between gap-3 rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3">
                <div className="flex items-center gap-2.5">
                  <span className="rounded-lg border border-charcoal-600 bg-charcoal-900 p-1.5 text-accent-cyan">{c.icon}</span>
                  <div>
                    <div className="text-xs font-semibold text-ink-primary">{c.name}</div>
                    <div className="text-[10px] text-ink-muted">{c.description}</div>
                  </div>
                </div>
                <Badge tone="warn">integration pending</Badge>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="File integrity monitoring" description="Planned policies — no hashing or watching runs in this build." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="grid gap-2.5">
              {fimPolicies.map((p) => (
                <div key={p.name} className="flex items-center justify-between gap-3 rounded-lg border border-charcoal-600 bg-charcoal-900/60 p-3">
                  <div>
                    <div className="text-xs font-semibold text-ink-primary">{p.name}</div>
                    <div className="text-[10px] text-ink-secondary">{p.description}</div>
                  </div>
                  <Button disabled disabledReason={p.reason}>{fimPolicies.indexOf(p) === 0 ? "Configure" : "Enable"}</Button>
                </div>
              ))}
            </div>
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">No file is hashed, no path is watched, no change history is written — the filesystem is never touched by this build.</p>
          </div>
        </div>
        <div>
          <SectionHeader title="Endpoint detail" description="Future per-device investigation view." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="mb-3 flex items-center justify-between">
              <span className="text-sm font-semibold text-ink-primary">Endpoint</span>
              <Badge tone="accent">awaiting selection</Badge>
            </div>
            <KeyValue label="Device identity" value="Not available" />
            <KeyValue label="Operating system" value="Not available" />
            <KeyValue label="Agent status" value="Not deployed" />
            <KeyValue label="Last seen" value="Not available" />
            <KeyValue label="Security posture" value="Not assessed" />
            <KeyValue label="Active findings" value="Not available" />
            <KeyValue label="Network connections" value="Not available" />
            <KeyValue label="Evidence references" value="Not available" />
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">Selecting an endpoint populates this panel from verified agent data only. No device information is ever invented.</p>
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Endpoint actions" description="Approval-gated workflow surface." />
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
                Endpoint containment requires validated evidence, security-policy validation (SecurityPolicy / HumanApprovalPolicy) and explicit human approval.
                This build never terminates processes, quarantines files or isolates devices.
              </span>
            </div>
          </div>
        </div>
        <div>
          <SectionHeader title="Endpoint activity" description="Future event timeline." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <ActivityTimeline
              items={[]}
              emptyState={
                <EmptyState
                  icon={<IconEndpoint size={18} />}
                  title="No endpoint events recorded"
                  description="Findings, scans and isolation events appear only after the agent and detection engine integrates. Nothing is simulated to fill this timeline."
                />
              }
            />
          </div>
        </div>
      </section>
    </PageContainer>
  );
}
