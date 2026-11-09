import { useRef, useState } from "react";
import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { EmptyState, StatusCard, StatusIndicator } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue } from "../components/ui/controls";
import { useAppStore, navItems } from "../store/appStore";
import { useSystemStore } from "../store/systemStore";
import {
  IconBell, IconChevronDown, IconCopilot, IconIncident, IconSearch
} from "../components/ui/icons";

const suggestedPrompts = [
  "Explain my current security readiness.",
  "How does internet bonding work?",
  "Explain suspicious DNS activity.",
  "Help me understand a threat indicator.",
  "Prepare an incident investigation checklist.",
  "Explain the difference between load balancing and true bonding."
] as const;

const providerStatusCards = [
  { title: "AI Provider", value: "Not configured", hint: "No provider is connected, authenticated or reachable in this build." },
  { title: "Model", value: "Not configured", hint: "Model identity comes from the provider's handshake — never invented client-side." },
  { title: "Connection", value: "Not connected", hint: "No provider connection exists; inference is unavailable." },
  { title: "Last Request", value: "Never", hint: "No request has been made, so no request time can be reported." },
  { title: "Token Usage", value: "Not available", hint: "Token accounting arrives with a real provider integration." }
] as const;

const approvalStages = [
  "AI proposes an action",
  "Evidence and rationale are displayed",
  "Policy validation occurs",
  "User reviews the proposal",
  "User explicitly approves or rejects",
  "Approved action is executed by the authorized backend",
  "Result is recorded in the audit log"
] as const;

const safetyBoundaries = [
  "AI cannot execute arbitrary shell commands.",
  "AI cannot directly change firewall settings.",
  "AI cannot modify DNS or network routes.",
  "AI cannot terminate endpoint processes automatically.",
  "Defensive actions require validated evidence and human approval.",
  "All future actions must use structured, policy-controlled interfaces."
] as const;

export default function AiCopilotPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const { active } = useAppStore();
  const bridgeOk = Boolean(foundation || database);
  const lifecycleVariation = loading ? "warn" : "idle" as const;
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "AI provider not configured";

  const currentPage = navItems.find((n) => n.id === active)?.label ?? "Overview";
  const [draft, setDraft] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="AI Security Copilot"
          subtitle="An intelligent assistant for understanding network and security information."
          actions={<StatusIndicator variation={lifecycleVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            <strong className="font-semibold text-ink-primary">AI provider not configured.</strong> No provider is connected in this build — inference is unavailable.
            Suggested prompts fill the composer only; no response is generated, and no findings are produced. Nothing is sent anywhere.
          </IntegrationNotice>
        </div>
      </section>

      <section className="grid gap-4 lg:grid-cols-[320px_1fr]">
        <aside className="flex flex-col gap-3">
          <SectionHeader title="Conversation history" description="Local UI surface for the future provider-backed chat." />
          <div className="flex flex-col gap-2">
            <Button variant="primary" onClick={() => inputRef.current?.focus()} disabled={false} className="w-full justify-center">
              New conversation
            </Button>
            <div className="relative flex items-center gap-2 rounded-lg border border-charcoal-600 bg-field px-2.5 py-2">
              <IconSearch size={13} className="shrink-0 text-ink-muted" />
              <input disabled placeholder="Search conversations…" aria-label="Search conversations" className="h-6 w-full bg-transparent text-xs text-ink-muted placeholder:text-ink-muted focus:outline-none disabled:cursor-not-allowed" />
            </div>
            <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
              <EmptyState
                icon={<IconCopilot size={18} />}
                title="No conversations yet"
                description="Conversations persist when the provider integration ships. Nothing is fabricated here — no prior chats, titles, timestamps or messages exist."
              />
            </div>
            <Button disabled disabledReason="No conversation exists to clear — this requires a configured provider and a real session.">Clear conversation</Button>
          </div>

          <div className="mt-1">
            <SectionHeader title="AI provider status" description="Unavailable until a real provider is connected." />
            <div className="grid gap-2">
              {providerStatusCards.map((c) => (
                <KeyValue key={c.title} label={c.title} value={c.value} />
              ))}
            </div>
          </div>
        </aside>

        <div className="flex min-w-0 flex-col overflow-hidden rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel">
          <div className="flex-1 overflow-y-auto px-5 py-6">
            <div className="mx-auto flex max-w-xl flex-col items-center gap-4 pt-6 text-center">
              <span className="rounded-2xl border border-accent-blue/25 bg-accent-blue/10 p-4 text-accent-cyan"><IconCopilot size={30} /></span>
              <h2 className="text-lg font-semibold text-ink-primary">Your Security Intelligence Assistant</h2>
              <p className="text-sm leading-6 text-ink-secondary">Understand security findings, analyze available evidence and explore defensive recommendations.</p>
              <div className="mt-1 grid w-full gap-2 text-left sm:grid-cols-2">
                {suggestedPrompts.map((prompt) => (
                  <button
                    key={prompt}
                    type="button"
                    onClick={() => setDraft(prompt)}
                    className="rounded-xl2 border border-charcoal-600 bg-charcoal-900/60 p-3 text-left text-[11px] leading-4 text-ink-secondary transition hover:border-accent-blue/40 hover:text-ink-primary focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-blue/60"
                  >
                    {prompt}
                  </button>
                ))}
              </div>
              <p className="mt-1 text-[10px] uppercase tracking-wider text-ink-muted">Suggested questions only — not evidence of actual detections</p>
            </div>
          </div>

          <div className="border-t border-charcoal-600 bg-charcoal-900/70 p-3">
            <div className="flex items-end gap-2 rounded-xl2 border border-charcoal-600 bg-field p-2">
              <IconCopilot size={16} className="mb-1.5 shrink-0 text-ink-muted" aria-hidden="true" />
              <label htmlFor="copilot-composer" className="sr-only">Message the copilot</label>
              <input
                ref={inputRef}
                id="copilot-composer"
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                disabled
                placeholder="AI provider not configured — composer disabled…"
                aria-label="Message the copilot"
                className="h-8 max-w-full flex-1 bg-transparent text-sm text-ink-muted placeholder:text-ink-muted focus:outline-none disabled:cursor-not-allowed"
              />
              <button
                type="button"
                disabled
                title="Sending requires a configured AI provider"
                aria-label="Send message"
                className="mb-0.5 inline-flex h-8 w-8 items-center justify-center rounded-lg border border-charcoal-600 bg-charcoal-800/50 text-ink-muted disabled:cursor-not-allowed"
              >
                <IconChevronDown size={15} className="-rotate-90" aria-hidden="true" />
              </button>
            </div>
            <div className="mt-2 flex items-center justify-between gap-3 px-1">
              <button type="button" disabled title="Attachments require a working provider and virus-scan pipeline" aria-label="Attach a file" className="inline-flex h-7 items-center gap-1.5 rounded-md border border-charcoal-600 px-2 text-[11px] text-ink-muted disabled:cursor-not-allowed">
                + Add attachment
              </button>
              <span className="text-[10px] text-ink-muted">UI preview — inference, providers and token accounting are integration-pending.</span>
            </div>
          </div>
        </div>
      </section>

      <section className="grid gap-4 xl:grid-cols-2">
        <div>
          <SectionHeader title="Context awareness preview" description="Future context inputs — live capability, surface by surface." />
          <div className="grid gap-2.5">
            <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3 flex items-center justify-between gap-3">
              <div className="min-w-0"><div className="text-xs font-semibold text-ink-primary">Current application page</div><div className="text-[11px] text-ink-secondary truncate">{currentPage}</div></div>
              <Badge tone="ok">Available — live UI state</Badge>
            </div>
            {[
              { name: "Available network information", description: "Interfaces, adapters, routes — requires verified platform collectors." },
              { name: "Security events", description: "Event stream enters context once the detection engine integrates." },
              { name: "Threat intelligence", description: "Indicator/reputation context appears after the TI module ships." },
              { name: "Uploaded files", description: "File parsing, malware inspection and evidence intake are unimplemented." }
            ].map((c) => (
              <div key={c.name} className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-3 flex items-center justify-between gap-3">
                <div className="min-w-0"><div className="text-xs font-semibold text-ink-primary">{c.name}</div><div className="text-[11px] text-ink-secondary">{c.description}</div></div>
                <Badge tone="neutral">integration pending</Badge>
              </div>
            ))}
          </div>
          <p className="mt-2 text-[10px] leading-4 text-ink-muted">No filesystem, clipboard, device or browser state is silently collected. Consent boundaries are explicit for every future input.</p>
        </div>
        <div>
          <SectionHeader title="Human approval workflow" description="Proposed defensive actions never execute without approval." />
          <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
            <ol className="space-y-2">
              {approvalStages.map((stage, index) => (
                <li key={stage} className="flex items-start gap-2.5 text-xs leading-5 text-ink-secondary">
                  <span className="mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-md border border-charcoal-600 bg-charcoal-800 font-mono text-[10px] text-ink-muted">{index + 1}</span>
                  {stage}
                </li>
              ))}
            </ol>
            <div className="mt-3 flex items-center gap-2 rounded-lg border border-amber-900/40 bg-amber-900/10 p-3">
              <IconBell size={14} className="shrink-0 text-amber-300" />
              <span className="text-[11px] leading-4 text-amber-200">Integration pending — no proposal exists, so approve/reject controls stay disabled. Execution requires a validated finding, policy validation and your explicit approval.</span>
            </div>
            <div className="mt-3 flex gap-2">
              <Button disabled disabledReason="No proposal exists to approve — requires the AI backend and human review loop.">Approve</Button>
              <Button disabled disabledReason="No proposal exists to reject — requires the AI backend and human review loop.">Reject</Button>
            </div>
          </div>
        </div>
      </section>

      <section>
        <SectionHeader title="Safety boundaries" description="Enforced today — not aspirational." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {safetyBoundaries.map((rule) => (
            <div key={rule} className="rounded-xl2 border border-amber-900/30 bg-amber-900/5 p-3.5 flex items-start gap-2.5">
              <span className="mt-0.5 text-amber-400"><IconIncident size={14} /></span>
              <span className="text-[11px] leading-4 text-ink-secondary">{rule}</span>
            </div>
          ))}
        </div>
      </section>
    </PageContainer>
  );
}
