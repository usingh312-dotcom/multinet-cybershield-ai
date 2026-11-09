import { useState } from "react";
import { navItems, useAppStore } from "../store/appStore";
import { useSystemStore } from "../store/systemStore";
import { StatusIndicator, type StatusVariation } from "./ui/feedback";
import { IconButton, SearchInput } from "./ui/widgets";
import { IconBell, IconPanelLeft, IconUser } from "./ui/icons";

type SystemStatus = { variation: StatusVariation; label: string };

function deriveSystemStatus(foundationReady: boolean, databaseReady: boolean, bridgeError: string | null, connecting: boolean): SystemStatus {
  if (connecting && !foundationReady) return { variation: "warn", label: "Connecting…" };
  if (bridgeError) return { variation: "idle", label: "Design-time preview" };
  if (foundationReady && databaseReady) return { variation: "ok", label: "System ready" };
  if (foundationReady) return { variation: "warn", label: "Partial backend" };
  return { variation: "idle", label: "Backend unavailable" };
}

export function ApplicationHeader() {
  const { active, toggleSidebar } = useAppStore();
  const { foundation, database, error, loading } = useSystemStore();
  const [query, setQuery] = useState("");
  const item = navItems.find((n) => n.id === active);
  const group = item?.group ?? "";

  const status = deriveSystemStatus(
    Boolean(foundation),
    Boolean(database?.migrations_applied),
    error,
    loading
  );

  return (
    <header className="flex h-14 items-center gap-3 border-b border-charcoal-600 bg-charcoal-900/70 px-4">
      <IconButton icon={<IconPanelLeft size={15} />} label="Toggle sidebar" onClick={toggleSidebar} />
      <nav aria-label="Breadcrumb" className="flex min-w-0 items-center gap-2 text-sm">
        <span className="shrink-0 text-ink-muted">MultiNet CyberShield AI</span>
        {group && <span className="shrink-0 text-ink-muted/60" aria-hidden="true">/</span>}
        {group && <span className="shrink-0 text-ink-muted">{group}</span>}
        <span className="shrink-0 text-ink-muted/60" aria-hidden="true">/</span>
        <span aria-current="page" className="truncate font-medium text-ink-primary">{item?.label ?? "Overview"}</span>
      </nav>

      <div className="ml-4 hidden lg:block">
        <SearchInput value={query} onChange={setQuery} placeholder="Global search (UI preview)" />
      </div>

      <div className="ml-auto flex items-center gap-2.5">
        <StatusLabel status={status} database={database} />
        <IconButton icon={<IconBell size={15} />} label="Notifications (UI preview)" />
        <IconButton icon={<IconUser size={15} />} label="Profile & settings (UI preview)" />
      </div>
    </header>
  );
}

function StatusLabel({ status, database }: { status: SystemStatus; database: ReturnType<typeof useSystemStore.getState>["database"] }) {
  const title = database
    ? `Database: ${database.migrations_applied} migration(s) applied · foreign keys ${database.foreign_keys_enabled ? "on" : "off"} · ${database.db_file.split(/[\\/]/).pop()}`
    : "Backend status unavailable in this environment";
  return (
    <div className="hidden md:block" title={title}>
      <StatusIndicator variation={status.variation} label={status.label} />
    </div>
  );
}
