import React, { useEffect, useRef, useState } from "react";
import { AlertTriangle, CheckCircle2, Clock3 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { MangaPipelineStatusSummary } from "../../../api/manga";
import { nonEmptyText } from "../../../utils/text";
import { EmptyHint, type ChapterRow } from "../shared";
import {
  mangaImageStatusMatchesWorkspaceStage,
  mangaPageStageLabel,
  mangaPipelineStageMatchesWorkspaceStage,
  mangaStageLabel,
  type MangaWorkspaceStage,
} from "./mangaTabUtils";
import { useI18n } from "../../../i18n";
import { getScrollRailCue, scrollRailItemIntoView } from "../../../utils/scrollRail";

interface MangaChapterListProps {
  chapters: ChapterRow[];
  statuses: Record<number, MangaPipelineStatusSummary | null>;
  workspaceStage?: MangaWorkspaceStage;
  selectedChapter: ChapterRow | null;
  onSelect: (chapter: ChapterRow) => void;
}

const pipelineStoppedWithError = (
  status: MangaPipelineStatusSummary | null | undefined,
): boolean =>
  status?.pipeline_status === "failed" || status?.pipeline_status === "interrupted";

const chapterPipelineTone = (
  status: MangaPipelineStatusSummary | null | undefined,
): string => {
  if (!status?.pipeline_status) return "text-book-text-muted";
  if (pipelineStoppedWithError(status)) return "text-error";
  if (status.pipeline_status === "completed") return "text-success";
  if (status.pipeline_status === "running") return "text-book-primary";
  return "text-book-text-muted";
};

const chapterPipelineIcon = (status: MangaPipelineStatusSummary | null | undefined) => {
  if (pipelineStoppedWithError(status)) return <AlertTriangle size={11} />;
  if (status?.pipeline_status === "completed") return <CheckCircle2 size={11} />;
  if (status?.pipeline_status === "running") return <LoadingSpinner size="sm" />;
  return <Clock3 size={11} />;
};

export const MangaChapterList: React.FC<MangaChapterListProps> = ({
  chapters,
  statuses,
  workspaceStage,
  selectedChapter,
  onSelect,
}) => {
  const { t } = useI18n();
  const railRef = useRef<HTMLUListElement>(null);
  const selectedButtonRef = useRef<HTMLButtonElement>(null);
  const [hasBefore, setHasBefore] = useState(false);
  const [hasMore, setHasMore] = useState(false);

  useEffect(() => {
    const rail = railRef.current;
    if (!rail) return;
    const updateCue = () => {
      const cue = getScrollRailCue(rail);
      setHasBefore(cue.hasBefore);
      setHasMore(cue.hasMore);
    };
    const frame = window.requestAnimationFrame(() => {
      scrollRailItemIntoView(rail, selectedButtonRef.current, "smooth");
      updateCue();
    });
    rail.addEventListener("scroll", updateCue, { passive: true });
    window.addEventListener("resize", updateCue);
    const resizeObserver =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(updateCue);
    resizeObserver?.observe(rail);
    return () => {
      window.cancelAnimationFrame(frame);
      rail.removeEventListener("scroll", updateCue);
      window.removeEventListener("resize", updateCue);
      resizeObserver?.disconnect();
    };
  }, [chapters.length, selectedChapter?.id]);

  if (chapters.length === 0) {
    return (
      <div className="a4n-flat-surface p-3">
        <EmptyHint text={t("comic.noChapters")} />
      </div>
    );
  }

  return (
    <nav
      aria-label={t("comic.chapterListLabel")}
      aria-describedby="comic-chapter-list-scroll-hint"
      className="a4n-media-sidebar-surface flex min-h-0 min-w-0 flex-col md:max-h-full md:overflow-hidden md:p-2"
    >
      <div className="mb-2 flex shrink-0 items-baseline gap-2 px-1 md:px-2">
        <h2 className="text-xs font-semibold text-book-text-main">{t("novel.chapters")}</h2>
        <span className="text-small text-book-text-muted">{t("comic.chapterCount", { count: chapters.length })}</span>
      </div>
      <ul
        ref={railRef}
        data-has-before={hasBefore ? "true" : undefined}
        data-has-more={hasMore ? "true" : undefined}
        className="a4n-tab-rail-wrap a4n-tab-rail-wrap--canvas a4n-mobile-scroll-rail flex w-full min-w-0 gap-1.5 overflow-x-auto p-1 md:min-h-0 md:flex-1 md:flex-col md:gap-1 md:overflow-x-hidden md:overflow-y-auto md:overscroll-contain md:p-0 custom-scrollbar"
      >
        {chapters.map((chapter) => {
          const status = statuses[chapter.id];
          const imageStatusVisible = mangaImageStatusMatchesWorkspaceStage(workspaceStage);
          const workflowStatus = mangaPipelineStageMatchesWorkspaceStage(
            workspaceStage,
            status?.stage,
          )
            ? status
            : null;
          const total =
            imageStatusVisible && status != null
              ? status.pending + status.generating + status.completed + status.failed + status.cancelled
              : 0;
          const completedRatio = total > 0 ? (status?.completed ?? 0) / total : 0;
          const failedRatio = total > 0 ? (status?.failed ?? 0) / total : 0;
          const generatingRatio = total > 0 ? (status?.generating ?? 0) / total : 0;
          const imageHasFailures = imageStatusVisible && (status?.failed ?? 0) > 0;
          const workflowFailed = pipelineStoppedWithError(workflowStatus);
          const showPipelineStatus =
            imageStatusVisible ||
            workflowStatus?.pipeline_status != null ||
            workflowStatus?.can_resume === true ||
            workflowStatus?.message != null;
          const statusTone = workflowFailed || imageHasFailures
            ? "text-error"
            : workflowStatus
              ? chapterPipelineTone(workflowStatus)
              : imageStatusVisible && (status?.completed ?? 0) > 0
                ? "text-success"
                : "text-book-text-muted";
          const statusIcon = workflowFailed || imageHasFailures
            ? <AlertTriangle size={11} />
            : workflowStatus
              ? chapterPipelineIcon(workflowStatus)
              : imageStatusVisible && (status?.completed ?? 0) > 0
                ? <CheckCircle2 size={11} />
                : <Clock3 size={11} />;
          const stageLabel = workflowStatus?.stage
            ? mangaStageLabel(workflowStatus.stage, t)
            : workspaceStage === "image_creation"
              ? mangaPageStageLabel("image", t)
              : workspaceStage == null
                ? mangaStageLabel(undefined, t)
                : "";
          const isSelected = selectedChapter?.id === chapter.id;
          return (
            <li key={chapter.id}>
              <button
                ref={isSelected ? selectedButtonRef : undefined}
                type="button"
                onClick={() => onSelect(chapter)}
                aria-current={isSelected ? "true" : undefined}
                data-active={isSelected ? "1" : undefined}
                className="a4n-sidebar-item min-w-[12rem] text-xs md:min-w-0"
              >
                <div className="mb-1 truncate font-medium">
                  {t("comic.chapterNumber", { number: chapter.chapter_number })}
                  {nonEmptyText(chapter.title) ? ` · ${nonEmptyText(chapter.title)}` : ""}
                </div>
                {showPipelineStatus ? (
                  <div
                    className={`mb-1 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 text-small ${
                      isSelected ? "text-book-primary" : statusTone
                    }`}
                  >
                    <span className="inline-flex min-w-0 items-center gap-1 truncate">
                      {statusIcon}
                      {stageLabel}
                    </span>
                    {total > 0 ? (
                      <span className="font-mono">
                        {status?.completed ?? 0}/{total}
                      </span>
                    ) : workflowStatus?.can_resume ? (
                      <span>{t("comic.resumable")}</span>
                    ) : (
                      <span>{t("comic.notGenerated")}</span>
                    )}
                  </div>
                ) : null}
                {total > 0 ? (
                  <div
                    className={`flex h-1 overflow-hidden rounded-full ${
                      isSelected ? "bg-book-primary/15" : "bg-book-bg-tertiary"
                    }`}
                  >
                    <span
                      className="bg-success"
                      style={{ width: `${completedRatio * 100}%` }}
                    />
                    <span
                      className="bg-error"
                      style={{ width: `${failedRatio * 100}%` }}
                    />
                    <span
                      className="bg-book-primary"
                      style={{ width: `${generatingRatio * 100}%` }}
                    />
                  </div>
                ) : workflowStatus?.message ? (
                  <div
                    className={`truncate text-small ${
                      isSelected ? "text-book-primary" : "text-book-text-muted"
                    }`}
                  >
                    {workflowStatus.message}
                  </div>
                ) : null}
              </button>
            </li>
          );
        })}
      </ul>
      <span id="comic-chapter-list-scroll-hint" className="sr-only">
        {t("comic.chapterRailScrollHint")}
      </span>
    </nav>
  );
};
