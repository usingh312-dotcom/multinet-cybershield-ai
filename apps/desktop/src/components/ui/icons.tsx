import type { ReactNode } from "react";

type IconProps = {
  className?: string;
  size?: number;
};

function Svg({ children, size = 16, className = "" }: IconProps & { children: ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export const IconOverview = (p: IconProps) => (
  <Svg {...p}><rect x="3" y="3" width="7" height="9" rx="1.5" /><rect x="14" y="3" width="7" height="5" rx="1.5" /><rect x="14" y="12" width="7" height="9" rx="1.5" /><rect x="3" y="16" width="7" height="5" rx="1.5" /></Svg>
);
export const IconNetworkManager = (p: IconProps) => (
  <Svg {...p}><circle cx="12" cy="4.5" r="2.2" /><circle cx="4.5" cy="18" r="2.2" /><circle cx="19.5" cy="18" r="2.2" /><path d="M11 6.5 5.6 16M13 6.5l5.4 9.5M6.7 18h10.6" /></Svg>
);
export const IconBonding = (p: IconProps) => (
  <Svg {...p}><path d="M9.5 14.5 14.5 9.5" /><path d="M7 11 5 13a3.5 3.5 0 0 0 5 5l2-2" /><path d="M17 13l2-2a3.5 3.5 0 0 0-5-5l-2 2" /></Svg>
);
export const IconAnalytics = (p: IconProps) => (
  <Svg {...p}><path d="M4 19V5" /><path d="M4 19h16" /><path d="M7.5 15.5 11 11l3 2.5 4.5-6" /></Svg>
);
export const IconSpeed = (p: IconProps) => (
  <Svg {...p}><path d="M5 17a8 8 0 1 1 14 0" /><path d="M12 17l3.5-5" /><circle cx="12" cy="17" r="1.4" /></Svg>
);
export const IconShield = (p: IconProps) => (
  <Svg {...p}><path d="M12 3l7 2.5v5.6c0 4.6-3 7.6-7 9.4-4-1.8-7-4.8-7-9.4V5.5L12 3Z" /></Svg>
);
export const IconThreatDetection = (p: IconProps) => (
  <Svg {...p}><circle cx="11" cy="11" r="6.5" /><path d="M20 20l-3.4-3.4" /><path d="M11 8.2v3.4l2.2 1.3" /></Svg>
);
export const IconThreatIntel = (p: IconProps) => (
  <Svg {...p}><circle cx="12" cy="12" r="8.5" /><path d="M3.5 12h17" /><path d="M12 3.5c2.6 2.3 3.9 5.2 3.9 8.5s-1.3 6.2-3.9 8.5c-2.6-2.3-3.9-5.2-3.9-8.5s1.3-6.2 3.9-8.5Z" /></Svg>
);
export const IconEndpoint = (p: IconProps) => (
  <Svg {...p}><rect x="4" y="4" width="16" height="16" rx="2.5" /><path d="M10 4V2.8M14 4V2.8M10 21.2V20M14 21.2V20M4 10H2.8M4 14H2.8M21.2 10H20M21.2 14H20M10 12h.01M14 12h.01" /><circle cx="12" cy="12" r="1.6" /></Svg>
);
export const IconDns = (p: IconProps) => (
  <Svg {...p}><path d="M17 4l3 3-3 3" /><path d="M20 7h-7a6 6 0 0 0-6 6v0" /><path d="M7 20l-3-3 3-3" /><path d="M4 17h7a6 6 0 0 0 6-6v0" /></Svg>
);
export const IconFirewall = (p: IconProps) => (
  <Svg {...p}><path d="M4 6h16M4 12h16M4 18h16" /><path d="M8 6v-2M12 12v-2M10 18v-2" /><path d="M16 6v3M18 12v2M15 18v3" /></Svg>
);
export const IconIncident = (p: IconProps) => (
  <Svg {...p}><path d="M12 4a6 6 0 0 1 6 6v3l1.6 3.3a1 1 0 0 1-.9 1.4H5.3a1 1 0 0 1-.9-1.4L6 13v-3a6 6 0 0 1 6-6Z" /><path d="M10 19.5a2 2 0 0 0 4 0" /><path d="M12 4V2.5" /></Svg>
);
export const IconReports = (p: IconProps) => (
  <Svg {...p}><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8l-5-5Z" /><path d="M14 3v5h5" /><path d="M9 13h6M9 17h4" /></Svg>
);
export const IconCopilot = (p: IconProps) => (
  <Svg {...p}><rect x="5" y="7" width="14" height="12" rx="3" /><path d="M12 7V4.5" /><circle cx="9" cy="13" r="0.6" fill="currentColor" /><circle cx="15" cy="13" r="0.6" fill="currentColor" /><path d="M9.5 16.5h5" /></Svg>
);
export const IconSettings = (p: IconProps) => (
  <Svg {...p}><circle cx="12" cy="12" r="3" /><path d="M19 12c0-.5.3-1 .3-1.5a7.3 7.3 0 0 0-.8-1.5l.3-1.2a1 1 0 0 0-1-1l-1.2.2A7 7 0 0 0 15 6.2l-.2-1.2a1 1 0 0 0-1-1L12.6 4a7 7 0 0 0-1.4 0l-1.1.1a1 1 0 0 0-1 1L8.9 6.2a7 7 0 0 0-1.5.9L6.2 7a1 1 0 0 0-1 1l.3 1.2a7.3 7.3 0 0 0 0 2.8l-.3 1.2a1 1 0 0 0 1 1l1.2-.2c.5.4.9.7 1.5.9l.2 1.2a1 1 0 0 0 1 1l1.1.1c.5.2.9.2 1.4 0l1.2-.1a1 1 0 0 0 1-1l.2-1.2c.6-.2 1-.5 1.5-.9l1.2.2a1 1 0 0 0 1-1l-.3-1.2c.4-.5.7-.9.8-1.5z" transform="translate(0.5 0.5)" /></Svg>
);
export const IconSearch = (p: IconProps) => (
  <Svg {...p}><circle cx="11" cy="11" r="6.5" /><path d="M20 20l-3.4-3.4" /></Svg>
);
export const IconBell = (p: IconProps) => (
  <Svg {...p}><path d="M18 16v-5a6 6 0 1 0-12 0v5l-1.5 2.5h15L18 16Z" /><path d="M10 20.5a2 2 0 0 0 4 0" /></Svg>
);
export const IconUser = (p: IconProps) => (
  <Svg {...p}><circle cx="12" cy="8.5" r="3.5" /><path d="M5 20a7 7 0 0 1 14 0" /></Svg>
);
export const IconChevronDown = (p: IconProps) => (
  <Svg {...p}><path d="M6 9l6 6 6-6" /></Svg>
);
export const IconPanelLeft = (p: IconProps) => (
  <Svg {...p}><rect x="3.5" y="4" width="17" height="16" rx="2.5" /><path d="M9.5 4v16" /></Svg>
);
export const IconActivity = (p: IconProps) => (
  <Svg {...p}><path d="M3 12h4l2.5-6 5 12L17 12h4" /></Svg>
);
export const IconDatabase = (p: IconProps) => (
  <Svg {...p}><ellipse cx="12" cy="5.5" rx="7" ry="2.8" /><path d="M5 5.5V18.5c0 1.6 3.1 2.8 7 2.8s7-1.2 7-2.8V5.5" /><path d="M5 12c0 1.6 3.1 2.8 7 2.8s7-1.2 7-2.8" /></Svg>
);
export const IconAlert = (p: IconProps) => (
  <Svg {...p}><path d="M12 4l8.5 15h-17L12 4Z" /><path d="M12 10.5v3.5M12 17h.01" /></Svg>
);
export const IconInfo = (p: IconProps) => (
  <Svg {...p}><circle cx="12" cy="12" r="8.5" /><path d="M12 11v5M12 7.5v.2" /></Svg>
);
export const IconArrowRight = (p: IconProps) => (
  <Svg {...p}><path d="M5 12h13M13 6.5 18.5 12 13 17.5" /></Svg>
);
export const IconCheck = (p: IconProps) => (
  <Svg {...p}><path d="M5 12.5 10 17.5 19 7" /></Svg>
);
export const IconLock = (p: IconProps) => (
  <Svg {...p}><rect x="5.5" y="10.5" width="13" height="9" rx="2" /><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5" /><path d="M12 14v2.5" /></Svg>
);
export const IconWifi = (p: IconProps) => (
  <Svg {...p}><path d="M4 9.5a12 12 0 0 1 16 0" /><path d="M7 13a8 8 0 0 1 10 0" /><path d="M10 16.5a4 4 0 0 1 4 0" /><path d="M12 20h.01" /></Svg>
);
export const IconX = (p: IconProps) => (
  <Svg {...p}><path d="M6 6l12 12M18 6 6 18" /></Svg>
);
