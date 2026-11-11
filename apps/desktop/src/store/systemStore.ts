import { create } from "zustand";
import { useMemo } from "react";
import {
  NativeCommandError,
  detectNativeRuntime,
  getDatabaseStatus,
  getFoundationStatus,
  type DatabaseStatus,
  type FoundationStatus
} from "../api";

export type SystemStore = {
  foundation: FoundationStatus | null;
  database: DatabaseStatus | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
};

function describe(error: unknown): string {
  if (error instanceof NativeCommandError) {
    return `${error.command}: ${error.reason}`;
  }
  return error instanceof Error ? error.message : String(error);
}

export const useSystemStore = create<SystemStore>((set) => ({
  foundation: null,
  database: null,
  loading: false,
  error: null,
  refresh: async () => {
    const results = await Promise.allSettled([getFoundationStatus(), getDatabaseStatus()]);
    const rejected = results.find((r) => r.status === "rejected");
    const error = rejected && rejected.status === "rejected" ? describe(rejected.reason) : null;
    set({
      foundation: results[0].status === "fulfilled" ? results[0].value : null,
      database: results[1].status === "fulfilled" ? results[1].value : null,
      error,
      loading: false
    });
  }
}));

/**
 * Current native-runtime probe.
 *
 * The runtime is probed on render rather than cached in the store: the host
 * decides it, and a cached flag would keep reporting the wrong answer after
 * the frontend is reloaded inside a different host.
 */
export function useNativeRuntime() {
  return useMemo(() => detectNativeRuntime(), []);
}

/**
 * Clears the collected system status.
 *
 * The store is a module-level singleton, so a test that rendered a screen
 * would otherwise leak the status of the previous test into the next one.
 */
export function resetSystemStore(): void {
  useSystemStore.setState({ foundation: null, database: null, loading: false, error: null });
}
