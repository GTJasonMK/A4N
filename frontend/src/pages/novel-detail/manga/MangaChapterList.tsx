import React from "react";
import { AlertTriangle, CheckCircle2, Clock3, Frame, Loader2 } from "lucide-react";
import type { PipelineStatus } from "../../../api/manga";
import { BookCard } from "../../../components/ui/BookCard";
import { nonEmptyText } from "../../../utils/text";
import { EmptyHint, type ChapterRow } from "../shared";
import { mangaStageLabel } from "./mangaTabUtils";

interface MangaChapterListProps {
  chapters: ChapterRow[];
  statuses: Record<number, PipelineStatus | null>;
  selectedChapter: ChapterRow | null;
  onSelect: (chapter: ChapterRow) => void;
}

const chapterPipelineTone = (status: PipelineStatus | null | undefined): string => {
  if (!status?.pipeline_status) return "text-book-text-muted";
  if (status.pipeline_status === "failed") return "text-error";
  if (status.pipeline_status === "completed") return "text-success";
  if (status.pipeline_status === "running") return "text-book-primary";
  return "text-book-text-muted";
};

const chapterPipelineIcon = (status: PipelineStatus | null | undefined) => {
  if (status?.pipeline_status === "failed") return <AlertTriangle size={11} />;
  if (status?.pipeline_status === "completed") return <CheckCircle2 size={11} />;
  if (status?.pipeline_status === "running") return <Loader2 size={11} className="animate-spin" />;
  return <Clock3 size={11} />;
};

export const MangaChapterList: React.FC<MangaChapterListProps> = ({
  chapters,
  statuses,
  selectedChapter,
  onSelect,
}) => (
  <BookCard className="flex max-h-72 min-h-0 flex-col p-3 lg:h-[calc(100vh-260px)] lg:max-h-none">
    <h3 className="mb-2 inline-flex shrink-0 items-center gap-2 font-serif text-sm text-book-text-main">
      <Frame size={14} className="text-book-primary" /> 章节漫画
    </h3>
    {chapters.length === 0 ? (
      <EmptyHint text="项目还没有章节。先生成章节，再来这里启动漫画流水线。" />
    ) : (
      <ul className="flex min-w-max gap-1 overflow-x-auto lg:block lg:min-w-0 lg:flex-1 lg:space-y-1 lg:overflow-y-auto custom-scrollbar">
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
                className={`min-w-[11rem] rounded border px-2.5 py-2 text-left text-xs transition-colors lg:w-full lg:min-w-0 ${
                  isSelected
                    ? "border-book-primary bg-book-primary/10 text-book-primary"
                    : "border-transparent text-book-text-sub hover:bg-book-bg-tertiary hover:text-book-text-main"
                }`}
              >
                <div className="mb-1 flex items-center justify-between gap-2">
                  <span className="shrink-0 font-mono">#{chapter.chapter_number}</span>
                  <span className="min-w-0 flex-1 truncate text-right">
                    {nonEmptyText(chapter.title) ?? `第 ${chapter.chapter_number} 章`}
                  </span>
                </div>
                <div
                  className={`mb-1 flex items-center justify-between gap-2 text-[10px] ${
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
                    className={`flex h-1 overflow-hidden rounded-sm ${
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
                    className={`truncate text-[10px] ${
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
    )}
  </BookCard>
);
