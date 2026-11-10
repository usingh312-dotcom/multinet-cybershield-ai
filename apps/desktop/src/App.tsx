import { Suspense, lazy, useEffect } from "react";
import { useSystemStore } from "./store/systemStore";
import { type NavId, navItems, useAppStore } from "./store/appStore";
import { AppLayout } from "./components/AppLayout";
import { PageContainer, PageHeader } from "./components/ui/structure";
import { EmptyState } from "./components/ui/feedback";
import { IconDatabase } from "./components/ui/icons";
import OverviewPage from "./pages/Overview";

// Route-level dynamic imports: each heavy or infrequently-used screen loads on
// first navigation.Overview stays part of the shell; Recharts travels only with
// the TrafficAnalytics chunk.
const NetworkManagerPage = lazy(() => import("./pages/NetworkManager"));
const InternetBondingPage = lazy(() => import("./pages/InternetBonding"));
const TrafficAnalyticsPage = lazy(() => import("./pages/TrafficAnalytics"));
const SpeedTestPage = lazy(() => import("./pages/SpeedTest"));
const BenchmarkLabPage = lazy(() => import("./pages/BenchmarkLab"));
const IncidentResponsePage = lazy(() => import("./pages/IncidentResponse"));
const SettingsPage = lazy(() => import("./pages/Settings"));
const SecurityOverviewPage = lazy(() => import("./pages/SecurityOverview"));
const ThreatDetectionPage = lazy(() => import("./pages/ThreatDetection"));
const ThreatIntelligencePage = lazy(() => import("./pages/ThreatIntelligence"));
const EndpointSecurityPage = lazy(() => import("./pages/EndpointSecurity"));
const DnsProtectionPage = lazy(() => import("./pages/DnsProtection"));
const AiFirewallPage = lazy(() => import("./pages/AiFirewall"));
const SecurityReportsPage = lazy(() => import("./pages/SecurityReports"));
const AiCopilotPage = lazy(() => import("./pages/AiCopilot"));

function PageFallback() {
  return (
    <PageContainer>
      <div role="status" aria-live="polite" className="animate-pulse space-y-4" aria-busy="true" aria-label="Loading screen">
        <div className="h-7 w-64 rounded-lg bg-charcoal-700/50" />
        <div className="h-4 w-96 rounded bg-charcoal-700/40" />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="h-28 rounded-xl3 border border-charcoal-600/60 bg-elevated/60 shadow-panel" />
          ))}
        </div>
        <div className="h-64 rounded-xl2 border border-charcoal-600/60 bg-elevated/60 shadow-panel" />
      </div>
    </PageContainer>
  );
}

export default function App() {
  const { active } = useAppStore();
  const refresh = useSystemStore((s) => s.refresh);

  useEffect(() => { void refresh(); }, [refresh]);

  return (
    <AppLayout>
      <Suspense fallback={<PageFallback />}>
        {active === "overview" ? <OverviewPage />
          : active === "network-manager" ? <NetworkManagerPage />
          : active === "internet-bonding" ? <InternetBondingPage />
          : active === "traffic-analytics" ? <TrafficAnalyticsPage />
          : active === "speed-test" ? <SpeedTestPage />
          : active === "benchmark-lab" ? <BenchmarkLabPage />
          : active === "incident-response" ? <IncidentResponsePage />
          : active === "settings" ? <SettingsPage />
          : active === "security-overview" ? <SecurityOverviewPage />
          : active === "threat-detection" ? <ThreatDetectionPage />
          : active === "threat-intelligence" ? <ThreatIntelligencePage />
          : active === "endpoint-security" ? <EndpointSecurityPage />
          : active === "dns-protection" ? <DnsProtectionPage />
          : active === "ai-firewall" ? <AiFirewallPage />
          : active === "security-reports" ? <SecurityReportsPage />
          : active === "ai-copilot" ? <AiCopilotPage />
          : <PlaceholderPage active={active} />}
      </Suspense>
    </AppLayout>
  );
}

function PlaceholderPage({ active }: { active: NavId }) {
  const item = navItems.find((n) => n.id === active);
  return (
    <PageContainer>
      <PageHeader
        title={item?.label ?? "Screen"}
        subtitle="Phase 1 placeholder — functional implementation is scheduled for Phase 2+ with verified platform integrations."
      />
      <div className="pt-6">
        <EmptyState
          icon={<IconDatabase size={18} />}
          title="Implementation pending"
          description="This screen is Phase 1 navigation scaffolding. Scope, data contracts and platform integrations are defined in later phases. No placeholder metrics are displayed."
        />
      </div>
    </PageContainer>
  );
}
