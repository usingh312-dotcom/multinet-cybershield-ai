import { invoke, isTauri } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Native bridge
//
// The application runs in two very different hosts:
//
//   1. The Tauri desktop runtime (WebView2) — the real product. Native
//      commands are reachable through the imported `invoke`.
//   2. A plain browser pointed at the Vite dev server — a developer preview.
//      There is no IPC channel here, so every native command must be refused
//      *before* it is dispatched.
//
// `isTauri()` is the supported Tauri 2 runtime probe. The project does not
// enable `withGlobalTauri`, so `window.__TAURI__` is intentionally absent and
// must never be used.
// ---------------------------------------------------------------------------

export type NativeRuntime = {
  /** True when the frontend is hosted inside the Tauri desktop runtime. */
  isTauri: boolean;
  /** Human readable explanation shown in browser-preview mode. */
  previewMessage: string;
};

export const BROWSER_PREVIEW_MESSAGE =
  "Browser preview: the native MultiNet bridge is unavailable, so live network, traffic and speed-test data cannot be read. Run the desktop application (npm run tauri:dev) to query the real Rust backend.";

/**
 * Probe the host runtime. Safe to call repeatedly and outside React.
 */
export function detectNativeRuntime(): NativeRuntime {
  let native = false;
  try {
    native = isTauri();
  } catch {
    native = false;
  }
  return {
    isTauri: native,
    previewMessage: native ? "" : BROWSER_PREVIEW_MESSAGE
  };
}

/** True when running inside the Tauri desktop runtime. */
export function isNativeRuntime(): boolean {
  return detectNativeRuntime().isTauri;
}

/**
 * Error raised when a native command cannot be dispatched or fails.
 *
 * The command name and the underlying reason are always preserved so the UI
 * can report exactly which bridge call failed — a failed command is never
 * silently replaced with fallback data.
 */
export class NativeCommandError extends Error {
  readonly command: string;
  readonly reason: string;
  readonly isRuntimeUnavailable: boolean;
  /** The structured error code the backend reported, when it reported one. */
  readonly code: string | null;
  /** True when the backend marked the failure as retryable. */
  readonly retryable: boolean;

  constructor(
    command: string,
    reason: string,
    isRuntimeUnavailable = false,
    code: string | null = null,
    retryable = false
  ) {
    super(`Native command "${command}" failed: ${reason}`);
    this.name = "NativeCommandError";
    this.command = command;
    this.reason = reason;
    this.isRuntimeUnavailable = isRuntimeUnavailable;
    this.code = code;
    this.retryable = retryable;
  }
}

/** Reads the structured `IpcError` payload a Tauri command rejects with. */
function structuredError(error: unknown): { code: string; message: string; retryable: boolean } | null {
  if (typeof error !== "object" || error === null) return null;
  const candidate = error as Record<string, unknown>;
  if (typeof candidate.code !== "string" || typeof candidate.message !== "string") return null;
  return {
    code: candidate.code,
    message: candidate.message,
    retryable: candidate.retryable === true
  };
}

function describeUnknown(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  const structured = structuredError(error);
  if (structured) return structured.message;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

/**
 * Dispatch a native command with the runtime guard applied.
 *
 * - Refuses to dispatch when the host is not the Tauri runtime.
 * - Captures the exact IPC error and the command name on failure.
 * - Never returns fallback data on failure.
 */
export async function invokeNative<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isNativeRuntime()) {
    throw new NativeCommandError(command, BROWSER_PREVIEW_MESSAGE, true);
  }
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const structured = structuredError(error);
    throw new NativeCommandError(
      command,
      describeUnknown(error),
      false,
      structured?.code ?? null,
      structured?.retryable ?? false
    );
  }
}

export type FoundationStatus = {
  product: string;
  version: string;
  phase: string;
  note: string;
};

export type DatabaseStatus = {
  db_file: string;
  migrations_applied: number;
  latest_version: number | null;
  foreign_keys_enabled: boolean;
};

export async function getFoundationStatus(): Promise<FoundationStatus> {
  return invokeNative<FoundationStatus>("get_foundation_status");
}

export async function getDatabaseStatus(): Promise<DatabaseStatus> {
  return invokeNative<DatabaseStatus>("get_database_status");
}

// Day 10 contracts — applied to NEW commands only. Rejections carry the
// structured IpcError payload (Tauri serializes the Rust error as the
// rejection reason). Empty result arrays are valid, truthful results:
// the backend returns real rows only and never synthesizes records.

export type IpcError = {
  code: string;
  message: string;
  retryable: boolean;
};

export type BackendIncidentRecord = {
  id: string;
  title: string;
  severity: string;
  category: string | null;
  source: string | null;
  createdAt: string;
  assignedTo: string | null;
  status: string;
};

export type BackendSecurityEvent = {
  id: string;
  source: string;
  severity: string;
  category: string | null;
  summary: string;
  occurredAt: string;
  ingestedAt: string;
};

export async function getIncidents(): Promise<BackendIncidentRecord[]> {
  return invokeNative<BackendIncidentRecord[]>("get_incidents");
}

export async function getSecurityEvents(): Promise<BackendSecurityEvent[]> {
  return invokeNative<BackendSecurityEvent[]>("get_security_events");
}

// ---------------------------------------------------------------------------
// Diagnostics contracts
//
// Every type below mirrors the Rust DTOs in
// `apps/desktop/src-tauri/src/diagnostics.rs` exactly. All of those DTOs use
// `#[serde(rename_all = "camelCase")]`, so the wire format is camelCase.
// ---------------------------------------------------------------------------

/** `diagnostics::InterfaceDto` */
export type BackendInterface = {
  id: string;
  name: string;
  description: string;
  kind: string;
  interfaceIndex: number;
  isUp: boolean;
  isPhysical: boolean;
  ipv4Addresses: string[];
  ipv6Addresses: string[];
  macAddress: string | null;
  mtu: number | null;
  linkSpeedBps: number | null;
};

/** `diagnostics::TrafficInterfaceDto` */
export type BackendTrafficInterface = {
  interfaceId: string;
  interfaceIndex: number;
  name: string;
  description: string;
  kind: string;
  isPhysical: boolean;
  isUp: boolean;
  rxBps: number | null;
  txBps: number | null;
  sessionRxBytes: number;
  sessionTxBytes: number;
  totalRxBytes: number;
  totalTxBytes: number;
  counterReset: boolean;
};

/** `diagnostics::TrafficSnapshotDto` */
export type BackendTrafficSnapshot = {
  sampledAt: string;
  source: string;
  monitorActive: boolean;
  interfaces: BackendTrafficInterface[];
  physicalRxBps: number | null;
  physicalTxBps: number | null;
  sessionRxBytes: number;
  sessionTxBytes: number;
  totalRxBytes: number;
  totalTxBytes: number;
  peakCombinedBps: number;
};

/** `diagnostics::TrafficSampleDto` */
export type BackendTrafficSample = {
  interfaceId: string;
  interfaceIndex: number;
  rxRateBps: number | null;
  txRateBps: number | null;
  rxBytes: number;
  txBytes: number;
  counterReset: boolean;
  sampledAt: string;
};

/** `diagnostics::TrafficMonitorStatusDto` */
export type BackendTrafficMonitorStatus = {
  running: boolean;
  lastSampleAgeMs: number | null;
  source: string;
};

/** `diagnostics::SpeedTestProgress` (from the `speed-test` crate) */
export type BackendSpeedTestProgress = {
  stage:
    | "preparing"
    | "latency"
    | "download"
    | "upload"
    | "finalizing"
    | "complete"
    | "cancelled"
    | "failed";
  fraction: number;
  currentMbps: number | null;
  detail: string;
};

/** `speed_test::SpeedTestReport` */
export type BackendSpeedTestReport = {
  serverUrl: string;
  serverName: string;
  serverObservedClientIp: string;
  interfaceLocalIp: string | null;
  downloadMbps: number;
  uploadMbps: number;
  pingMs: number;
  jitterMs: number;
  lossPct: number;
  downloadBytes: number;
  uploadBytes: number;
  pingSamples: number;
  pingReceived: number;
  rttSamples: number[];
  durationMs: number;
  startedAt: string;
  completedAt: string;
};

/** `diagnostics::SpeedTestStatusDto` */
export type BackendSpeedTestStatus = {
  id: string;
  state: "running" | "completed" | "failed" | "cancelled";
  progress: BackendSpeedTestProgress | null;
  report: BackendSpeedTestReport | null;
  error: string | null;
};

/** `diagnostics::SpeedTestHistoryDto` */
export type BackendSpeedTestHistoryEntry = {
  id: string;
  serverUrl: string;
  serverName: string;
  interfaceId: string | null;
  interfaceLocalIp: string | null;
  downloadMbps: number;
  uploadMbps: number;
  pingMs: number;
  jitterMs: number;
  lossPct: number;
  durationMs: number;
  startedAt: string;
  completedAt: string;
};

/** `diagnostics::StartSpeedTestRequest` */
export type StartSpeedTestRequest = {
  serverUrl: string;
  interfaceId?: string | null;
  interfaceLocalIp?: string | null;
  downloadBytes: number;
  uploadBytes: number;
  pingSamples: number;
  timeoutSecs: number;
  allowSelfSignedLoopback?: boolean;
};

export async function listNetworkInterfaces(): Promise<BackendInterface[]> {
  return invokeNative<BackendInterface[]>("list_network_interfaces");
}

export async function getTrafficMonitorStatus(): Promise<BackendTrafficMonitorStatus> {
  return invokeNative<BackendTrafficMonitorStatus>("get_traffic_monitor_status");
}

export async function setTrafficMonitorRunning(running: boolean): Promise<BackendTrafficMonitorStatus> {
  return invokeNative<BackendTrafficMonitorStatus>("set_traffic_monitor_running", { running });
}

export async function getTrafficSnapshot(): Promise<BackendTrafficSnapshot> {
  return invokeNative<BackendTrafficSnapshot>("get_traffic_snapshot");
}

export async function getTrafficHistory(
  interfaceId: string | null,
  seconds: number
): Promise<BackendTrafficSample[]> {
  return invokeNative<BackendTrafficSample[]>("get_traffic_history", { interfaceId, seconds });
}

export async function startSpeedTest(request: StartSpeedTestRequest): Promise<string> {
  return invokeNative<string>("start_speed_test", { request });
}

export async function getSpeedTestStatus(id: string): Promise<BackendSpeedTestStatus> {
  return invokeNative<BackendSpeedTestStatus>("get_speed_test_status", { id });
}

export async function cancelSpeedTest(id: string): Promise<void> {
  return invokeNative<void>("cancel_speed_test", { id });
}

export async function getSpeedTestHistory(): Promise<BackendSpeedTestHistoryEntry[]> {
  return invokeNative<BackendSpeedTestHistoryEntry[]>("get_speed_test_history");
}
