import { navItems, useAppStore } from "./store/appStore";
import { cn } from "./lib/cn";

export default function Sidebar() {
  const { active, setActive } = useAppStore();
  const groups = Array.from(new Set(navItems.map((n) => n.group)));
  return (
    <aside className="w-64 shrink-0 border-r border-charcoal-700 bg-charcoal-900/80 p-3 overflow-y-auto">
      <div className="px-2 py-3 mb-2">
        <div className="text-sm font-semibold tracking-wide text-slate-100">MultiNet CyberShield AI</div>
        <div className="text-xs text-cyan-300/80">Connect. Protect. Detect.</div>
      </div>
      {groups.map((g) => (
        <div key={g} className="mb-3">
          <div className="px-2 pb-1 text-[11px] uppercase tracking-wider text-slate-500">{g}</div>
          <div className="space-y-0.5">
            {navItems.filter((n) => n.group === g).map((n) => (
              <button key={n.id} onClick={() => setActive(n.id)}
                className={cn(
                  "w-full text-left rounded-md px-2 py-1.5 text-sm transition",
                  active === n.id ? "bg-accent-indigo/20 text-cyan-200 ring-1 ring-accent-indigo/40" : "text-slate-300 hover:bg-charcoal-800"
                )}>
                {n.label}
              </button>
            ))}
          </div>
        </div>
      ))}
    </aside>
  );
}
