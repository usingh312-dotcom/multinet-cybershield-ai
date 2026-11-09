// Frontend-only contracts for the AI Security Copilot surface (Phase 2 Day 6).
// No AIProvider is connected in this build: these types describe the shape a
// real provider integration will bind to later. No assistant logic is bundled here.

export type CopilotRole = "user" | "assistant" | "system";

export type CopilotMessageStatus = "sending" | "delivered" | "failed";

export type CopilotMessage = {
  id: string;
  role: CopilotRole;
  content: string;
  /** ISO-8601 timestamp from the provider/client clock. */
  timestamp: string;
  status: CopilotMessageStatus;
};

export type CopilotConversation = {
  id: string;
  title: string;
  /** ISO-8601 creation timestamp. */
  createdAt: string;
  /** ISO-8601 last-activity timestamp. */
  updatedAt: string;
};

export type ApprovalStatus = "pending_review" | "approved" | "rejected" | "expired";

export type RiskLevel = "low" | "moderate" | "elevated" | "high";

export type ProposedSecurityAction = {
  id: string;
  /** Structured machine-checkable action identifier (e.g. "block.domain", "process.terminate"). */
  actionType: string;
  /** Human-readable rationale the user will be shown before approving. */
  rationale: string;
  /** Opaque references into the evidence store (never raw payloads embedded here). */
  evidenceReferences: string[];
  riskLevel: RiskLevel;
  approvalStatus: ApprovalStatus;
  /** ISO-8601 creation timestamp. */
  createdAt: string;
};

export type CopilotConnectionStatus = "not_configured" | "integration_pending" | "connected" | "error";

export type AIProviderStatus = {
  /** Provider display name — never claimed as connected while integration is pending. */
  provider: string;
  /** Model identifier — never invented client-side. */
  model: string;
  connectionStatus: CopilotConnectionStatus;
  /** ISO-8601 timestamp of the last successful provider call; null while disconnected. */
  lastRequestAt: string | null;
};
