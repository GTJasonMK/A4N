import React from "react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { ImportReview } from "../../../api/import";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { asJsonRecord } from "../../../utils/jsonRecord";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";

interface ImportReviewDetailPanelProps {
  activeReview: ImportReview | null;
  activeReviewJson: string;
  activeReviewJsonError: string | null;
  activeReviewJsonDirty: boolean;
  actBusy: string | null;
  externalBusy: boolean;
  onAnalysisJsonChange: (value: string) => void;
  onModify: () => void | Promise<void>;
}

export const ImportReviewDetailPanel: React.FC<ImportReviewDetailPanelProps> = ({
  activeReview,
  activeReviewJson,
  activeReviewJsonError,
  activeReviewJsonDirty,
  actBusy,
  externalBusy,
  onAnalysisJsonChange,
  onModify,
}) => {
  const actionBusy =
    actBusy !== null || externalBusy || activeReview?.review_status !== "pending";
  const modifyReady = activeReviewJsonDirty && activeReviewJsonError == null;

  if (activeReview == null) return null;
  const analysisSummary = importAnalysisSummary(activeReview.analysis_result);

  return (
    <BookCard className="sticky top-2 h-fit p-3">
      <div className="mb-3 border-b border-book-border/35 pb-3">
        <h3 className="break-words text-sm font-semibold text-book-text-main">
          {activeReview.filename}
        </h3>
        <p className="mt-1 text-small leading-relaxed text-book-text-muted">
          {getOperationStatusLabel(activeReview.review_status)}
        </p>
      </div>
      <div className="space-y-2 text-xs">
        {analysisSummary ? (
          <section className="pb-2.5">
            <div className="a4n-kicker mb-1.5">内容摘要</div>
            {analysisSummary.chapterNumber != null || analysisSummary.title ? (
              <div className="text-small font-medium text-book-text-main">
                {analysisSummary.chapterNumber != null
                  ? `第 ${analysisSummary.chapterNumber} 章`
                  : ""}
                {analysisSummary.chapterNumber != null && analysisSummary.title ? " · " : ""}
                {analysisSummary.title}
              </div>
            ) : null}
            {analysisSummary.summary ? (
              <p className="mt-2 text-small leading-relaxed text-book-text-sub">
                {analysisSummary.summary}
              </p>
            ) : null}
            {analysisSummary.content ? (
              <p className="mt-2 line-clamp-8 whitespace-pre-wrap text-small leading-relaxed text-book-text-muted">
                {analysisSummary.content}
              </p>
            ) : null}
          </section>
        ) : null}
        <details className="border-t border-book-border/35 pt-2">
          <summary className="flex cursor-pointer list-none items-center justify-between gap-3 rounded-md py-1.5 text-small text-book-text-muted hover:text-book-text-main [&::-webkit-details-marker]:hidden">
            <span>高级编辑</span>
            <span className="text-caption">章节导入结构</span>
          </summary>
          <div className="mt-2">
            <label className="mb-1 block text-caption text-book-text-muted">分析结构</label>
            <textarea
              value={activeReviewJson}
              disabled={actionBusy}
              onChange={(e) => onAnalysisJsonChange(e.target.value)}
              className="pyqt-input h-48 w-full resize-y font-mono text-small leading-relaxed"
            />
            {activeReviewJsonError && (
              <div role="alert" className="mt-1 text-small text-error">{activeReviewJsonError}</div>
            )}
            <div className="mt-2 flex justify-end">
              <BookButton
                variant="secondary"
                onClick={() => void onModify()}
                disabled={actionBusy || !modifyReady}
              >
                {actBusy?.startsWith("modify-") ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" /> 保存中…
                  </span>
                ) : (
                  "保存并接受"
                )}
              </BookButton>
            </div>
          </div>
        </details>
      </div>
    </BookCard>
  );
};

interface ImportAnalysisSummary {
  chapterNumber: number | null;
  title: string;
  summary: string;
  content: string;
}

function importAnalysisSummary(value: unknown): ImportAnalysisSummary | null {
  const record = asJsonRecord(value);
  if (!record) return null;
  const chapterNumber =
    typeof record.chapter_number === "number" && Number.isFinite(record.chapter_number)
      ? record.chapter_number
      : null;
  const title = firstText(record.chapter_title);
  const summary = firstText(record.summary);
  const content = firstText(record.content);
  if (chapterNumber == null && !title && !summary && !content) return null;
  return { chapterNumber, title, summary, content };
}

function firstText(...values: unknown[]): string {
  for (const value of values) {
    if (typeof value !== "string") continue;
    const text = value.trim();
    if (text) return text;
  }
  return "";
}
