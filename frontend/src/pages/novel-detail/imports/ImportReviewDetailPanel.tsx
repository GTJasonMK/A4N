import React from "react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { ImportReviewSummary } from "../../../api/import";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { asJsonRecord } from "../../../utils/jsonRecord";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { useI18n } from "../../../i18n";

interface ImportReviewDetailPanelProps {
  activeReview: ImportReviewSummary | null;
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
  const { locale, t } = useI18n();
  const analysisStructureId = React.useId();
  const reviewTitleRef = React.useRef<HTMLHeadingElement>(null);
  const actionBusy =
    actBusy !== null || externalBusy || activeReview?.review_status !== "pending";
  const modifyReady = activeReviewJsonDirty && activeReviewJsonError == null;

  React.useEffect(() => {
    if (actBusy?.startsWith("modify-")) reviewTitleRef.current?.focus();
  }, [actBusy]);

  const modifyDisabledHint = activeReview?.review_status !== "pending" || activeReviewJsonError
    ? null
    : actBusy?.startsWith("modify-")
      ? null
      : actBusy !== null || externalBusy
        ? t("novel.importReview.detail.actionInProgress")
        : !activeReviewJsonDirty
          ? t("novel.importReview.detail.editToEnable")
          : null;

  if (activeReview == null) return null;
  const analysisSummary = importAnalysisSummary(activeReview.analysis_result);

  return (
    <BookCard className="sticky top-2 h-fit p-3">
      <div className="mb-3 border-b border-book-border/35 pb-3">
        <h3
          ref={reviewTitleRef}
          tabIndex={-1}
          className="break-words text-sm font-semibold text-book-text-main focus:outline-2 focus:outline-book-primary/35 focus:outline-offset-2"
        >
          {activeReview.filename}
        </h3>
        <p role="status" className="mt-1 text-small leading-relaxed text-book-text-muted">
          {getOperationStatusLabel(activeReview.review_status, locale)}
        </p>
      </div>
      <div className="space-y-2 text-xs">
        {analysisSummary ? (
          <section className="pb-2.5">
            <div className="a4n-kicker mb-1.5">{t("novel.importReview.detail.summary")}</div>
            {analysisSummary.chapterNumber != null || analysisSummary.title ? (
              <div className="text-small font-medium text-book-text-main">
                {analysisSummary.chapterNumber != null
                  ? t("novel.importReview.detail.chapter", { number: analysisSummary.chapterNumber })
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
            <span>{t("novel.importReview.detail.advancedEdit")}</span>
            <span className="text-caption">{t("novel.importReview.detail.importStructure")}</span>
          </summary>
          <div className="mt-2">
            <label htmlFor={analysisStructureId} className="mb-1 block text-caption text-book-text-muted">{t("novel.importReview.detail.analysisStructure")}</label>
            <textarea
              id={analysisStructureId}
              value={activeReviewJson}
              disabled={actionBusy}
              onChange={(e) => onAnalysisJsonChange(e.target.value)}
              className="pyqt-input h-48 w-full resize-y font-mono text-small leading-relaxed"
            />
            {activeReviewJsonError && (
              <div role="alert" className="mt-1 text-small text-error">{activeReviewJsonError}</div>
            )}
            <div className="mt-2 flex flex-wrap items-center justify-between gap-2">
              {modifyDisabledHint ? (
                <p role="status" className="min-w-0 flex-1 text-caption leading-relaxed text-book-text-muted">
                  {modifyDisabledHint}
                </p>
              ) : null}
              <BookButton
                variant="secondary"
                onClick={() => void onModify()}
                disabled={actionBusy || !modifyReady}
                className="ml-auto shrink-0"
              >
                {actBusy?.startsWith("modify-") ? (
                  <span role="status" className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" /> {t("novel.importReview.detail.saving")}
                  </span>
                ) : (
                  t("novel.importReview.detail.saveAndAccept")
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
