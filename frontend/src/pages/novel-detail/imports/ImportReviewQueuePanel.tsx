import React from "react";
import { Eye, Loader2, Upload as ImportIcon } from "lucide-react";
import type {
  ImportAnalysisStatus,
  ImportReview,
  ProjectImportAnalysisProgress,
} from "../../../api/import";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { progressDisplayText } from "../../../utils/progressDisplay";
import { EmptyHint } from "../shared";
import { projectProgressPercent } from "./importTabUtils";

export type ReviewAction = "accept" | "reject" | "delete";

interface ImportReviewQueuePanelProps {
  reviews: ImportReview[];
  loading: boolean;
  analysis: ImportAnalysisStatus[];
  projectProgress: ProjectImportAnalysisProgress | null;
  selected: Set<number>;
  analyzeBusy: boolean;
  canCancelAnalysis: boolean;
  cancelBusy: boolean;
  batchBusy: boolean;
  actBusy: string | null;
  onStartAnalysis: () => void | Promise<void>;
  onPreviewAnalysisRequest: () => void | Promise<void>;
  onCancelAnalysis: () => void | Promise<void>;
  onBatchAccept: () => void | Promise<void>;
  onToggleSelect: (id: number) => void;
  onViewDetail: (id: number) => void | Promise<void>;
  onAct: (id: number, action: ReviewAction) => void | Promise<void>;
}

const reviewStatusClassName = (status: string): string => {
  if (status === "accepted") return "!border-success/50 !text-success";
  if (status === "rejected") return "!border-error/50 !text-error";
  return "";
};

export const ImportReviewQueuePanel: React.FC<ImportReviewQueuePanelProps> = ({
  reviews,
  loading,
  analysis,
  projectProgress,
  selected,
  analyzeBusy,
  canCancelAnalysis,
  cancelBusy,
  batchBusy,
  actBusy,
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
  const progressPercent = projectProgress ? projectProgressPercent(projectProgress) : null;
  const progressStageName = projectProgress?.current_stage
    ? projectProgress.stages?.[projectProgress.current_stage]?.name
    : undefined;
  const progressDisplay = projectProgress
    ? progressDisplayText({
        message: projectProgress.message,
        stageHint: progressStageName ?? projectProgress.current_stage,
        fallbackStage: "导入项目分析",
      })
    : null;
  const reviewActionBusy = actBusy !== null;
  const reviewSelectionLocked = batchBusy || reviewActionBusy;

  return (
    <BookCard className="p-4">
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-sm font-serif text-book-text-main inline-flex items-center gap-2">
          <ImportIcon size={14} className="text-book-primary" /> 审核队列（{reviews.length}）
        </h3>
        <div className="inline-flex min-w-0 flex-1 flex-wrap items-center justify-end gap-2 text-xs">
          <BookButton
            variant="secondary"
            onClick={() => void onPreviewAnalysisRequest()}
            disabled={analyzeBusy || cancelBusy || analysisStatus === "analyzing"}
          >
            <Eye size={11} />
            预览请求
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={() => void onStartAnalysis()}
            disabled={analyzeBusy || cancelBusy || analysisStatus === "analyzing"}
          >
            {analyzeBusy ? (
              <Loader2 size={11} className="animate-spin" />
            ) : analysisStatus === "analyzing" ? (
              "分析中"
            ) : (
              analyzeActionLabel
            )}
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={() => void onCancelAnalysis()}
            disabled={cancelBusy || !canCancelAnalysis}
          >
            {cancelBusy ? <Loader2 size={11} className="animate-spin" /> : "取消分析"}
          </BookButton>
          <BookButton
            variant="primary"
            onClick={() => void onBatchAccept()}
            disabled={batchBusy || reviewActionBusy || selected.size === 0}
          >
            {batchBusy ? (
              <Loader2 size={11} className="animate-spin" />
            ) : (
              `批量接受 (${selected.size})`
            )}
          </BookButton>
        </div>
      </div>

      {(analysis.length > 0 || projectProgress) && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2 mb-2 text-[10px]">
          {analysis.length > 0 && (
            <div className="rounded border border-book-border/40 bg-book-bg-tertiary/30 p-2">
              <div className="text-book-text-muted mb-1">审核状态</div>
              <div className="flex flex-wrap gap-1">
                {analysis.map((item) => (
                  <span key={item.status} className="pyqt-pill !text-[10px]">
                    {item.status}: {item.count}
                  </span>
                ))}
              </div>
            </div>
          )}
          {projectProgress && (
            <div className="rounded border border-book-border/40 bg-book-bg-tertiary/30 p-2">
              <div className="flex items-center justify-between gap-2">
                <span className="min-w-0 truncate font-semibold text-book-text-sub">
                  {progressDisplay?.stage ?? "项目分析"}
                </span>
                <span className="font-mono">
                  {progressPercent == null ? "未返回" : `${progressPercent}%`}
                </span>
              </div>
              <div className="mt-1 flex items-center gap-2">
                <span className="text-book-text-muted">状态</span>
                <span className="pyqt-pill !text-[10px]">{projectProgress.status}</span>
              </div>
              <div className="relative mt-1 h-4 overflow-hidden rounded border border-book-border/50 bg-book-bg-paper">
                <div
                  className="h-full bg-book-primary/35"
                  style={{ width: progressPercent == null ? "0%" : `${progressPercent}%` }}
                />
                <div className="pointer-events-none absolute inset-x-1 top-1/2 -translate-y-1/2 truncate text-[10px] leading-3 text-book-text-main">
                  {progressDisplay?.detail ?? "等待子阶段更新…"}
                </div>
              </div>
              {!progressDisplay?.detail ? (
                <div className="mt-1 text-book-text-muted">后端未返回进度消息</div>
              ) : null}
              {projectProgress.error ? (
                <div className="mt-1 text-[10px] leading-4 text-error truncate">
                  {projectProgress.error}
                </div>
              ) : null}
            </div>
          )}
        </div>
      )}

      {reviews.length === 0 ? (
        <EmptyHint text={loading ? "加载中…" : "队列为空"} />
      ) : (
        <ul className="space-y-1">
          {reviews.map((review) => (
            <li
              key={review.id}
              className="a4n-stat-tile px-2 py-1.5 text-xs"
            >
              <div className="flex items-center gap-2 flex-wrap">
                <input
                  type="checkbox"
                  checked={selected.has(review.id)}
                  disabled={reviewSelectionLocked}
                  onChange={() => onToggleSelect(review.id)}
                  className="accent-book-primary"
                />
                <span className="font-mono text-[10px] text-book-text-muted shrink-0">
                  #{review.id}
                </span>
                <span className="pyqt-pill !text-[10px]">{review.file_type}</span>
                <span
                  className={`pyqt-pill !text-[10px] ${reviewStatusClassName(
                    review.review_status,
                  )}`}
                >
                  {review.review_status}
                </span>
                <span className="truncate flex-1 text-book-text-main">{review.filename}</span>
                <span className="text-book-text-muted text-[10px] shrink-0">
                  {review.parsed_chapters} 章 ·{" "}
                  {review.dedup_rate != null
                    ? `${(review.dedup_rate * 100).toFixed(0)}%`
                    : "?"}{" "}
                  去重
                </span>
                <button
                  type="button"
                  onClick={() => void onViewDetail(review.id)}
                  disabled={reviewSelectionLocked}
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[24px] sm:!px-1.5"
                >
                  详情
                </button>
                <button
                  type="button"
                  onClick={() => void onAct(review.id, "accept")}
                  disabled={
                    reviewSelectionLocked || review.review_status === "accepted"
                  }
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] !border-success/40 !text-success sm:!min-h-[24px] sm:!px-1.5"
                >
                  接受
                </button>
                <button
                  type="button"
                  onClick={() => void onAct(review.id, "reject")}
                  disabled={
                    reviewSelectionLocked || review.review_status === "rejected"
                  }
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] !border-warning/40 !text-warning sm:!min-h-[24px] sm:!px-1.5"
                >
                  拒绝
                </button>
                <button
                  type="button"
                  onClick={() => void onAct(review.id, "delete")}
                  disabled={reviewSelectionLocked}
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] !border-error/50 !text-error sm:!min-h-[24px] sm:!px-1.5"
                >
                  删除
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </BookCard>
  );
};
