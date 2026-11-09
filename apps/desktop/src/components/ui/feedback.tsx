import type { ReactNode } from "react";
import { cn } from "../../lib/cn";
import { IconAlert, IconCheck, IconInfo } from "./icons";

export type StatusVariation = "ok" | "idle" | "warn" | "danger";

const variationStyles: Record<StatusVariation, { dot: string; label: string }> = {
  ok: { dot: "bg-accent-neon shadow-glowGreen", label: "text-emerald-300" },
  idle: { dot: "bg-status-idle", label: "text-ink-secondary" },
  warn: { dot: "bg-status-warn", label: "text-amber-300" },
  danger: { dot: "bg-status-danger", label: "text-red-300" }
};

type StatusIndicatorProps = {
  variation: StatusVariation;
  label: string;
  pulse?: boolean;
  className?: string;
};

export function StatusIndicator({ variation, label, pulse = false, className }: StatusIndicatorProps) {
  const s = variationStyles[variation];
  return (
    <span className={cn("inline-flex items-center gap-1.5 text-xs font-medium", s.label, className)} title={label}>
      <span aria-hidden="true" className={cn("h-2 w-2 rounded-full relative", s.dot)}>
        {pulse && <span className={cn("absolute inset-0 rounded-full animate-ping opacity-40", s.dot)} />}
      </span>
      {label}
    </span>
  );
}

type StatusCardProps = {
  title: string;
  value: string;
  sub?: string;
  variation: StatusVariation;
  hint?: string;
  icon?: ReactNode;
};

export function StatusCard({ title, value, sub, variation, hint, icon }: StatusCardProps) {
  return (
    <div className="flex flex-col gap-2 rounded-xl3 border border-charcoal-600 bg-surface p-3.5 shadow-panel transition hover:border-cyan-400/25">
      <div className="flex items-center justify-between gap-2">
        <span className="text-[10px] font-semibold uppercase tracking-[0.16em] text-ink-muted">{title}</span>
        {icon && <span className="text-ink-muted">{icon}</span>}
      </div>
      <div className="flex items-baseline gap-2">
        <div className="flex flex-col gap-1">
          <span className="text-base font-semibold text-ink-primary">{value}</span>
          {sub && <span className="text-[10px] leading-none text-ink-muted">{sub}</span>}
        </div>
        <StatusIndicator variation={variation} label={variationLabel(variation)} />
      </div>
      {hint && <p className="text-[11px] leading-5 text-ink-secondary">{hint}</p>}
    </div>
  );
}

function variationLabel(variation: StatusVariation): string {
  switch (variation) {
    case "ok": return "Active";
    case "warn": return "Attention";
    case "danger": return "Fault";
    default: return "Idle";
  }
}

const statusIconByVariation: Record<StatusVariation, ReactNode> = {
  ok: <IconCheck size={18} className="text-accent-neon" />,
  warn: <IconAlert size={18} className="text-status-warn" />,
  danger: <IconAlert size={18} className="text-status-danger" />,
  idle: <IconInfo size={18} className="text-ink-muted" />
};

type EngineStatusCardProps = {
  name: string;
  badge: string;
  badgeVariation: StatusVariation;
  description: string;
  points: string[];
};

export function EngineStatusCard({ name, badge, badgeVariation, description, points }: EngineStatusCardProps) {
  return (
    <div className="flex flex-col gap-2.5 rounded-xl3 border border-charcoal-600 bg-surface p-4 shadow-panel transition hover:border-cyan-400/25">
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-semibold text-ink-primary">{name}</span>
        <StatusIndicator variation={badgeVariation} label={badge} />
      </div>
      <div className="flex gap-2">{statusIconByVariation[badgeVariation]}</div>
      <p className="text-xs leading-5 text-ink-secondary">{description}</p>
      <ul className="mt-0.5 space-y-1">
        {points.map((pt) => (
          <li key={pt} className="flex gap-2 text-xs leading-5 text-ink-secondary">
            <span aria-hidden="true" className="mt-[7px] h-1 w-1 shrink-0 rounded-full bg-accent-cyan/80 shadow-glow" />
            {pt}
          </li>
        ))}
      </ul>
    </div>
  );
}

type EmptyStateProps = {
  icon?: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
};

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex h-full min-h-[140px] flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-cyan-400/20 bg-charcoal-900/40 px-6 py-8 text-center">
      {icon && <div className="rounded-xl2 border border-cyan-400/25 bg-cyan-400/8 p-2.5 text-accent-cyan">{icon}</div>}
      <div className="text-sm font-medium text-ink-primary">{title}</div>
      <p className="max-w-md text-xs leading-5 text-ink-secondary">{description}</p>
      {action}
    </div>
  );
}
