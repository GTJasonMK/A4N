import React, { useEffect, useLayoutEffect, useRef } from "react";
import type { ChapterData } from "../../api/chapter";
import type { ChapterOutline } from "../../api/outline";
import type { ChapterRow } from "../novel-detail/shared";
import { ImagesTab } from "../novel-detail/ImagesTab";
import { AnalysisSubTab } from "./AnalysisSubTab";
import { InlineOptimizationPatchPreview } from "./InlineOptimizationPatchPreview";
import type { OptimizationPatchPreviewController } from "../novel-detail/optimize/patchPreviewTypes";

export type BottomTab =
  | "content"
  | "analysis"
  | "images";

interface ChapterWorkspaceProps {
  projectId: number;
  activeChapter: ChapterData | null;
  activeOutline: ChapterOutline | null;
  activeChapterContext: ChapterRow | null;
  activeChapterBusy: boolean;
  activeChapterGeneratingMessage: string;
  analysisBlockedReason: string | null;
  editorContent: string;
  optimizationPatchPreview: OptimizationPatchPreviewController | null;
  autoScrollEditorContent: boolean;
  wordCount: number;
  generatingAnalysis: boolean;
  bottomTab: BottomTab;
  onBottomTabChange: (tab: BottomTab) => void;
  onEditorContentChange: (content: string) => void;
  onGenerateAnalysis: () => void | Promise<void>;
  onPreviewGenerateAnalysis: () => void | Promise<void>;
  onResumeAnalysisSync: () => void | Promise<void>;
  onPreviewResumeAnalysisSync: () => void | Promise<void>;
}

const TABS: ReadonlyArray<readonly [BottomTab, string]> = [
  ["content", "正文"],
  ["analysis", "分析"],
  ["images", "图像"],
];

const selectedChapterTitle = (
  chapter: ChapterData | null,
  outline: ChapterOutline | null,
): string => {
  if (chapter != null) {
    return chapter.title == null ? "" : chapter.title;
  }
  if (outline != null) {
    return outline.title == null ? "" : outline.title;
  }
  return "";
};

export const ChapterWorkspace: React.FC<ChapterWorkspaceProps> = ({
  projectId,
  activeChapter,
  activeOutline,
  activeChapterContext,
  activeChapterBusy,
  activeChapterGeneratingMessage,
  analysisBlockedReason,
  editorContent,
  optimizationPatchPreview,
  autoScrollEditorContent,
  wordCount,
  generatingAnalysis,
  bottomTab,
  onBottomTabChange,
  onEditorContentChange,
  onGenerateAnalysis,
  onPreviewGenerateAnalysis,
  onResumeAnalysisSync,
  onPreviewResumeAnalysisSync,
}) => {
  const editorRef = useRef<HTMLTextAreaElement | null>(null);
  const shouldAutoScrollRef = useRef(true);
  const lastScrollTopRef = useRef(0);

  const editorIsNearBottom = () => {
    const editor = editorRef.current;
    if (!editor) return true;
    const distanceFromBottom =
      editor.scrollHeight - editor.scrollTop - editor.clientHeight;
    return distanceFromBottom <= 24;
  };

  const updateEditorScrollIntent = () => {
    const editor = editorRef.current;
    if (!editor) return;
    lastScrollTopRef.current = editor.scrollTop;
    shouldAutoScrollRef.current = editorIsNearBottom();
  };

  useEffect(() => {
    shouldAutoScrollRef.current = true;
    lastScrollTopRef.current = 0;
  }, [activeChapter?.id, activeOutline?.id]);

  useEffect(() => {
    if (!autoScrollEditorContent) return;
    shouldAutoScrollRef.current = editorIsNearBottom();
  }, [autoScrollEditorContent]);

  useLayoutEffect(() => {
    if (!autoScrollEditorContent || bottomTab !== "content") return;
    const editor = editorRef.current;
    if (!editor) return;

    if (shouldAutoScrollRef.current) {
      editor.scrollTop = editor.scrollHeight;
      lastScrollTopRef.current = editor.scrollTop;
      return;
    }

    editor.scrollTop = lastScrollTopRef.current;
  }, [autoScrollEditorContent, bottomTab, editorContent]);

  return (
    <main className="flex min-h-0 min-w-0 flex-col overflow-hidden bg-book-bg p-3">
      {activeChapter == null && activeOutline == null ? (
        <div className="m-auto text-center text-book-text-muted text-sm">
          请从左侧选择一章
        </div>
      ) : (
        <>
          <div className="mb-2 flex min-w-0 shrink-0 flex-wrap items-center gap-2">
            <input
              type="text"
              value={selectedChapterTitle(activeChapter, activeOutline)}
              readOnly
              className="book-control min-w-0 flex-1 rounded-sm px-2 py-1 text-sm font-serif"
            />
            <div className="flex min-w-0 shrink-0 items-center gap-2">
              <span className="text-[11px] text-book-text-muted">
                {wordCount.toLocaleString()} 字
              </span>
              {activeChapter?.status && (
                <span className="pyqt-pill !text-[10px]">{activeChapter.status}</span>
              )}
            </div>
          </div>

          <div
            className="tab-bar flex min-w-0 shrink-0 flex-wrap border-b border-book-border bg-book-bg-paper"
            role="tablist"
            aria-label="章节工作区"
          >
            {TABS.map(([id, label]) => (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={bottomTab === id}
                onClick={() => onBottomTabChange(id)}
                data-active={bottomTab === id ? "1" : undefined}
                className="pyqt-tab"
              >
                {label}
              </button>
            ))}
          </div>

          <div className="mt-2 min-h-0 min-w-0 flex-1">
            {bottomTab === "content" && (
              optimizationPatchPreview ? (
                <InlineOptimizationPatchPreview
                  preview={optimizationPatchPreview}
                />
              ) : (
                <textarea
                  ref={editorRef}
                  value={editorContent}
                  onScroll={updateEditorScrollIntent}
                  onChange={(event) => onEditorContentChange(event.target.value)}
                  readOnly={activeChapterBusy}
                  title={activeChapterBusy ? activeChapterGeneratingMessage : undefined}
                  placeholder={
                    activeChapter
                      ? "开始写作…"
                      : "本章尚未生成。点击右侧『RAG 查询』/『正文优化』，或在此输入正文。"
                  }
                  className="book-control block mx-auto w-full max-w-3xl h-full rounded-sm p-3 text-sm font-serif leading-loose resize-none"
                />
              )
            )}
            {bottomTab === "analysis" && activeChapter && (
              <AnalysisSubTab
                projectId={projectId}
                chapterNumber={activeChapter.chapter_number}
                analysis={activeChapter.analysis_data}
                generating={generatingAnalysis}
                analysisBlockedReason={analysisBlockedReason}
                canGenerate={
                  !activeChapterBusy &&
                  !!activeChapter.content.trim() &&
                  !analysisBlockedReason
                }
                onGenerate={onGenerateAnalysis}
                onPreviewGenerate={onPreviewGenerateAnalysis}
                canResumeSync={
                  !activeChapterBusy &&
                  !!activeChapter.content.trim() &&
                  !analysisBlockedReason &&
                  activeChapter.analysis_sync_can_resume
                }
                onResumeSync={onResumeAnalysisSync}
                onPreviewResumeSync={onPreviewResumeAnalysisSync}
              />
            )}
            {bottomTab === "images" && (
              <div className="h-full min-h-0 overflow-y-auto custom-scrollbar">
                <ImagesTab projectId={projectId} fixedChapter={activeChapterContext} />
              </div>
            )}
            {bottomTab === "analysis" && !activeChapter && (
              <div className="m-auto text-center text-book-text-muted text-xs">
                请先选择已生成的章节
              </div>
            )}
          </div>
        </>
      )}
    </main>
  );
};
