import { useEffect, useRef, useState } from "react";
import { PageContainer, PageHeader, SectionHeader } from "../components/ui/structure";
import { IntegrationNotice, Badge } from "../components/ui/controls";
import { useAppStore, type NavId } from "../store/appStore";
import { useSystemStore } from "../store/systemStore";
import { cn } from "../lib/cn";
import {
  IconAnalytics, IconArrowRight, IconBonding, IconLock, IconNetworkManager,
  IconShield, IconSpeed, IconWifi, IconX
} from "../components/ui/icons";
import type { ReactNode } from "react";

/**
 * Design-sample dataset for the bonding dashboard. The bonding engine is not
 * integrated in this build, so every metric rendered below comes from this
 * static sample set and is labelled as such - nothing is measured live.
 */
type PathId = "wifi6e" | "usb5g" | "hotspot";
type PathColor = "cyan" | "neon" | "indigo";

type PathSpec = {
  id: PathId;
  name: string;
  tag: string;
  color: PathColor;
  bars: 1 | 2 | 3 | 4;
  down: number;
  up: number;
  ping: number;
  quality: string;
  primary?: boolean;
};

const samplePaths: PathSpec[] = [
  { id: "wifi6e", name: "Main Wi-Fi 1", tag: "5 GHz  802.11ax", color: "cyan", bars: 4, down: 98.6, up: 23.4, ping: 12, quality: "Excellent", primary: true },
  { id: "usb5g", name: "USB 5G", tag: "NR Sub-6  Tethered", color: "neon", bars: 4, down: 67.8, up: 18.7, ping: 18, quality: "Very Good" },
  { id: "hotspot", name: "Hotspot Wi-Fi 2", tag: "2.4 GHz  802.11n", color: "indigo", bars: 3, down: 47.3, up: 15.6, ping: 28, quality: "Good" }
];

const colorText: Record<PathColor, string> = {
  cyan: "text-accent-cyan",
  neon: "text-accent-neon",
  indigo: "text-accent-indigo"
};
const colorBar: Record<PathColor, string> = {
  cyan: "bg-accent-cyan",
  neon: "bg-accent-neon",
  indigo: "bg-accent-indigo"
};
const colorChip: Record<PathColor, string> = {
  cyan: "border-cyan-400/30 bg-cyan-400/10",
  neon: "border-accent-neon/30 bg-accent-neon/10",
  indigo: "border-accent-indigo/30 bg-accent-indigo/10"
};
const colorDot: Record<PathColor, string> = {
  cyan: "bg-accent-cyan shadow-glow",
  neon: "bg-accent-neon shadow-glowGreen",
  indigo: "bg-accent-indigo"
};

const pathIcons: Record<PathId, ReactNode> = {
  wifi6e: <IconWifi size={15} />,
  usb5g: <IconBonding size={15} />,
  hotspot: <IconNetworkManager size={15} />
};

function MiniIconBulb({ size = 15 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M9.5 18h5" /><path d="M10.5 21h3" />
      <path d="M12 3a6.5 6.5 0 0 1 3.6 11.9c-.7.5-1.1 1-1.1 1.8v.3h-5v-.3c0-.8-.4-1.3-1.1-1.8A6.5 6.5 0 0 1 12 3Z" />
    </svg>
  );
}

function MiniIconBeaker({ size = 15 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M9.5 3h5" /><path d="M10 3v5.2L5.6 16a2.4 2.4 0 0 0 2.2 3.4h8.4a2.4 2.4 0 0 0 2.2-3.4L14 8.2V3" /><path d="M7.5 14h9" />
    </svg>
  );
}

function Sparkline() {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    let raf = 0;
    let t = Math.random() * 40;

    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      canvas.width = canvas.clientWidth * dpr;
      canvas.height = canvas.clientHeight * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    window.addEventListener("resize", resize);

    const draw = () => {
      t += 0.016;
      const w = canvas.clientWidth;
      const h = canvas.clientHeight;
      if (w > 0 && h > 0) {
        ctx.clearRect(0, 0, w, h);
        const pts: Array<[number, number]> = [];
        for (let i = 0; i <= 60; i++) {
          const x = (i / 60) * w;
          const wave =
            Math.sin(i * 0.33 + t) * 0.26 +
            Math.sin(i * 0.11 - t * 0.7) * 0.16 +
            Math.sin(i * 0.87 - t * 1.9) * 0.08;
          pts.push([x, h * 0.68 - wave * h * 0.5]);
        }
        ctx.beginPath();
        pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
        const fill = ctx.createLinearGradient(0, 0, 0, h);
        fill.addColorStop(0, "rgba(0, 229, 255, 0.18)");
        fill.addColorStop(1, "rgba(0, 229, 255, 0)");
        ctx.lineTo(w, h);
        ctx.lineTo(0, h);
        ctx.closePath();
        ctx.fillStyle = fill;
        ctx.fill();
        ctx.beginPath();
        pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
        const stroke = ctx.createLinearGradient(0, 0, w, 0);
        stroke.addColorStop(0, "rgba(0, 229, 255, 0.25)");
        stroke.addColorStop(0.6, "rgba(0, 229, 255, 0.85)");
        stroke.addColorStop(1, "rgba(0, 255, 135, 0.9)");
        ctx.strokeStyle = stroke;
        ctx.lineWidth = 2;
        ctx.shadowColor = "rgba(0, 229, 255, 0.5)";
        ctx.shadowBlur = 8;
        ctx.stroke();
        ctx.shadowBlur = 0;
      }
      raf = requestAnimationFrame(draw);
    };
    draw();
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
    };
  }, []);

  return <canvas ref={ref} className="h-16 w-56" aria-hidden="true" />;
}

function SignalBars({ level, color }: { level: number; color: PathColor }) {
  const heights = [5, 8, 11, 14];
  return (
    <span className="inline-flex items-end gap-[3px]" role="img" aria-label={`Signal ${level} of 4 (sample)`}>
      {heights.map((h, i) => (
        <span
          key={h}
          style={{ height: h }}
          className={cn("w-[3px] rounded-sm", i < level ? colorBar[color] : "bg-charcoal-600")}
        />
      ))}
    </span>
  );
}

export default function InternetBondingPage() {
  const setActive = useAppStore((s) => s.setActive);
  const { foundation, error, loading } = useSystemStore();
  const [excluded, setExcluded] = useState<Set<PathId>>(new Set());
  const [dialogPath, setDialogPath] = useState<PathSpec | null>(null);
  const [draftPriority, setDraftPriority] = useState(50);
  const [priorities, setPriorities] = useState<Record<PathId, number>>({ wifi6e: 70, usb5g: 50, hotspot: 30 });
  const [toast, setToast] = useState<string | null>(null);
  const sliderRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 3200);
    return () => window.clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    if (!dialogPath) return;
    sliderRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setDialogPath(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [dialogPath]);

  const activePaths = samplePaths.filter((p) => !excluded.has(p.id));
  const total = activePaths.reduce((sum, p) => sum + p.down, 0);
  const fastest = activePaths.reduce((max, p) => Math.max(max, p.down), 0);
  const multiple = fastest > 0 ? total / fastest : 0;
  const shareOf = (p: PathSpec) => (total > 0 ? (p.down / total) * 100 : 0);

  const toggleExcluded = (p: PathSpec) => {
    setExcluded((prev) => {
      const next = new Set(prev);
      if (next.has(p.id)) next.delete(p.id);
      else next.add(p.id);
      return next;
    });
    setToast(`${p.name} ${excluded.has(p.id) ? "included back into" : "excluded from"} the bonding pool (sample view).`);
  };

  const openDialog = (p: PathSpec) => {
    setDraftPriority(priorities[p.id]);
    setDialogPath(p);
  };

  const savePriority = () => {
    if (!dialogPath) return;
    setPriorities((prev) => ({ ...prev, [dialogPath.id]: draftPriority }));
    setToast(`Priority set to ${draftPriority} for ${dialogPath.name} (sample settings only).`);
    setDialogPath(null);
  };

  const backendLine = error
    ? "Backend bridge unavailable."
    : foundation
      ? `Backend reachable · v${foundation.version}.`
      : loading
        ? "Connecting to backend…"
        : "Backend unavailable.";

  const nextActions: Array<{ label: string; icon: ReactNode; hint: string; onClick?: () => void; badge?: string }> = [
    { label: "What can you do next?", icon: <MiniIconBulb />, hint: "Explore tools with the sample dashboard.", onClick: () => setToast("Try Run Speed Test or View Traffic Analytics from this row.") },
    { label: "Run Speed Test", icon: <IconSpeed />, hint: "Open the diagnostics speed test screen.", onClick: () => setActive("speed-test" as NavId) },
    { label: "View Traffic Analytics", icon: <IconAnalytics />, hint: "Open the diagnostics analytics screen.", onClick: () => setActive("traffic-analytics" as NavId) },
    { label: "Threat Simulation", icon: <MiniIconBeaker />, hint: "Demonstration entry only.", onClick: () => setToast("Threat simulation is a demo-only entry.") }
  ];

  return (
    <PageContainer>
      <section>
        <PageHeader
          title="Internet Bonding"
          subtitle="Combine multiple internet connections through an aggregated tunnel."
          actions={<Badge tone="accent">Design Sample</Badge>}
        />
        <div className="mt-3">
          <IntegrationNotice>
            <span>
              Interface preview on the reference sample dataset. The bonding engine is an interface-level stub in this build:
              <strong className="font-semibold text-ink-primary"> no tunnel is created, no traffic is routed and no relay is contacted.</strong>{" "}
              All speed, adapter and status values below are the fixed design sample (marked where shown). {backendLine}
            </span>
          </IntegrationNotice>
        </div>
      </section>

      <section>
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {[
            { title: "Tunnel Status", value: "Wintun Tunnel", sub: "Sample · online", icon: <IconShield size={14} />, valueClass: "text-accent-neon" },
            { title: "Security Status", value: "Protected", sub: "Sample · all systems active", icon: <IconLock size={14} />, valueClass: "text-accent-cyan" },
            { title: "Data Encryption", value: "AES-256", sub: "Sample · secure", icon: <IconLock size={14} />, valueClass: "text-ink-primary" },
            { title: "Last Check", value: "Just now", sub: "Sample · zero packet loss", icon: <IconAnalytics size={14} />, valueClass: "text-ink-primary" }
          ].map((c) => (
            <div key={c.title} className="rounded-xl3 border border-charcoal-600 bg-surface p-3.5 shadow-panel transition hover:border-cyan-400/25">
              <div className="flex items-center justify-between gap-2">
                <span className="text-[10px] font-semibold uppercase tracking-[0.16em] text-ink-muted">{c.title}</span>
                <span className="text-ink-muted">{c.icon}</span>
              </div>
              <div className="mt-2 text-base font-semibold text-ink-primary">
                <span className={c.valueClass}>{c.value}</span>
              </div>
              <p className="mt-0.5 font-mono text-[10px] uppercase text-ink-muted">{c.sub}</p>
            </div>
          ))}
        </div>
      </section>

      <section>
        <div className="relative overflow-hidden rounded-xl3 border border-charcoal-600 bg-surface p-5 shadow-panel">
          <div className="pointer-events-none absolute -right-24 -top-24 h-64 w-64 rounded-full bg-accent-cyan/8 blur-3xl" />
          <div className="relative grid items-center gap-6 lg:grid-cols-[auto_1fr_auto]">
            <div className="flex items-start gap-3">
              <span className="mt-0.5 flex h-11 w-11 shrink-0 items-center justify-center rounded-xl2 border border-accent-neon/40 bg-accent-neon/10 text-accent-neon shadow-glowGreen">
                <IconShield size={20} />
              </span>
              <div className="max-w-[13rem]">
                <div className="text-sm font-semibold leading-5 text-ink-primary">Your connection is working well</div>
                <p className="mt-1 text-xs leading-4 text-ink-secondary">All active adapters are stable and healthy.</p>
                <Badge tone="ok" className="mt-2">Sample state</Badge>
              </div>
            </div>

            <div className="text-center lg:text-left">
              <div className="text-[10px] font-semibold uppercase tracking-[0.2em] text-ink-muted">Aggregate download</div>
              <div className="mt-1 flex items-baseline justify-center gap-2 lg:justify-start">
                <span className="font-mono text-4xl font-semibold tracking-tight text-ink-primary [text-shadow:0_0_24px_rgba(0,229,255,0.35)]">
                  {multiple > 0 ? total.toFixed(1) : "0.0"}
                </span>
                <span className="font-mono text-sm text-accent-cyan">Mbps</span>
              </div>
              <p className="mt-1 text-[11px] text-ink-muted">
                Sum of {activePaths.length} included sample path{activePaths.length === 1 ? "" : "s"} (60s wave below)
              </p>
            </div>

            <div className="flex flex-col items-center gap-1.5 lg:items-end">
              <span className="text-[10px] font-semibold uppercase tracking-[0.16em] text-ink-muted">
                {multiple > 0 ? `${multiple.toFixed(1)}x your fastest connection` : "Awaiting included paths"}
              </span>
              <Sparkline />
              <span className="font-mono text-[9px] uppercase text-ink-muted">sample wave · 60 s · not measured</span>
            </div>
          </div>
        </div>
      </section>

      <section>
        <SectionHeader
          title="Traffic is shared across your active connections"
          description="Distribution is recalculated from the sample adapter set as you toggle paths."
          action={<Badge>Sample</Badge>}
        />
        <div className="rounded-xl3 border border-charcoal-600 bg-surface p-4 shadow-panel">
          <div
            role="img"
            aria-label={activePaths.length
              ? `Sample traffic distribution: ${activePaths.map((p) => `${p.name} ${shareOf(p).toFixed(0)} percent`).join(", ")}`
              : "All sample paths excluded"}
            className="flex h-3 w-full overflow-hidden rounded-full bg-charcoal-700"
          >
            {activePaths.map((p, i) => (
              <div
                key={p.id}
                style={{ width: `${shareOf(p)}%` }}
                className={cn(
                  colorBar[p.color],
                  "h-full transition-all duration-300",
                  i === 0 && "rounded-l-full",
                  i === activePaths.length - 1 && "rounded-r-full"
                )}
              />
            ))}
          </div>
          <ul className="mt-3 flex flex-wrap gap-x-5 gap-y-1.5">
            {samplePaths.map((p) => (
              <li key={p.id} className={cn("flex items-center gap-1.5 text-xs", excluded.has(p.id) && "opacity-45 line-through")}>
                <span aria-hidden="true" className={cn("h-2 w-2 rounded-full", colorDot[p.color])} />
                <span className="text-ink-secondary">{p.name}</span>
                <span className="font-mono text-ink-primary">{excluded.has(p.id) ? "excluded" : `${shareOf(p).toFixed(0)}%`}</span>
              </li>
            ))}
          </ul>
        </div>
      </section>

      <section>
        <SectionHeader
          title="Active adapters"
          description="Sample adapter cards. Use Exclude to pull a path out of the pool and watch the aggregate change."
        />
        <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {samplePaths.map((p) => {
            const isExcluded = excluded.has(p.id);
            return (
              <div
                key={p.id}
                className={cn(
                  "rounded-xl3 border border-charcoal-600 bg-surface p-4 shadow-panel transition",
                  isExcluded ? "opacity-55 grayscale" : "hover:border-cyan-400/25"
                )}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex items-center gap-2.5">
                    <span className={cn("flex h-8 w-8 items-center justify-center rounded-lg border", colorChip[p.color], colorText[p.color])}>
                      {pathIcons[p.id]}
                    </span>
                    <div>
                      <div className="flex items-center gap-1.5 text-sm font-semibold text-ink-primary">
                        {p.name}
                        {p.primary && <Badge tone="accent">Primary</Badge>}
                        {isExcluded && <Badge tone="warn">Excluded</Badge>}
                      </div>
                      <div className="mt-0.5 font-mono text-[10px] uppercase text-ink-muted">{p.tag}</div>
                    </div>
                  </div>
                  <span className={cn(colorText[p.color], !isExcluded && "drop-shadow-[0_0_6px_currentColor]")}>
                    <SignalBars level={p.bars} color={p.color} />
                  </span>
                </div>

                <div className="mt-3 grid grid-cols-3 gap-2">
                  {[
                    { k: "Down", v: `${p.down.toFixed(1)} Mbps` },
                    { k: "Up", v: `${p.up.toFixed(1)} Mbps` },
                    { k: "Ping", v: `${p.ping} ms` }
                  ].map((m) => (
                    <div key={m.k} className="rounded-lg border border-charcoal-600 bg-charcoal-800 px-2 py-1.5">
                      <div className="text-[9px] font-semibold uppercase tracking-wider text-ink-muted">{m.k}</div>
                      <div className={isExcluded ? "font-mono text-[11px] text-ink-muted" : "font-mono text-[11px] text-accent-cyan"}>{m.v}</div>
                    </div>
                  ))}
                </div>

                <div className="mt-2.5 flex items-center justify-between text-xs">
                  <span className="text-ink-secondary">
                    Quality <span className="font-medium text-status-ok">{p.quality}</span> · sample
                  </span>
                  <span className={colorText[p.color]}>{"▮".repeat(p.bars)}</span>
                </div>

                <div className="mt-3 flex items-center justify-between border-t border-charcoal-600/70 pt-2.5">
                  <button
                    type="button"
                    onClick={() => toggleExcluded(p)}
                    className="rounded px-1.5 py-1 text-xs text-accent-cyan transition hover:text-white focus:text-white"
                    aria-pressed={isExcluded}
                  >
                    {isExcluded ? "Include" : "Exclude"}
                  </button>
                  <button
                    type="button"
                    onClick={() => openDialog(p)}
                    className="inline-flex items-center gap-1 rounded px-1.5 py-1 text-xs text-ink-secondary transition hover:text-ink-primary focus:text-ink-primary"
                  >
                    Manage <IconArrowRight size={12} />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      </section>

      <section>
        <SectionHeader title="What can you do next?" description="Quick actions navigate to real screens or acknowledge demo scope." />
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {nextActions.map((a) => (
            <button
              key={a.label}
              type="button"
              onClick={a.onClick}
              className="group flex items-start gap-3 rounded-xl3 border border-charcoal-600 bg-surface p-3.5 text-left shadow-panel transition hover:border-cyan-400/35 hover:bg-charcoal-750 focus-visible:border-cyan-400/35"
            >
              <span className="mt-0.5 rounded-lg border border-charcoal-600 bg-charcoal-800 p-1.5 text-accent-cyan">{a.icon}</span>
              <span>
                <span className="flex items-center gap-1.5 text-xs font-semibold text-ink-primary">
                  {a.label}
                  {a.label === "Threat Simulation" && <Badge tone="warn">Demo</Badge>}
                </span>
                <span className="mt-0.5 block text-[11px] leading-4 text-ink-secondary">{a.hint}</span>
              </span>
            </button>
          ))}
        </div>
      </section>

      {dialogPath && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm"
          onClick={() => setDialogPath(null)}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="adapter-dialog-title"
            className="w-full max-w-sm rounded-xl3 border border-charcoal-600 bg-charcoal-900 p-5 shadow-pop"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between">
              <h2 id="adapter-dialog-title" className="flex items-center gap-2 text-sm font-semibold text-ink-primary">
                <span className={cn("rounded-lg border p-1.5", colorChip[dialogPath.color], colorText[dialogPath.color])}>
                  {pathIcons[dialogPath.id]}
                </span>
                {dialogPath.name}
              </h2>
              <button
                type="button"
                onClick={() => setDialogPath(null)}
                aria-label="Close adapter dialog"
                className="rounded-md p-1 text-ink-muted transition hover:bg-charcoal-800 hover:text-ink-primary"
              >
                <IconX size={14} />
              </button>
            </div>
            <p className="mt-1 text-[11px] text-ink-secondary">Priority slider · sample settings, nothing is persisted.</p>

            <label htmlFor="adapter-priority" className="mt-4 flex items-center justify-between text-xs text-ink-secondary">
              <span>Priority</span>
              <span className="font-mono text-accent-cyan">{draftPriority}</span>
            </label>
            <input
              ref={sliderRef}
              id="adapter-priority"
              type="range"
              min={1}
              max={100}
              value={draftPriority}
              onChange={(e) => setDraftPriority(Number(e.target.value))}
              style={{ accentColor: "#00E5FF" }}
              className="mt-1.5 w-full"
            />
            <div className="mt-0.5 flex justify-between font-mono text-[9px] uppercase text-ink-muted">
              <span>Low</span><span>High</span>
            </div>

            <div className="mt-4 grid grid-cols-3 gap-2 text-center">
              <div className="rounded-lg border border-charcoal-600 bg-charcoal-800 px-2 py-1.5">
                <div className="text-[9px] uppercase tracking-wider text-ink-muted">Down</div>
                <div className="font-mono text-[11px] text-ink-primary">{dialogPath.down.toFixed(1)} Mbps</div>
              </div>
              <div className="rounded-lg border border-charcoal-600 bg-charcoal-800 px-2 py-1.5">
                <div className="text-[9px] uppercase tracking-wider text-ink-muted">Up</div>
                <div className="font-mono text-[11px] text-ink-primary">{dialogPath.up.toFixed(1)} Mbps</div>
              </div>
              <div className="rounded-lg border border-charcoal-600 bg-charcoal-800 px-2 py-1.5">
                <div className="text-[9px] uppercase tracking-wider text-ink-muted">Ping</div>
                <div className="font-mono text-[11px] text-ink-primary">{dialogPath.ping} ms</div>
              </div>
            </div>

            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setDialogPath(null)}
                className="rounded-lg border border-charcoal-600 bg-charcoal-800 px-3 py-1.5 text-xs text-ink-primary transition hover:bg-charcoal-750 focus:bg-charcoal-750"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={savePriority}
                className="rounded-lg bg-gradient-to-r from-blue-700 via-accent-blue to-cyan-500 px-3 py-1.5 text-xs font-medium text-white shadow-glow transition hover:brightness-110 focus:brightness-110"
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}

      {toast && (
        <div
          role="status"
          aria-live="polite"
          className="fixed bottom-12 right-4 z-50 flex items-center gap-2 rounded-xl2 border border-cyan-400/35 bg-charcoal-900 px-3.5 py-2.5 text-xs text-ink-primary shadow-pop"
        >
          <span aria-hidden="true" className="h-1.5 w-1.5 rounded-full bg-accent-cyan shadow-glow" />
          {toast}
        </div>
      )}
    </PageContainer>
  );
}
