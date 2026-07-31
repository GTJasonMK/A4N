import React, { useEffect, useLayoutEffect, useRef } from "react";
import type { ChapterData } from "../../api/chapter";
import type { ChapterOutline } from "../../api/outline";
import type { ChapterRow } from "../novel-detail/shared";
import { ImagesTab } from "../novel-detail/ImagesTab";
import { ShootingScriptTab } from "../novel-detail/ShootingScriptTab";
import { AnalysisSubTab } from "./AnalysisSubTab";
import { InlineOptimizationPatchPreview } from "./InlineOptimizationPatchPreview";
import type { OptimizationPatchPreviewController } from "../novel-detail/optimize/patchPreviewTypes";
import { getOperationStatusLabel } from "../../utils/workflowPresentation";

export type BottomTab =
  | "content"
  | "analysis"
  | "images"
  | "shooting_script";

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
  canEdit: boolean;
}

const TABS: ReadonlyArray<readonly [BottomTab, string]> = [
  ["content", "正文"],
  ["analysis", "分析"],
  ["images", "图像"],
  ["shooting_script", "剧本改编"],
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
  canEdit,
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
    <section aria-label="章节编辑区" className="flex min-h-0 min-w-0 flex-col overflow-hidden bg-book-bg p-2.5 sm:p-3">
      {activeChapter == null && activeOutline == null ? (
        <div className="a4n-empty-state a4n-empty-state--compact m-auto text-sm">
          请从左侧选择一章
        </div>
      ) : (
        <>
          <div className="mb-2.5 flex min-w-0 shrink-0 flex-wrap items-center gap-2">
            <h2 className="min-w-0 flex-1 truncate px-1 py-1 font-serif text-body-lg font-semibold text-book-text-main">
              {selectedChapterTitle(activeChapter, activeOutline) || `第 ${activeChapter?.chapter_number ?? activeOutline?.chapter_number ?? "—"} 章`}
            </h2>
            <div className="flex min-w-0 shrink-0 items-center gap-2">
              <span className="text-small text-book-text-muted">
                {wordCount.toLocaleString()} 字
                {activeChapter?.status
                  ? ` · ${getOperationStatusLabel(activeChapter.status)}`
                  : ""}
              </span>
            </div>
          </div>

          <nav
            className="tab-bar flex min-w-0 shrink-0 flex-wrap border-b border-book-border/80 bg-book-bg-paper/70"
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
          </nav>

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
                      : "本章尚未生成。可打开右侧『资料检索』或『正文优化』，也可以直接输入正文。"
                  }
                  aria-label="章节正文"
                  className="book-control block h-full w-full resize-none rounded-xl px-4 py-3 font-serif text-sm leading-[2] !shadow-none sm:px-5 sm:py-4"
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
            {bottomTab === "shooting_script" && (
              <div className="h-full min-h-0 overflow-y-auto custom-scrollbar">
                <ShootingScriptTab
                  projectId={projectId}
                  canEdit={canEdit}
                  chapter={activeChapter}
                />
              </div>
            )}
            {bottomTab === "analysis" && !activeChapter && (
              <div className="a4n-empty-state a4n-empty-state--compact m-auto text-xs">
                请先选择已生成的章节
              </div>
            )}
          </div>
        </>
      )}
    </section>
  );
};
