import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export type DataTableColumn<T> = {
  id: string;
  header: ReactNode;
  align?: "left" | "right" | "center";
  minWidth?: number;
  render?: (row: T, rowIndex: number) => ReactNode;
};

type DataTableProps<T> = {
  columns: DataTableColumn<T>[];
  rows: T[];
  rowKey: (row: T, index: number) => string;
  emptyState?: ReactNode;
  loadingState?: ReactNode;
  loading?: boolean;
  minWidth?: number;
  caption?: string;
};

const alignClassName = {
  left: "text-left",
  right: "text-right",
  center: "text-center"
} as const;

export function DataTable<T>({ columns, rows, rowKey, emptyState, loadingState, loading = false, minWidth = 720, caption }: DataTableProps<T>) {
  const colCount = columns.length;
  return (
    <div className="overflow-x-auto">
      <table style={{ minWidth }} className="w-full text-left text-xs" aria-busy={loading || undefined}>
        {caption && <caption className="sr-only">{caption}</caption>}
        <thead>
          <tr className="border-b border-charcoal-600 bg-charcoal-800/40 text-[10px] uppercase tracking-[0.14em] text-ink-muted">
            {columns.map((col) => (
              <th
                key={col.id}
                scope="col"
                style={col.minWidth ? { minWidth: col.minWidth } : undefined}
                className={cn("px-4 py-2.5 font-semibold", alignClassName[col.align ?? "left"])}
              >
                {col.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {loading && loadingState ? (
            <tr>
              <td colSpan={colCount} className="px-4 py-6">
                {loadingState}
              </td>
            </tr>
          ) : rows.length === 0 ? (
            emptyState ? (
              <tr>
                <td colSpan={colCount} className="px-4 py-5">
                  {emptyState}
                </td>
              </tr>
            ) : null
          ) : (
            rows.map((row, i) => (
              <tr key={rowKey(row, i)} className="border-b border-charcoal-600/60 transition last:border-0 hover:bg-charcoal-800/50">
                {columns.map((col) => (
                  <td key={col.id} className={cn("px-4 py-2.5 text-ink-secondary", alignClassName[col.align ?? "left"])}>
                    {col.render ? col.render(row, i) : null}
                  </td>
                ))}
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}
