import { useId, useState, type ReactNode } from "react";
import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { StatusIndicator } from "../components/ui/feedback";
import { Badge, Button, IntegrationNotice, KeyValue, SelectField } from "../components/ui/controls";
import { useAppStore } from "../store/appStore";
import { useSystemStore } from "../store/systemStore";
import type { ThemePreference } from "../types/incidents";
import { cn } from "../lib/cn";
import {
  IconBell, IconCopilot, IconDatabase, IconInfo, IconNetworkManager, IconOverview,
  IconSettings, IconShield
} from "../components/ui/icons";

type SettingsSectionId =
  | "general" | "appearance" | "network" | "security"
  | "ai-provider" | "notifications" | "privacy" | "about";

const sections: { id: SettingsSectionId; label: string; icon: ReactNode }[] = [
  { id: "general", label: "General", icon: <IconSettings size={14} /> },
  { id: "appearance", label: "Appearance", icon: <IconOverview size={14} /> },
  { id: "network", label: "Network", icon: <IconNetworkManager size={14} /> },
  { id: "security", label: "Security", icon: <IconShield size={14} /> },
  { id: "ai-provider", label: "AI Provider", icon: <IconCopilot size={14} /> },
  { id: "notifications", label: "Notifications", icon: <IconBell size={14} /> },
  { id: "privacy", label: "Privacy & Data", icon: <IconDatabase size={14} /> },
  { id: "about", label: "About", icon: <IconInfo size={14} /> }
];

const APP_VERSION = "0.1.0";

export default function SettingsPage() {
  const { foundation, database, error, loading } = useSystemStore();
  const { sidebarCollapsed, toggleSidebar } = useAppStore();
  const [section, setSection] = useState<SettingsSectionId>("general");
  const [themePreview, setThemePreview] = useState<ThemePreference>("dark");
  const panelId = useId();

  const readinessVariation = loading ? "warn" : "idle";
  const readinessLabel = loading ? "Connecting…" : error ? "Backend unavailable" : "Preferences not persisted";
  const version = foundation?.version ?? APP_VERSION;
  const dbLocation = database?.db_file ?? null;

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Settings"
          subtitle="Configure MultiNet CyberShield AI application preferences and integration readiness."
          actions={<StatusIndicator variation={readinessVariation} label={readinessLabel} />}
        />
        <div className="mt-3">
          <IntegrationNotice>
            Settings are a <strong className="font-semibold text-ink-primary">design-time preview</strong>. No preference is persisted — there is no settings backend yet.
            Only the compact-navigation toggle is wired to live UI state (the sidebar). Every other control is disabled or marked preview. Credentials are never stored by this build.
          </IntegrationNotice>
        </div>
      </section>

      <div className="grid gap-6 lg:grid-cols-[220px_1fr]">
        <nav aria-label="Settings sections" className="flex flex-row flex-wrap gap-1 lg:flex-col">
          {sections.map((s) => {
            const isActive = section === s.id;
            return (
              <button
                key={s.id}
                type="button"
                onClick={() => setSection(s.id)}
                aria-current={isActive ? "true" : undefined}
                aria-controls={panelId}
                className={cn(
                  "flex items-center gap-2 rounded-lg border px-3 py-2 text-left text-xs font-medium transition",
                  isActive
                    ? "border-accent-blue/30 bg-accent-blue/12 text-ink-primary"
                    : "border-transparent text-ink-secondary hover:bg-charcoal-800 hover:text-ink-primary"
                )}
              >
                <span className={isActive ? "text-accent-cyan" : "text-ink-muted"}>{s.icon}</span>
                {s.label}
              </button>
            );
          })}
        </nav>

        <div id={panelId} role="region" aria-label={`${sections.find((s) => s.id === section)?.label ?? "Settings"} settings`} className="min-w-0">
          {section === "general" && (
            <section>
              <SectionHeader title="General" description="Verified application identity and startup behaviour." />
              <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
                <KeyValue label="Application name" value="MultiNet CyberShield AI" />
                <KeyValue label="Application version" value={version} mono />
                <KeyValue label="Tagline" value="Connect. Protect. Detect." />
                <KeyValue label="Launch behavior" value="Integration pending" />
                <KeyValue label="Startup preferences" value="Integration pending" />
              </div>
              <div className="mt-3 grid gap-3 sm:grid-cols-2">
                <SelectField label="Language" options={["English (US)"]} disabled disabledReason="Localization requires an i18n resource pipeline — integration pending." hint="Additional languages arrive with the localization integration." />
                <SelectField label="Startup behavior" options={["Launch minimized", "Launch to tray"]} disabled disabledReason="Startup registration requires the OS integration layer — integration pending." hint="Options shown for shape only — integration pending." />
              </div>
            </section>
          )}

          {section === "appearance" && (
            <section>
              <SectionHeader title="Appearance" description="Theme and density. Nothing here is persisted yet." />
              <fieldset className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
                <legend className="sr-only">Theme preference</legend>
                <div className="text-xs font-semibold text-ink-primary">Theme</div>
                <p className="mt-0.5 text-[10px] text-ink-muted">Dark renders as-is. Light and System are previews — the theme engine is not implemented. No preference is saved.</p>
                <div className="mt-2 grid gap-2 sm:grid-cols-3" role="radiogroup" aria-label="Theme preview">
                  {(["dark", "light", "system"] as const).map((t) => {
                    const previewOnly = t !== "dark";
                    const checked = themePreview === t;
                    return (
                      <label
                        key={t}
                        title={previewOnly ? "Theme engine not implemented — preview only, not rendered" : "Currently rendering"}
                        className={cn(
                          "flex cursor-pointer items-center gap-2 rounded-lg border p-2.5 text-xs capitalize transition",
                          checked ? "border-accent-blue/40 bg-accent-blue/10 text-ink-primary" : "border-charcoal-600 text-ink-secondary",
                          previewOnly && "opacity-70"
                        )}
                      >
                        <input
                          type="radio"
                          name="theme-preview"
                          value={t}
                          checked={checked}
                          onChange={() => setThemePreview(t)}
                          className="accent-blue-500"
                        />
                        {t}{previewOnly ? " (preview)" : " (active)"}
                      </label>
                    );
                  })}
                </div>
              </fieldset>
              <div className="mt-3 rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
                <div className="flex items-center justify-between gap-4">
                  <div>
                    <div className="text-xs font-semibold text-ink-primary">Compact navigation</div>
                    <div className="text-[10px] text-ink-secondary">Working control — collapses the application sidebar immediately. Not persisted across restarts.</div>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={sidebarCollapsed}
                    aria-label="Toggle compact navigation"
                    onClick={toggleSidebar}
                    className={cn(
                      "relative h-6 w-11 shrink-0 rounded-full border transition",
                      sidebarCollapsed ? "border-accent-blue/50 bg-accent-blue/40" : "border-charcoal-600 bg-charcoal-800"
                    )}
                  >
                    <span className={cn("absolute top-0.5 h-4 w-4 rounded-full bg-ink-primary transition-all", sidebarCollapsed ? "left-6" : "left-1")} />
                  </button>
                </div>
                <div className="mt-3 border-t border-charcoal-600/60 pt-3">
                  <SelectField label="Interface density" options={["Comfortable", "Compact"]} disabled disabledReason="Density scaling requires a working theming engine — integration pending." hint="Preview only — no layout density is applied." />
                </div>
              </div>
            </section>
          )}

          {section === "network" && (
            <section>
              <SectionHeader title="Network" description="Future network preferences — this machine's networking is never read or modified." />
              <div className="grid gap-3 sm:grid-cols-2">
                <SelectField label="Preferred network interface" options={["Automatic (integration pending)"]} disabled disabledReason="Interface discovery requires the platform integration — no adapter is enumerated." hint="Options populate from the verified collector only." />
                <SelectField label="Connection monitoring" options={["Enabled when collector attaches"]} disabled disabledReason="Monitoring preferences require the network collector integration." hint="Integration pending." />
                <SelectField label="Bonding preferences" options={["Adaptive (preview)"]} disabled disabledReason="Bonding configuration requires the bonding engine integration." hint="Integration pending." />
                <SelectField label="Failover preferences" options={["Automatic (preview)"]} disabled disabledReason="Failover policy requires the bonding engine integration." hint="Integration pending." />
                <SelectField label="DNS protection preferences" options={["Disabled (no resolver integration)"]} disabled disabledReason="DNS protection requires the DNS module integration and approval policy." hint="Integration pending." />
              </div>
              <p className="mt-3 text-[10px] leading-4 text-ink-muted">No OS networking configuration is read, planned or changed by this screen. Do not use it to alter routing, DNS or adapters.</p>
            </section>
          )}

          {section === "security" && (
            <section>
              <SectionHeader title="Security" description="Monitoring preferences and approval requirements. Restrictions are never weakened." />
              <div className="rounded-xl2 border border-amber-900/30 bg-amber-900/5 p-4 text-[11px] leading-5 text-amber-200">
                Defensive actions — process termination, endpoint isolation, firewall rules, DNS changes, blocklists — always require validated evidence, structured SecurityPolicy checks, explicit human approval, authorized backend execution, an audit record and rollback where technically feasible. These settings cannot bypass that chain.
              </div>
              <div className="mt-3 grid gap-3 sm:grid-cols-2">
                <SelectField label="Security monitoring" options={["Off (no engine connected)"]} disabled disabledReason="Monitoring requires a verified detection engine." hint="Integration pending." />
                <SelectField label="Threat intelligence" options={["Off (no feed connected)"]} disabled disabledReason="Intelligence feeds require provider integration and human approval." hint="Integration pending." />
                <SelectField label="Endpoint monitoring" options={["Off (no agent onboarded)"]} disabled disabledReason="Endpoint monitoring requires the agent collector integration." hint="Integration pending." />
                <SelectField label="Firewall approvals" options={["Explicit approval required (fixed)"]} disabled disabledReason="Approval gating is mandatory and cannot be disabled from settings." hint="This requirement is not configurable." />
                <SelectField label="Incident response" options={["Approval-gated (fixed)"]} disabled disabledReason="Incident response execution requires the approved architecture." hint="Integration pending." />
                <SelectField label="Audit logging" options={["Not available (no audit sink)"]} disabled disabledReason="Audit logging requires the authorized backend audit pipeline." hint="Integration pending." />
              </div>
            </section>
          )}

          {section === "ai-provider" && (
            <section>
              <SectionHeader title="AI Provider" description="No AI provider is connected. Credential saving is disabled." action={<Badge tone="warn">not connected</Badge>} />
              <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
                <KeyValue label="Provider" value="Not configured" />
                <KeyValue label="Model" value="Not configured" />
                <KeyValue label="Connection status" value={<StatusIndicator variation="idle" label="Not connected" />} />
                <KeyValue label="API endpoint" value="Not configured" />
                <KeyValue label="Default AI model" value="Not configured" />
              </div>
              <div className="mt-3 rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
                <label htmlFor="ai-endpoint" className="mb-1 block text-xs font-medium text-ink-secondary">API endpoint</label>
                <input id="ai-endpoint" type="url" disabled aria-disabled="true" title="Provider configuration requires the secure integration." placeholder="https://provider.example/… (pending integration)" className="h-8 w-full rounded-lg border border-charcoal-600/60 bg-charcoal-800/40 px-2.5 text-xs text-ink-muted disabled:cursor-not-allowed" />
                <label htmlFor="ai-credential" className="mb-1 mt-3 block text-xs font-medium text-ink-secondary">Credential</label>
                <input id="ai-credential" type="password" autoComplete="off" disabled aria-disabled="true" title="Secure credential storage integration pending." placeholder="Secure credential storage integration pending" className="h-8 w-full rounded-lg border border-charcoal-600/60 bg-charcoal-800/40 px-2.5 text-xs text-ink-muted disabled:cursor-not-allowed" />
                <div className="mt-3 flex flex-wrap items-center gap-2">
                  <Button disabled disabledReason="Secure credential storage integration pending — credentials are never stored in this build (no localStorage, no bundle embedding, no logging).">Save credential</Button>
                  <Button disabled disabledReason="Test connection requires a stored credential and the provider integration.">Test connection</Button>
                </div>
                <p className="mt-3 text-[10px] leading-4 text-ink-muted">No API key is stored, printed, logged or embedded in the frontend bundle. No external provider is called. Secure OS keychain/DPAPI integration is pending; until it exists, credential saving and connection testing stay disabled.</p>
              </div>
            </section>
          )}

          {section === "notifications" && (
            <section>
              <SectionHeader title="Notifications" description="UI preview — this build sends no notifications." action={<Badge tone="neutral">preview</Badge>} />
              <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
                {[
                  ["Critical security alerts", "Requires the detection engine and notification service."],
                  ["High-severity events", "Requires the detection engine and notification service."],
                  ["Network connection changes", "Requires the network collector integration."],
                  ["Bonding status", "Requires the bonding engine integration."],
                  ["Incident approvals", "Requires the incident and approval pipeline."],
                  ["Application notifications", "Requires the OS notification integration."]
                ].map(([label, reason]) => (
                  <label key={label as string} title={reason as string} className="flex cursor-not-allowed items-center justify-between gap-4 border-b border-charcoal-600/60 py-2.5 text-xs text-ink-muted last:border-0">
                    <span>{label as string} <span className="text-[10px] text-ink-muted/70">(preview)</span></span>
                    <input type="checkbox" disabled aria-disabled="true" aria-label={label as string} className="accent-blue-500 disabled:cursor-not-allowed" />
                  </label>
                ))}
              </div>
              <p className="mt-3 text-[10px] leading-4 text-ink-muted">No notification is sent, queued or scheduled by this build. A working preference service and OS notification integration are required before these toggles become active.</p>
            </section>
          )}

          {section === "privacy" && (
            <section>
              <SectionHeader title="Privacy & Data" description="What is known about local data. Nothing is deleted from this screen." />
              <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
                <KeyValue label="Local database location" value={dbLocation ? <span className="break-all font-mono text-[10px]">{dbLocation}</span> : "Not available — backend bridge unavailable"} />
                <KeyValue label="Database engine" value="SQLite via SQLx" />
                <KeyValue label="Stored event data" value="Not available — no event persistence implemented in this phase" />
                <KeyValue label="Audit log location" value="Not available — no audit sink integrated" />
                <KeyValue label="Data retention" value="Not configured" />
                <KeyValue label="Telemetry policy" value="Not verified — no telemetry SDK is wired in this build; claims are not made" />
                <KeyValue label="AI data handling" value="No AI provider connected — no prompt or evidence leaves this device" />
                <KeyValue label="Export and deletion policy" value="Not available — export/delete requires the backend integration; the database is never modified by this screen" />
              </div>
              <p className="mt-3 text-[10px] leading-4 text-ink-muted">Paths shown above come from the backend bridge when connected; otherwise reported as unavailable. No secret or credential surface is exposed. This screen never deletes database files.</p>
            </section>
          )}

          {section === "about" && (
            <section>
              <SectionHeader title="About" description="Verified product facts and current capability status." />
              <div className="rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
                <div className="flex items-center gap-3 border-b border-charcoal-600/60 pb-3">
                  <span className="flex h-11 w-11 items-center justify-center rounded-xl2 border border-accent-blue/35 bg-gradient-to-b from-accent-blue/25 to-accent-indigo/10 text-accent-cyan">
                    <IconShield size={20} />
                  </span>
                  <div>
                    <div className="text-sm font-semibold text-ink-primary">MultiNet CyberShield AI</div>
                    <div className="text-[11px] text-accent-cyan/80">Connect. Protect. Detect.</div>
                  </div>
                </div>
                <div className="mt-2">
                  <KeyValue label="Version" value={version} mono />
                  <KeyValue label="Architecture" value="Tauri 2 + React + Rust" />
                  <KeyValue label="Database" value="SQLite + SQLx" />
                  <KeyValue label="Identifier" value="ai.multinet.cybershield" mono />
                </div>
              </div>
              <div className="mt-3 rounded-xl2 border border-charcoal-600 bg-elevated shadow-panel p-4">
                <div className="text-xs font-semibold text-ink-primary">Integration and platform compatibility</div>
                <ul className="mt-2 space-y-1.5 text-[11px] leading-4 text-ink-secondary">
                  <li className="flex items-center justify-between gap-3"><span>Windows desktop (primary target)</span><Badge tone="ok">supported build</Badge></li>
                  <li className="flex items-center justify-between gap-3"><span>Network / bonding / speed-test engines</span><Badge tone="warn">integration pending</Badge></li>
                  <li className="flex items-center justify-between gap-3"><span>Security detection &amp; intelligence</span><Badge tone="warn">integration pending</Badge></li>
                  <li className="flex items-center justify-between gap-3"><span>Endpoint / firewall / incident response</span><Badge tone="warn">integration pending</Badge></li>
                  <li className="flex items-center justify-between gap-3"><span>AI provider (copilot / recommendations)</span><Badge tone="warn">not connected</Badge></li>
                  <li className="flex items-center justify-between gap-3"><span>macOS / Linux desktop</span><Badge tone="neutral">not verified</Badge></li>
                </ul>
                <p className="mt-3 text-[10px] leading-4 text-ink-muted">Implemented screens are design surfaces unless marked otherwise. No planned capability is claimed operational without a verified, connected backend.</p>
              </div>
            </section>
          )}
        </div>
      </div>
    </PageContainer>
  );
}
