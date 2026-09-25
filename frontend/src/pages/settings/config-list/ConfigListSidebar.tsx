import { CheckCircle2, ChevronRight, XCircle } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { formatConfigTestTime } from "./testResultUtils";
import type { ConfigItem, ConfigItemMeta, ConfigTestResult } from "./types";
import { useI18n } from "../../../i18n";

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
  const { locale, t } = useI18n();
  return (
    <aside className="min-w-0 max-h-none overflow-visible md:min-h-0 md:overflow-y-auto md:overflow-x-hidden custom-scrollbar">
      {loading && items.length === 0 ? (
        <div className="flex items-center justify-center h-20 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" className="mr-2" /> {t("common.loading")}
        </div>
      ) : items.length === 0 ? (
        <div className="a4n-empty-state a4n-empty-state--compact text-xs leading-relaxed">
          {t("settings.noConfig", { name: title })}
          <br />
          {t("settings.clickCreateToStart")}
        </div>
      ) : (
        <ul className="grid min-w-0 grid-cols-1 gap-2 p-px auto-rows-max content-start">
          {items.map((item) => {
            const meta = formatItem(item.config, item.isActive);
            const selected = item.id === selectedId && !creating;
            const testResult = testResults[item.id] ?? null;

            return (
              <li key={item.id} className="min-w-0">
                <button
                  type="button"
                  onClick={() => onSelect(item.id)}
                  disabled={busy}
                  aria-current={selected ? "true" : undefined}
                  className={`group w-full min-w-0 rounded-xl border bg-book-bg-paper px-3 py-2.5 text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-book-primary disabled:cursor-not-allowed disabled:opacity-60 ${
                    selected
                      ? "border-book-primary/35 bg-book-primary/8"
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
                          title={`${formatConfigTestTime(testResult.testedAt, {}, locale)} · ${testResult.message}`}
                        >
                          {testResult.status === "success" ? (
                            <CheckCircle2 size={10} />
                          ) : (
                            <XCircle size={10} />
                          )}
                          {testResult.status === "success" ? t("common.passed") : t("workflow.failed")}
                        </span>
                      ) : null}
                      {item.isActive && (
                        <span className="text-book-primary">
                          {activeLabel}
                        </span>
                      )}
                      {item.readonly && !item.isActive ? (
                        <span className="text-book-text-muted">{t("common.readOnly")}</span>
                      ) : null}
                      <ChevronRight
                        size={13}
                        aria-hidden="true"
                        className={
                          selected
                            ? "text-book-primary"
                            : "text-book-text-muted transition-colors group-hover:text-book-primary"
                        }
                      />
                    </span>
                  </div>
                  {meta.secondary && (
                    <div className="text-xs text-book-text-sub break-words">{meta.secondary}</div>
                  )}
                  {meta.tertiary && (
                    <div className="text-small text-book-text-muted [overflow-wrap:anywhere]">
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
