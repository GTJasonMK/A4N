import React, { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { getModelUsageReport, type ModelUsageReport } from "../../api/modelUsage";
import { extractErrorMessage } from "../../api";
import { BookIconButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import { formatNumber, useI18n } from "../../i18n";

export const ModelUsageCostPanel: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const { t, locale } = useI18n();
  const { addToast } = useToast();
  const [report, setReport] = useState<ModelUsageReport | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = async () => {
    setLoading(true);
    try {
      setReport(
        await getModelUsageReport({
          projectId,
          days: 30,
          limit: 100,
        }),
      );
    } catch (error) {
      addToast(extractErrorMessage(error, t("novel.costLoadFailed")), "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, [projectId]);

  if (loading && !report) {
    return <LoadingSpinner size="md" text={t("novel.costLoading")} />;
  }

  return (
    <section className="space-y-4" aria-label={t("novel.costTitle")}>
      <header className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <h2 className="text-base font-semibold text-book-text-main">{t("novel.costTitle")}</h2>
          <p className="mt-1 text-small text-book-text-muted">{t("novel.costSubtitle")}</p>
        </div>
        <BookIconButton
          label={t("novel.costRefresh")}
          onClick={() => void refresh()}
          disabled={loading}
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </header>

      {!report || report.recent.length === 0 ? (
        <div className="border-l-2 border-book-border/70 py-3 pl-4 text-sm text-book-text-muted">
          {t("novel.costEmpty")}
        </div>
      ) : (
        <>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {report.totals.map((total) => (
              <div key={total.currency} className="a4n-flat-surface min-w-0 px-3 py-3">
                <div className="text-small text-book-text-muted">{total.currency}</div>
                <div className="mt-1 break-all font-mono text-lg tabular-nums text-book-text-main">
                  {formatMicros(total.amountMicros, locale)}
                </div>
                <div className="mt-1 text-small text-book-text-muted">
                  {t("novel.costCalls", { count: formatNumber(total.pricedCalls, locale) })}
                </div>
                {total.partialCalls > 0 && (
                  <div className="text-small text-book-text-muted">
                    {t("novel.costPartialCalls", {
                      count: formatNumber(total.partialCalls, locale),
                    })}
                  </div>
                )}
                {total.unpricedCalls > 0 && (
                  <div className="text-small text-book-text-muted">
                    {t("novel.costUnpricedCalls", {
                      count: formatNumber(total.unpricedCalls, locale),
                    })}
                  </div>
                )}
              </div>
            ))}
          </div>
          {report.unsettledCalls > 0 && (
            <p className="text-small text-book-text-muted">
              {t("novel.costUnsettledCalls", {
                count: formatNumber(report.unsettledCalls, locale),
              })}
            </p>
          )}
          <div className="overflow-x-auto border-y border-book-border/40">
            <table className="w-full min-w-[680px] text-left text-small">
              <thead className="text-book-text-muted">
                <tr className="border-b border-book-border/40">
                  <th className="px-2 py-2 font-medium">{t("novel.costCapability")}</th>
                  <th className="px-2 py-2 font-medium">{t("novel.costModel")}</th>
                  <th className="px-2 py-2 font-medium">{t("novel.costStatus")}</th>
                  <th className="px-2 py-2 font-medium">{t("novel.costAmount")}</th>
                  <th className="px-2 py-2 font-medium">{t("novel.costTime")}</th>
                </tr>
              </thead>
              <tbody>
                {report.recent.slice(0, 50).map((entry) => (
                  <tr key={entry.id} className="border-b border-book-border/25 last:border-0">
                    <td className="px-2 py-2 text-book-text-main">{entry.capability}</td>
                    <td className="max-w-[240px] truncate px-2 py-2 text-book-text-muted">
                      {entry.provider ?? "-"} · {entry.model ?? "-"}
                    </td>
                    <td className="px-2 py-2 text-book-text-muted">
                      {entry.status} · {entry.confidence}
                    </td>
                    <td className="px-2 py-2 font-mono tabular-nums text-book-text-main">
                      {entry.amountMicros == null
                        ? t("novel.costUnpriced")
                        : entry.currency + " " + formatMicros(entry.amountMicros, locale)}
                    </td>
                    <td className="whitespace-nowrap px-2 py-2 text-book-text-muted">
                      {entry.startedAt}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </section>
  );
};

function formatMicros(value: string, locale: string): string {
  const amount = Number(value) / 1_000_000;
  return Number.isFinite(amount)
    ? amount.toLocaleString(locale, { minimumFractionDigits: 2, maximumFractionDigits: 6 })
    : value;
}
