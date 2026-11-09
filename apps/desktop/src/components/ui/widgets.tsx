import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export type Activity = {
  title: string;
  description: string;
  time: string;
  tone?: "info" | "warn" | "danger";
};

type IconButtonProps = {
  icon: ReactNode;
  label: string;
  onClick?: () => void;
  active?: boolean;
  className?: string;
};

export function IconButton({ icon, label, onClick, active = false, className }: IconButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={label}
      aria-label={label}
      className={cn(
        "inline-flex h-8 w-8 items-center justify-center rounded-lg border text-ink-secondary transition",
        active
          ? "border-cyan-400/40 bg-cyan-400/12 text-accent-cyan shadow-glow"
          : "border-charcoal-600 bg-charcoal-800 hover:border-cyan-400/30 hover:bg-charcoal-750 hover:text-accent-cyan"
      )}
    >
      {icon}
    </button>
  );
}

type SearchInputProps = {
  value?: string;
  onChange?: (value: string) => void;
  placeholder?: string;
  className?: string;
};

export function SearchInput({ value, onChange, placeholder = "Search the workspace…", className }: SearchInputProps) {
  return (
    <label className={cn("relative flex h-8 w-72 items-center", className)}>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange?.(e.target.value)}
        placeholder={placeholder}
        aria-label="Global search"
          className="h-full w-full rounded-lg border border-charcoal-600 bg-field pl-8 pr-3 text-xs text-ink-primary placeholder:text-ink-muted focus:border-cyan-400/50"
      />
      <svg
        viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" strokeWidth={1.8}
        strokeLinecap="round" strokeLinejoin="round"
        className="pointer-events-none absolute left-2.5 text-ink-muted" aria-hidden="true"
      >
        <circle cx="11" cy="11" r="6.5" stroke="currentColor" /><path d="M20 20l-3.4-3.4" stroke="currentColor" />
      </svg>
    </label>
  );
}

type ActivityTimelineProps = {
  items: Activity[];
  emptyState: ReactNode;
};

export function ActivityTimeline({ items, emptyState }: ActivityTimelineProps) {
  if (items.length === 0) {
    return <>{emptyState}</>;
  }
  return (
    <ol className="space-y-3">
      {items.map((item) => (
        <li key={`${item.time}-${item.title}`} className="relative pl-6">
          <span
            className={cn(
              "absolute left-0 top-1 h-2.5 w-2.5 rounded-full border-2 border-elevated",
              item.tone === "danger" ? "bg-status-danger" : item.tone === "warn" ? "bg-status-warn" : "bg-accent-cyan shadow-glow"
            )}
          />
          {items.indexOf(item) < items.length - 1 && <span className="absolute left-[4.5px] top-4 h-full w-px bg-charcoal-600" />}
          <div className="text-xs font-medium text-ink-primary">{item.title}</div>
          <div className="text-xs text-ink-secondary">{item.description}</div>
          <div className="mt-0.5 font-mono text-[10px] text-ink-muted">{item.time}</div>
        </li>
      ))}
    </ol>
  );
}
