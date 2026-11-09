import { useEffect, useRef, useState } from "react";
import { navItems, useAppStore } from "../store/appStore";
import { useSystemStore } from "../store/systemStore";
import { ApplicationHeader } from "./ApplicationHeader";
import { ApplicationSidebar } from "./ApplicationSidebar";
import { ModuleTabs } from "./ModuleTabs";

function useSessionUptime(): string {
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    const startedAt = Date.now();
    const timer = window.setInterval(() => setSeconds(Math.floor((Date.now() - startedAt) / 1000)), 1000);
    return () => window.clearInterval(timer);
  }, []);
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(h)}:${pad(m)}:${pad(s)}`;
}

export function AppLayout({ children }: { children: React.ReactNode }) {
  const { active } = useAppStore();
  const { foundation, database, error } = useSystemStore();
  const item = navItems.find((n) => n.id === active);
  const mainRef = useRef<HTMLElement>(null);
  const uptime = useSessionUptime();

  // Route changes must not strand keyboard focus on the previous page.
  useEffect(() => {
    mainRef.current?.focus({ preventScroll: true });
  }, [item?.id]);

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-charcoal-950 text-ink-primary">
      <div className="flex min-h-0 flex-1 overflow-hidden">
        <ApplicationSidebar />
        <div className="flex min-w-0 flex-1 flex-col">
          <ApplicationHeader />
          <main
            ref={mainRef}
            tabIndex={-1}
            role="main"
            aria-label={`${item?.label ?? "Overview"} screen`}
            className="relative min-h-0 flex-1 overflow-y-auto bg-charcoal-950 outline-none"
          >
            <div className="pointer-events-none fixed left-0 top-0 h-56 w-[46rem] bg-gradient-to-b from-accent-cyan/7 to-transparent" />
            <ModuleTabs />
            <div className="relative inset-x-0" key={item?.id}>
              {children}
            </div>
          </main>
        </div>
      </div>
      <footer
        aria-label="Application status bar"
        className="z-20 flex h-9 shrink-0 items-center gap-4 border-t border-charcoal-600 bg-charcoal-900 px-4 text-[11px] text-ink-secondary"
      >
        <span className="inline-flex items-center gap-1.5">
          <span aria-hidden="true" className="h-1.5 w-1.5 rounded-full bg-status-warn" />
          Status: <span className="font-medium text-ink-primary">Not initialized</span>
        </span>
        <span aria-hidden="true" className="text-charcoal-600">|</span>
        <span className="inline-flex items-center gap-1.5">
          Uptime <span className="font-mono text-ink-primary" title="Time since this UI session started">{uptime}</span>
        </span>
        <span aria-hidden="true" className="text-charcoal-600">|</span>
        <span className="inline-flex items-center gap-1.5">
          <span
            aria-hidden="true"
            className={
              "h-1.5 w-1.5 rounded-full " +
              (error ? "bg-status-idle" : foundation && database ? "bg-status-ok" : "bg-status-warn")
            }
          />
          Backend <span className="font-medium text-ink-primary">{error ? "Offline" : foundation && database ? "Online" : "Limited"}</span>
        </span>
        <div className="ml-auto flex items-center gap-4">
          <span className="hidden md:inline">Engine <span className="font-mono text-ink-primary">{foundation ? `v${foundation.version}` : "—"}</span></span>
        </div>
      </footer>
    </div>
  );
}
