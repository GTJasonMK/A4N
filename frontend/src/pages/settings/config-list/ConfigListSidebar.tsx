import { CheckCircle2, Loader2, XCircle } from "lucide-react";
import { formatConfigTestTime } from "./testResultUtils";
import type { ConfigItem, ConfigItemMeta, ConfigTestResult } from "./types";

interface ConfigListSidebarProps<C> {
  title: string;
  items: ConfigItem<C>[];
  loading: boolean;
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
  selectedId,
  creating,
  formatItem,
  testResults,
  activeLabel,
  onSelect,
}: ConfigListSidebarProps<C>) {
  return (
    <aside className="pyqt-card max-h-48 overflow-x-auto p-1 md:max-h-none md:overflow-y-auto custom-scrollbar">
      {loading && items.length === 0 ? (
        <div className="flex items-center justify-center h-20 text-xs text-book-text-muted">
          <Loader2 size={14} className="animate-spin mr-2" /> 加载…
        </div>
      ) : items.length === 0 ? (
        <div className="text-center text-xs text-book-text-muted py-6 px-4 leading-relaxed">
          尚未配置任何 {title}。
          <br />
          点击左上角"新增"开始。
        </div>
      ) : (
        <ul className="flex min-w-max gap-1 md:block md:min-w-0 md:space-y-0.5">
          {items.map((item) => {
            const meta = formatItem(item.config, item.isActive);
            const selected = item.id === selectedId && !creating;
            const testResult = testResults[item.id] ?? null;

            return (
              <li key={item.id}>
                <button
                  type="button"
                  onClick={() => onSelect(item.id)}
                  className={`group min-w-[13rem] rounded border px-3 py-2 text-left transition-colors md:w-full md:min-w-0 ${
                    selected
                      ? "bg-book-primary/10 border-book-primary"
                      : "border-transparent hover:bg-book-bg-tertiary"
                  }`}
                >
                  <div className="flex items-center justify-between gap-2 mb-0.5">
                    <span className="text-sm font-semibold text-book-text-main truncate">
                      {meta.primary}
                    </span>
                    <span className="flex shrink-0 items-center gap-1">
                      {testResult ? (
                        <span
                          className={`inline-flex items-center gap-0.5 text-[10px] ${
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
                        <span className="pyqt-pill !text-[10px] !py-0 !px-1.5 border-book-primary/40 bg-book-primary/10 text-book-primary">
                          {activeLabel}
                        </span>
                      )}
                      {item.readonly && !item.isActive ? (
                        <span className="pyqt-pill !text-[10px] !py-0 !px-1.5">
                          只读
                        </span>
                      ) : null}
                    </span>
                  </div>
                  {meta.secondary && (
                    <div className="text-xs text-book-text-sub truncate">{meta.secondary}</div>
                  )}
                  {meta.tertiary && (
                    <div className="text-[11px] text-book-text-muted truncate">
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
