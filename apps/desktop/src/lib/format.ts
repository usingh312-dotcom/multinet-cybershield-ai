/** Formatting helpers for real backend measurements. */

export function formatBitsPerSecond(bps: number | null | undefined): string {
  if (bps === null || bps === undefined || !Number.isFinite(bps)) return "—";
  const mbps = (bps * 8) / 1_000_000;
  if (mbps >= 1000) return `${(mbps / 1000).toFixed(2)} Gbps`;
  if (mbps >= 1) return `${mbps.toFixed(2)} Mbps`;
  return `${(mbps * 1000).toFixed(0)} Kbps`;
}

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes)) return "—";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(unit === 0 ? 0 : 2)} ${units[unit]}`;
}

export function formatMbps(mbps: number | null | undefined): string {
  if (mbps === null || mbps === undefined || !Number.isFinite(mbps)) return "—";
  return `${mbps.toFixed(2)} Mbps`;
}

export function formatMs(ms: number | null | undefined): string {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return "—";
  return `${ms.toFixed(2)} ms`;
}

export function formatPercent(pct: number | null | undefined): string {
  if (pct === null || pct === undefined || !Number.isFinite(pct)) return "—";
  return `${pct.toFixed(2)}%`;
}

/** Renders a backend RFC3339 timestamp as a local wall-clock time. */
export function formatTimestamp(timestamp: string | null | undefined): string {
  if (!timestamp) return "—";
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) return timestamp;
  return parsed.toLocaleString();
}

/** Renders a backend RFC3339 timestamp as a compact local clock time. */
export function formatClock(timestamp: string | null | undefined): string {
  if (!timestamp) return "—";
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) return timestamp;
  return parsed.toLocaleTimeString();
}

/** Human label for the Rust `InterfaceKind` serialization. */
export function interfaceKindLabel(kind: string): string {
  switch (kind) {
    case "ethernet":
      return "Ethernet";
    case "wifi":
      return "Wi-Fi";
    case "loopback":
      return "Loopback";
    case "virtual":
      return "Virtual";
    case "tunnel":
      return "Tunnel";
    case "ppp":
      return "PPP";
    default:
      return kind.charAt(0).toUpperCase() + kind.slice(1);
  }
}
