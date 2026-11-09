import type { ConnectionQuality } from "../../types/telemetry";
import { EmptyState, StatusCard, type StatusVariation } from "./feedback";
import { IconThreatIntel } from "./icons";
import { DataTable, type DataTableColumn } from "./DataTable";
import type { ReactNode } from "react";

type TableRow = { cells: ReactNode[] };

type InterfaceUsageTableProps = {
  rows: TableRow[];
  headers: string[];
  emptyState: ReactNode;
};

export function InterfaceUsageTable({ rows, headers, emptyState }: InterfaceUsageTableProps) {
  const columns: DataTableColumn<TableRow>[] = headers.map((header, index) => ({
    id: `col-${index}`,
    header,
    render: (row) => row.cells[index]
  }));
  return (
    <DataTable
      columns={columns}
      rows={rows}
      rowKey={(_, index) => `usage-${index}`}
      minWidth={640}
      caption="Interface usage totals — verified collectors only"
      emptyState={emptyState}
    />
  );
}

const protocolSlots = [
  { key: "tcp", label: "TCP" },
  { key: "udp", label: "UDP" },
  { key: "icmp", label: "ICMP" },
  { key: "dns", label: "DNS" },
  { key: "other", label: "Other traffic" }
] as const;

export function ProtocolDistribution() {
  return (
    <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
      <EmptyState
        icon={<IconThreatIntel size={18} />}
        title="Traffic classification not available"
        description="Protocol distribution (TCP/UDP/ICMP/DNS/other) renders from verified classification collectors only. No share of traffic is estimated or inferred."
      />
      <div className="mt-4 grid gap-2 sm:grid-cols-5">
        {protocolSlots.map((p) => (
          <div key={p.key} className="rounded-lg border border-charcoal-600 bg-charcoal-900 px-3 py-2 text-center">
            <div className="text-[11px] font-semibold text-ink-secondary">{p.label}</div>
            <div className="mt-0.5 text-[10px] text-ink-muted">no data</div>
          </div>
        ))}
      </div>
    </div>
  );
}

type NetworkQualityCardProps = {
  quality: ConnectionQuality | null;
};

export function NetworkQualityCards({ quality }: NetworkQualityCardProps) {
  void quality; // real ConnectionQuality arrives from verified collectors only.
  const specs: Array<{ label: string; value: string; variation: StatusVariation; hint: string }> = [
    { label: "Latency", value: "Not measured", variation: "idle", hint: "Round-trip latency appears once an approved probe source exists." },
    { label: "Jitter", value: "Not measured", variation: "idle", hint: "Variation analysis requires stable sampling from real collectors." },
    { label: "Packet loss", value: "Not available", variation: "idle", hint: "Loss estimation is never extrapolated in this build." },
    { label: "Connection stability", value: "Not assessed", variation: "idle", hint: "Stability scores aggregate verified samples over time." }
  ];
  return (
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      {specs.map((s) => (
        <StatusCard key={s.label} title={s.label} value={s.value} variation={s.variation} hint={s.hint} />
      ))}
    </div>
  );
}

export function AnalyticsPlaceholder({ title, description, icon }: { title: string; description: string; icon: ReactNode }) {
  return (
    <EmptyState
      icon={icon}
      title={title}
      description={description}
    />
  );
}
