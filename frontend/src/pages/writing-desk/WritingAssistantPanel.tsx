import React, { useState } from "react";
import {
  Eye,
  Film,
  Loader2,
  Wand2,
} from "lucide-react";
import { type ChapterData } from "../../api/chapter";
import { type ChapterOutline } from "../../api/outline";
import { BookButton } from "../../components/ui/BookButton";
import { BookInput, BookTextarea } from "../../components/ui/BookInput";
import { OptimizeTab } from "../novel-detail/OptimizeTab";
import type { OptimizationPatchPreviewController } from "../novel-detail/optimize/patchPreviewTypes";
import { AssistantContextPanel } from "./AssistantContextPanel";
import { StreamControlBar } from "./StreamControlBar";

interface WritingAssistantPanelProps {
  projectId: number;
  writingNotes: string;
  minWordCount: string;
  maxWordCount: string;
  wordCountRangeError: string | null;
  activeNumber: number | null;
  activeChapter: ChapterData | null;
  activeChapterBusy: boolean;
  activeChapterGeneratingMessage: string;
  canGenerateChapter: boolean;
  generationBlockReason: string | null;
  streamTaskId: string | null;
  streamBusy: boolean;
  streamMessage: string | null;
  resetBusy: boolean;
  canResetGeneratedData: boolean;
  resetDisabledReason: string | null;
  previewing: boolean;
  chapters: ChapterData[];
  outlines: ChapterOutline[];
  onWritingNotesChange: (value: string) => void;
  onMinWordCountChange: (value: string) => void;
  onMaxWordCountChange: (value: string) => void;
  onStreamStart: () => void | Promise<void>;
  onStreamCancel: () => void | Promise<void>;
  onResetGeneratedData: () => void | Promise<void>;
  onPreviewPrompt: () => void | Promise<void>;
  onOpenScenes: () => void | Promise<void>;
  onChapterContentChanged: (chapterId: number) => void | Promise<void>;
  onPatchPreviewChange: (
    preview: OptimizationPatchPreviewController | null,
  ) => void;
}

type AssistantMode = "generate" | "optimize";

export const WritingAssistantPanel: React.FC<WritingAssistantPanelProps> = ({
  projectId,
  writingNotes,
  minWordCount,
  maxWordCount,
  wordCountRangeError,
  activeNumber,
  activeChapter,
  activeChapterBusy,
  activeChapterGeneratingMessage,
  canGenerateChapter,
  generationBlockReason,
  streamTaskId,
  streamBusy,
  streamMessage,
  resetBusy,
  canResetGeneratedData,
  resetDisabledReason,
  previewing,
  chapters,
  outlines,
  onWritingNotesChange,
  onMinWordCountChange,
  onMaxWordCountChange,
  onStreamStart,
  onStreamCancel,
  onResetGeneratedData,
  onPreviewPrompt,
  onOpenScenes,
  onChapterContentChanged,
  onPatchPreviewChange,
}) => {
  const [assistantMode, setAssistantMode] = useState<AssistantMode>("generate");
  const busyTitle = activeChapterBusy ? activeChapterGeneratingMessage : undefined;

  return (
    <aside
      className={`h-full min-w-0 overflow-hidden border-l border-book-border bg-book-bg-paper/40 ${
        assistantMode === "optimize"
          ? "flex min-h-0 flex-col overflow-hidden"
          : "overflow-y-auto custom-scrollbar p-2.5"
      }`}
    >
      <div
        className={`flex min-w-0 flex-wrap items-center gap-2 ${
          assistantMode === "optimize"
            ? "shrink-0 border-b border-book-border bg-book-bg-tertiary px-2 py-2"
            : "mb-3"
        }`}
      >
        <h3 className="min-w-0 flex-1 truncate text-xs font-semibold uppercase tracking-wider text-book-text-sub">
          <Wand2 size={11} className="inline mr-1" /> 正文优化
        </h3>
        <div className="flex shrink-0 rounded border border-book-border bg-book-bg-tertiary p-0.5">
          {[
            ["generate", "生成"],
            ["optimize", "Agent"],
          ].map(([mode, label]) => (
            <button
              key={mode}
              type="button"
              onClick={() => setAssistantMode(mode as AssistantMode)}
              className={`rounded-sm px-2 py-1 text-[11px] transition-colors ${
                assistantMode === mode
                  ? "bg-book-bg-paper text-book-text-main shadow-sm"
                  : "text-book-text-sub hover:bg-book-bg-paper hover:text-book-text-main"
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {assistantMode === "generate" ? (
        <div className="flex min-w-0 flex-col gap-2">
          <BookTextarea
            label="写作备注 / 重试要求"
            value={writingNotes}
            onChange={(event) => onWritingNotesChange(event.target.value)}
            rows={3}
            maxLength={1200}
            placeholder="例如：加强悬疑氛围，减少解释性旁白"
            className="text-xs leading-relaxed"
            disabled={streamBusy || !!streamTaskId}
          />

          <div className="min-w-0 rounded border border-book-border/50 bg-book-bg-paper/30 p-2">
            <div className="mb-2 text-[11px] font-semibold text-book-text-sub">
              正文字数范围
            </div>
            <div className="grid min-w-0 grid-cols-[repeat(auto-fit,minmax(4.75rem,1fr))] gap-2">
              <BookInput
                label="最少字数"
                type="number"
                inputMode="numeric"
                min={1}
                step={100}
                value={minWordCount}
                onChange={(event) => onMinWordCountChange(event.target.value)}
                disabled={streamBusy || !!streamTaskId}
                className="h-8 text-xs"
                error={wordCountRangeError?.startsWith("最少") ? wordCountRangeError : undefined}
              />
              <BookInput
                label="最多字数"
                type="number"
                inputMode="numeric"
                min={1}
                step={100}
                value={maxWordCount}
                onChange={(event) => onMaxWordCountChange(event.target.value)}
                disabled={streamBusy || !!streamTaskId}
                className="h-8 text-xs"
                error={wordCountRangeError?.startsWith("最多") ? wordCountRangeError : undefined}
              />
            </div>
            {wordCountRangeError &&
              !wordCountRangeError.startsWith("最少") &&
              !wordCountRangeError.startsWith("最多") && (
                <div className="mt-1 text-xs text-error">{wordCountRangeError}</div>
              )}
          </div>

          <StreamControlBar
            activeNumber={activeNumber}
            streamTaskId={streamTaskId}
            streamBusy={streamBusy}
            streamMessage={streamMessage}
            canStart={canGenerateChapter}
            startDisabledReason={generationBlockReason ?? wordCountRangeError}
            resetBusy={resetBusy}
            canReset={canResetGeneratedData}
            resetDisabledReason={resetDisabledReason}
            onStart={onStreamStart}
            onCancel={onStreamCancel}
            onReset={onResetGeneratedData}
          />

          <BookButton
            variant="secondary"
            onClick={onPreviewPrompt}
            disabled={!activeNumber || previewing || !!wordCountRangeError}
            className="w-full !px-2"
            title={wordCountRangeError ?? "直接构造并查看本次章节生成将使用的完整提示词"}
          >
            {previewing ? (
              <span className="inline-flex min-w-0 items-center gap-2">
                <Loader2 size={14} className="shrink-0 animate-spin" /> 预览中…
              </span>
            ) : (
              <span className="inline-flex min-w-0 items-center gap-2">
                <Eye size={16} className="shrink-0" />
                <span className="truncate">预览提示词</span>
              </span>
            )}
          </BookButton>

          <BookButton
            variant="secondary"
            onClick={() => setAssistantMode("optimize")}
            disabled={!activeChapter || activeChapterBusy}
            className="w-full !px-2"
            title={busyTitle}
          >
            <span className="inline-flex min-w-0 items-center gap-2">
              <Wand2 size={16} className="shrink-0" />
              <span className="truncate">内容优化（Agent）</span>
            </span>
          </BookButton>

          <BookButton
            variant="secondary"
            onClick={onOpenScenes}
            disabled={!activeChapter || activeChapterBusy}
            className="w-full !px-2"
            title={busyTitle}
          >
            <span className="inline-flex min-w-0 items-center gap-2">
              <Film size={16} className="shrink-0" />
              <span className="truncate">场景管理</span>
            </span>
          </BookButton>
        </div>
      ) : activeChapter ? (
        <div className="min-h-0 min-w-0 flex-1">
          <OptimizeTab
            projectId={projectId}
            fixedChapterId={activeChapter.id}
            embedded
            onChapterContentChanged={onChapterContentChanged}
            onPatchPreviewChange={onPatchPreviewChange}
          />
        </div>
      ) : (
        <div className="m-3 rounded border border-book-border bg-book-bg-paper p-4 text-center text-xs text-book-text-sub">
          请先选择一个已生成章节
        </div>
      )}

      {assistantMode === "generate" ? (
        <>
          <div className="story-divider my-5" />
          <AssistantContextPanel
            chapters={chapters}
            outlines={outlines}
            activeNumber={activeNumber}
          />
        </>
      ) : null}
    </aside>
  );
};
