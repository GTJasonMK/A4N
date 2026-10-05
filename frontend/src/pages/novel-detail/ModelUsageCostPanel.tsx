import React, { useEffect, useRef, useState } from "react";
import { Eye, RefreshCw } from "lucide-react";
import {
  getModelUsageReport,
  getModelUsageRequest,
  type ModelUsageReport,
  type ModelUsageRequest,
} from "../../api/modelUsage";
import { extractErrorMessage } from "../../api";
import { BookIconButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { useToast } from "../../components/feedback/Toast";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { formatDateTime, formatNumber, useI18n, type MessageKey } from "../../i18n";
import { getOperationStatusLabel } from "../../utils/workflowPresentation";

const CAPABILITY_LABELS: Record<string, MessageKey> = {
  chapter_generation: "workflow.taskChapterGeneration",
  chapter_analysis: "workflow.kindChapterAnalysis",
  scene_image: "workflow.taskImageGeneration",
  image_generation: "workflow.taskImageGeneration",
  video_generation: "workflow.taskVideoGeneration",
  inspiration: "novel.inspiration",
};

export const ModelUsageCostPanel: React.FC<{ projectId: number | null }> = ({ projectId }) => {
  const { t, locale } = useI18n();
  const { addToast } = useToast();
  const tRef = useRef(t);
  tRef.current = t;
  const [result, setResult] = useState<{ projectId: number | null; report: ModelUsageReport } | null>(null);
  const report = result?.projectId === projectId ? result.report : null;
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const requestRef = useRef(0);
  const requestPreviewRef = useRef(0);
  const [requestModalOpen, setRequestModalOpen] = useState(false);
  const [requestLoading, setRequestLoading] = useState(false);
  const [requestPreview, setRequestPreview] = useState<ModelRequestPreview | null>(null);

  const refresh = async () => {
    const request = ++requestRef.current;
    setLoading(true);
    setLoadError(null);
    try {
      const report = await getModelUsageReport({ projectId, days: 30, limit: 100 });
      if (request === requestRef.current) setResult({ projectId, report });
    } catch (error) {
      if (request !== requestRef.current) return;
      const message = extractErrorMessage(error, tRef.current("novel.costLoadFailed"));
      setLoadError(message);
    } finally {
      if (request === requestRef.current) setLoading(false);
    }
  };

  const viewRequest = async (id: number) => {
    if (requestLoading) return;
    const requestSeq = ++requestPreviewRef.current;
    setRequestModalOpen(true);
    setRequestLoading(true);
    setRequestPreview(null);
    try {
      const request = await getModelUsageRequest({ id });
      if (requestSeq === requestPreviewRef.current) setRequestPreview(requestPreviewData(request));
    } catch (error) {
      if (requestSeq === requestPreviewRef.current) {
        setRequestModalOpen(false);
        addToast(extractErrorMessage(error, tRef.current("novel.costRequestLoadFailed")), "error");
      }
    } finally {
      if (requestSeq === requestPreviewRef.current) setRequestLoading(false);
    }
  };

  useEffect(() => {
    setRequestModalOpen(false);
    setRequestLoading(false);
    setRequestPreview(null);
    void refresh();
    return () => {
      requestRef.current += 1;
      requestPreviewRef.current += 1;
    };
  }, [projectId]);

  const closeRequestModal = () => {
    requestPreviewRef.current += 1;
    setRequestModalOpen(false);
    setRequestLoading(false);
    setRequestPreview(null);
  };

  if (loading && !report) {
    return <LoadingSpinner size="md" text={t("novel.costLoading")} />;
  }

  return (
    <section className="space-y-4" aria-label={t("novel.costTitle")}>
      <header className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <h2 className="text-base font-semibold text-book-text-main">{t("novel.costTitle")}</h2>
        </div>
        <BookIconButton
          label={t("novel.costRefresh")}
          onClick={() => void refresh()}
          disabled={loading}
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </header>

      {loadError ? <p role="alert" className="text-sm text-error">{loadError}</p> : null}
      {loadError && !report ? null : !report || report.recent.length === 0 ? (
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
          <div className="custom-scrollbar overflow-x-auto border-y border-book-border/40" role="region" tabIndex={0} aria-label={t("novel.costTitle")}>
            <table className="w-full min-w-[760px] text-left text-small">
              <thead className="text-book-text-muted">
                <tr className="border-b border-book-border/40">
                  <th className="px-2 py-2 font-medium">{t("novel.costAmount")}</th>
                  <th className="px-2 py-2 font-medium">{t("novel.costStatus")}</th>
                  <th className="px-2 py-2 font-medium">{t("novel.costCapability")}</th>
                  <th className="px-2 py-2 font-medium">{t("novel.costModel")}</th>
                  <th className="px-2 py-2 font-medium">{t("novel.costTime")}</th>
                  <th className="px-2 py-2 text-right font-medium">{t("novel.costRequest")}</th>
                </tr>
              </thead>
              <tbody>
                {report.recent.map((entry) => (
                  <tr key={entry.id} className="border-b border-book-border/25 last:border-0">
                    <td className="whitespace-nowrap px-2 py-2 font-mono tabular-nums text-book-text-main">
                      {entry.amountMicros == null
                        ? t("novel.costUnpriced")
                        : entry.currency + " " + formatMicros(entry.amountMicros, locale)}
                    </td>
                    <td className="px-2 py-2 text-book-text-muted" title={`${entry.status} · ${entry.confidence}`}>
                      <div className={entry.status === "failed" ? "text-error" : undefined}>{getOperationStatusLabel(entry.status, locale)}</div>
                      <div className="text-caption">{entry.confidence === "reported" ? t("novel.costReported") : entry.confidence === "partial" ? t("novel.costPartial") : entry.confidence === "unpriced" ? t("novel.costUnpriced") : entry.confidence}</div>
                    </td>
                    <td className="px-2 py-2 text-book-text-main">
                      <div>{CAPABILITY_LABELS[entry.capability] ? t(CAPABILITY_LABELS[entry.capability]) : entry.capability}</div>
                      <div className="text-caption text-book-text-muted">{entry.workflow} / {entry.stepKey}</div>
                    </td>
                    <td className="max-w-[240px] truncate px-2 py-2 text-book-text-muted" title={`${entry.provider ?? ""} · ${entry.model ?? ""}`}>
                      {entry.provider ?? "-"} · {entry.model ?? "-"}
                    </td>
                    <td className="whitespace-nowrap px-2 py-2 text-book-text-muted">
                      {formatDateTime(entry.startedAt, locale)}
                    </td>
                    <td className="px-2 py-1 text-right">
                      {entry.requestAvailable ? (
                        <BookIconButton
                          label={t("novel.costViewRequest")}
                          size="sm"
                          disabled={requestLoading}
                          onClick={() => void viewRequest(entry.id)}
                        >
                          <Eye size={14} />
                        </BookIconButton>
                      ) : <span aria-label={t("novel.costRequestUnavailable")}>-</span>}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
      <RequestPreviewModal
        open={requestModalOpen}
        loading={requestLoading}
        data={requestPreview}
        recorded
        fallbackTitle={t("novel.costRequestTitle")}
        onClose={closeRequestModal}
      />
    </section>
  );
};

function requestPreviewData(request: ModelUsageRequest): ModelRequestPreview {
  const requestText = JSON.stringify(request.requestJson);
  const title = `${request.workflow} / ${request.stepKey}`;
  const totalChars = Array.from(requestText).length;
  return {
    workflow: request.workflow,
    title,
    ready_step_count: 1,
    total_chars: totalChars,
    steps: [{
      key: request.requestId,
      title,
      kind: request.capability,
      prompt_name: null,
      state: "ready",
      note: [request.provider, request.model, request.status, request.startedAt].filter(Boolean).join(" · "),
      system_prompt: null,
      user_prompt: null,
      messages: null,
      request: request.requestJson,
      context_manifest: null,
      context_plan: null,
      context_pack_refs: null,
      total_chars: totalChars,
      char_counts: null,
    }],
  };
}

function formatMicros(value: string, locale: string): string {
  const amount = Number(value) / 1_000_000;
  return Number.isFinite(amount)
    ? amount.toLocaleString(locale, { minimumFractionDigits: 2, maximumFractionDigits: 6 })
    : value;
}
