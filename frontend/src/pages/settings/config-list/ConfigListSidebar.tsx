import { useEffect, useRef } from "react";
import { CheckCircle2, XCircle } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { formatConfigTestTime } from "./testResultUtils";
import type { ConfigItem, ConfigItemMeta, ConfigTestResult } from "./types";

interface ConfigListSidebarProps<C> {
  title: string;
  items: ConfigItem<C>[];
  loading: boolean;
  busy: boolean;
  selectedId: number | null;
  creating: boolean;
  formatItem: (config: C, isActive: boolean) => ConfigItemMeta;
  testResults: Record<number, ConfigTestResult>;
  activeLabel: string;
  onSelect: (id: number) => void;
}

export function ConfigListSidebar<C>({
  title,
  items,
  loading,
  busy,
  selectedId,
  creating,
  formatItem,
  testResults,
  activeLabel,
  onSelect,
}: ConfigListSidebarProps<C>) {
  const selectedRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (creating || selectedId == null) return;
    if (!window.matchMedia("(max-width: 767px)").matches) return;
    const frame = window.requestAnimationFrame(() => {
      selectedRef.current?.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [creating, items.length, selectedId]);

  return (
    <aside className="max-h-48 overflow-y-auto overflow-x-hidden md:max-h-none custom-scrollbar">
      {loading && items.length === 0 ? (
        <div className="flex items-center justify-center h-20 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" className="mr-2" /> 加载…
        </div>
      ) : items.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--compact text-xs leading-relaxed">
          尚未配置任何 {title}。
          <br />
          点击左上角"新增"开始。
        </div>
      ) : (
        <ul className="grid gap-2 p-px auto-rows-max content-start">
          {items.map((item) => {
            const meta = formatItem(item.config, item.isActive);
            const selected = item.id === selectedId && !creating;
            const testResult = testResults[item.id] ?? null;

            return (
              <li key={item.id}>
                <button
                  ref={selected ? selectedRef : undefined}
                  type="button"
                  onClick={() => onSelect(item.id)}
                  disabled={busy}
                  aria-current={selected ? "true" : undefined}
                  className={`group w-full rounded-xl border bg-book-bg-paper px-3 py-2.5 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-60 ${
                    selected
                      ? "border-book-primary/55 bg-book-primary/10 shadow-[inset_3px_0_0_var(--color-book-primary)]"
                      : "border-book-border/40 hover:border-book-primary/35 hover:bg-book-bg-tertiary"
                  }`}
                >
                  <div className="mb-0.5 flex flex-wrap items-center gap-x-2 gap-y-1">
                    <span className="min-w-0 flex-1 break-words text-sm font-semibold text-book-text-main">
                      {meta.primary}
                    </span>
                    <span className="flex shrink-0 flex-wrap items-center gap-1 text-caption">
                      {testResult ? (
                        <span
                          className={`inline-flex items-center gap-0.5 text-small ${
                            testResult.status === "success"
                              ? "text-success"
                              : "text-error"
                          }`}
                          title={`${formatConfigTestTime(testResult.testedAt)} · ${testResult.message}`}
                        >
                          {testResult.status === "success" ? (
                            <CheckCircle2 size={10} />
                          ) : (
                            <XCircle size={10} />
                          )}
                          {testResult.status === "success" ? "通过" : "失败"}
                        </span>
                      ) : null}
                      {item.isActive && (
                        <span className="text-book-primary">
                          {activeLabel}
                        </span>
                      )}
                      {item.readonly && !item.isActive ? (
                        <span className="text-book-text-muted">只读</span>
                      ) : null}
                    </span>
                  </div>
                  {meta.secondary && (
                    <div className="text-xs text-book-text-sub break-words">{meta.secondary}</div>
                  )}
                  {meta.tertiary && (
                    <div className="text-small text-book-text-muted break-words">
                      {meta.tertiary}
                    </div>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </aside>
  );
}
