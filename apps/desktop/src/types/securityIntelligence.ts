// Frontend-only contracts for threat-intelligence indicators/feeds and DNS
// events/policies. No backend surface exists in this phase: these types describe
// what the CyberShield Security Engine will emit once those integrations are real.
// Nothing in this file produces, simulates or seeds feed or event data.

import type { InvestigationStatus, ThreatSeverity } from "./securityEvents";

export type IndicatorType = "ipv4" | "ipv6" | "domain" | "url" | "file_hash" | "ip_range";

export type IndicatorReputation = "malicious" | "suspicious" | "unknown" | "trusted";

export type ThreatIndicator = {
  id: string;
  /** The indicator value (address, domain, URL, hash) from a verified source — never invented client-side. */
  value: string;
  type: IndicatorType;
  reputation: IndicatorReputation;
  severity: ThreatSeverity;
  /** Source-assigned confidence in [0, 1]; omitted until a trusted source reports it. */
  confidence: number;
  /** Detector/collector identifier that supplied the indicator. */
  source: string;
  firstSeen: string | null;
  lastSeen: string | null;
  status: InvestigationStatus;
};

export type FeedConnectionStatus = "not_configured" | "integration_pending" | "connected" | "error";

export type FeedFreshness = "unknown" | "stale" | "fresh";

export type ThreatIntelligenceFeed = {
  id: string;
  name: string;
  /** Provider display name — never claimed as integrated while integration is pending. */
  provider: string;
  feedType: string;
  connectionStatus: FeedConnectionStatus;
  lastSynchronizedAt: string | null;
  freshness: FeedFreshness;
  enabled: boolean;
  /** Source-reported reliability in [0, 1]; omitted when not published. */
  reliability: number | null;
};

export type DnsQueryType = "A" | "AAAA" | "CNAME" | "MX" | "TXT" | "NS" | "PTR";

export type DnsEvent = {
  id: string;
  /** Queried domain from the verified resolver log — never fabricated client-side. */
  domain: string;
  queryType: DnsQueryType;
  /** Opaque collector-side client reference (never a raw address displayed here). */
  clientReference: string;
  /** Opaque collector-side resolver reference. */
  resolverReference: string;
  reputation: IndicatorReputation;
  /** ISO-8601 timestamp from the collector clock. */
  timestamp: string;
  status: InvestigationStatus;
};

export type DnsPolicyCategory =
  | "malicious"
  | "suspicious"
  | "allowlist"
  | "blocklist"
  | "anomaly_detection"
  | "tunneling_detection"
  | "policy_frequency";

export type DnsPolicyStatus = "disabled" | "pending_approval" | "enabled";

export type DnsPolicy = {
  id: string;
  name: string;
  category: DnsPolicyCategory;
  enabled: boolean;
  /** Enforcement always requires explicit human approval — policies never auto-enable. */
  approvalRequired: boolean;
  status: DnsPolicyStatus;
};

export const indicatorTypeLabels: Record<IndicatorType, string> = {
  ipv4: "IPv4",
  ipv6: "IPv6",
  domain: "Domain",
  url: "URL",
  file_hash: "File hash",
  ip_range: "IP range"
};

export const reputationLabels: Record<IndicatorReputation, string> = {
  malicious: "Malicious",
  suspicious: "Suspicious",
  unknown: "Unknown",
  trusted: "Trusted"
};

export const feedConnectionLabels: Record<FeedConnectionStatus, string> = {
  not_configured: "Not configured",
  integration_pending: "Integration pending",
  connected: "Connected",
  error: "Error"
};

export const feedFreshnessLabels: Record<FeedFreshness, string> = {
  unknown: "Unknown",
  stale: "Stale",
  fresh: "Fresh"
};

/**
 * Compile-time contract check (no runtime test framework in this project,
 * see Phase2 Day 5 report §6). Instantiating this class signature fails
 * type-checking if any contract shape is loosened accidentally.
 */
export const contractSanity: {
  indicatorTypes: readonly IndicatorType[];
  reputations: readonly IndicatorReputation[];
  queryTypes: readonly DnsQueryType[];
  policyCategories: readonly DnsPolicyCategory[];
} = {
  indicatorTypes: ["ipv4", "ipv6", "domain", "url", "file_hash", "ip_range"],
  reputations: ["malicious", "suspicious", "unknown", "trusted"],
  queryTypes: ["A", "AAAA", "CNAME", "MX", "TXT", "NS", "PTR"],
  policyCategories: ["malicious", "suspicious", "allowlist", "blocklist", "anomaly_detection", "tunneling_detection", "policy_frequency"]
};
