import type { ReactNode } from "react";
import { cn } from "../../lib/cn";
import { IconInfo } from "./icons";

const variants = {
  primary: "border-transparent bg-gradient-to-r from-blue-700 via-accent-blue to-cyan-500 text-white shadow-glow hover:brightness-110 focus:brightness-110",
  secondary: "border-charcoal-600 bg-charcoal-800 text-ink-primary hover:bg-charcoal-750 focus:bg-charcoal-750",
  ghost: "border-transparent bg-transparent text-ink-secondary hover:text-accent-cyan focus:text-accent-cyan"
} as const;

type ButtonProps = {
  children: ReactNode;
  onClick?: () => void;
  variant?: keyof typeof variants;
  disabled?: boolean;
  disabledReason?: string;
  icon?: ReactNode;
  className?: string;
};

export function Button({ children, onClick, variant = "secondary", disabled = false, disabledReason, icon, className }: ButtonProps) {
  const effectiveVariant = disabled ? variant === "primary" ? "secondary" : variant : variant;
  return (
    <button
      type="button"
      onClick={disabled ? undefined : onClick}
      aria-disabled={disabled}
      title={disabled ? disabledReason : undefined}
      className={cn(
        "inline-flex h-8 items-center gap-1.5 rounded-lg border px-3 text-xs font-medium transition disabled:cursor-not-allowed",
        variants[effectiveVariant as keyof typeof variants],
        className,
        disabled && "cursor-not-allowed border-charcoal-600 bg-charcoal-800/50 text-ink-muted opacity-70"
      )}
    >
      {icon}
      {children}
    </button>
  );
}

type BadgeTone = "neutral" | "ok" | "warn" | "danger" | "accent";

const badgeTones: Record<BadgeTone, string> = {
  neutral: "border-charcoal-600 bg-charcoal-800 text-ink-secondary",
  ok: "border-emerald-900/60 bg-emerald-900/30 text-emerald-300",
  warn: "border-amber-900/60 bg-amber-900/25 text-amber-300",
  danger: "border-red-900/60 bg-red-900/25 text-red-300",
  accent: "border-cyan-400/40 bg-cyan-400/10 text-cyan-300"
};

export function Badge({ children, tone = "neutral", className }: { children: ReactNode; tone?: BadgeTone; className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider", badgeTones[tone], className)}>
      {children}
    </span>
  );
}

type KeyValueProps = {
  label: string;
  value: ReactNode;
  mono?: boolean;
};

export function KeyValue({ label, value, mono = false }: KeyValueProps) {
  return (
    <div className="flex items-baseline justify-between gap-6 border-b border-charcoal-600/60 py-2 last:border-0">
      <span className="text-xs text-ink-secondary">{label}</span>
      <span className={cn("text-xs text-ink-primary", mono && "font-mono")}>{value}</span>
    </div>
  );
}

type SelectFieldProps = {
  label: string;
  options: string[];
  disabled: boolean;
  disabledReason?: string;
  hint?: string;
};

export function SelectField({ label, options, disabled, disabledReason, hint }: SelectFieldProps) {
  return (
    <div>
      <label className="mb-1 block text-xs font-medium text-ink-secondary">{label}</label>
      <div className="relative">
        <select
          disabled={disabled}
          aria-disabled={disabled}
          title={disabled ? disabledReason : undefined}
          className={cn(
            "h-8 w-full appearance-none rounded-lg border border-charcoal-600 bg-field pl-2.5 pr-7 text-xs text-ink-primary disabled:cursor-not-allowed disabled:border-charcoal-600/60 disabled:bg-charcoal-800/40 disabled:text-ink-muted"
          )}
        >
          {/* Disabled selects announce availability only — no values are stored until backend integration exists. */}
          {options.map((opt) => (
            <option key={opt} value={opt}>{opt}{disabled ? " — pending" : ""}</option>
          ))}
        </select>
        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-ink-muted" aria-hidden="true">
          <path d="M6 9l6 6 6-6" />
        </svg>
      </div>
      {hint && <p className="mt-1 text-[10px] leading-4 text-ink-muted">{hint}</p>}
    </div>
  );
}

export function IntegrationNotice({ children }: { children: ReactNode }) {
  return (
    <div className="flex items-start gap-2.5 rounded-xl3 border border-charcoal-600 bg-surface px-4 py-3 text-xs leading-5 text-ink-secondary shadow-panel">
      <span className="mt-0.5 rounded-lg border border-cyan-400/25 bg-cyan-400/8 p-1.5 text-accent-cyan"><IconInfo size={13} /></span>
      <div>{children}</div>
    </div>
  );
}
