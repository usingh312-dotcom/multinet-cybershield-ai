import { useState } from "react";
import {
  defaultRouteFor,
  parentForRoute,
  sidebarSections,
  useAppStore,
  type NavParentId,
  type SidebarEntry,
  type SidebarEntryId
} from "../store/appStore";
import { useSystemStore } from "../store/systemStore";
import { cn } from "../lib/cn";
import {
  IconAnalytics, IconBonding, IconCheck, IconChevronDown, IconCopilot, IconDns, IconEndpoint,
  IconFirewall, IconIncident, IconNetworkManager, IconOverview, IconPanelLeft,
  IconReports, IconShield, IconSpeed, IconThreatDetection, IconThreatIntel, IconUser
} from "./ui/icons";
import { IconButton } from "./ui/widgets";
import { StatusIndicator } from "./ui/feedback";
import type { ReactNode } from "react";

const navIcons: Record<string, (p: { size?: number; className?: string }) => ReactNode> = {
  "overview": IconOverview,
  "network-manager": IconNetworkManager,
  "internet-bonding": IconBonding,
  "traffic-analytics": IconAnalytics,
  "speed-test": IconSpeed,
  "security-overview": IconShield,
  "threat-detection": IconThreatDetection,
  "threat-intelligence": IconThreatIntel,
  "endpoint-security": IconEndpoint,
  "dns-protection": IconDns,
  "ai-firewall": IconFirewall,
  "incident-response": IconIncident,
  "security-reports": IconReports,
  "ai-copilot": IconCopilot,
  "diagnostics": IconAnalytics,
  "security-center": IconShield,
  "incidents-reports": IconIncident
};

function ProductMark({ compact = false }: { compact?: boolean }) {
  return (
    <div className={cn("flex items-center gap-2.5 px-2 py-3", compact && "justify-center px-0")}>
      <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl2 border border-cyan-400/35 bg-gradient-to-b from-cyan-400/25 to-blue-700/20 text-accent-cyan shadow-glow">
        <IconShield size={18} />
      </span>
      {!compact && (
        <div className="leading-tight">
          <div className="flex items-center gap-1.5 text-sm font-semibold tracking-wide text-ink-primary">
            MultiNet CyberShield
            <span className="rounded bg-gradient-to-r from-blue-600 to-cyan-500 px-1 py-px text-[9px] font-bold text-white">AI</span>
          </div>
          <div className="text-[10px] text-accent-cyan/70">Secure. Smart. Always On.</div>
        </div>
      )}
    </div>
  );
}

export function ApplicationSidebar() {
  const { active, setActive, sidebarCollapsed, toggleSidebar } = useAppStore();
  const { foundation, database, error, loading } = useSystemStore();
  const activeParent = parentForRoute(active);
  const [expanded, setExpanded] = useState<SidebarEntryId[]>(activeParent ? [activeParent] : []);

  const isParentOpen = (entry: SidebarEntry) =>
    Boolean(entry.children) &&
    (expanded.includes(entry.id) || entry.children!.some((child) => child.id === active));

  const toggleParent = (id: SidebarEntryId) =>
    setExpanded((prev) => (prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id]));

  const selectParent = (entry: SidebarEntry) => {
    if (!expanded.includes(entry.id)) {
      setExpanded((prev) => [...prev, entry.id]);
    }
    setActive(defaultRouteFor(entry.id as NavParentId));
  };

  const allOk = Boolean(foundation) && Boolean(database?.migrations_applied);
  const operational = loading && allOk
    ? { title: "Connecting to backend…", detail: "" }
    : allOk
      ? { title: "All systems operational", detail: error ? "Bridge warning present" : "No issues detected" }
      : { title: "Limited backend services", detail: error ? error.slice(0, 60) : "Bonding engine not initialized" };

  return (
    <aside
      aria-label="Application sidebar"
      className={cn(
        "flex shrink-0 flex-col border-r border-charcoal-600 bg-charcoal-900 transition-[width] duration-150",
        sidebarCollapsed ? "w-16" : "w-60"
      )}
    >
      {sidebarCollapsed ? (
        <button type="button" onClick={toggleSidebar} className="py-2" title="Expand sidebar" aria-label="Expand sidebar">
          <ProductMark compact />
        </button>
      ) : (
        <ProductMark />
      )}

      <nav
        aria-label="Primary navigation"
        className={cn("min-h-0 flex-1 space-y-4 overflow-y-auto pb-2", sidebarCollapsed ? "px-1.5" : "px-2.5")}
      >
        {sidebarSections.map((section) => (
          <div key={section.label} role="group" aria-label={section.label}>
            {!sidebarCollapsed && (
              <div className="px-2 pb-1.5 text-[10px] font-semibold uppercase tracking-[0.18em] text-ink-muted">
                {section.label}
              </div>
            )}
            <ul className="space-y-0.5">
              {section.items.map((entry) => {
                const Icon = navIcons[entry.id];
                const hasChildren = Boolean(entry.children?.length);
                const isActive = active === entry.id;
                const isParentActive = Boolean(entry.children?.some((child) => child.id === active));
                const highlighted = isActive || isParentActive;
                const open = isParentOpen(entry);
                const childrenId = `sidebar-children-${entry.id}`;

                return (
                  <li key={entry.id}>
                    <div className="flex items-center gap-0.5">
                      <button
                        type="button"
                        onClick={() => (hasChildren ? selectParent(entry) : setActive(entry.id as typeof active))}
                        title={sidebarCollapsed ? entry.label : undefined}
                        aria-current={highlighted ? "page" : undefined}
                        aria-expanded={hasChildren ? open : undefined}
                        aria-controls={hasChildren ? childrenId : undefined}
                        className={cn(
                          "flex min-w-0 flex-1 items-center gap-2.5 rounded-lg px-2 py-1.5 text-sm transition",
                          highlighted
                            ? "border border-cyan-400/40 bg-gradient-to-r from-cyan-400/15 via-blue-600/10 to-transparent text-ink-primary"
                            : "border border-transparent text-ink-secondary hover:bg-charcoal-800 hover:text-ink-primary focus:bg-charcoal-800",
                          entry.primary &&
                            (highlighted
                              ? "border-cyan-400/50 shadow-glow"
                              : "text-ink-primary"),
                          !sidebarCollapsed && highlighted && "shadow-glow"
                        )}
                      >
                        {Icon && (
                          <span
                            className={cn(
                              "shrink-0",
                              highlighted || entry.primary
                                ? entry.primary && highlighted ? "text-accent-neon" : "text-accent-cyan"
                                : "text-ink-muted"
                            )}
                          >
                            {Icon({})}
                          </span>
                        )}
                        {!sidebarCollapsed && <span className="truncate">{entry.label}</span>}
                      </button>

                      {hasChildren && !sidebarCollapsed && (
                        <button
                          type="button"
                          onClick={() => toggleParent(entry.id)}
                          aria-label={`${open ? "Collapse" : "Expand"} ${entry.label}`}
                          aria-expanded={open}
                          aria-controls={childrenId}
                          className="rounded-md p-1.5 text-ink-muted transition hover:bg-charcoal-800 hover:text-ink-primary focus:bg-charcoal-800"
                        >
                          <IconChevronDown
                            size={14}
                            className={cn("transition-transform", open && "rotate-180")}
                          />
                        </button>
                      )}
                    </div>

                    {hasChildren && !sidebarCollapsed && open && (
                      <ul id={childrenId} aria-label={`${entry.label} sections`} className="ml-3 mt-0.5 space-y-0.5 border-l border-charcoal-600/70 pl-2">
                        {entry.children!.map((child) => {
                          const ChildIcon = navIcons[child.id];
                          const childActive = active === child.id;
                          return (
                            <li key={child.id}>
                              <button
                                type="button"
                                onClick={() => setActive(child.id)}
                                aria-current={childActive ? "page" : undefined}
                                className={cn(
                                  "flex w-full items-center gap-2 rounded-md px-2 py-1 text-[13px] transition",
                                  childActive
                                    ? "bg-cyan-400/12 text-ink-primary"
                                    : "text-ink-secondary hover:bg-charcoal-800 hover:text-ink-primary focus:bg-charcoal-800"
                                )}
                              >
                                {ChildIcon && (
                                  <span className={cn("shrink-0", childActive ? "text-accent-cyan" : "text-ink-muted")}>
                                    {ChildIcon({ size: 14 })}
                                  </span>
                                )}
                                <span className="truncate">{child.label}</span>
                              </button>
                            </li>
                          );
                        })}
                      </ul>
                    )}
                  </li>
                );
              })}
            </ul>
          </div>
        ))}
      </nav>

      <div
        className={cn(
          "flex flex-col gap-2 border-t border-charcoal-600 p-2",
          sidebarCollapsed && "items-center"
        )}
      >
        {!sidebarCollapsed && (
          <div className="rounded-lg border border-charcoal-600 bg-charcoal-800 px-2.5 py-2">
            <div className="flex items-center gap-2 text-[11px] font-medium text-ink-primary">
              <span className="flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-status-ok/15 text-status-ok" aria-hidden="true">
                <IconCheck size={11} />
              </span>
              <span className="truncate">{operational.title}</span>
            </div>
            <p className="mt-0.5 truncate pl-6 text-[10px] text-ink-muted">
              {operational.detail || "No issues detected"}
            </p>
            <div className="mt-1.5 flex items-center justify-between pl-6 text-[10px] text-ink-muted">
              <span className="font-mono">v{foundation?.version ?? "—"}</span>
              <StatusIndicator
                variation={allOk ? "ok" : "warn"}
                label={allOk ? "Core online" : "Core limited"}
                className="!text-[9px] leading-none"
              />
            </div>
          </div>
        )}

        <div className="flex w-full items-center justify-between gap-1">
          <button
            type="button"
            onClick={() => setActive("settings")}
            aria-current={active === "settings" ? "page" : undefined}
            title="Desktop Operator"
            className={cn(
              "flex min-w-0 items-center gap-2 rounded-lg px-2 py-1.5 text-xs transition",
              active === "settings"
                ? "bg-cyan-400/12 text-ink-primary"
                : "text-ink-secondary hover:text-ink-primary focus:text-ink-primary",
              sidebarCollapsed && "justify-center"
            )}
          >
            <IconUser />
            {!sidebarCollapsed && <span className="truncate">Desktop Operator</span>}
          </button>
          <IconButton
            icon={<IconPanelLeft size={15} />}
            label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            onClick={toggleSidebar}
            active={sidebarCollapsed}
          />
        </div>
      </div>
    </aside>
  );
}
