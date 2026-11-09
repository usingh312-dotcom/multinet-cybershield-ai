import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator, type StatusVariation } from "../components/ui/feedback";
import { ActivityTimeline } from "../components/ui/widgets";
import { Badge, IntegrationNotice } from "../components/ui/controls";
import { useAppStore, type NavId } from "../store/appStore";
import { useSystemStore } from "../store/systemStore";
import {
  IconActivity, IconAlert, IconArrowRight, IconBonding, IconCopilot, IconDns,
  IconEndpoint, IconFirewall, IconIncident, IconShield, IconThreatDetection, IconThreatIntel
} from "../components/ui/icons";

const summaryCards: Array<{ title: string; value: string; hint: string; icon: React.ReactNode }> = [
  {
    title: "Security Engine",
    value: "Not initialized",
    hint: "SecurityEngine exists as typed interfaces; no monitor is attached and no engine loop runs.",
    icon: <IconShield size={14} />
  },
  {
    title: "Threat Monitoring",
    value: "Not monitored",
    hint: "Detection feeds are interface-level stubs. No validation has occurred.",
    icon: <IconThreatDetection size={14} />
  },
  {
    title: "Endpoint Monitoring",
    value: "Not integrated",
    hint: "Endpoint collectors are not wired to this build; no process or device data is read.",
    icon: <IconEndpoint size={14} />
  },
  {
    title: "DNS Protection",
    value: "Not configured",
    hint: "No resolver policy is enforced; DNS settings are never modified by this build.",
    icon: <IconDns size={14} />
  },
  {
    title: "Firewall Management",
    value: "Integration pending",
    hint: "The firewall manager is designed only; no rule is created, changed or enforced.",
    icon: <IconFirewall size={14} />
  },
  {
    title: "Threat Intelligence",
    value: "No security telemetry",
    hint: "Intelligence feeds arrive in later phases; nothing is ingested or inferred meanwhile.",
    icon: <IconThreatIntel size={14} />
  }
];

const coverageModules: Array<{ name: string; badge: "Planned" | "Integration pending" | "Not initialized"; tone: "warn" | "neutral"; description: string; icon: React.ReactNode }> = [
  {
    name: "Network security",
    badge: "Planned",
    tone: "neutral",
    description: "Traffic inspection attaches to the verified TrafficMetrics collector once platform integration exists.",
    icon: <IconBonding size={14} />
  },
  {
    name: "Endpoint security",
    badge: "Integration pending",
    tone: "warn",
    description: "Process and device collectors are specified; no endpoint state is read in this build.",
    icon: <IconEndpoint size={14} />
  },
  {
    name: "DNS security",
    badge: "Not initialized",
    tone: "neutral",
    description: "Resolver hardening requires the DNS protection module; no changes are applied to the system today.",
    icon: <IconDns size={14} />
  },
  {
    name: "Firewall protection",
    badge: "Planned",
    tone: "neutral",
    description: "Rule authoring and enforcement arrive with the firewall manager; nothing is enforced meanwhile.",
    icon: <IconFirewall size={14} />
  },
  {
    name: "Threat intelligence",
    badge: "Integration pending",
    tone: "warn",
    description: "Indicator feeds and matching engines are future work; no external feed is contacted.",
    icon: <IconThreatIntel size={14} />
  },
  {
    name: "Incident response",
    badge: "Planned",
    tone: "neutral",
    description: "Incident lifecycle is designed around HumanApprovalPolicy; no response workflow is active.",
    icon: <IconIncident size={14} />
  }
];

const architectureModules = [
  { name: "Intrusion Detection", description: "Network anomaly detectors over verified traffic", icon: <IconActivity size={14} /> },
  { name: "Endpoint Monitoring", description: "Process and device telemetry collectors", icon: <IconEndpoint size={14} /> },
  { name: "Threat Intelligence", description: "Indicator matching against curated feeds", icon: <IconThreatIntel size={14} /> },
  { name: "DNS Protection", description: "Resolver policy and domain classification", icon: <IconDns size={14} /> },
  { name: "Firewall Manager", description: "Policy-checked rule authoring and enforcement", icon: <IconFirewall size={14} /> },
  { name: "Incident Response", description: "Approval-gated investigation and response", icon: <IconIncident size={14} /> }
] as const;

const quickLinks: Array<{ id: NavId; label: string; description: string; icon: React.ReactNode }> = [
  { id: "threat-detection", label: "Threat Detection", description: "Security event workspace", icon: <IconThreatDetection size={15} /> },
  { id: "threat-intelligence", label: "Threat Intelligence", description: "Indicator feeds (future)", icon: <IconThreatIntel size={15} /> },
  { id: "endpoint-security", label: "Endpoint Security", description: "Process & device collectors", icon: <IconEndpoint size={15} /> },
  { id: "dns-protection", label: "DNS Protection", description: "Resolver policy module", icon: <IconDns size={15} /> },
  { id: "ai-firewall", label: "AI Firewall", description: "Policy-gated rule engine", icon: <IconFirewall size={15} /> },
  { id: "incident-response", label: "Incident Response", description: "Investigation lifecycle", icon: <IconIncident size={15} /> }
];

export default function SecurityOverviewPage() {
  const setActive = useAppStore((s) => s.setActive);
  const { foundation, database, error, loading } = useSystemStore();

  const engineOk = Boolean(foundation);
  const databaseOk = Boolean(database);
  const securityVariation: StatusVariation = loading ? "warn" : "idle";
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "Security engine not initialized";

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Security Overview"
          subtitle="Monitor security engine readiness, protection coverage and security events."
          actions={<StatusIndicator variation={securityVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error ? (
              <>Backend bridge unavailable — {error}. Security readiness below reflects the static frontend state only.</>
            ) : (
              <>
                Application foundation {engineOk ? <strong className="font-semibold text-emerald-300">initialized</strong> : <strong className="font-semibold text-ink-primary">not reached</strong>} ·
                {" "}Database {databaseOk ? <strong className="font-semibold text-emerald-300">initialized</strong> : <strong className="font-semibold text-ink-primary">not reached</strong>} ·
                {" "}Security engine <strong className="font-semibold text-ink-primary">not initialized</strong>. A completed database migration means persistence is ready —
                {" "}<strong className="font-semibold text-amber-300">it does not mean security protection is active</strong>.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Security summary" description="Truthful engine states only — no secure status is claimed without a validated implementation." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {summaryCards.map((c) => (
            <StatusCard key={c.title} title={c.title} value={c.value} variation="idle" hint={c.hint} icon={c.icon} />
          ))}
        </div>
      </section>

      <section>
        <SectionHeader
          title="Protection coverage"
          description="Planned and pending modules. No coverage scores or percentages are invented."
          action={<Badge tone="warn">posture unmeasured</Badge>}
        />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {coverageModules.map((m) => (
            <div key={m.name} className="rounded-xl3 border border-charcoal-600 bg-elevated shadow-panel p-3.5">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-xs font-semibold text-ink-primary">
                  <span className="rounded-lg border border-charcoal-600 bg-charcoal-900 p-1.5 text-accent-cyan">{m.icon}</span>
                  {m.name}
                </div>
                <Badge tone={m.tone}>{m.badge}</Badge>
              </div>
              <div className="mt-2.5 flex gap-1" aria-hidden="true">
                {Array.from({ length: 12 }).map((_, i) => (
                  <span key={i} className="h-1.5 flex-1 rounded-sm bg-charcoal-700/60" />
                ))}
              </div>
              <div className="mt-1 text-[10px] uppercase tracking-wider text-ink-muted">coverage not measured</div>
              <p className="mt-1.5 text-[11px] leading-4 text-ink-secondary">{m.description}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Recent security activity" description="Real-time and historical event stream." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <ActivityTimeline
              items={[]}
              emptyState={
                <EmptyState
                  icon={<IconAlert size={18} />}
                  title="No security events recorded"
                  description="No event sources are connected to this build. An empty timeline reflects that fact — it is never interpreted as absence of threats and never filled with sample incidents."
                />
              }
            />
          </div>
        </div>
        <div>
          <SectionHeader
            title="Security engine architecture"
            description="Conceptual data flow between the engine and its defense modules."
            action={<Badge tone="warn">architecture preview — not operational</Badge>}
          />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="rounded-xl2 border border-accent-blue/25 bg-charcoal-900/60 p-4 text-center">
              <div className="mx-auto flex w-fit items-center gap-2 rounded-lg border border-accent-blue/30 bg-accent-blue/10 px-3 py-2">
                <span className="text-accent-cyan"><IconShield size={15} /></span>
                <span className="text-sm font-semibold text-ink-primary">CyberShield Security Engine</span>
                <Badge tone="neutral">interfaces only</Badge>
              </div>
              <p className="mt-2 text-[11px] leading-4 text-ink-secondary">
                SecurityEngine, SecurityPolicy and AuditLogger are typed Phase 1 interfaces. Detection, endpoint and
                enforcement modules below are planned components — none of them runs in this build.
              </p>
            </div>
            <div className="my-3 text-center text-ink-muted" aria-hidden="true">↓</div>
            <div className="grid gap-2 sm:grid-cols-2">
              {architectureModules.map((m) => (
                <div key={m.name} className="rounded-lg border border-charcoal-600 bg-charcoal-900/60 p-2.5">
                  <div className="flex items-center gap-2 text-[11px] font-semibold text-ink-secondary">
                    <span className="text-ink-muted">{m.icon}</span>
                    {m.name}
                    <Badge tone="neutral">planned</Badge>
                  </div>
                  <div className="mt-0.5 text-[10px] leading-4 text-ink-muted">{m.description}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      <section>
        <SectionHeader
          title="Quick access"
          description="Navigate to security modules."
          action={
            <button
              type="button"
              onClick={() => setActive("threat-detection")}
              title="Open Threat Detection"
              aria-label="Open Threat Detection"
              className="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-charcoal-600 bg-charcoal-800 text-ink-secondary transition hover:bg-charcoal-750 hover:text-ink-primary"
            >
              <IconArrowRight size={14} />
            </button>
          }
        />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {quickLinks.map((link) => (
            <button
              key={link.id}
              type="button"
              onClick={() => setActive(link.id)}
              className="group flex items-start gap-3 rounded-xl2 border border-charcoal-600 bg-elevated p-3 text-left shadow-panel transition hover:border-accent-blue/40 hover:bg-charcoal-750"
            >
              <span className="mt-0.5 rounded-lg border border-charcoal-600 bg-charcoal-800 p-1.5 text-accent-cyan">{link.icon}</span>
              <span>
                <span className="block text-xs font-semibold text-ink-primary">{link.label}</span>
                <span className="mt-0.5 block text-[11px] leading-4 text-ink-secondary">{link.description}</span>
              </span>
            </button>
          ))}
        </div>
      </section>
    </PageContainer>
  );
}
