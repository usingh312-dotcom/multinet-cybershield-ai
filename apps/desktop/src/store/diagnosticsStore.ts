import { create } from "zustand";
import {
  NativeCommandError,
  cancelSpeedTest,
  detectNativeRuntime,
  getSpeedTestHistory,
  getSpeedTestStatus,
  getTrafficHistory,
  getTrafficMonitorStatus,
  getTrafficSnapshot,
  listNetworkInterfaces,
  startSpeedTest,
  type BackendInterface,
  type BackendSpeedTestHistoryEntry,
  type BackendSpeedTestStatus,
  type BackendTrafficMonitorStatus,
  type BackendTrafficSample,
  type BackendTrafficSnapshot,
  type StartSpeedTestRequest
} from "../api";

/** Loading phase of a diagnostics slice. */
export type LoadState = "idle" | "loading" | "ready" | "error";

export type DiagnosticsError = {
  /** The native command that failed, e.g. `list_network_interfaces`. */
  command: string;
  /** The exact reason reported by the bridge. */
  reason: string;
  /** True when the failure is caused by the browser-preview host, not the backend. */
  runtimeUnavailable: boolean;
};

function toDiagnosticsError(error: unknown, fallbackCommand: string): DiagnosticsError {
  if (error instanceof NativeCommandError) {
    return {
      command: error.command,
      reason: error.reason,
      runtimeUnavailable: error.isRuntimeUnavailable
    };
  }
  return {
    command: fallbackCommand,
    reason: error instanceof Error ? error.message : String(error),
    runtimeUnavailable: false
  };
}

export type DiagnosticsStore = {
  // Runtime
  nativeRuntime: boolean;
  previewMessage: string;

  // Interface inventory
  interfaces: BackendInterface[];
  interfacesState: LoadState;
  interfacesError: DiagnosticsError | null;

  // Traffic
  snapshot: BackendTrafficSnapshot | null;
  snapshotState: LoadState;
  snapshotError: DiagnosticsError | null;
  monitor: BackendTrafficMonitorStatus | null;
  history: BackendTrafficSample[];
  historyState: LoadState;
  historyError: DiagnosticsError | null;
  historySeconds: number;

  // Speed test
  speedRunId: string | null;
  speedStatus: BackendSpeedTestStatus | null;
  speedState: LoadState;
  speedError: DiagnosticsError | null;
  speedHistory: BackendSpeedTestHistoryEntry[];
  speedHistoryState: LoadState;
  speedHistoryError: DiagnosticsError | null;

  // Actions
  refreshInterfaces: () => Promise<void>;
  refreshTraffic: () => Promise<void>;
  refreshTrafficHistory: (seconds?: number) => Promise<void>;
  setHistoryWindow: (seconds: number) => Promise<void>;
  runSpeedTest: (request: StartSpeedTestRequest) => Promise<string | null>;
  pollSpeedTest: (id: string) => Promise<void>;
  cancelCurrentSpeedTest: () => Promise<void>;
  refreshSpeedHistory: () => Promise<void>;
};

export const useDiagnosticsStore = create<DiagnosticsStore>((set, get) => ({
  nativeRuntime: detectNativeRuntime().isTauri,
  previewMessage: detectNativeRuntime().previewMessage,

  interfaces: [],
  interfacesState: "idle",
  interfacesError: null,

  snapshot: null,
  snapshotState: "idle",
  snapshotError: null,
  monitor: null,
  history: [],
  historyState: "idle",
  historyError: null,
  historySeconds: 300,

  speedRunId: null,
  speedStatus: null,
  speedState: "idle",
  speedError: null,
  speedHistory: [],
  speedHistoryState: "idle",
  speedHistoryError: null,

  refreshInterfaces: async () => {
    set({ interfacesState: "loading", interfacesError: null });
    try {
      const interfaces = await listNetworkInterfaces();
      set({ interfaces, interfacesState: "ready", interfacesError: null });
    } catch (error) {
      set({
        interfaces: [],
        interfacesState: "error",
        interfacesError: toDiagnosticsError(error, "list_network_interfaces")
      });
    }
  },

  refreshTraffic: async () => {
    set({ snapshotState: "loading", snapshotError: null });
    try {
      const snapshot = await getTrafficSnapshot();
      set({ snapshot, snapshotState: "ready", snapshotError: null });
    } catch (error) {
      set({
        snapshot: null,
        snapshotState: "error",
        snapshotError: toDiagnosticsError(error, "get_traffic_snapshot")
      });
    }
    try {
      const monitor = await getTrafficMonitorStatus();
      set({ monitor });
    } catch {
      // The monitor status is supplementary; the snapshot remains authoritative.
    }
  },

  refreshTrafficHistory: async (seconds?: number) => {
    const window = seconds ?? get().historySeconds;
    set({ historyState: "loading", historyError: null, historySeconds: window });
    try {
      const history = await getTrafficHistory(null, window);
      set({ history, historyState: "ready", historyError: null });
    } catch (error) {
      set({
        history: [],
        historyState: "error",
        historyError: toDiagnosticsError(error, "get_traffic_history")
      });
    }
  },

  setHistoryWindow: async (seconds: number) => {
    await get().refreshTrafficHistory(seconds);
  },

  runSpeedTest: async (request) => {
    set({ speedState: "loading", speedError: null, speedStatus: null, speedRunId: null });
    try {
      const id = await startSpeedTest(request);
      set({ speedRunId: id, speedState: "ready" });
      return id;
    } catch (error) {
      set({
        speedRunId: null,
        speedState: "error",
        speedError: toDiagnosticsError(error, "start_speed_test")
      });
      return null;
    }
  },

  pollSpeedTest: async (id) => {
    try {
      const status = await getSpeedTestStatus(id);
      set({ speedStatus: status, speedState: "ready", speedError: null });
      if (status.state === "completed" || status.state === "failed" || status.state === "cancelled") {
        await get().refreshSpeedHistory();
      }
    } catch (error) {
      set({ speedState: "error", speedError: toDiagnosticsError(error, "get_speed_test_status") });
    }
  },

  cancelCurrentSpeedTest: async () => {
    const id = get().speedRunId;
    if (!id) return;
    try {
      await cancelSpeedTest(id);
    } catch (error) {
      set({ speedState: "error", speedError: toDiagnosticsError(error, "cancel_speed_test") });
    }
  },

  refreshSpeedHistory: async () => {
    set({ speedHistoryState: "loading", speedHistoryError: null });
    try {
      const speedHistory = await getSpeedTestHistory();
      set({ speedHistory, speedHistoryState: "ready", speedHistoryError: null });
    } catch (error) {
      set({
        speedHistory: [],
        speedHistoryState: "error",
        speedHistoryError: toDiagnosticsError(error, "get_speed_test_history")
      });
    }
  }
}));

/**
 * Clears every collected diagnostic value.
 *
 * The store is a module-level singleton, so a test that rendered a screen
 * would otherwise leak its interface list, traffic snapshot or speed-test
 * history into the next test. Only the measured data is reset; the actions
 * stay in place.
 */
export function resetDiagnosticsStore(): void {
  useDiagnosticsStore.setState({
    interfaces: [],
    interfacesState: "idle",
    interfacesError: null,
    snapshot: null,
    snapshotState: "idle",
    snapshotError: null,
    monitor: null,
    history: [],
    historyState: "idle",
    historyError: null,
    historySeconds: 300,
    speedRunId: null,
    speedStatus: null,
    speedState: "idle",
    speedError: null,
    speedHistory: [],
    speedHistoryState: "idle",
    speedHistoryError: null
  });
}
