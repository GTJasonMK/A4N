import React from "react";
import { Check, CheckCheck, Eye, Sparkles, Square, Trash2, X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type {
  ImportReviewStatus,
  ImportReviewSummary,
  ProjectImportAnalysisProgress,
} from "../../../api/import";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { progressDisplayText } from "../../../utils/progressDisplay";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { EmptyHint } from "../shared";

export type ReviewAction = "accept" | "reject" | "delete";

interface ImportReviewQueuePanelProps {
  reviews: ImportReviewSummary[];
  loading: boolean;
  projectProgress: ProjectImportAnalysisProgress | null;
  selected: Set<number>;
  analyzeBusy: boolean;
  canCancelAnalysis: boolean;
  cancelBusy: boolean;
  batchBusy: boolean;
  actBusy: string | null;
  externalBusy: boolean;
  onStartAnalysis: () => void | Promise<void>;
  onPreviewAnalysisRequest: () => void | Promise<void>;
  onCancelAnalysis: () => void | Promise<void>;
  onBatchAccept: () => void | Promise<void>;
  onToggleSelect: (id: number) => void;
  onViewDetail: (id: number) => void | Promise<void>;
  onAct: (id: number, action: ReviewAction) => void | Promise<void>;
}

export const ImportReviewQueuePanel: React.FC<ImportReviewQueuePanelProps> = ({
  reviews,
  loading,
  projectProgress,
  selected,
  analyzeBusy,
  canCancelAnalysis,
  cancelBusy,
  batchBusy,
  actBusy,
  externalBusy,
  onStartAnalysis,
  onPreviewAnalysisRequest,
  onCancelAnalysis,
  onBatchAccept,
  onToggleSelect,
  onViewDetail,
  onAct,
}) => {
  const analysisStatus = projectProgress?.status ?? null;
  const analyzeActionLabel =
    analysisStatus === "failed" || analysisStatus === "cancelled" ? "继续分析" : "分析项目";
  const progressDisplay = projectProgress
    ? progressDisplayText({
        message: projectProgress.message,
        stageHint: projectProgress.stages.generating_analysis_data.name,
      })
    : null;
  const reviewActionBusy = actBusy !== null;
  const reviewSelectionLocked =
    batchBusy || reviewActionBusy || externalBusy || analysisStatus === "analyzing";
  const visibleAnalysis = Object.entries(
    reviews.reduce<Partial<Record<ImportReviewStatus, number>>>((counts, review) => {
      counts[review.review_status] = (counts[review.review_status] ?? 0) + 1;
      return counts;
    }, {}),
  ).map(([status, count]) => ({ status: status as ImportReviewStatus, count }));

  return (
    <section className="space-y-3">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <h3 className="text-sm font-semibold text-book-text-main">审核队列</h3>
          {reviews.length > 0 ? (
            <p className="mt-0.5 text-small text-book-text-muted">
              {reviews.length} 条记录
              {selected.size > 0 ? ` · 已选 ${selected.size} 条` : ""}
            </p>
          ) : null}
        </div>
        <div className="flex items-center justify-end gap-2">
          {analysisStatus === "analyzing" ? (
            canCancelAnalysis || cancelBusy ? (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => void onCancelAnalysis()}
                disabled={cancelBusy}
                className="!min-h-8 !text-xs"
              >
                {cancelBusy ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" /> 停止中…
                  </span>
                ) : (
                  <>
                    <Square size={12} /> 停止分析
                  </>
                )}
              </BookButton>
            ) : null
          ) : selected.size > 0 ? (
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onBatchAccept()}
              disabled={batchBusy || reviewActionBusy || externalBusy}
              className="!min-h-8 !text-xs"
            >
              {batchBusy ? (
                <span className="inline-flex items-center gap-1.5">
                  <LoadingSpinner size="sm" /> 接受中（{selected.size}）…
                </span>
              ) : (
                <>
                  <CheckCheck size={13} /> 接受所选（{selected.size}）
                </>
              )}
            </BookButton>
          ) : (
            <>
              {!analyzeBusy && !cancelBusy && !externalBusy ? (
                <BookIconButton
                  label="查看分析模型请求"
                  variant="secondary"
                  size="sm"
                  onClick={() => void onPreviewAnalysisRequest()}
                >
                  <Eye size={14} />
                </BookIconButton>
              ) : null}
              <BookButton
                variant={selected.size > 0 ? "secondary" : "primary"}
                size="sm"
                onClick={() => void onStartAnalysis()}
                disabled={analyzeBusy || cancelBusy || externalBusy}
                className="!min-h-8 !text-xs"
              >
                {analyzeBusy ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" />
                    {analyzeActionLabel === "继续分析" ? "继续分析中…" : "分析中…"}
                  </span>
                ) : (
                  <>
                    <Sparkles size={13} /> {analyzeActionLabel}
                  </>
                )}
              </BookButton>
            </>
          )}
        </div>
      </div>

      {(visibleAnalysis.length > 0 || projectProgress) && (
        <div className="a4n-flat-surface divide-y divide-book-border/35 overflow-hidden px-3 text-small">
          {visibleAnalysis.length > 0 && (
            <div className="flex flex-wrap items-center gap-2 py-2">
              <div className="shrink-0 text-book-text-muted">审核状态</div>
              <div className="min-w-0 flex-1 text-book-text-sub">
                {visibleAnalysis
                  .map((item) => `${getOperationStatusLabel(item.status)} ${item.count}`)
                  .join(" · ")}
              </div>
            </div>
          )}
          {projectProgress && (
            <div className="py-2">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium text-book-text-main">
                  {projectProgress.stages.generating_analysis_data.name}
                </span>
                <span className="text-book-text-muted">{getOperationStatusLabel(projectProgress.status)}</span>
                <span className="ml-auto font-mono text-book-text-muted">
                  {projectProgress.overall_progress}%
                </span>
              </div>
              <div className="relative mt-1.5 h-4 overflow-hidden rounded bg-book-bg-tertiary">
                <div
                  className="h-full bg-book-primary/35"
                  style={{ width: `${projectProgress.overall_progress}%` }}
                />
                {progressDisplay?.detail ? (
                  <div className="pointer-events-none absolute inset-x-1.5 top-1/2 -translate-y-1/2 truncate text-caption leading-3 text-book-text-main">
                    {progressDisplay.detail}
                  </div>
                ) : null}
              </div>
              {projectProgress.error ? (
                <div role="alert" className="mt-1 truncate text-small leading-4 text-error">
                  {projectProgress.error}
                </div>
              ) : null}
            </div>
          )}
        </div>
      )}

      {reviews.length === 0 ? (
        <EmptyHint text={loading ? "加载中…" : "暂无待审核内容。"} />
      ) : (
        <ul className="grid gap-2">
          {reviews.map((review) => {
            const reviewSelectable =
              review.review_status === "pending" && !reviewSelectionLocked;
            return (
            <li key={review.id} className="a4n-flat-surface p-3 text-xs">
              <div className="flex flex-wrap items-start gap-2">
                <button
                  type="button"
                  aria-pressed={selected.has(review.id)}
                  aria-label={`选择 ${review.filename}`}
                  disabled={!reviewSelectable}
                  onClick={() => onToggleSelect(review.id)}
                  className={`mt-0.5 inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-[9px] border transition-colors focus-visible:outline-2 focus-visible:outline-book-primary/35 disabled:cursor-not-allowed disabled:opacity-50 ${
                    selected.has(review.id)
                      ? "border-book-primary bg-book-primary text-white"
                      : "border-book-border/70 bg-book-bg-paper/45 text-transparent hover:border-book-primary/55"
                  }`}
                >
                  <Check size={13} />
                </button>
                <div className="min-w-0 flex-1">
                  <button
                    type="button"
                    onClick={() => void onViewDetail(review.id)}
                    disabled={reviewSelectionLocked}
                    className="block w-full rounded-md text-left outline-none transition-colors hover:text-book-primary focus-visible:ring-2 focus-visible:ring-book-primary/35 disabled:opacity-60"
                    aria-label={`查看 ${review.filename} 的审核详情`}
                  >
                    <span className="block truncate font-medium text-book-text-main">
                      {review.filename}
                    </span>
                    <span className="mt-1 block text-small text-book-text-muted">
                      {getOperationStatusLabel(review.review_status)}
                    </span>
                  </button>
                </div>
                {!reviewSelectionLocked ? (
                  <Dropdown
                    items={[
                      ...(review.review_status === "pending"
                        ? [
                            { label: "接受审核", icon: <Check size={12} />, onClick: () => void onAct(review.id, "accept") },
                            { label: "拒绝审核", icon: <X size={12} />, onClick: () => void onAct(review.id, "reject") },
                            { type: "divider" as const },
                          ]
                        : []),
                      {
                        label: "删除审核记录",
                        icon: <Trash2 size={12} />,
                        danger: true,
                        onClick: () => void onAct(review.id, "delete"),
                      },
                    ]}
                  />
                ) : null}
              </div>
            </li>
            );
          })}
        </ul>
      )}
    </section>
  );
};
