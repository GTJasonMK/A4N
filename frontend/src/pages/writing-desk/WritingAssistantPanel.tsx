import React, { useState } from "react";
import { Eye, Film, RotateCcw, Wand2 } from "lucide-react";
import { type ChapterData } from "../../api/chapter";
import { type ChapterOutline } from "../../api/outline";
import { BookInput, BookTextarea } from "../../components/ui/BookInput";
import { Dropdown } from "../../components/ui/Dropdown";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { OptimizeTab } from "../novel-detail/OptimizeTab";
import type { OptimizationPatchPreviewController } from "../novel-detail/optimize/patchPreviewTypes";
import { AssistantContextPanel } from "./AssistantContextPanel";
import { StreamControlBar } from "./StreamControlBar";

interface WritingAssistantPanelProps {
  embeddedInModal?: boolean;
  projectId: number;
  writingNotes: string;
  minWordCount: string;
  maxWordCount: string;
  wordCountRangeError: string | null;
  activeNumber: number | null;
  activeChapter: ChapterData | null;
  activeChapterBusy: boolean;
  canGenerateChapter: boolean;
  generationBlockReason: string | null;
  streamTaskId: string | null;
  streamBusy: boolean;
  streamMessage: string | null;
  resetBusy: boolean;
  canResetGeneratedData: boolean;
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
  embeddedInModal = false,
  projectId,
  writingNotes,
  minWordCount,
  maxWordCount,
  wordCountRangeError,
  activeNumber,
  activeChapter,
  activeChapterBusy,
  canGenerateChapter,
  generationBlockReason,
  streamTaskId,
  streamBusy,
  streamMessage,
  resetBusy,
  canResetGeneratedData,
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
  const generationActive = Boolean(streamTaskId || streamBusy || resetBusy);

  return (
    <aside
      aria-label="写作助手"
      className={`a4n-desk-rail flex h-full min-h-0 min-w-0 flex-col overflow-hidden !p-0 ${
        embeddedInModal ? "!border-0 !bg-transparent" : "border-l border-book-border/80"
      }`}
    >
      <div
        className={`flex shrink-0 items-center justify-between gap-2 border-b border-book-border/60 px-3 py-2 ${
          embeddedInModal ? "pt-0" : ""
        }`}
      >
        <span className="text-[11px] text-book-text-muted">
          {activeNumber != null ? `第 ${activeNumber} 章` : "尚未选择章节"}
        </span>
        <SegmentedControl
          ariaLabel="写作助手模式"
          value={assistantMode}
          onChange={setAssistantMode}
          options={[
            { id: "generate" as const, label: "生成", icon: <Wand2 size={11} /> },
            { id: "optimize" as const, label: "优化" },
          ]}
        />
      </div>

      {assistantMode === "generate" ? (
        <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar">
          <div className="flex min-h-full min-w-0 flex-col gap-3 p-3">
            {!generationActive ? (
              <>
                <BookTextarea
                  label="写作备注 / 重试要求"
                  value={writingNotes}
                  onChange={(event) => onWritingNotesChange(event.target.value)}
                  rows={3}
                  maxLength={1200}
                  placeholder="例如：加强悬疑氛围，减少解释性旁白"
                  className="text-xs leading-relaxed"
                />

                <details
                  className="group border-y border-book-border/45 py-2 text-xs"
                  open={wordCountRangeError ? true : undefined}
                >
                  <summary className="flex cursor-pointer list-none items-center justify-between gap-2 text-book-text-muted hover:text-book-text-main">
                    <span>正文长度</span>
                    <span className={wordCountRangeError ? "text-error" : "text-book-text-sub"}>
                      {wordCountRangeError ?? `${minWordCount || "—"}–${maxWordCount || "—"} 字`}
                    </span>
                  </summary>
                  <div className="mt-3 grid min-w-0 grid-cols-2 gap-2">
                    <BookInput
                      label="最少字数"
                      type="number"
                      inputMode="numeric"
                      min={1}
                      step={100}
                      value={minWordCount}
                      onChange={(event) => onMinWordCountChange(event.target.value)}
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
                      className="h-8 text-xs"
                      error={wordCountRangeError?.startsWith("最多") ? wordCountRangeError : undefined}
                    />
                  </div>
                  {wordCountRangeError &&
                    !wordCountRangeError.startsWith("最少") &&
                    !wordCountRangeError.startsWith("最多") ? (
                      <div className="mt-2 text-xs text-error">{wordCountRangeError}</div>
                    ) : null}
                </details>
              </>
            ) : null}

            <div className="mt-auto flex min-w-0 items-end gap-2">
              <div className="min-w-0 flex-1">
                <StreamControlBar
                  activeNumber={activeNumber}
                  streamTaskId={streamTaskId}
                  streamBusy={streamBusy}
                  streamMessage={streamMessage}
                  canStart={canGenerateChapter}
                  startDisabledReason={generationBlockReason ?? wordCountRangeError}
                  resetBusy={resetBusy}
                  onStart={onStreamStart}
                  onCancel={onStreamCancel}
                />
              </div>
              {!streamTaskId && !streamBusy && !resetBusy ? (
                <Dropdown
                  items={[
                    {
                      label: previewing ? "正在构造模型请求" : "查看模型请求",
                      icon: <Eye size={12} className={previewing ? "animate-pulse" : undefined} />,
                      disabled: !activeNumber || previewing || !!wordCountRangeError,
                      onClick: () => void onPreviewPrompt(),
                    },
                    {
                      label: "场景管理",
                      icon: <Film size={12} />,
                      disabled: !activeChapter || activeChapterBusy,
                      onClick: () => void onOpenScenes(),
                    },
                    ...(canResetGeneratedData
                      ? [
                          { type: "divider" as const },
                          {
                            label: "重置生成数据",
                            icon: <RotateCcw size={12} />,
                            danger: true,
                            onClick: () => void onResetGeneratedData(),
                          },
                        ]
                      : []),
                  ]}
                />
              ) : null}
            </div>

            {!generationActive ? (
              <AssistantContextPanel
                chapters={chapters}
                outlines={outlines}
                activeNumber={activeNumber}
              />
            ) : null}
          </div>
        </div>
      ) : activeChapter ? (
        <div className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
          <OptimizeTab
            projectId={projectId}
            fixedChapterId={activeChapter.id}
            embedded
            onChapterContentChanged={onChapterContentChanged}
            onPatchPreviewChange={onPatchPreviewChange}
          />
        </div>
      ) : (
        <div className="a4n-empty-state a4n-empty-state--compact m-3 text-xs">
          请先选择一个已生成章节
        </div>
      )}
    </aside>
  );
};
