import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { StatusCard, StatusIndicator, EmptyState } from "../components/ui/feedback";
import { Button } from "../components/ui/controls";
import { ActivityTimeline } from "../components/ui/widgets";
import { IconNetworkManager, IconActivity, IconAlert, IconShield, IconLock, IconBell } from "../components/ui/icons";
import { useAppStore } from "../store/appStore";
import { useSystemStore } from "../store/systemStore";

type Status = "ok" | "idle" | "warn" | "danger";

export default function OverviewPage() {
  const setActive = useAppStore((s) => s.setActive);
  const { foundation, database, error, loading } = useSystemStore();

  const bridgeOk = Boolean(foundation) || Boolean(database);
  const readinessVariation: Status = !bridgeOk ? (loading ? "warn" : "idle") : database ? "ok" : "warn";
  const readinessLabel = readinessVariation === "ok" ? "Foundation ready" : bridgeOk ? "Partial backend" : loading ? "Connecting\u2026" : "Backend unavailable";

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Network & Security Overview"
          subtitle="Your network and security at a glance"
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="flex items-center justify-between gap-3">
          <Button variant="primary" icon={<IconNetworkManager size={13} />} onClick={() => setActive("network-manager")}>
            Open Network Manager
          </Button>
          <span className="text-[11px] leading-4 text-ink-secondary">
            Inspect interfaces, set up bonding and manage adapters from Network Manager.
          </span>
        </div>
        {error && (
          <div className="mt-3 rounded-xl3 border border-charcoal-600 bg-surface px-4 py-3 text-xs leading-5 text-ink-secondary shadow-panel">
            Backend bridge unavailable \u2014 {error}. Network status stays honest until the platform collectors integrate.
          </div>
        )}
      </section>

      <section>
        <SectionHeader title="Network status" description="Honest, unverified states \u2014 no connectivity is simulated or implied." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <StatusCard title="Network connection" value="Not connected" sub="No interface selected" variation="idle" icon={<IconActivity size={14} />} hint="Ring/platform integration pending \u2014 this build performs no route changes." />
          <StatusCard title="Network interfaces" value="0 detected" sub="No adapters enumerated" variation="idle" icon={<IconShield size={14} />} hint="Discovery exists behind typed interfaces (mock only). No telemetry available." />
          <StatusCard title="Bonding session" value="Not active" sub="No session running" variation="idle" icon={<IconLock size={14} />} hint="Bonding engine is an interface-level stub; no sessions have started." />
          <StatusCard title="Internet latency" value="Not measured" sub="Probe when connected" variation="idle" icon={<IconBell size={14} />} hint="No probe sources are attached. Latency values are never simulated." />
        </div>
      </section>

      <section>
        <SectionHeader title="Security status" description="Text and color show state; nothing implies protection is active." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <StatusCard title="Firewall" value="Disabled" sub="Rules not loaded" variation="warn" icon={<IconShield size={14} />} hint="Protection rules load when the security engine integrates." />
          <StatusCard title="Threat detection" value="No provider" sub="Engine not connected" variation="idle" icon={<IconAlert size={14} />} hint="Detection feeds are mock-only. No validation has occurred." />
          <StatusCard title="ARP protection" value="Not running" sub="Guard inactive" variation="idle" icon={<IconLock size={14} />} hint="Local spoofing guard starts with the protection backend." />
          <StatusCard title="Last scan" value="Never" sub="No scan run" variation="idle" icon={<IconBell size={14} />} hint="Scan workflows arrive in later phases with verified data sources." />
        </div>
      </section>

      <section>
        <SectionHeader title="Network activity" description="Reserved for verified telemetry." />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
          <EmptyState
            icon={<IconActivity size={18} />}
            title="No telemetry yet"
            description="Network activity charts display real collector output after platform integration. No simulated traffic series are rendered at any time."
          />
        </div>
      </section>

      <section>
        <SectionHeader title="Security events" description="Real-time and historical event stream." />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
          <ActivityTimeline
            items={[]}
            emptyState={
              <EmptyState
                icon={<IconAlert size={18} />}
                title="No security events recorded"
                description="An empty timeline reflects that no event sources are connected. It is never interpreted as absence of threats."
              />
            }
          />
        </div>
      </section>
    </PageContainer>
  );
}
