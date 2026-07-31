import React from "react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { ImportReview, ImportReviewStatus } from "../../../api/import";
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
  onUpdateStatus: (id: number, status: ImportReviewStatus) => void | Promise<void>;
}

const IMPORT_REVIEW_STATUS_OPTIONS: ImportReviewStatus[] = [
  "pending",
  "analyzing",
  "reviewing",
  "accepted",
  "rejected",
  "modified",
];

export const ImportReviewDetailPanel: React.FC<ImportReviewDetailPanelProps> = ({
  activeReview,
  activeReviewJson,
  activeReviewJsonError,
  activeReviewJsonDirty,
  actBusy,
  externalBusy,
  onAnalysisJsonChange,
  onModify,
  onUpdateStatus,
}) => {
  const actionBusy = actBusy !== null || externalBusy;
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
          {activeReview.file_type.toUpperCase()} · {getOperationStatusLabel(activeReview.review_status)} ·{" "}
          {activeReview.parsed_chapters} 章
          {activeReview.dedup_rate != null
            ? ` · 去重 ${(activeReview.dedup_rate * 100).toFixed(1)}%`
            : ""}
        </p>
        {activeReview.detected_encoding || activeReview.dedup_paragraphs > 0 ? (
          <p className="mt-0.5 text-small text-book-text-muted">
            {activeReview.detected_encoding ? `编码 ${activeReview.detected_encoding}` : ""}
            {activeReview.detected_encoding && activeReview.dedup_paragraphs > 0 ? " · " : ""}
            {activeReview.dedup_paragraphs > 0 ? `去重段落 ${activeReview.dedup_paragraphs}` : ""}
          </p>
        ) : null}
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
            <span className="text-caption">分析结构与审核状态</span>
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
            <div className="mt-2 flex flex-col-reverse gap-2 sm:flex-row sm:items-center sm:justify-between">
              <select
                className="pyqt-input !px-2 !py-1 text-small"
                defaultValue=""
                disabled={actionBusy}
                onChange={(e) => {
                  const status = e.target.value as ImportReviewStatus;
                  if (status) void onUpdateStatus(activeReview.id, status);
                  e.currentTarget.value = "";
                }}
              >
                <option value="" disabled>
                  更新状态…
                </option>
                {IMPORT_REVIEW_STATUS_OPTIONS.map((status) => (
                  <option key={status} value={status}>
                    {getOperationStatusLabel(status)}
                  </option>
                ))}
              </select>
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
                  "保存修改"
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
  const title = firstText(record.modified_title, record.chapter_title, record.title);
  const summary = firstText(record.modified_summary, record.summary);
  const content = firstText(record.modified_content, record.content_preview, record.content);
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
