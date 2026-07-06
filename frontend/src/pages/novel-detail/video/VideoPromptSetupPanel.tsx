import React, { useEffect, useMemo, useState } from "react";
import { Copy, Eye, ListChecks, Loader2, Play, RefreshCw, RotateCcw, Trash2, Upload, Wand2 } from "lucide-react";
import type { ChapterVideoPrompt, GeneratedVideo, VideoWorkflowMode } from "../../../api/video";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { StyleTemplatePicker } from "../StyleTemplatePicker";
import { EmptyHint, chapterGeneratingMessage, type ChapterRow } from "../shared";
import { VIDEO_STYLE_TEMPLATES } from "../styleTemplates";
import {
  parseNumberInput,
  normalizeDurationOptions,
  promptOutputLanguageOfPrompt,
  readSavedReferenceImageIds,
  videoPromptProviderMediaBlockedReason,
  PROMPT_OUTPUT_LANGUAGES,
  PROMPT_OUTPUT_LANGUAGE_LABELS,
  VIDEO_ASPECT_RATIOS,
  type VideoAspectRatio,
  type VideoDurationLimits,
  type VideoGenOptions,
} from "./videoTabUtils";
import { VideoDurationOptionsPicker } from "./VideoDurationOptionsPicker";
import { promptTitle, statusClass } from "./videoGenerationDisplay";

type PromptPanelTab = "generate" | "prompts";
type VideoPromptGenerationRequestOptions = {
  regenerateExisting?: boolean;
  promptBatchSize?: number;
};

interface VideoPromptSetupPanelProps {
  chapter: ChapterRow;
  prompts: ChapterVideoPrompt[];
  generationPrompts: ChapterVideoPrompt[];
  selectedPrompt: ChapterVideoPrompt | null;
  selectedPromptId: number | null;
  workflowMode: VideoWorkflowMode;
  providerBackend: string | null;
  providerOptionsJson: unknown;
  pendingVideo: GeneratedVideo | null;
  framePromptCount: number;
  expectedBoundaryFrameCount: number | null;
  expectedVideoPromptCount: number | null;
  totalVideoPromptCount: number;
  totalVideoChainPromptCount: number;
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  promptInputSelectionError: string | null;
  loading: boolean;
  acting: string | null;
  chapterGenerating: boolean;
  chapterContentReady: boolean;
  previewingPromptRequests: boolean;
  selectedPromptStale: boolean;
  selectedPromptRunning: boolean;
  selectedPromptDirty: boolean;
  savedReferenceIdsError: string | null;
  invalidPromptReferenceIds: number[];
  systemReferenceIds: Set<number>;
  validPromptReferenceIds: Set<number>;
  selectedPromptFrameInputMissingMessage: string | null;
  onLoad: () => void | Promise<void>;
  onOptionsChange: React.Dispatch<React.SetStateAction<VideoGenOptions>>;
  onGeneratePrompts: (options?: VideoPromptGenerationRequestOptions) => void | Promise<void>;
  onPreviewPromptRequests: () => void | Promise<void>;
  onSelectPrompt: (id: number) => void;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onDeleteAllVideoPrompts: () => void | Promise<void>;
  onPollPendingVideo: (video: GeneratedVideo) => void | Promise<void>;
  onGenerateVideo: (prompt?: ChapterVideoPrompt) => void | Promise<void>;
  onGenerateAllVideos: () => void | Promise<void>;
  onImportVideo: () => boolean | Promise<boolean>;
  onCopyPrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
}

const OptionsDurationControl: React.FC<{
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  disabled?: boolean;
  onOptionsChange: React.Dispatch<React.SetStateAction<VideoGenOptions>>;
}> = ({ options, durationLimits, disabled = false, onOptionsChange }) => {
  if (durationLimits.durationMode === "fixed" && durationLimits.supportedDurations.length > 0) {
    return (
      <select
        value={options.durationSecs}
        onChange={(event) => {
          if (disabled) return;
          const durationSecs = parseNumberInput(event.target.value);
          if (
            durationSecs !== null &&
            durationLimits.supportedDurations.includes(durationSecs)
          ) {
            onOptionsChange((prev) => ({
              ...prev,
              durationSecs,
              durationOptions: normalizeDurationOptions(durationSecs, prev.durationOptions),
            }));
          }
        }}
        disabled={disabled}
        className="pyqt-input !px-2 !py-1.5"
      >
        {durationLimits.supportedDurations.map((duration) => (
          <option key={duration} value={duration}>
            {duration}s
          </option>
        ))}
      </select>
    );
  }

  return (
    <input
      type="number"
      min={durationLimits.minDurationSecs}
      max={durationLimits.maxDurationSecs}
      value={options.durationSecs}
      onChange={(event) => {
        if (disabled) return;
        const durationSecs = parseNumberInput(event.target.value);
        if (durationSecs !== null) {
          onOptionsChange((prev) => ({
            ...prev,
            durationSecs,
            durationOptions: normalizeDurationOptions(durationSecs, prev.durationOptions),
          }));
        }
      }}
      disabled={disabled}
      className="pyqt-input !px-2 !py-1.5"
    />
  );
};

export const VideoPromptSetupPanel: React.FC<VideoPromptSetupPanelProps> = ({
  chapter,
  prompts,
  generationPrompts,
  selectedPrompt,
  selectedPromptId,
  workflowMode,
  providerBackend,
  providerOptionsJson,
  pendingVideo,
  framePromptCount,
  expectedBoundaryFrameCount,
  expectedVideoPromptCount,
  totalVideoPromptCount,
  totalVideoChainPromptCount,
  options,
  durationLimits,
  promptInputSelectionError,
  loading,
  acting,
  chapterGenerating,
  chapterContentReady,
  previewingPromptRequests,
  selectedPromptStale,
  selectedPromptRunning,
  selectedPromptDirty,
  savedReferenceIdsError,
  invalidPromptReferenceIds,
  systemReferenceIds,
  validPromptReferenceIds,
  selectedPromptFrameInputMissingMessage,
  onLoad,
  onOptionsChange,
  onGeneratePrompts,
  onPreviewPromptRequests,
  onSelectPrompt,
  onDeletePrompt,
  onDeleteAllVideoPrompts,
  onPollPendingVideo,
  onGenerateVideo,
  onGenerateAllVideos,
  onImportVideo,
  onCopyPrompt,
}) => {
  const [promptPanelTab, setPromptPanelTab] = useState<PromptPanelTab>("generate");
  const chatVideoMode = workflowMode === "chat_video";
  const framePromptCountLabel =
    expectedBoundaryFrameCount == null
      ? String(framePromptCount)
      : `${framePromptCount}/${expectedBoundaryFrameCount}`;
  const remainingVideoPromptCount =
    expectedVideoPromptCount == null
      ? null
      : Math.max(0, expectedVideoPromptCount - generationPrompts.length);
  const promptBatchInputDisabled =
    expectedVideoPromptCount === 0 || remainingVideoPromptCount === 0;
  const promptBatchStateMax = Math.max(1, remainingVideoPromptCount ?? 1);
  const promptBatchInputMax = promptBatchInputDisabled ? 0 : promptBatchStateMax;
  const promptBatchInputMin = promptBatchInputDisabled ? 0 : 1;
  const promptBatchInputValue = promptBatchInputDisabled ? 0 : options.promptBatchSize;
const framePromptLabel = chatVideoMode
    ? "首尾帧"
    : workflowMode === "storyboard"
      ? "分镜板"
    : workflowMode === "first_frame_chain"
      ? "首帧"
      : "边界帧";
  const promptGenerationLabel = chatVideoMode
      ? "Chat 视频提示词"
    : workflowMode === "storyboard"
      ? "分镜视频提示词"
      : "视频提示词";
  const materialReferenceCount = (ids: number[]) =>
    ids.filter((id) => !systemReferenceIds.has(id)).length;
  const {
    runningVideoPrompt,
    generatedVideoPromptCount,
    missingVideoPromptCount,
    runningPromptForVideoGeneration,
    providerMediaBlockedReasonForAll,
  } = useMemo(() => {
    let runningVideoPrompt: ChapterVideoPrompt | null = null;
    let generatedVideoPromptCount = 0;
    let missingVideoPromptCount = 0;
    let runningPromptForVideoGeneration: ChapterVideoPrompt | null = null;
    let providerMediaBlockedReasonForAll: string | null = null;

    for (const prompt of generationPrompts) {
      if (!runningVideoPrompt && prompt.status === "running") {
        runningVideoPrompt = prompt;
      }
      if (prompt.generated_video_id != null) {
        generatedVideoPromptCount += 1;
      }
      const missingVideoPrompt =
        prompt.prompt_type === "video_prompt" && prompt.generated_video_id == null;
      if (missingVideoPrompt) {
        missingVideoPromptCount += 1;
        if (providerMediaBlockedReasonForAll == null) {
          const referenceState = readSavedReferenceImageIds(prompt.reference_image_ids_json);
          if (!referenceState.error) {
            providerMediaBlockedReasonForAll = videoPromptProviderMediaBlockedReason(
              prompt,
              referenceState.ids.length,
              materialReferenceCount(referenceState.ids),
              providerBackend,
              providerOptionsJson,
            );
          }
        }
      }
      if (
        !runningPromptForVideoGeneration &&
        prompt.prompt_type === "video_prompt" &&
        prompt.status === "running"
      ) {
        runningPromptForVideoGeneration = prompt;
      }
    }

    return {
      runningVideoPrompt,
      generatedVideoPromptCount,
      missingVideoPromptCount,
      runningPromptForVideoGeneration,
      providerMediaBlockedReasonForAll,
    };
  }, [generationPrompts, providerBackend, providerOptionsJson, systemReferenceIds]);
  const runningVideoPromptBlockedReason = runningVideoPrompt
    ? `${promptGenerationLabel} #${runningVideoPrompt.id} 正在运行，不能继续生成`
    : undefined;
  const videoPromptBaseGenerationBlockedReason =
    durationLimits.configError ??
    promptInputSelectionError ??
    (chapterGenerating ? chapterGeneratingMessage(chapter, `生成${promptGenerationLabel}`) : undefined) ??
    (!chapterContentReady ? `当前章节正文为空，不能生成${promptGenerationLabel}` : undefined) ??
    (!options.style.trim() ? `请填写${promptGenerationLabel}风格` : undefined) ??
    (expectedVideoPromptCount === 0
      ? "请先生成片段规划，才能生成视频提示词"
      : undefined) ??
    runningVideoPromptBlockedReason;
  const videoPromptGenerationBlockedReason =
    videoPromptBaseGenerationBlockedReason ??
    (remainingVideoPromptCount === 0
      ? `${promptGenerationLabel}已全部生成，无需继续生成`
      : undefined) ??
    (options.promptBatchSize > promptBatchStateMax
      ? `本次生成数量不能超过当前剩余 ${promptBatchStateMax} 条`
      : undefined);
  const videoPromptRegenerateBlockedReason =
    videoPromptBaseGenerationBlockedReason ??
    (generatedVideoPromptCount > 0
      ? `已有 ${generatedVideoPromptCount} 条视频提示词生成了视频，请先删除对应视频`
      : undefined);
  const generateAllVideosBlockedReason =
    (chatVideoMode ? "Chat 生视频模式请复制提示词到外部视频工具生成" : undefined) ??
    durationLimits.configError ??
    (chapterGenerating ? chapterGeneratingMessage(chapter, "生成全部视频") : undefined) ??
    (!chapterContentReady ? "当前章节正文为空，不能生成视频" : undefined) ??
    (runningPromptForVideoGeneration
      ? `视频提示词 #${runningPromptForVideoGeneration.id} 正在运行，不能生成全部视频`
      : undefined) ??
    (missingVideoPromptCount === 0
      ? "所有视频提示词都已生成视频，无需继续生成"
      : undefined) ??
    providerMediaBlockedReasonForAll ??
    (selectedPromptDirty &&
    selectedPrompt?.prompt_type === "video_prompt" &&
    selectedPrompt.generated_video_id == null
      ? "当前视频提示词有未保存编辑，请先保存后再生成全部视频"
      : undefined);
  const deleteAllVideoPromptsBlockedReason =
    chapterGenerating
      ? chapterGeneratingMessage(chapter, "清空视频链路")
      : totalVideoChainPromptCount === 0
        ? "没有可清空的视频链路记录"
        : undefined;
  const showPromptRegenerateAction = prompts.length > 0;
  const showPromptPreviewAction = !chatVideoMode;
  const promptAuxiliaryActionCount =
    (showPromptRegenerateAction ? 1 : 0) + (showPromptPreviewAction ? 1 : 0);

  useEffect(() => {
    if (options.promptBatchSize >= 1 && options.promptBatchSize <= promptBatchStateMax) return;
    onOptionsChange((prev) => ({
      ...prev,
      promptBatchSize: Math.min(Math.max(prev.promptBatchSize, 1), promptBatchStateMax),
    }));
  }, [onOptionsChange, options.promptBatchSize, promptBatchStateMax]);

  const renderPromptRow = (prompt: ChapterVideoPrompt) => {
    const selected = selectedPromptId === prompt.id;
    const referenceState = readSavedReferenceImageIds(prompt.reference_image_ids_json);
    const stalePromptReferenceIds = referenceState.error
      ? []
      : referenceState.ids.filter((id) => !validPromptReferenceIds.has(id));
    const stalePromptReferenceBlockedReason =
      stalePromptReferenceIds.length > 0
        ? `当前提示词保存了已失效参考图：${stalePromptReferenceIds.join(", ")}，请先清除后保存`
        : undefined;
    const promptLanguage = promptOutputLanguageOfPrompt(prompt);
    const title = promptTitle(prompt);
    const actionSelectionTitle = "先打开这条视频提示词，确认草稿和素材绑定后再执行";
    const deleteBlockedReason =
      chapterGenerating
        ? chapterGeneratingMessage(chapter, "删除提示词")
        : prompt.status === "running"
          ? `提示词 #${prompt.id} 正在运行，不能删除`
          : undefined;
    const promptHasVideo = prompt.generated_video_id != null;
    const actionBaseBlockedReason =
      prompt.prompt_type !== "video_prompt"
        ? "请选择 video_prompt 类型的提示词"
        : prompt.status === "running"
          ? `视频提示词 #${prompt.id} 正在运行`
          : promptHasVideo
            ? "该视频提示词已关联视频，请先删除对应视频"
            : chapterGenerating
              ? chapterGeneratingMessage(chapter, "生成或导入视频")
              : !chapterContentReady
                ? "当前章节正文为空，不能生成或导入视频"
                : undefined;
    const promptProviderMediaBlockedReason = referenceState.error
      ? undefined
      : videoPromptProviderMediaBlockedReason(
          prompt,
          referenceState.ids.length,
          materialReferenceCount(referenceState.ids),
          providerBackend,
          providerOptionsJson,
        ) ?? undefined;
    const selectedGenerateBlockedReason =
      durationLimits.configError ??
      actionBaseBlockedReason ??
      (selectedPromptStale ? "当前视频提示词已过期，请重新生成" : undefined) ??
      (savedReferenceIdsError ? `保存的参考图数据异常：${savedReferenceIdsError}` : undefined) ??
      (invalidPromptReferenceIds.length > 0
        ? `当前草稿参考图包含不可用图片 ID：${invalidPromptReferenceIds.join(", ")}`
        : undefined) ??
      (selectedPromptDirty ? "当前编辑内容未保存，请先保存后再生成" : undefined) ??
      stalePromptReferenceBlockedReason ??
      selectedPromptFrameInputMissingMessage ??
      promptProviderMediaBlockedReason ??
      (selectedPromptRunning ? "该视频提示词正在运行，不能生成视频" : undefined) ??
      undefined;
    const selectedImportBlockedReason =
      actionBaseBlockedReason ??
      (selectedPromptStale ? "当前视频提示词已过期，请重新生成" : undefined) ??
      (selectedPromptDirty ? "当前编辑内容未保存，请先保存后再导入" : undefined) ??
      (selectedPromptRunning ? "该视频提示词正在运行，不能导入视频" : undefined) ??
      undefined;
    const generateVideoDisabled =
      selected
        ? acting === "generate-all-videos" ||
          acting === `generate-video-${prompt.id}` ||
          selectedGenerateBlockedReason != null
        : acting === "generate-all-videos" ||
          actionBaseBlockedReason != null ||
          stalePromptReferenceBlockedReason != null ||
          promptProviderMediaBlockedReason != null;
    const importVideoDisabled =
      selected
        ? acting === "generate-all-videos" ||
          acting === `import-video-${prompt.id}` ||
          selectedImportBlockedReason != null
        : acting === "generate-all-videos" || actionBaseBlockedReason != null;
    const runSelectedAction = (action: () => unknown | Promise<unknown>) => {
      if (!selected) {
        onSelectPrompt(prompt.id);
        return;
      }
      void action();
    };
    return (
      <li
        key={prompt.id}
        className={`overflow-hidden rounded border transition-colors ${
          selected
            ? "border-book-primary bg-book-primary/10"
            : "border-book-border/50 bg-book-bg-paper/50 hover:border-book-primary/50 hover:bg-book-bg-tertiary/35"
        }`}
      >
        <button
          type="button"
          onClick={() => onSelectPrompt(prompt.id)}
          className="block min-w-0 p-2 text-left focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary"
        >
          <div className="mb-1.5 flex min-w-0 flex-wrap items-center gap-1 text-xs">
            <span className="font-mono text-book-text-main">#{prompt.shot_number}</span>
            <span className="min-w-0 flex-1 truncate font-medium text-book-text-main">
              {title}
            </span>
            <span className="pyqt-pill !text-[10px]">{prompt.duration_secs}s</span>
            <span className="pyqt-pill !text-[10px]">
              {PROMPT_OUTPUT_LANGUAGE_LABELS[promptLanguage]}
            </span>
            <span className={`pyqt-pill !text-[10px] ${statusClass(prompt.status)}`}>
              {prompt.status}
            </span>
            {prompt.generated_video_id != null && (
              <span className="pyqt-pill !text-[10px] !text-success">
                已成片
              </span>
            )}
            {referenceState.error ? (
              <span
                className="pyqt-pill !border-error/50 !text-[10px] !text-error"
                title={referenceState.error}
              >
                素材异常
              </span>
            ) : referenceState.ids.length > 0 && (
              <span className="pyqt-pill !text-[10px]">参考 {referenceState.ids.length}</span>
            )}
          </div>
          {prompt.scene_summary && (
            <div className="mb-1 line-clamp-1 text-[11px] text-book-text-sub">
              {prompt.scene_summary}
            </div>
          )}
          <div className="line-clamp-2 text-[11px] leading-relaxed text-book-text-sub">
            {prompt.visual_prompt}
          </div>
          {prompt.motion_prompt && (
            <div className="mt-1 line-clamp-1 text-[10px] leading-relaxed text-book-text-muted">
              Motion: {prompt.motion_prompt}
            </div>
          )}
        </button>
        <div className="flex flex-wrap items-center justify-end gap-1 border-t border-book-border/35 px-2 py-1.5">
          {chatVideoMode ? (
            <button
              type="button"
              onClick={() => runSelectedAction(() => onCopyPrompt(prompt))}
              disabled={prompt.prompt_type !== "video_prompt"}
              title={
                prompt.prompt_type !== "video_prompt"
                  ? "请选择 video_prompt 类型的提示词"
                  : selected
                    ? "复制 Chat 视频提示词"
                    : actionSelectionTitle
              }
              className="pyqt-btn-secondary inline-flex !min-h-9 shrink-0 items-center gap-1 !px-2 text-[10px] sm:!min-h-[24px]"
            >
              <Copy size={11} />
              复制词
            </button>
          ) : (
            <button
              type="button"
              onClick={() => {
                if (!selected) onSelectPrompt(prompt.id);
                void onGenerateVideo(prompt);
              }}
              disabled={generateVideoDisabled}
              title={
                selected
                  ? selectedGenerateBlockedReason ?? "生成当前视频提示词对应的视频"
                  : actionBaseBlockedReason ??
                    stalePromptReferenceBlockedReason ??
                    promptProviderMediaBlockedReason ??
                    "生成这条视频提示词对应的视频"
              }
              className="pyqt-btn-primary inline-flex !min-h-9 shrink-0 items-center gap-1 !px-2 text-[10px] sm:!min-h-[24px]"
            >
              {acting === `generate-video-${prompt.id}` ? (
                <Loader2 size={11} className="animate-spin" />
              ) : (
                <Play size={11} />
              )}
              生成视频
            </button>
          )}
          <button
            type="button"
            onClick={() => runSelectedAction(onImportVideo)}
            disabled={importVideoDisabled}
            title={
              selected
                ? selectedImportBlockedReason ?? "导入外部视频并绑定当前提示词"
                : actionBaseBlockedReason ?? actionSelectionTitle
            }
            className="pyqt-btn-secondary inline-flex !min-h-9 shrink-0 items-center gap-1 !px-2 text-[10px] sm:!min-h-[24px]"
          >
            {acting === `import-video-${prompt.id}` ? (
              <Loader2 size={11} className="animate-spin" />
            ) : (
              <Upload size={11} />
            )}
            导入视频
          </button>
          <button
            type="button"
            onClick={() => void onDeletePrompt(prompt)}
            disabled={deleteBlockedReason != null || acting === `delete-prompt-${prompt.id}`}
            title={deleteBlockedReason ?? "删除视频提示词"}
            className="inline-flex min-h-9 shrink-0 items-center gap-1 rounded border border-error/35 bg-transparent px-2 text-[10px] font-semibold text-error transition-colors hover:bg-error/10 focus-visible:outline-2 focus-visible:outline-error disabled:cursor-not-allowed disabled:opacity-45 sm:min-h-[24px]"
          >
            {acting === `delete-prompt-${prompt.id}` ? (
              <Loader2 size={11} className="animate-spin" />
            ) : (
              <Trash2 size={11} />
            )}
            删除
          </button>
        </div>
      </li>
    );
  };

  return (
    <BookCard className="flex min-h-0 flex-col p-3">
      <div className="mb-3 flex items-center gap-2">
        <div className="flex min-w-0 flex-1 overflow-hidden rounded-sm border border-book-border/40 bg-book-bg-paper/30">
          <button
            type="button"
            data-active={promptPanelTab === "generate" ? "1" : undefined}
            onClick={() => setPromptPanelTab("generate")}
            className="pyqt-tab min-w-0 flex-1 justify-center !px-2"
          >
            <Wand2 size={12} />
            <span className="truncate">生成设置</span>
          </button>
          <button
            type="button"
            data-active={promptPanelTab === "prompts" ? "1" : undefined}
            onClick={() => setPromptPanelTab("prompts")}
            className="pyqt-tab min-w-0 flex-1 justify-center !px-2"
          >
            <ListChecks size={12} />
            <span className="truncate">视频提示词</span>
            <span className="font-mono text-[10px]">{prompts.length}</span>
          </button>
        </div>
        <button
          type="button"
          onClick={() => void onLoad()}
          disabled={loading}
          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
          title="刷新"
        >
          <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
        </button>
      </div>

      {promptPanelTab === "generate" ? (
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
            <StyleTemplatePicker
              label={`${promptGenerationLabel}风格`}
              value={options.style}
              templates={VIDEO_STYLE_TEMPLATES}
              rows={4}
              onChange={(style) => onOptionsChange((prev) => ({ ...prev, style }))}
            />
            <div className="mt-2 grid grid-cols-1 gap-2 text-xs sm:grid-cols-2">
              <label className="flex min-w-0 flex-col gap-1">
                <span className="text-book-text-muted">本次生成</span>
                <input
                  type="number"
                  min={promptBatchInputMin}
                  max={promptBatchInputMax}
                  value={promptBatchInputValue}
                  disabled={promptBatchInputDisabled}
                  onChange={(event) => {
                    const promptBatchSize = parseNumberInput(event.target.value);
                    if (promptBatchSize !== null) {
                      onOptionsChange((prev) => ({
                        ...prev,
                        promptBatchSize: Math.min(Math.max(promptBatchSize, 1), promptBatchStateMax),
                      }));
                    }
                  }}
                  className="pyqt-input !px-2 !py-1.5"
                />
              </label>
              <label className="flex min-w-0 flex-col gap-1">
                <span className="text-book-text-muted">默认时长</span>
                <OptionsDurationControl
                  options={options}
                  durationLimits={durationLimits}
                  disabled={durationLimits.configError != null}
                  onOptionsChange={onOptionsChange}
                />
              </label>
              <label className="flex min-w-0 flex-col gap-1 sm:col-span-2">
                <span className="text-book-text-muted">可选时长</span>
                <VideoDurationOptionsPicker
                  options={options}
                  durationLimits={durationLimits}
                  disabled={durationLimits.configError != null}
                  onOptionsChange={onOptionsChange}
                />
              </label>
              <label className="flex min-w-0 flex-col gap-1">
                <span className="text-book-text-muted">画幅</span>
                <select
                  value={options.aspectRatio}
                  onChange={(event) =>
                    onOptionsChange((prev) => ({
                      ...prev,
                      aspectRatio: VIDEO_ASPECT_RATIOS.includes(event.target.value as VideoAspectRatio)
                        ? (event.target.value as VideoAspectRatio)
                        : prev.aspectRatio,
                    }))
                  }
                  className="pyqt-input !px-2 !py-1.5"
                >
                  {VIDEO_ASPECT_RATIOS.map((ratio) => (
                    <option key={ratio} value={ratio}>
                      {ratio}
                    </option>
                  ))}
                </select>
              </label>
              <label className="flex min-w-0 flex-col gap-1">
                <span className="text-book-text-muted">提示词语言</span>
                <select
                  value={options.promptOutputLanguage}
                  onChange={(event) =>
                    onOptionsChange((prev) => ({
                      ...prev,
                      promptOutputLanguage: PROMPT_OUTPUT_LANGUAGES.includes(
                        event.target.value as VideoGenOptions["promptOutputLanguage"],
                      )
                        ? (event.target.value as VideoGenOptions["promptOutputLanguage"])
                        : prev.promptOutputLanguage,
                    }))
                  }
                  className="pyqt-input !px-2 !py-1.5"
                >
                  {PROMPT_OUTPUT_LANGUAGES.map((language) => (
                    <option key={language} value={language}>
                      {PROMPT_OUTPUT_LANGUAGE_LABELS[language]}
                    </option>
                  ))}
                </select>
              </label>
            </div>

            {durationLimits.configError && (
              <div className="mt-2 rounded border border-error/40 bg-error/10 px-2 py-1.5 text-[11px] text-error">
                {durationLimits.configError}
              </div>
            )}
            {promptInputSelectionError && (
              <div className="mt-2 rounded border border-error/40 bg-error/10 px-2 py-1.5 text-[11px] text-error">
                {promptInputSelectionError}
              </div>
            )}

            <div className="mt-2 grid grid-cols-2 gap-2 text-[11px]">
              <div className="a4n-stat-tile px-2 py-1">
                <div className="text-book-text-muted">{framePromptLabel}</div>
                <div className="font-mono text-book-text-main">{framePromptCountLabel}</div>
              </div>
              <div className="a4n-stat-tile px-2 py-1">
                <div className="text-book-text-muted">全部视频提示词</div>
                <div className="font-mono text-book-text-main">
                  {expectedVideoPromptCount == null
                    ? prompts.length
                    : `${prompts.length}/${expectedVideoPromptCount}`}
                </div>
              </div>
              {remainingVideoPromptCount != null && (
                <div className="a4n-stat-tile col-span-2 px-2 py-1">
                  <div className="text-book-text-muted">当前语言剩余视频提示词</div>
                  <div className="font-mono text-book-text-main">
                    {remainingVideoPromptCount}
                  </div>
                </div>
              )}
            </div>
          </div>

          <div className="mt-3 shrink-0 border-t border-book-border/40 pt-3">
            {pendingVideo && (
              <button
                type="button"
                onClick={() => void onPollPendingVideo(pendingVideo)}
                disabled={acting === `poll-${pendingVideo.id}`}
                className="pyqt-btn-secondary mb-2 inline-flex w-full items-center justify-center gap-1 !min-h-9 !px-2 text-xs sm:!min-h-[28px]"
                title="恢复上次提交的视频任务状态"
              >
                {acting === `poll-${pendingVideo.id}` ? (
                  <Loader2 size={12} className="animate-spin" />
                ) : (
                  <RefreshCw size={12} />
                )}
                恢复视频任务 #{pendingVideo.id}
              </button>
            )}

            {videoPromptGenerationBlockedReason && (
              <div className="mb-2 rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-[11px] text-warning">
                {videoPromptGenerationBlockedReason}
              </div>
            )}

            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onGeneratePrompts()}
              disabled={
                acting === "generate-video-prompts" ||
                chapterGenerating ||
                !chapterContentReady ||
                !options.style.trim() ||
                durationLimits.configError != null ||
                promptInputSelectionError != null ||
                runningVideoPrompt != null ||
                expectedVideoPromptCount === 0 ||
                remainingVideoPromptCount === 0 ||
                options.promptBatchSize > promptBatchStateMax
              }
              title={videoPromptGenerationBlockedReason}
              className="w-full !min-h-10"
            >
              <span className="inline-flex items-center gap-1">
                {acting === "generate-video-prompts" ? (
                  <Loader2 size={13} className="animate-spin" />
                ) : (
                  <Wand2 size={13} />
                )}
                生成下一批{promptGenerationLabel}
              </span>
            </BookButton>

            {promptAuxiliaryActionCount > 0 && (
              <div
                className={`mt-2 grid gap-2 ${
                  promptAuxiliaryActionCount > 1 ? "grid-cols-1 sm:grid-cols-2" : "grid-cols-1"
                }`}
              >
                {showPromptRegenerateAction && (
                  <BookButton
                    variant="secondary"
                    size="sm"
                    onClick={() => void onGeneratePrompts({ regenerateExisting: true })}
                    disabled={
                      acting === "generate-video-prompts" ||
                      chapterGenerating ||
                      !chapterContentReady ||
                      !options.style.trim() ||
                      durationLimits.configError != null ||
                      promptInputSelectionError != null ||
                      runningVideoPrompt != null ||
                      generatedVideoPromptCount > 0
                    }
                    title={videoPromptRegenerateBlockedReason}
                    className="w-full !px-2"
                  >
                    <span className="inline-flex min-w-0 items-center gap-1">
                      {acting === "generate-video-prompts" ? (
                        <Loader2 size={13} className="shrink-0 animate-spin" />
                      ) : (
                        <RotateCcw size={13} className="shrink-0" />
                      )}
                      <span className="truncate">重新生成</span>
                    </span>
                  </BookButton>
                )}

                {showPromptPreviewAction && (
                  <BookButton
                    variant="ghost"
                    size="sm"
                    onClick={() => void onPreviewPromptRequests()}
                    disabled={previewingPromptRequests || promptInputSelectionError != null}
                    title={promptInputSelectionError ?? undefined}
                    className="w-full !px-2"
                  >
                    <span className="inline-flex min-w-0 items-center gap-1">
                      {previewingPromptRequests ? (
                        <Loader2 size={13} className="shrink-0 animate-spin" />
                      ) : (
                        <Eye size={13} className="shrink-0" />
                      )}
                      <span className="truncate">查看完整提示词</span>
                    </span>
                  </BookButton>
                )}
              </div>
            )}
          </div>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
          <div className="mb-2 flex items-center justify-between gap-2 text-[11px] text-book-text-muted">
            <div className="min-w-0">
              <div>最终视频提示词 {prompts.length}</div>
              {totalVideoPromptCount !== prompts.length && (
                <div className="truncate text-[10px] text-book-text-muted">
                  本章总计 {totalVideoPromptCount} 条
                </div>
              )}
              {selectedPrompt && (
                <div className="font-mono text-book-text-sub">选中 #{selectedPrompt.id}</div>
              )}
            </div>
            <div className="flex shrink-0 flex-wrap justify-end gap-1">
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => void onGenerateAllVideos()}
                disabled={
                  acting === "generate-all-videos" ||
                  acting?.startsWith("generate-video-") ||
                  generateAllVideosBlockedReason != null
                }
                title={generateAllVideosBlockedReason ?? "为所有未成片的视频提示词增量生成视频"}
                className="!min-h-9 !px-2 sm:!min-h-7"
              >
                <span className="inline-flex items-center gap-1">
                  {acting === "generate-all-videos" ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Play size={12} />
                  )}
                  生成全部视频
                </span>
              </BookButton>
              <BookButton
                variant="danger"
                size="sm"
                onClick={() => void onDeleteAllVideoPrompts()}
                disabled={
                  acting === "generate-all-videos" ||
                  acting === "delete-video-prompts" ||
                  deleteAllVideoPromptsBlockedReason != null
                }
                title={deleteAllVideoPromptsBlockedReason}
                className="!min-h-9 !px-2 sm:!min-h-7"
              >
                <span className="inline-flex items-center gap-1">
                  {acting === "delete-video-prompts" ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Trash2 size={12} />
                  )}
                  清空链路
                </span>
              </BookButton>
            </div>
          </div>
          {prompts.length === 0 ? (
            <EmptyHint text="本章还没有视频提示词。" />
          ) : (
            <ul className="space-y-2">{prompts.map(renderPromptRow)}</ul>
          )}
        </div>
      )}
    </BookCard>
  );
};
