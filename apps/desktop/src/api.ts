import { invoke } from "@tauri-apps/api/core";

export type FoundationStatus = {
  product: string;
  version: string;
  phase: string;
  note: string;
};

export async function getFoundationStatus(): Promise<FoundationStatus> {
  return invoke<FoundationStatus>("get_foundation_status");
}
