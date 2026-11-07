import { create } from "zustand";

export type NavId =
  | "overview" | "network-manager" | "internet-bonding" | "traffic-analytics" | "speed-test"
  | "security-overview" | "threat-detection" | "threat-intelligence" | "endpoint-security" | "dns-protection"
  | "ai-firewall" | "incident-response" | "security-reports" | "ai-copilot" | "settings";

export const navItems: { id: NavId; label: string; group: string }[] = [
  { id: "overview", label: "Overview", group: "Network" },
  { id: "network-manager", label: "Network Manager", group: "Network" },
  { id: "internet-bonding", label: "Internet Bonding", group: "Network" },
  { id: "traffic-analytics", label: "Traffic Analytics", group: "Network" },
  { id: "speed-test", label: "Speed Test", group: "Network" },
  { id: "security-overview", label: "Security Overview", group: "Security" },
  { id: "threat-detection", label: "Threat Detection", group: "Security" },
  { id: "threat-intelligence", label: "Threat Intelligence", group: "Security" },
  { id: "endpoint-security", label: "Endpoint Security", group: "Security" },
  { id: "dns-protection", label: "DNS Protection", group: "Security" },
  { id: "ai-firewall", label: "AI Firewall", group: "Security" },
  { id: "incident-response", label: "Incident Response", group: "Security" },
  { id: "security-reports", label: "Security Reports", group: "Security" },
  { id: "ai-copilot", label: "AI Copilot", group: "AI" },
  { id: "settings", label: "Settings", group: "System" }
];

type State = { active: NavId; setActive: (id: NavId) => void };
export const useAppStore = create<State>((set) => ({ active: "overview", setActive: (active) => set({ active }) }));
