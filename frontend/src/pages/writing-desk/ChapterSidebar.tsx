import React, { useMemo, useState } from "react";
import { RefreshCw, Search } from "lucide-react";
import { type ChapterData } from "../../api/chapter";
import { type ChapterOutline } from "../../api/outline";
import { BookIconButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import { ChapterCard } from "./ChapterCard";
import { FlippableBlueprintCard } from "./FlippableBlueprintCard";
import { countChapterBodyCharacters } from "./wordCount";

interface ChapterSidebarProps {
  projectId: number;
  chapters: ChapterData[];
  outlines: ChapterOutline[];
  chapterNumbers: number[];
  filteredChapterNumbers: number[];
  activeNumber: number | null;
  activeEditorContent: string;
  activeOutline: ChapterOutline | null;
  chapterQuery: string;
  draftSet: ReadonlySet<number>;
  profilesRefreshKey: number;
  onChapterQueryChange: (query: string) => void;
  onClearChapterQuery: () => void;
  onOpenProfiles: () => void;
  onRefreshChapters: () => void | Promise<unknown>;
  onSelectChapter: (chapterNumber: number) => void | Promise<unknown>;
  onOpenProjectDetail: () => void | Promise<void>;
}

const chapterTitle = (
  chapter: ChapterData | undefined,
  outline: ChapterOutline | undefined,
): string | null => {
  const chapterTitle = chapter?.title?.trim();
  if (chapterTitle) return chapterTitle;
  const outlineTitle = outline?.title?.trim();
  return outlineTitle ? outlineTitle : null;
};

const outlineTitle = (outline: ChapterOutline): string | null => {
  const title = outline.title?.trim();
  return title ? title : null;
};

const outlineSummary = (outline: ChapterOutline): string | null => {
  const summary = outline.summary?.trim();
  return summary ? summary : null;
};

const SIDEBAR_GRID_ROWS = "max-content minmax(0, 1fr) max-content";
const CHAPTER_LIST_GRID_ROWS = "auto auto minmax(0, 1fr)";

export const ChapterSidebar: React.FC<ChapterSidebarProps> = ({
  projectId,
  chapters,
  outlines,
  chapterNumbers,
  filteredChapterNumbers,
  activeNumber,
  activeEditorContent,
  activeOutline,
  chapterQuery,
  draftSet,
  profilesRefreshKey,
  onChapterQueryChange,
  onClearChapterQuery,
  onOpenProfiles,
  onRefreshChapters,
  onSelectChapter,
  onOpenProjectDetail,
}) => {
  const [outlineDetailOpen, setOutlineDetailOpen] = useState(false);
  const chaptersByNumber = useMemo(
    () => new Map(chapters.map((chapter) => [chapter.chapter_number, chapter])),
    [chapters],
  );
  const outlinesByNumber = useMemo(
    () => new Map(outlines.map((outline) => [outline.chapter_number, outline])),
    [outlines],
  );
  const currentOutlineTitle = activeOutline ? outlineTitle(activeOutline) : null;
  const currentOutlineSummary = activeOutline ? outlineSummary(activeOutline) : null;

  return (
    <aside
      className="grid h-full min-h-0 min-w-0 gap-2 overflow-hidden border-r border-book-border bg-book-bg-paper p-2"
      style={{ gridTemplateRows: SIDEBAR_GRID_ROWS }}
    >
      <FlippableBlueprintCard
        projectId={projectId}
        chapterCount={chapters.length}
        totalChapters={outlines.length}
        refreshKey={profilesRefreshKey}
        onOpenProfiles={onOpenProfiles}
      />
      <section
        className="grid min-h-0 min-w-0 overflow-hidden"
        style={{ gridTemplateRows: CHAPTER_LIST_GRID_ROWS }}
      >
        <div className="flex min-w-0 shrink-0 items-center gap-1.5">
          <h3 className="min-w-0 flex-1 truncate text-[10px] font-semibold uppercase tracking-wider text-book-text-sub">
            章节列表
            {chapterNumbers.length > 0 && (
              <span className="ml-1 font-mono text-book-text-muted">
                {filteredChapterNumbers.length}/{chapterNumbers.length}
              </span>
            )}
          </h3>
          <BookIconButton
            label="刷新"
            onClick={() => void onRefreshChapters()}
            variant="secondary"
            size="md"
          >
            <RefreshCw size={12} />
          </BookIconButton>
        </div>
        <div className="relative mt-1 min-w-0 shrink-0">
          <input
            type="text"
            value={chapterQuery}
            onChange={(event) => onChapterQueryChange(event.target.value)}
            placeholder="搜索章节、标题、章纲…"
            className="pyqt-input h-8 w-full pl-7 pr-7 text-[11px]"
          />
          <Search
            size={12}
            className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-book-text-muted"
          />
          {chapterQuery.trim() && (
            <button
              type="button"
              onClick={onClearChapterQuery}
            className="absolute right-1 top-1/2 h-8 w-8 -translate-y-1/2 rounded-sm text-[11px] text-book-text-muted hover:bg-book-primary/10 hover:text-book-primary sm:right-1.5 sm:h-5 sm:w-5"
              aria-label="清空章节搜索"
              title="清空搜索"
            >
              ×
            </button>
          )}
        </div>
        <div className="custom-scrollbar mt-2 min-h-0 min-w-0 overflow-y-auto pr-1">
          <ul className="space-y-1">
            {filteredChapterNumbers.map((chapterNumber) => {
              const chapter = chaptersByNumber.get(chapterNumber);
              const outline = outlinesByNumber.get(chapterNumber);
              const bodyContent =
                chapterNumber === activeNumber ? activeEditorContent : chapter?.content;
              const wordCount =
                bodyContent == null ? null : countChapterBodyCharacters(bodyContent);
              return (
                <li key={chapterNumber}>
                  <ChapterCard
                    chapterNumber={chapterNumber}
                    title={chapterTitle(chapter, outline)}
                    wordCount={wordCount}
                    status={chapter?.status}
                    hasContent={!!chapter?.content}
                    hasDraft={draftSet.has(chapterNumber)}
                    selected={chapterNumber === activeNumber}
                    onClick={() => void onSelectChapter(chapterNumber)}
                  />
                </li>
              );
            })}
          </ul>
          {chapterNumbers.length > 0 && filteredChapterNumbers.length === 0 && (
            <div className="mt-4 text-center text-xs text-book-text-muted">没有匹配的章节</div>
          )}
          {!chapters.length && !outlines.length && (
            <div className="mt-4 text-center text-xs text-book-text-muted">
              暂无章节，去&nbsp;
              <button
                type="button"
                onClick={() => void onOpenProjectDetail()}
                className="text-book-primary hover:underline"
              >
                项目详情
              </button>
              &nbsp;生成大纲
            </div>
          )}
        </div>
      </section>

      <div className="min-w-0 overflow-hidden border-t border-book-border pt-1">
        <button
          type="button"
          onClick={() => activeOutline && setOutlineDetailOpen(true)}
          className="a4n-interactive-row h-full min-w-0 w-full overflow-hidden px-1.5 py-1 text-left text-[11px] leading-snug text-book-text-sub disabled:cursor-default disabled:text-book-text-muted disabled:hover:border-book-border disabled:hover:bg-book-bg-paper disabled:hover:shadow-none"
          disabled={!activeOutline}
          title={activeOutline ? "查看本章章纲" : "暂无本章章纲"}
        >
          {activeOutline ? (
            <>
              <div className="flex min-w-0 items-center gap-1.5">
                <span className="shrink-0 font-semibold text-book-text-muted">章纲</span>
                <span className="min-w-0 flex-1 truncate font-semibold text-book-text-main">
                  {currentOutlineTitle ?? `第 ${activeOutline.chapter_number} 章`}
                </span>
              </div>
              {currentOutlineSummary && (
                <div className="mt-0.5 truncate text-[10px] text-book-text-sub">
                  {currentOutlineSummary}
                </div>
              )}
              {!currentOutlineSummary && (
                <div className="mt-0.5 truncate text-[10px]" aria-hidden="true">
                  &nbsp;
                </div>
              )}
            </>
          ) : (
            <div className="min-w-0">
              <div className="flex min-w-0 items-center gap-1.5">
                <span className="shrink-0 font-semibold text-book-text-muted">章纲</span>
                <span className="min-w-0 flex-1 truncate">暂无本章章纲</span>
              </div>
              <div className="mt-0.5 truncate text-[10px]" aria-hidden="true">
                &nbsp;
              </div>
            </div>
          )}
        </button>
      </div>

      <Modal
        isOpen={outlineDetailOpen && !!activeOutline}
        onClose={() => setOutlineDetailOpen(false)}
        title="本章章纲"
        maxWidthClassName="max-w-xl"
      >
        {activeOutline && (
          <div className="space-y-3 text-xs leading-relaxed text-book-text-sub">
            <div>
              <div className="mb-1 text-[11px] font-semibold text-book-text-muted">
                第 {activeOutline.chapter_number} 章
              </div>
              <div className="font-serif text-base font-semibold text-book-text-main">
                {currentOutlineTitle ?? "未命名章节"}
              </div>
            </div>
            <div className="whitespace-pre-wrap text-book-text-main">
              {currentOutlineSummary ?? "本章章纲暂无摘要。"}
            </div>
          </div>
        )}
      </Modal>
    </aside>
  );
};
