import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

type SectionHeaderProps = {
  title: string;
  description?: string;
  action?: ReactNode;
};

export function SectionHeader({ title, description, action }: SectionHeaderProps) {
  return (
    <div className="mb-3 flex items-end justify-between gap-4">
      <div>
        <h2 className="flex items-center gap-2 text-sm font-semibold tracking-wide text-ink-primary">
          <span aria-hidden="true" className="h-3.5 w-0.5 rounded-full bg-gradient-to-b from-accent-cyan to-blue-500 drop-shadow-[0_0_4px_rgba(0,229,255,0.6)]" />
          {title}
        </h2>
        {description && <p className="mt-0.5 pl-3 text-xs text-ink-secondary">{description}</p>}
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </div>
  );
}

type PageHeaderProps = {
  title: string;
  subtitle: string;
  actions?: ReactNode;
  icon?: ReactNode;
};

export function PageHeader({ title, subtitle, actions, icon }: PageHeaderProps) {
  return (
    <div className="relative flex flex-wrap items-end justify-between gap-3 rounded-xl3 border border-charcoal-600 bg-surface px-5 py-4 shadow-panel">
      <div className="pointer-events-none absolute inset-y-0 left-0 w-1 rounded-l-xl3 bg-gradient-to-b from-accent-cyan via-blue-500 to-transparent drop-shadow-[0_0_6px_rgba(0,229,255,0.55)]" />
      <div className="pointer-events-none absolute -right-16 -top-16 h-40 w-40 rounded-full bg-accent-cyan/6 blur-3xl" />
      <div className="relative flex items-start gap-3">
        {icon && (
          <span className="mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl2 border border-cyan-400/30 bg-gradient-to-b from-cyan-400/15 to-blue-700/10 text-accent-cyan shadow-glow">
            {icon}
          </span>
        )}
        <div>
          <h1 className={cn("font-semibold tracking-tight text-ink-primary", icon ? "text-xl" : "text-xl")}>{title}</h1>
          <p className="mt-1 max-w-2xl text-sm text-ink-secondary">{subtitle}</p>
        </div>
      </div>
      {actions && <div className="relative flex items-center gap-2">{actions}</div>}
    </div>
  );
}

type PageContainerProps = {
  children: ReactNode;
  className?: string;
};

export function PageContainer({ children, className }: PageContainerProps) {
  return (
    <div className={cn("flex w-full max-w-6xl flex-col gap-7 p-6", className)}>
      {children}
    </div>
  );
}
