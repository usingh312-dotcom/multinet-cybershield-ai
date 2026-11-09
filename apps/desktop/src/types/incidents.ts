// Frontend-only contracts for Incident Response and application preferences (Phase 2 Day 8).
// No backend adapter exists in this build: these types describe what the
// CyberShield Security Engine will emit once incident integration is real.
// Nothing in this file produces, simulates or seeds incident/evidence/preference data.

import type { ThreatSeverity } from "./securityEvents";

export type IncidentCategory =
  | "network"
  | "endpoint"
  | "dns"
  | "firewall"
  | "intelligence"
  | "policy";

export type IncidentLifecycle =
  | "new"
  | "investigating"
  | "contained"
  | "resolved"
  | "false_positive";

export type IncidentRecord = {
  id: string;
  title: string;
  severity: ThreatSeverity;
  category: IncidentCategory;
  /** Detector/collector identifier that opened the incident. */
  source: string;
  /** ISO-8601 timestamp from the verified engine clock. */
  createdAt: string;
  assignedTo: string | null;
  status: IncidentLifecycle;
};

export type IncidentEvidenceType =
  | "network_event"
  | "endpoint_event"
  | "dns_event"
  | "threat_intelligence"
  | "audit_log";

export type EvidenceIntegrityStatus = "unverified" | "verified" | "failed";

export type IncidentEvidence = {
  id: string;
  incidentId: string;
  evidenceType: IncidentEvidenceType;
  /** Opaque reference into the evidence store — never raw payloads embedded here. */
  evidenceReference: string;
  /** ISO-8601 timestamp from the collector clock. */
  collectedAt: string | null;
  integrityStatus: EvidenceIntegrityStatus;
};

export type IncidentTimelineEventType =
  | "incident_created"
  | "evidence_collected"
  | "investigation_started"
  | "severity_reviewed"
  | "action_proposed"
  | "approval_requested"
  | "action_approved"
  | "response_completed"
  | "incident_resolved";

export type IncidentTimelineEvent = {
  id: string;
  incidentId: string;
  eventType: IncidentTimelineEventType;
  /** ISO-8601 timestamp from the engine/audit clock. */
  timestamp: string;
  /** Opaque reference to the audit record, when recorded. */
  auditReference: string | null;
};

export type ThemePreference = "dark" | "light" | "system";
export type NavigationDensity = "comfortable" | "compact";
export type InterfaceDensity = "comfortable" | "compact";

export type ApplicationPreferences = {
  theme: ThemePreference;
  navigationDensity: NavigationDensity;
  interfaceDensity: InterfaceDensity;
  language: string;
  notificationsEnabled: boolean;
};

export const incidentStatusLabels: Record<IncidentLifecycle, string> = {
  new: "New",
  investigating: "Investigating",
  contained: "Contained",
  resolved: "Resolved",
  false_positive: "False positive"
};

export const incidentCategoryLabels: Record<IncidentCategory, string> = {
  network: "Network",
  endpoint: "Endpoint",
  dns: "DNS",
  firewall: "Firewall",
  intelligence: "Threat intelligence",
  policy: "Policy violation"
};

export const evidenceTypeLabels: Record<IncidentEvidenceType, string> = {
  network_event: "Network event reference",
  endpoint_event: "Endpoint event reference",
  dns_event: "DNS event reference",
  threat_intelligence: "Threat intelligence reference",
  audit_log: "Audit log reference"
};

export const timelineEventLabels: Record<IncidentTimelineEventType, string> = {
  incident_created: "Incident created",
  evidence_collected: "Evidence collected",
  investigation_started: "Investigation started",
  severity_reviewed: "Severity reviewed",
  action_proposed: "Action proposed",
  approval_requested: "Approval requested",
  action_approved: "Action approved",
  response_completed: "Response completed",
  incident_resolved: "Incident resolved"
};
