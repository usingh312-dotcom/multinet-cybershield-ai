import { create } from "zustand";

export type NavId =
  | "overview" | "network-manager" | "internet-bonding" | "traffic-analytics" | "speed-test"
  | "security-overview" | "threat-detection" | "threat-intelligence" | "endpoint-security" | "dns-protection"
  | "ai-firewall" | "incident-response" | "security-reports" | "ai-copilot" | "settings";

/** Consolidated sidebar parents that group several existing routes. */
export type NavParentId = "diagnostics" | "security-center" | "incidents-reports";
export type SidebarEntryId = NavId | NavParentId;

export type NavChild = { id: NavId; label: string };
export type SidebarEntry = {
  id: SidebarEntryId;
  label: string;
  /** Visually emphasized primary destination. */
  primary?: boolean;
  children?: NavChild[];
};
export type SidebarSection = { label: string; items: SidebarEntry[] };

/** Consolidated modules: the first child is the default route. */
export const moduleGroups: { parent: NavParentId; label: string; items: NavChild[] }[] = [
  {
    parent: "diagnostics",
    label: "Diagnostics",
    items: [
      { id: "traffic-analytics", label: "Traffic Analytics" },
      { id: "speed-test", label: "Speed Test" }
    ]
  },
  {
    parent: "security-center",
    label: "Security Center",
    items: [
      { id: "security-overview", label: "Security Overview" },
      { id: "threat-detection", label: "Threat Detection" },
      { id: "threat-intelligence", label: "Threat Intelligence" },
      { id: "endpoint-security", label: "Endpoint Security" },
      { id: "dns-protection", label: "DNS Protection" },
      { id: "ai-firewall", label: "AI Firewall" }
    ]
  },
  {
    parent: "incidents-reports",
    label: "Incidents & Reports",
    items: [
      { id: "incident-response", label: "Incident Response" },
      { id: "security-reports", label: "Security Reports" }
    ]
  }
];

/** Sidebar structure: primary destinations plus consolidated modules. */
export const sidebarSections: SidebarSection[] = [
  {
    label: "Network",
    items: [
      { id: "overview", label: "Network Overview" },
      { id: "network-manager", label: "Network Manager" },
      { id: "internet-bonding", label: "Internet Bonding", primary: true },
      { id: "diagnostics", label: "Diagnostics", children: moduleGroups[0].items }
    ]
  },
  {
    label: "Protection",
    items: [
      { id: "security-center", label: "Security Center", children: moduleGroups[1].items },
      { id: "incidents-reports", label: "Incidents & Reports", children: moduleGroups[2].items }
    ]
  },
  {
    label: "Intelligence",
    items: [{ id: "ai-copilot", label: "AI Copilot" }]
  }
];

/** Flat route list retained for page titles, breadcrumbs and deep links. */
export const navItems: { id: NavId; label: string; group: string }[] = [
  { id: "overview", label: "Overview", group: "Network" },
  { id: "network-manager", label: "Network Manager", group: "Network" },
  { id: "internet-bonding", label: "Internet Bonding", group: "Network" },
  { id: "traffic-analytics", label: "Traffic Analytics", group: "Diagnostics" },
  { id: "speed-test", label: "Speed Test", group: "Diagnostics" },
  { id: "security-overview", label: "Security Overview", group: "Security Center" },
  { id: "threat-detection", label: "Threat Detection", group: "Security Center" },
  { id: "threat-intelligence", label: "Threat Intelligence", group: "Security Center" },
  { id: "endpoint-security", label: "Endpoint Security", group: "Security Center" },
  { id: "dns-protection", label: "DNS Protection", group: "Security Center" },
  { id: "ai-firewall", label: "AI Firewall", group: "Security Center" },
  { id: "incident-response", label: "Incident Response", group: "Incidents & Reports" },
  { id: "security-reports", label: "Security Reports", group: "Incidents & Reports" },
  { id: "ai-copilot", label: "AI Copilot", group: "Intelligence" },
  { id: "settings", label: "Settings", group: "System" }
];

/** Returns the consolidated module that owns a route, when applicable. */
export function moduleForRoute(id: NavId) {
  return moduleGroups.find((group) => group.items.some((item) => item.id === id));
}

/** Returns the sidebar parent that owns a route, when applicable. */
export function parentForRoute(id: NavId): NavParentId | undefined {
  return moduleForRoute(id)?.parent;
}

/** First route of a consolidated module (used when a parent row is clicked). */
export function defaultRouteFor(parent: NavParentId): NavId {
  return moduleGroups.find((group) => group.parent === parent)!.items[0].id;
}

export type State = { active: NavId; setActive: (id: NavId) => void; sidebarCollapsed: boolean; toggleSidebar: () => void };
export const useAppStore = create<State>((set) => ({
  // Internet Bonding is the primary destination highlighted on launch.
  active: "internet-bonding",
  setActive: (active) => set({ active }),
  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed }))
}));
