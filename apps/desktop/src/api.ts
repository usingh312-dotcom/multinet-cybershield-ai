import { invoke } from "@tauri-apps/api/core";

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
  return invoke<FoundationStatus>("get_foundation_status");
}

export async function getDatabaseStatus(): Promise<DatabaseStatus> {
  return invoke<DatabaseStatus>("get_database_status");
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
  return invoke<BackendIncidentRecord[]>("get_incidents");
}

export async function getSecurityEvents(): Promise<BackendSecurityEvent[]> {
  return invoke<BackendSecurityEvent[]>("get_security_events");
}
