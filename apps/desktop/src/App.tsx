import { useEffect, useState } from "react";
import Sidebar from "./components/Sidebar";
import { navItems, useAppStore } from "./store/appStore";
import { getFoundationStatus, type FoundationStatus } from "./api";

export default function App() {
  const { active } = useAppStore();
  const [status, setStatus] = useState<FoundationStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const label = navItems.find((n) => n.id === active)?.label ?? "Overview";

  useEffect(() => {
    getFoundationStatus().then(setStatus).catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="h-screen flex bg-charcoal-950 text-slate-100">
      <Sidebar />
      <main className="flex-1 min-w-0 flex flex-col">
        <header className="border-b border-charcoal-700 bg-charcoal-900/50 px-6 py-4 flex items-center justify-between">
          <div>
            <h1 className="text-lg font-semibold">{label}</h1>
            <p className="text-xs text-slate-400">Phase 1 foundation — no live network or security operations</p>
          </div>
          <div className="text-xs rounded-md border border-charcoal-700 bg-charcoal-800 px-3 py-1.5 text-slate-300">
            {status ? `${status.product} v${status.version}` : error ? "Tauri bridge unavailable (web preview)" : "Loading..."}
          </div>
        </header>
        <section className="flex-1 p-6 overflow-auto">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
            <Card title="Architecture" body="Three independent subsystems: MultiNet Network Engine, CyberShield Security Engine, AI Intelligence Engine. Interfaces are typed; implementations are stubs/mocks in Phase 1." />
            <Card title="Safety posture" body="No network routes, firewall rules, DNS, or OS security settings are modified by this foundation. AI cannot execute arbitrary shell commands." />
            <Card title="Measurements" body="No production network or threat metrics are displayed. Future telemetry will be sourced from verified collectors and labeled accordingly." />
          </div>
          <div className="mt-4 rounded-lg border border-charcoal-700 bg-charcoal-900 p-4">
            <div className="text-sm font-medium text-slate-200">Selected module</div>
            <div className="mt-1 text-sm text-slate-400">
              <span className="text-cyan-300">{label}</span> — UI placeholder. Functional implementation is scheduled for Phase 2+ with verified platform integrations and tests.
            </div>
          </div>
          {status && (
            <pre className="mt-4 text-xs text-slate-500 whitespace-pre-wrap">{JSON.stringify(status, null, 2)}</pre>
          )}
        </section>
      </main>
    </div>
  );
}

function Card({ title, body }: { title: string; body: string }) {
  return (
    <div className="rounded-lg border border-charcoal-700 bg-charcoal-900 p-4">
      <div className="text-sm font-medium text-slate-100">{title}</div>
      <div className="mt-2 text-sm leading-6 text-slate-400">{body}</div>
    </div>
  );
}
