import { useEffect, useMemo, useState } from "react";
import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator, type StatusVariation } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue } from "../components/ui/controls";
import { DataTable, type DataTableColumn } from "../components/ui/DataTable";
import { useNativeRuntime, useSystemStore } from "../store/systemStore";
import { useDiagnosticsStore } from "../store/diagnosticsStore";
import type { BackendInterface } from "../api";
import {
  formatBitsPerSecond,
  formatBytes,
  formatTimestamp,
  interfaceKindLabel
} from "../lib/format";
import { IconNetworkManager, IconSpeed } from "../components/ui/icons";

type InterfaceRow = {
  interface: BackendInterface;
  rxBps: number | null;
  txBps: number | null;
};

const interfaceColumns: DataTableColumn<InterfaceRow>[] = [
  {
    id: "interface",
    header: "Interface",
    render: (row) => (
      <div>
        <div className="font-semibold text-ink-primary">{row.interface.name}</div>
        <div className="text-[11px] text-ink-muted">{row.interface.description}</div>
      </div>
    )
  },
  {
    id: "type",
    header: "Type",
    render: (row) => (
      <div className="flex flex-wrap items-center gap-1.5">
        <span className="text-ink-secondary">{interfaceKindLabel(row.interface.kind)}</span>
        <Badge tone={row.interface.isPhysical ? "ok" : "neutral"}>
          {row.interface.isPhysical ? "physical" : "virtual"}
        </Badge>
      </div>
    )
  },
  {
    id: "status",
    header: "Status",
    render: (row) => (
      <StatusIndicator
        variation={row.interface.isUp ? "ok" : "idle"}
        label={row.interface.isUp ? "Up" : "Down"}
      />
    )
  },
  {
    id: "addresses",
    header: "IPv4 / IPv6",
    render: (row) => (
      <div className="font-mono text-[11px] text-ink-secondary">
        {row.interface.ipv4Addresses.length > 0 ? (
          <div>{row.interface.ipv4Addresses.join(", ")}</div>
        ) : (
          <div className="text-ink-muted">no IPv4</div>
        )}
        {row.interface.ipv6Addresses.length > 0 && (
          <div className="text-ink-muted">{row.interface.ipv6Addresses.join(", ")}</div>
        )}
      </div>
    )
  },
  {
    id: "download",
    header: "Download",
    align: "right",
    render: (row) => <span className="font-mono">{formatBitsPerSecond(row.rxBps)}</span>
  },
  {
    id: "upload",
    header: "Upload",
    align: "right",
    render: (row) => <span className="font-mono">{formatBitsPerSecond(row.txBps)}</span>
  },
  {
    id: "index",
    header: "Index",
    align: "right",
    render: (row) => <span className="font-mono text-ink-muted">{row.interface.interfaceIndex}</span>
  }
];

export default function NetworkManagerPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const { isTauri: nativeRuntime, previewMessage } = useNativeRuntime();
  const {
    interfaces,
    interfacesState,
    interfacesError,
    refreshInterfaces,
    snapshot,
    refreshTraffic
  } = useDiagnosticsStore();
  const [selectedId, setSelectedId] = useState<string | null>(null);

  useEffect(() => {
    void refreshInterfaces();
    void refreshTraffic();
  }, [refreshInterfaces, refreshTraffic]);

  const trafficByInterface = useMemo(() => {
    const map = new Map<string, { rxBps: number | null; txBps: number | null }>();
    for (const entry of snapshot?.interfaces ?? []) {
      map.set(entry.interfaceId, { rxBps: entry.rxBps, txBps: entry.txBps });
    }
    return map;
  }, [snapshot]);

  const rows = useMemo<InterfaceRow[]>(
    () =>
      interfaces.map((iface) => ({
        interface: iface,
        rxBps: trafficByInterface.get(iface.id)?.rxBps ?? null,
        txBps: trafficByInterface.get(iface.id)?.txBps ?? null
      })),
    [interfaces, trafficByInterface]
  );

  const selected = useMemo(
    () => interfaces.find((iface) => iface.id === selectedId) ?? null,
    [interfaces, selectedId]
  );

  const bridgeOk = Boolean(foundation || database);
  const readinessVariation: StatusVariation = !bridgeOk
    ? loading
      ? "warn"
      : "idle"
    : database
      ? "ok"
      : "warn";
  const readinessLabel = readinessVariation === "ok"
    ? "Platform ready for integration"
    : bridgeOk
      ? "Backend reachable"
      : loading
        ? "Connecting…"
        : "Backend unavailable";

  const physicalCount = interfaces.filter((iface) => iface.isPhysical).length;
  const upCount = interfaces.filter((iface) => iface.isUp).length;

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Network Manager"
          subtitle="Manage and inspect your network connections."
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            {!nativeRuntime ? (
              <>{previewMessage}</>
            ) : error ? (
              <>Backend bridge unavailable — {error}. No adapter data is read from the OS and the screen never invents values.</>
            ) : interfacesError ? (
              <>
                Native command <code className="font-mono">{interfacesError.command}</code> failed — {interfacesError.reason}.{" "}
                No interface record is fabricated.
              </>
            ) : (
              <>
                Interface inventory is read live from the Windows IP Helper API via the Rust{" "}
                <code className="font-mono">list_network_interfaces</code> command. Names, addresses and
                classifications come from the operating system — nothing on this screen is invented.
              </>
            )}
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <SectionHeader title="Connection summary" description="Truthful readiness states only." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <StatusCard
            title="Available Interfaces"
            value={interfacesState === "ready" ? String(interfaces.length) : interfacesState === "loading" ? "Loading…" : "Unavailable"}
            sub={interfacesState === "ready" ? `${physicalCount} physical` : undefined}
            variation={interfacesState === "ready" ? "ok" : "idle"}
            hint="Counted from the live operating-system inventory."
          />
          <StatusCard
            title="Active Connections"
            value={interfacesState === "ready" ? String(upCount) : "Unavailable"}
            variation={interfacesState === "ready" ? "ok" : "idle"}
            hint="Adapters the operating system currently reports as up."
          />
          <StatusCard
            title="Default Network Route"
            value={interfacesState === "ready" ? "Not reported" : "Unavailable"}
            variation="idle"
            hint="Route inspection is not implemented by the diagnostics backend yet."
          />
          <StatusCard
            title="Network Monitoring Status"
            value={snapshot ? (snapshot.monitorActive ? "Running" : "Stopped") : "Unavailable"}
            variation={snapshot?.monitorActive ? "ok" : "idle"}
            hint="The GetIfTable2 sampler writes one sample per live interface every second."
          />
        </div>
      </section>

      <section>
        <SectionHeader
          title="Network interfaces"
          description="Live inventory reported by the operating system."
          action={
            <div className="flex items-center gap-2">
              <Button
                onClick={() => void refreshInterfaces()}
                disabled={interfacesState === "loading" || !nativeRuntime}
                disabledReason={
                  !nativeRuntime
                    ? "The native bridge is unavailable in a browser preview."
                    : "Refreshing the interface inventory."
                }
              >
                Refresh interfaces
              </Button>
            </div>
          }
        />
        <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <DataTable
            columns={interfaceColumns}
            rows={rows}
            rowKey={(row) => row.interface.id}
            loading={interfacesState === "loading"}
            loadingState={
              <div className="px-4 py-6 text-center text-xs text-ink-secondary">
                Reading the operating-system interface inventory…
              </div>
            }
            minWidth={860}
            caption="Network interfaces — discovered adapters only"
            emptyState={
              <EmptyState
                icon={<IconNetworkManager size={18} />}
                title="No network interfaces available"
                description={
                  interfacesState === "error"
                    ? "The operating system reported no usable adapter, or the inventory command failed. No adapter record is ever fabricated."
                    : "The operating system reported no network adapters."
                }
              />
            }
          />
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader
            title="Connection details"
            description="Per-interface inspector backed by the live inventory."
          />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <div className="mb-3 flex items-center justify-between">
              <span className="text-sm font-semibold text-ink-primary">Interface identifier</span>
              {selected ? (
                <Badge tone="accent">{selected.name}</Badge>
              ) : (
                <Badge tone="neutral">awaiting selection</Badge>
              )}
            </div>
            {selected ? (
              <>
                <KeyValue label="Interface type" value={interfaceKindLabel(selected.kind)} />
                <KeyValue
                  label="Physical classification"
                  value={selected.isPhysical ? "Physical adapter" : "Virtual adapter"}
                />
                <KeyValue label="Connection state" value={selected.isUp ? "Up" : "Down"} />
                <KeyValue
                  label="IP configuration"
                  value={selected.ipv4Addresses.join(", ") || "No IPv4 address"}
                  mono
                />
                <KeyValue
                  label="IPv6 configuration"
                  value={selected.ipv6Addresses.join(", ") || "No IPv6 address"}
                  mono
                />
                <KeyValue label="MAC address" value={selected.macAddress ?? "Not reported"} mono />
                <KeyValue label="MTU" value={selected.mtu ?? "Not reported"} mono />
                <KeyValue
                  label="Link speed"
                  value={selected.linkSpeedBps ? formatBitsPerSecond(selected.linkSpeedBps) : "Not reported"}
                />
                <KeyValue label="Interface index" value={selected.interfaceIndex} mono />
              </>
            ) : (
              <>
                <KeyValue label="Interface type" value="Not determined" />
                <KeyValue label="Connection state" value="Not connected" />
                <KeyValue label="IP configuration" value="Not selected" />
                <KeyValue label="DNS configuration" value="Not reported" />
                <KeyValue label="Gateway" value="Not reported" />
                <KeyValue label="Interface health" value="Not monitored" />
                <p className="mt-3 text-[10px] leading-4 text-ink-muted">
                  Select a row in the inventory above to populate this panel from live platform data only.
                </p>
              </>
            )}
          </div>
          {interfaces.length > 0 && (
            <div className="mt-3 flex flex-wrap items-center gap-2">
              <span className="text-[10px] uppercase tracking-wider text-ink-muted">Select</span>
              {interfaces.map((iface) => (
                <button
                  key={iface.id}
                  type="button"
                  onClick={() => setSelectedId(iface.id === selectedId ? null : iface.id)}
                  className="rounded-md border border-charcoal-600 bg-charcoal-900 px-2 py-1 text-xs text-ink-secondary hover:border-cyan-400/40 hover:text-ink-primary"
                >
                  {iface.name}
                </button>
              ))}
            </div>
          )}
        </div>
        <div>
          <SectionHeader title="Network diagnostics" description="Live counters reported by the sampler." />
          <div className="grid gap-3">
            {snapshot && snapshot.interfaces.length > 0 ? (
              snapshot.interfaces.map((entry) => (
                <div
                  key={entry.interfaceId}
                  className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="text-xs font-semibold text-ink-primary">{entry.name}</div>
                      <div className="text-[11px] text-ink-secondary">
                        Down {formatBitsPerSecond(entry.rxBps)} · Up {formatBitsPerSecond(entry.txBps)}
                      </div>
                    </div>
                    <span className="font-mono text-[10px] text-ink-muted">
                      {formatTimestamp(entry.counterReset ? "counter reset" : entry.interfaceId)}
                    </span>
                  </div>
                  <div className="mt-2 grid grid-cols-2 gap-2 text-[11px] text-ink-muted">
                    <span>Session RX {formatBytes(entry.sessionRxBytes)}</span>
                    <span>Session TX {formatBytes(entry.sessionTxBytes)}</span>
                    <span>Total RX {formatBytes(entry.totalRxBytes)}</span>
                    <span>Total TX {formatBytes(entry.totalTxBytes)}</span>
                  </div>
                </div>
              ))
            ) : (
              <EmptyState
                icon={<IconSpeed size={18} />}
                title="No live counters yet"
                description="The GetIfTable2 sampler has not produced a per-interface sample yet, or the monitor is stopped. No rate is estimated."
              />
            )}
          </div>
        </div>
      </section>
    </PageContainer>
  );
}
