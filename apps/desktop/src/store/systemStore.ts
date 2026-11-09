import { create } from "zustand";
import {
  getDatabaseStatus,
  getFoundationStatus,
  type DatabaseStatus,
  type FoundationStatus
} from "../api";

type SystemStore = {
  foundation: FoundationStatus | null;
  database: DatabaseStatus | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
};

export const useSystemStore = create<SystemStore>((set) => ({
  foundation: null,
  database: null,
  loading: false,
  error: null,
  refresh: async () => {
    set({ loading: true });
    const results = await Promise.allSettled([getFoundationStatus(), getDatabaseStatus()]);
    const rejected = results.find((r) => r.status === "rejected");
    const error = rejected && rejected.status === "rejected"
      ? rejected.reason instanceof Error ? rejected.reason.message : String(rejected.reason)
      : null;
    set({
      foundation: results[0].status === "fulfilled" ? results[0].value : null,
      database: results[1].status === "fulfilled" ? results[1].value : null,
      error,
      loading: false
    });
  }
}));
