// Frontend-only contracts for Endpoint Security and AI Firewall (Phase 2 Day 7).
// No backend adapter exists in this build: these types describe what the
// CyberShield Security Engine will emit once those integrations are real.
// Nothing in this file produces, simulates or seeds device/finding/rule data.

import type { InvestigationStatus, ThreatSeverity } from "./securityEvents";
import type { RiskLevel } from "./copilot";

export type EndpointAgentStatus = "not_deployed" | "pending" | "connected" | "offline";

export type MonitoringStatus = "inactive" | "initializing" | "active" | "degraded";

export type EndpointRiskLevel = "unknown" | "low" | "medium" | "high" | "critical";

export type EndpointDevice = {
  id: string;
  hostname: string;
  operatingSystem: string;
  agentStatus: EndpointAgentStatus;
  monitoringStatus: MonitoringStatus;
  /** ISO-8601 timestamp from the collector clock. */
  lastSeen: string | null;
  riskLevel: EndpointRiskLevel;
};

export type EndpointFindingType =
  | "malware"
  | "suspicious_process"
  | "persistence"
  | "file_integrity"
  | "network_connection"
  | "privilege_escalation";

export type EndpointFinding = {
  id: string;
  deviceId: string;
  findingType: EndpointFindingType;
  severity: ThreatSeverity;
  evidenceReferences: string[];
  status: InvestigationStatus;
  detectedAt: string | null;
};

export type FirewallDirection = "inbound" | "outbound";
export type FirewallProtocol = "tcp" | "udp" | "icmp" | "any";
export type FirewallAction = "allow" | "block";

export type FirewallRule = {
  id: string;
  name: string;
  direction: FirewallDirection;
  protocol: FirewallProtocol;
  /** Opaque source/destination descriptors (addresses, CIDRs, refs) — never invented client-side. */
  source: string;
  destination: string;
  /** Port descriptor ("any", "443", "8080-8090", …) — never invented. */
  port: string;
  action: FirewallAction;
  priority: number;
  /** "candidate" until enabled by policy + human approval. */
  status: "candidate" | "enabled" | "disabled" | "superseded";
};

export type FirewallProposal = {
  id: string;
  title: string;
  /** Human-readable rationale the user reviews before any approval. */
  rationale: string;
  evidenceReferences: string[];
  proposedRule: FirewallRule;
  riskLevel: RiskLevel;
  approvalStatus: InvestigationStatus;
  /** True when a safe rollback can be executed where technically feasible. */
  reversible: boolean;
};

export const endpointAgentStatusLabels: Record<EndpointAgentStatus, string> = {
  not_deployed: "Not deployed",
  pending: "Pending",
  connected: "Connected",
  offline: "Offline"
};

export const monitoringStatusLabels: Record<MonitoringStatus, string> = {
  inactive: "Inactive",
  initializing: "Initializing",
  active: "Active",
  degraded: "Degraded"
};

export const endpointRiskLevelLabels: Record<EndpointRiskLevel, string> = {
  unknown: "Unknown",
  low: "Low",
  medium: "Medium",
  high: "High",
  critical: "Critical"
};

export const findingTypeLabels: Record<EndpointFindingType, string> = {
  malware: "Malware detection",
  suspicious_process: "Suspicious process behavior",
  persistence: "Unauthorized persistence",
  file_integrity: "File integrity changes",
  network_connection: "Suspicious network connections",
  privilege_escalation: "Privilege escalation indicators"
};
