import { cn } from "../../lib/cn";
import { IconInfo } from "./icons";

const RADIUS = 88;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
const ARC = 0.75; // three-quarter dial

type SpeedometerProps = {
  valueMbps: number | null;
  caption: string;
  pending: boolean;
  pendingReason?: string;
};

export function Speedometer({ valueMbps, caption, pending, pendingReason }: SpeedometerProps) {
  const dash = pending || valueMbps === null ? 0 : Math.min(1, valueMbps / 1000) * ARC;
  const gap = CIRCUMFERENCE * ARC - dash;

  return (
    <div className="flex flex-col items-center" title={pending ? pendingReason : caption}>
      <div className="relative">
        <svg width={220} height={220} viewBox="0 0 220 220" role="img" aria-label={caption}>
          <defs>
            <linearGradient id="dialGrad" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor="#2563EB" />
              <stop offset="55%" stopColor="#00E5FF" />
              <stop offset="100%" stopColor="#00FF87" />
            </linearGradient>
          </defs>
          <g transform="rotate(135 110 110)">
            <circle
              cx="110" cy="110" r={RADIUS} fill="none"
              stroke="#19273C" strokeWidth={14} strokeLinecap="round"
              strokeDasharray={`${CIRCUMFERENCE * ARC} ${CIRCUMFERENCE}`}
            />
            {dash > 0 && (
              <circle
                cx="110" cy="110" r={RADIUS} fill="none"
                stroke="url(#dialGrad)" strokeWidth={14} strokeLinecap="round"
                strokeDasharray={`${gap} ${CIRCUMFERENCE}`}
              />
            )}
          </g>
          <text x="110" y="104" textAnchor="middle" fill="#F0F4F8" fontSize="38" fontWeight="600" fontFamily="'JetBrains Mono', monospace">
            {pending ? "—" : valueMbps === null ? "—" : String(valueMbps)}
          </text>
          <text x="110" y="128" textAnchor="middle" fill="#94A3B8" fontSize="12" fontFamily="'JetBrains Mono', monospace">
            Mbps
          </text>
        </svg>
        {pending && (
          <span className="absolute left-1/2 top-[148px] -translate-x-1/2 whitespace-nowrap text-[10px] uppercase tracking-wider text-amber-300/90">
            integration pending
          </span>
        )}
      </div>
      <div className="mt-2 text-xs text-ink-secondary">{caption}</div>
    </div>
  );
}

type SpeedTestResultCardProps = {
  label: string;
  value: string;
  sub: string;
  className?: string;
};

export function SpeedTestResultCard({ label, value, sub, className }: SpeedTestResultCardProps) {
  return (
    <div className={cn("rounded-xl3 border border-charcoal-600 bg-surface p-3.5 shadow-panel transition hover:border-cyan-400/25", className)}>
      <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-ink-muted">{label}</div>
      <div className="mt-1.5 flex items-baseline gap-1">
        <span className="font-mono text-xl font-semibold text-ink-primary">{value}</span>
        <span className="font-mono text-[10px] uppercase text-ink-muted">{sub}</span>
      </div>
    </div>
  );
}

type SpeedTestStageProps = {
  index: number;
  title: string;
  description: string;
  state: "idle" | "active" | "done";
};

const stageState: Record<SpeedTestStageProps["state"], { ring: string; text: string }> = {
  idle: { ring: "border-charcoal-600 bg-charcoal-900 text-ink-muted", text: "text-ink-secondary" },
  active: { ring: "border-cyan-400/50 bg-cyan-400/15 text-accent-cyan shadow-glow", text: "text-ink-primary" },
  done: { ring: "border-accent-neon/50 bg-accent-neon/10 text-accent-neon", text: "text-ink-secondary" }
};

export function SpeedTestStage({ index, title, description, state }: SpeedTestStageProps) {
  const s = stageState[state];
  return (
    <div className="flex items-center gap-3">
      <span className={cn("flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border text-xs font-semibold", s.ring)}>
        {state === "done" ? "✓" : index}
      </span>
      <div className="min-w-0">
        <div className={cn("text-xs font-semibold", s.text)}>{title}</div>
        <div className="text-[11px] leading-4 text-ink-secondary">{description}</div>
      </div>
    </div>
  );
}
