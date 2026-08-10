import React from "react";
import { AlertTriangle, CheckCircle2, Clock3 } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { MangaPipelineStatusSummary } from "../../../api/manga";
import { nonEmptyText } from "../../../utils/text";
import { EmptyHint, type ChapterRow } from "../shared";
import { mangaStageLabel } from "./mangaTabUtils";

interface MangaChapterListProps {
  chapters: ChapterRow[];
  statuses: Record<number, MangaPipelineStatusSummary | null>;
  selectedChapter: ChapterRow | null;
  onSelect: (chapter: ChapterRow) => void;
}

const chapterPipelineTone = (
  status: MangaPipelineStatusSummary | null | undefined,
): string => {
  if (!status?.pipeline_status) return "text-book-text-muted";
  if (status.pipeline_status === "failed") return "text-error";
  if (status.pipeline_status === "completed") return "text-success";
  if (status.pipeline_status === "running") return "text-book-primary";
  return "text-book-text-muted";
};

const chapterPipelineIcon = (status: MangaPipelineStatusSummary | null | undefined) => {
  if (status?.pipeline_status === "failed") return <AlertTriangle size={11} />;
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
  if (chapters.length === 0) {
    return (
      <div className="a4n-flat-surface p-3">
        <EmptyHint text="项目还没有章节。先生成章节，再来这里启动漫画流水线。" />
      </div>
    );
  }

  return (
    <nav aria-label="漫画章节" className="flex min-h-0 flex-col">
      <div className="mb-2 flex shrink-0 items-baseline gap-2 px-1">
        <h2 className="text-xs font-semibold text-book-text-main">章节</h2>
        <span className="text-small text-book-text-muted">{chapters.length} 章</span>
      </div>
      <ul className="flex w-full min-w-0 gap-1.5 overflow-x-auto pb-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden lg:flex-1 lg:flex-col lg:gap-0 lg:overflow-x-hidden lg:overflow-y-auto lg:rounded-xl lg:border lg:border-book-border/50 lg:bg-book-bg-paper/30">
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
                className={`min-w-[12rem] rounded-xl border px-3 py-2.5 text-left text-xs transition-colors lg:w-full lg:min-w-0 lg:rounded-none lg:border-0 ${
                  isSelected
                    ? "border-book-primary/55 bg-book-primary/10 text-book-primary lg:shadow-[inset_2px_0_0_rgb(var(--color-primary))]"
                    : "border-book-border/50 bg-book-bg-paper/45 text-book-text-sub hover:border-book-primary/35 hover:bg-book-bg-tertiary hover:text-book-text-main lg:border-transparent lg:bg-transparent"
                }`}
              >
                <div className="mb-1 truncate font-medium text-book-text-main">
                  第 {chapter.chapter_number} 章
                  {nonEmptyText(chapter.title) ? ` · ${nonEmptyText(chapter.title)}` : ""}
                </div>
                <div
                  className={`mb-1 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 text-small ${
                    isSelected ? "text-book-primary" : chapterPipelineTone(status)
                  }`}
                >
                  <span className="inline-flex min-w-0 items-center gap-1 truncate">
                    {chapterPipelineIcon(status)}
                    {mangaStageLabel(status?.stage)}
                  </span>
                  {total > 0 ? (
                    <span className="font-mono">
                      {status?.completed ?? 0}/{total}
                    </span>
                  ) : status?.can_resume ? (
                    <span>可续传</span>
                  ) : (
                    <span>未生成</span>
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
