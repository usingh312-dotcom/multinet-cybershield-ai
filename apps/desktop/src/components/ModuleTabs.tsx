import { moduleForRoute, useAppStore } from "../store/appStore";
import { cn } from "../lib/cn";

/**
 * Internal navigation for consolidated modules (Diagnostics, Security Center,
 * Incidents & Reports). Renders only while a child route is active and leaves
 * the underlying page routes untouched.
 */
export function ModuleTabs() {
  const { active, setActive } = useAppStore();
  const module = moduleForRoute(active);
  if (!module) {
    return null;
  }

  const handleKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    const count = module.items.length;
    let next: number | null = null;
    if (event.key === "ArrowRight") next = (index + 1) % count;
    else if (event.key === "ArrowLeft") next = (index - 1 + count) % count;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = count - 1;

    if (next !== null) {
      event.preventDefault();
      const target = module.items[next];
      setActive(target.id);
      document.getElementById(`module-tab-${target.id}`)?.focus();
    }
  };

  return (
    <nav
      aria-label={`${module.label} sections`}
      className="sticky top-0 z-20 border-b border-charcoal-600/70 bg-charcoal-950/95 px-6 py-2 backdrop-blur"
    >
      <div className="flex items-center gap-2 overflow-x-auto">
        <span className="hidden shrink-0 text-[10px] font-semibold uppercase tracking-[0.14em] text-ink-muted sm:inline">
          {module.label}
        </span>
        <div className="flex items-center gap-0.5">
          {module.items.map((item, index) => {
            const selected = item.id === active;
            return (
              <button
                key={item.id}
                id={`module-tab-${item.id}`}
                type="button"
                onClick={() => setActive(item.id)}
                onKeyDown={(event) => handleKeyDown(event, index)}
                aria-current={selected ? "page" : undefined}
                tabIndex={selected ? 0 : -1}
                className={cn(
                  "whitespace-nowrap rounded-md px-2.5 py-1 text-[13px] transition",
                  selected
                    ? "bg-cyan-400/12 text-ink-primary shadow-glow"
                    : "text-ink-secondary hover:bg-charcoal-800 hover:text-ink-primary focus:bg-charcoal-800"
                )}
              >
                {item.label}
              </button>
            );
          })}
        </div>
      </div>
    </nav>
  );
}
