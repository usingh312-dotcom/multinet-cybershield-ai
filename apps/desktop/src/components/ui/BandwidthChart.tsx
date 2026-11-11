import { ResponsiveContainer, AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, Legend } from "recharts";
import type { BackendTrafficSample } from "../../api";
import { EmptyState } from "./feedback";
import { IconActivity } from "./icons";
import type { ReactNode } from "react";

type BandwidthChartProps = {
  data: BackendTrafficSample[];
  emptyState: ReactNode;
  height?: number;
};

/** Bits per second to megabits per second. */
const mbps = (bitsPerSecond: number | null) =>
  bitsPerSecond === null ? null : (bitsPerSecond * 8) / 1_000_000;

export function BandwidthChart({ data, emptyState, height = 320 }: BandwidthChartProps) {
  if (data.length === 0) {
    return <>{emptyState}</>;
  }

  // Boundary: the samples are backend measurements ordered by their own
  // timestamp. Nothing is smoothed, interpolated or synthesized here.
  const series = [...data]
    .sort((a, b) => a.sampledAt.localeCompare(b.sampledAt))
    .map((sample) => ({
      time: sample.sampledAt,
      downloadMbps: mbps(sample.rxRateBps),
      uploadMbps: mbps(sample.txRateBps)
    }));

  return (
    <div style={{ width: "100%", height }}>
      <ResponsiveContainer>
        <AreaChart data={series} margin={{ top: 8, right: 12, bottom: 0, left: 0 }}>
          <defs>
            <linearGradient id="dlFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#22C3DD" stopOpacity={0.35} />
              <stop offset="100%" stopColor="#22C3DD" stopOpacity={0.02} />
            </linearGradient>
            <linearGradient id="ulFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#6366F1" stopOpacity={0.30} />
              <stop offset="100%" stopColor="#6366F1" stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" stroke="#273449" vertical={false} />
          <XAxis dataKey="time" tick={{ fill: "#94A3B8", fontSize: 11 }} stroke="#273449" tickMargin={8} minTickGap={40} />
          <YAxis tick={{ fill: "#94A3B8", fontSize: 11 }} stroke="#273449" width={44} unit=" Mb/s" tickFormatter={(v: number) => String(v)} />
          <Tooltip
            contentStyle={{ background: "#151D28", border: "1px solid #273449", borderRadius: 12, color: "#F1F5F9", fontSize: 12 }}
            labelStyle={{ color: "#94A3B8" }}
          />
          <Legend wrapperStyle={{ color: "#94A3B8", fontSize: 12 }} />
          <Area type="monotone" dataKey="downloadMbps" name="Download" stroke="#22C3DD" strokeWidth={2} fill="url(#dlFill)" isAnimationActive={false} connectNulls={false} />
          <Area type="monotone" dataKey="uploadMbps" name="Upload" stroke="#6366F1" strokeWidth={2} fill="url(#ulFill)" isAnimationActive={false} connectNulls={false} />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

export function BandwidthChartEmpty() {
  return (
    <EmptyState
      icon={<IconActivity size={18} />}
      title="No traffic data to chart"
      description="Throughput history renders from the verified GetIfTable2 sampler feed only. No sample series, smoothing or synthetic spikes are displayed while the monitor is offline."
    />
  );
}
