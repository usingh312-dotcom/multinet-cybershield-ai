// Frontend-only contracts for future security events.
// No backend surface exists in this phase: these types describe the shape the
// CyberShield Security Engine will emit once detection integration is real.
// Nothing in this file produces, simulates or seeds event data.

export type ThreatSeverity = "critical" | "high" | "medium" | "low" | "informational";

export type ThreatCategory = "network" | "endpoint" | "dns" | "firewall" | "intelligence";

export type InvestigationStatus = "new" | "investigating" | "resolved" | "false_positive";

export type EvidenceReference = {
  referenceId: string;
  kind: string;
  note?: string;
};

export type SecurityEvent = {
  id: string;
  /** ISO-8601 timestamp produced by the verified engine clock — never fabricated client-side. */
  timestamp: string;
  title: string;
  description: string;
  severity: ThreatSeverity;
  category: ThreatCategory;
  /** Detector or collector identifier that produced the event. */
  source: string;
  status: InvestigationStatus;
  /** Engine-computed confidence score in [0, 1]; omitted until a validated feed exists. */
  confidence: number;
  evidenceReference: EvidenceReference | null;
};

export const severityOrder: ThreatSeverity[] = ["critical", "high", "medium", "low", "informational"];

export const severityLabels: Record<ThreatSeverity, string> = {
  critical: "Critical",
  high: "High",
  medium: "Medium",
  low: "Low",
  informational: "Informational"
};

export const categoryLabels: Record<ThreatCategory, string> = {
  network: "Network anomaly",
  endpoint: "Endpoint activity",
  dns: "Suspicious DNS activity",
  firewall: "Firewall events",
  intelligence: "Threat intelligence matches"
};

export const investigationStatusLabels: Record<InvestigationStatus, string> = {
  new: "New",
  investigating: "Investigating",
  resolved: "Resolved",
  false_positive: "False positive"
};
