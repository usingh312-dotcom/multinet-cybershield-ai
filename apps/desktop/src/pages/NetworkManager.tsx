import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator, type StatusVariation } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue } from "../components/ui/controls";
import { DataTable, type DataTableColumn } from "../components/ui/DataTable";
import { useSystemStore } from "../store/systemStore";
import {
  IconBonding, IconEndpoint, IconNetworkManager, IconSpeed
} from "../components/ui/icons";

const interfaceCategories = [
  { name: "Wi-Fi", description: "Wireless LAN adapters", icon: <IconSpeed size={15} /> },
  { name: "Ethernet", description: "Wired adapters", icon: <IconNetworkManager size={15} /> },
  { name: "USB tethering", description: "Tethered phone connections", icon: <IconBonding size={15} /> },
  { name: "USB Wi-Fi", description: "External wireless dongles", icon: <IconEndpoint size={15} /> }
] as const;

const diagnostics = [
  { name: "Connectivity checks", description: "Reachability probes behind the user's consent" },
  { name: "Latency tests", description: "Round-trip measurements from real collectors" },
  { name: "Packet loss", description: "Loss estimation with controlled intervals" },
  { name: "DNS resolution checks", description: "Resolver correctness inspection" },
  { name: "Route diagnostics", description: "Default-route path inspection" }
] as const;

type SystemReadinessBadgeProps = {
  variation: StatusVariation;
  label: string;
};

function DisplayedReadiness({ variation, label }: SystemReadinessBadgeProps) {
  return <StatusIndicator variation={variation} label={label} />;
}

// Typed for the future interface-record contract; no rows exist until verified collectors arrive.
const interfaceColumns: DataTableColumn<never>[] = [
  { id: "interface", header: "Interface" },
  { id: "type", header: "Type" },
  { id: "status", header: "Status" },
  { id: "addresses", header: "IPv4 / IPv6" },
  { id: "upload", header: "Upload" },
  { id: "download", header: "Download" },
  { id: "latency", header: "Latency" },
  { id: "actions", header: "Actions", align: "right" }
];

export default function NetworkManagerPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const bridgeOk = Boolean(foundation || database);
  const readinessVariation: StatusVariation = !bridgeOk ? (loading ? "warn" : "idle") : database ? "ok" : "warn";
  const readinessLabel = readinessVariation === "ok" ? "Platform ready for integration" : bridgeOk ? "Backend reachable" : loading ? "Connecting…" : "Backend unavailable";

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Network Manager"
          subtitle="Manage and inspect your network connections."
          actions={<DisplayedReadiness variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {error
              ? <>Backend bridge unavailable — {error}. Interface discovery has not been initialized in this build; no adapter data is read from the OS and the screen never invents values.</>
              : <>Interface discovery is <strong className="font-semibold text-ink-primary">not initialized</strong> in Phase 2. Network Manager is UI-complete: adapter inventory, detail panels, diagnostics and actions are designed but bound to no live collectors — no OS network state is read or modified.</>}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Connection summary" description="Truthful readiness states only." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <StatusCard title="Available Interfaces" value="Discovery not initialized" variation="idle" hint="Enumeration requires the platform integration; reported counts arrive only from verified collectors." />
          <StatusCard title="Active Connections" value="No telemetry available" variation="idle" hint="Connection tracking is not yet wired to the OS." />
          <StatusCard title="Default Network Route" value="Interface data unavailable" variation="idle" hint="Route inspection is interface-level in Phase 1 mock code; no live route is queried." />
          <StatusCard title="Network Monitoring Status" value="Platform integration pending" variation="idle" hint="Collectors will be attached in a later phase with validated data." />
        </div>
      </section>

      <section>
        <SectionHeader
          title="Network interfaces"
          description="Inventory with categories the platform will report. No devices have been detected."
          action={<ActionsCluster />}
        />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <DataTable
            columns={interfaceColumns}
            rows={[]}
            rowKey={(_, index) => `interface-${index}`}
            minWidth={720}
            caption="Network interfaces — discovered adapters only"
            emptyState={
              <EmptyState
                icon={<IconNetworkManager size={18} />}
                title="No network interfaces available"
                description="System interface discovery is not connected to this build. The columns above become populated only from real adapter enumeration — names, addresses and counts are never fabricated."
              />
            }
          />
          <div className="flex flex-wrap items-center gap-3 border-t border-charcoal-600 px-4 py-3">
            <span className="text-[10px] uppercase tracking-wider text-ink-muted">Expected categories</span>
            {interfaceCategories.map((c) => (
              <span key={c.name} className="inline-flex items-center gap-1.5 rounded-md border border-charcoal-600 bg-charcoal-900 px-2 py-1 text-xs text-ink-secondary">
                {c.icon}
                {c.name}
                <Badge tone="neutral">not detected</Badge>
              </span>
            ))}
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Connection details" description="Per-interface inspector template." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="mb-3 flex items-center justify-between">
              <span className="text-sm font-semibold text-ink-primary">Interface identifier</span>
              <Badge tone="accent">awaiting selection</Badge>
            </div>
            <KeyValue label="Interface type" value="Not determined" />
            <KeyValue label="Connection state" value="Not connected" />
            <KeyValue label="IP configuration" value="Integration pending" />
            <KeyValue label="DNS configuration" value="Not reported" />
            <KeyValue label="Gateway" value="Not reported" />
            <KeyValue label="Interface health" value="Not monitored" />
            <p className="mt-3 text-[10px] leading-4 text-ink-muted">
              Selecting an adapter populates this panel from live platform data only. Placeholder values are never shown for real-looking addresses.
            </p>
          </div>
        </div>
        <div>
          <SectionHeader title="Network diagnostics" description="Designed test surface; nothing executes in this phase." />
          <div className="grid gap-3">
            {diagnostics.map((d) => (
              <div key={d.name} className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="text-xs font-semibold text-ink-primary">{d.name}</div>
                    <div className="text-[11px] text-ink-secondary">{d.description}</div>
                  </div>
                  <Button disabled disabledReason="Requires platform integration. No diagnostic traffic is produced in this build.">
                    Run
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>
    </PageContainer>
  );
}

function ActionsCluster() {
  return (
    <div className="flex items-center gap-2">
      <Button disabled disabledReason="Interface discovery is not connected; there is nothing to refresh yet.">Refresh interfaces</Button>
      <Button disabled disabledReason="Select an interface once discovery is available.">View details</Button>
      <Button disabled disabledReason="Diagnostic execution requires backend integration (disabled by design).">Diagnostics</Button>
    </div>
  );
}
