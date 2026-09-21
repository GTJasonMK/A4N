import React from "react";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type {
  ChapterOutlinePacing,
  OutlineGenerationProgressSummary,
} from "../../../api/outline";
import { useI18n } from "../../../i18n";
import { CompletionRow, PacingSummary } from "./OutlineAdvancedPanelSections";

interface OutlineProgressPacingGridProps {
  progress: OutlineGenerationProgressSummary | null;
  progressLoading: boolean;
  autoPoll: boolean;
  onToggleAutoPoll: () => void;
  onLoadProgress: () => void;
  disabled: boolean;
  controlsDisabled: boolean;
  pacing: ChapterOutlinePacing | null;
  pacingLoading: boolean;
  onLoadPacing: () => void;
}

export const OutlineProgressPacingGrid: React.FC<
  OutlineProgressPacingGridProps
> = ({
  progress,
  progressLoading,
  autoPoll,
  onToggleAutoPoll,
  onLoadProgress,
  disabled,
  controlsDisabled,
  pacing,
  pacingLoading,
  onLoadPacing,
}) => {
  const { t } = useI18n();
  return (
    <div className="grid gap-3 lg:grid-cols-2">
      <section className="a4n-flat-surface p-3">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <h4 className="text-sm font-medium text-book-text-main">
            {t("outline.advancedPanel.progressTitle")}
          </h4>
          <button
            type="button"
            role="switch"
            aria-checked={autoPoll}
            onClick={() => onToggleAutoPoll()}
            disabled={controlsDisabled}
            className="ml-auto inline-flex items-center gap-2 text-small text-book-text-muted disabled:cursor-not-allowed disabled:opacity-50"
            title={
              autoPoll
                ? t("outline.advancedPanel.autoRefreshOn")
                : t("outline.advancedPanel.autoRefreshOff")
            }
          >
            <span>{t("outline.advancedPanel.autoRefresh")}</span>
            <span
              className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
                autoPoll ? "bg-book-primary" : "bg-book-text-muted/25"
              }`}
            >
              <span
                className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
                  autoPoll ? "translate-x-4" : "translate-x-[2px]"
                }`}
              />
            </span>
          </button>
          <BookButton
            variant="ghost"
            size="sm"
            onClick={onLoadProgress}
            disabled={disabled || progressLoading}
            className="!px-2 !text-small"
          >
            {progressLoading ? (
              <span className="inline-flex items-center gap-1.5">
                <LoadingSpinner size="sm" />{" "}
                {t("outline.advancedPanel.refreshing")}
              </span>
            ) : (
              t("outline.advancedPanel.refresh")
            )}
          </BookButton>
        </div>
        {progress ? (
          <div className="space-y-3">
            <CompletionRow
              label={t("outline.advancedPanel.partsProgress")}
              current={progress.completed_parts}
              total={progress.total_parts}
            />
            <CompletionRow
              label={t("outline.advancedPanel.chaptersProgress")}
              current={progress.completed_chapter_outlines}
              total={progress.total_chapter_outlines}
            />
          </div>
        ) : (
          <p className="text-small text-book-text-muted">
            {t("outline.advancedPanel.noProgress")}
          </p>
        )}
      </section>

      <section className="a4n-flat-surface p-3">
        <div className="mb-3 flex items-center gap-2">
          <div className="min-w-0 flex-1">
            <h4 className="text-sm font-medium text-book-text-main">
              {t("outline.advancedPanel.pacingTitle")}
            </h4>
            {!pacing ? (
              <p className="mt-0.5 text-small text-book-text-muted">
                {t("outline.advancedPanel.pacingHint")}
              </p>
            ) : null}
          </div>
          {pacingLoading ? (
            <span
              role="status"
              className="inline-flex min-h-8 shrink-0 items-center gap-1.5 text-small text-book-text-muted"
              title={t("outline.advancedPanel.analyzingPacing")}
            >
              <LoadingSpinner size="sm" />{" "}
              {t("outline.advancedPanel.analyzing")}
            </span>
          ) : pacing ? (
            <Dropdown
              items={[
                {
                  label: t("outline.advancedPanel.reanalyzePacing"),
                  onClick: onLoadPacing,
                  disabled,
                },
              ]}
            />
          ) : (
            <BookButton
              variant="secondary"
              size="sm"
              onClick={onLoadPacing}
              disabled={disabled}
              className="!text-xs"
            >
              {t("outline.advancedPanel.startAnalysis")}
            </BookButton>
          )}
        </div>
        {pacing ? <PacingSummary data={pacing} /> : null}
      </section>
    </div>
  );
};
