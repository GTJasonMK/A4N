import React from "react";
import { AlertTriangle, CheckCircle2, Clock3 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { MangaPipelineStatusSummary } from "../../../api/manga";
import { nonEmptyText } from "../../../utils/text";
import { EmptyHint, type ChapterRow } from "../shared";
import { mangaStageLabel } from "./mangaTabUtils";
import { useI18n } from "../../../i18n";
import { ThemedSelect } from "../../../components/ui/ThemedSelect";

interface MangaChapterListProps {
  chapters: ChapterRow[];
  statuses: Record<number, MangaPipelineStatusSummary | null>;
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
  selectedChapter,
  onSelect,
}) => {
  const { t } = useI18n();
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
      className="a4n-media-sidebar-surface flex min-h-0 min-w-0 flex-col md:max-h-full md:overflow-hidden md:p-2"
    >
      <label className="mb-3 flex min-w-0 items-center gap-2 border-b border-book-border/45 pb-3 text-xs md:hidden">
        <span className="shrink-0 font-medium text-book-text-muted">{t("novel.chapters")}</span>
        <ThemedSelect
          aria-label={t("comic.chapterListLabel")}
          value={selectedChapter?.id ?? ""}
          onChange={(value) => {
            const chapter = chapters.find((item) => String(item.id) === value);
            if (chapter) onSelect(chapter);
          }}
          className="pyqt-input h-9 min-w-0 flex-1 !px-2 !py-1.5 text-xs"
        >
          {chapters.map((chapter) => (
            <option key={chapter.id} value={chapter.id}>
              {t("comic.chapterNumber", { number: chapter.chapter_number })}
              {nonEmptyText(chapter.title) ? ` · ${nonEmptyText(chapter.title)}` : ""}
            </option>
          ))}
        </ThemedSelect>
      </label>
      <div className="mb-2 hidden shrink-0 items-baseline gap-2 px-2 md:flex">
        <h2 className="text-xs font-semibold text-book-text-main">{t("novel.chapters")}</h2>
        <span className="text-small text-book-text-muted">{t("comic.chapterCount", { count: chapters.length })}</span>
      </div>
      <ul className="flex w-full min-w-0 gap-1.5 overflow-x-auto p-1 md:min-h-0 md:flex-1 md:flex-col md:gap-1 md:overflow-x-hidden md:overflow-y-auto md:overscroll-contain md:p-0 custom-scrollbar">
        {chapters.map((chapter) => {
          const status = statuses[chapter.id];
          const total =
            status != null
              ? status.pending + status.generating + status.completed + status.failed + status.cancelled
              : 0;
          const completedRatio = total > 0 ? (status?.completed ?? 0) / total : 0;
          const failedRatio = total > 0 ? (status?.failed ?? 0) / total : 0;
          const generatingRatio = total > 0 ? (status?.generating ?? 0) / total : 0;
          const isSelected = selectedChapter?.id === chapter.id;
          return (
            <li key={chapter.id}>
              <button
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
                <div
                  className={`mb-1 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 text-small ${
                    isSelected ? "text-book-primary" : chapterPipelineTone(status)
                  }`}
                >
                  <span className="inline-flex min-w-0 items-center gap-1 truncate">
                    {chapterPipelineIcon(status)}
                    {mangaStageLabel(status?.stage, t)}
                  </span>
                  {total > 0 ? (
                    <span className="font-mono">
                      {status?.completed ?? 0}/{total}
                    </span>
                  ) : status?.can_resume ? (
                    <span>{t("comic.resumable")}</span>
                  ) : (
                    <span>{t("comic.notGenerated")}</span>
                  )}
                </div>
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
                ) : status?.message ? (
                  <div
                    className={`truncate text-small ${
                      isSelected ? "text-book-primary" : "text-book-text-muted"
                    }`}
                  >
                    {status.message}
                  </div>
                ) : null}
              </button>
            </li>
          );
        })}
      </ul>
    </nav>
  );
};
