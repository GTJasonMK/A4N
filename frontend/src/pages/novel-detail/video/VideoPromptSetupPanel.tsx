import React, { useEffect, useMemo, useState } from "react";
import { Eye, ListChecks, Loader2, Play, RefreshCw, RotateCcw, Trash2, Wand2 } from "lucide-react";
import type { ChapterVideoPrompt, GeneratedVideo } from "../../../api/video";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
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
  VIDEO_WORKFLOW_MODE_LABELS,
  type ChapterVideoWorkflowMode,
  type VideoAspectRatio,
  type VideoDurationLimits,
  type VideoGenOptions,
} from "./videoTabUtils";
import { VideoDurationOptionsPicker } from "./VideoDurationOptionsPicker";
import { promptTitle } from "./videoGenerationDisplay";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";

type PromptPanelTab = "generate" | "prompts";
type VideoPromptGenerationRequestOptions = {
  regenerateExisting?: boolean;
  promptBatchSize?: number;
  targetShotNumber?: number;
};

interface VideoPromptSetupPanelProps {
  chapter: ChapterRow;
  prompts: ChapterVideoPrompt[];
  generationPrompts: ChapterVideoPrompt[];
  selectedPrompt: ChapterVideoPrompt | null;
  selectedPromptId: number | null;
  workflowMode: ChapterVideoWorkflowMode;
  providerBackend: string | null;
  providerOptionsJson: unknown;
  pendingVideo: GeneratedVideo | null;
  framePromptCount: number;
  expectedBoundaryFrameCount: number | null;
  expectedVideoPromptCount: number | null;
  totalVideoChainPromptCount: number;
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  promptInputSelectionError: string | null;
  loading: boolean;
  acting: string | null;
  chapterGenerating: boolean;
  chapterContentReady: boolean;
  previewingPromptRequests: boolean;
  selectedPromptDirty: boolean;
  systemReferenceIds: Set<number>;
  validPromptReferenceIds: Set<number>;
  onLoad: () => void | Promise<void>;
  onOptionsChange: React.Dispatch<React.SetStateAction<VideoGenOptions>>;
  onGeneratePrompts: (options?: VideoPromptGenerationRequestOptions) => void | Promise<void>;
  onPreviewPromptRequests: () => void | Promise<void>;
  onSelectPrompt: (id: number) => void;
  onWorkflowModeChange: (mode: ChapterVideoWorkflowMode, shotNumber?: number) => void;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onDeleteAllVideoPrompts: () => void | Promise<void>;
  onPollPendingVideo: (video: GeneratedVideo) => void | Promise<void>;
  onGenerateAllVideos: () => void | Promise<void>;
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
  totalVideoChainPromptCount,
  options,
  durationLimits,
  promptInputSelectionError,
  loading,
  acting,
  chapterGenerating,
  chapterContentReady,
  previewingPromptRequests,
  selectedPromptDirty,
  systemReferenceIds,
  validPromptReferenceIds,
  onLoad,
  onOptionsChange,
  onGeneratePrompts,
  onPreviewPromptRequests,
  onSelectPrompt,
  onWorkflowModeChange,
  onDeletePrompt,
  onDeleteAllVideoPrompts,
  onPollPendingVideo,
  onGenerateAllVideos,
}) => {
  const [promptPanelTab, setPromptPanelTab] = useState<PromptPanelTab>("generate");
  const promptGroups = useMemo(() => {
    const groups = new Map<number, ChapterVideoPrompt[]>();
    for (const prompt of prompts) {
      const rows = groups.get(prompt.shot_number) ?? [];
      rows.push(prompt);
      groups.set(prompt.shot_number, rows);
    }
    return [...groups.entries()].sort(([left], [right]) => left - right);
  }, [prompts]);
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
    ? `${promptGenerationLabel}正在运行，不能继续生成`
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
      ? `镜头 ${runningPromptForVideoGeneration.shot_number} 正在生成视频，不能生成全部视频`
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
  const generatingPrompts = acting === "generate-video-prompts";
  const showGeneratePrimary =
    generatingPrompts || videoPromptGenerationBlockedReason == null;
  const generationProgressItems = [
    framePromptCount > 0 || (expectedBoundaryFrameCount ?? 0) > 0
      ? `${framePromptLabel} ${framePromptCountLabel}`
      : null,
    generationPrompts.length > 0 || (expectedVideoPromptCount ?? 0) > 0
      ? `视频提示词 ${
          expectedVideoPromptCount == null
            ? generationPrompts.length
            : `${generationPrompts.length}/${expectedVideoPromptCount}`
        }`
      : null,
    remainingVideoPromptCount != null && remainingVideoPromptCount > 0
      ? `剩余 ${remainingVideoPromptCount}`
      : null,
  ].filter((item): item is string => item != null);

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
    const promptLanguage = promptOutputLanguageOfPrompt(prompt);
    const title = promptTitle(prompt);
    const deleteBlockedReason =
      chapterGenerating
        ? chapterGeneratingMessage(chapter, "删除提示词")
        : prompt.status === "running"
          ? `镜头 ${prompt.shot_number} 正在运行，不能删除`
          : undefined;
    const referenceSummary = referenceState.error
      ? "素材信息异常"
      : stalePromptReferenceIds.length > 0
        ? `${stalePromptReferenceIds.length} 张参考图已失效`
        : referenceState.ids.length > 0
          ? `${referenceState.ids.length} 张参考图`
          : null;
    const metadata = [
      `${prompt.duration_secs} 秒`,
      prompt.workflow_mode === "boundary_frames" ||
      prompt.workflow_mode === "first_frame_chain" ||
      prompt.workflow_mode === "storyboard" ||
      prompt.workflow_mode === "reference_media" ||
      prompt.workflow_mode === "chat_video"
        ? VIDEO_WORKFLOW_MODE_LABELS[prompt.workflow_mode]
        : prompt.workflow_mode,
      PROMPT_OUTPUT_LANGUAGE_LABELS[promptLanguage],
      getOperationStatusLabel(prompt.status),
      prompt.generated_video_id != null ? "已生成视频" : null,
      referenceSummary,
    ].filter((value): value is string => Boolean(value));

    return (
      <li
        key={prompt.id}
        className={`group flex min-w-0 items-start transition-colors ${
          selected
            ? "bg-book-primary/8"
            : "hover:bg-book-bg-tertiary/45"
        }`}
      >
        <button
          type="button"
          onClick={() => onSelectPrompt(prompt.id)}
          className="min-w-0 flex-1 px-3 py-3 text-left focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary"
        >
          <div className="flex min-w-0 items-baseline gap-1.5">
            <span className="shrink-0 text-xs text-book-text-main">
              镜头 {prompt.shot_number}
            </span>
            <span className="min-w-0 flex-1 truncate text-xs font-medium text-book-text-main">
              · {title}
            </span>
          </div>
          <div
            className={`mt-1 line-clamp-1 text-[10px] ${
              referenceState.error || stalePromptReferenceIds.length > 0
                ? "text-error"
                : "text-book-text-muted"
            }`}
            title={referenceState.error ?? undefined}
          >
            {metadata.join(" · ")}
          </div>
          {prompt.scene_summary && (
            <div className="mt-1.5 line-clamp-1 text-[11px] text-book-text-sub">
              {prompt.scene_summary}
            </div>
          )}
          <div className="mt-1 line-clamp-2 text-[11px] leading-relaxed text-book-text-muted">
            {prompt.visual_prompt}
          </div>
        </button>
        <div className="shrink-0 pr-2 pt-2.5">
          {deleteBlockedReason == null && acting !== `delete-prompt-${prompt.id}` ? (
            <Dropdown
              items={[
                {
                  label: "删除视频提示词",
                  danger: true,
                  icon: <Trash2 size={13} />,
                  onClick: () => void onDeletePrompt(prompt),
                },
              ]}
            />
          ) : null}
        </div>
      </li>
    );
  };

  return (
    <section className="flex min-h-0 flex-col">
      <div className="mb-3 flex items-center gap-2">
        <SegmentedControl
          ariaLabel="视频提示词面板"
          value={promptPanelTab}
          onChange={setPromptPanelTab}
          options={[
            { id: "generate" as const, label: "生成", icon: <Wand2 size={12} /> },
            {
              id: "prompts" as const,
              label: `提示词${prompts.length > 0 ? ` ${prompts.length}` : ""}`,
              icon: <ListChecks size={12} />,
            },
          ]}
        />
        <span className="flex-1" />
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
        <div className="a4n-flat-surface flex min-h-0 flex-1 flex-col p-3">
          <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
            <StyleTemplatePicker
              label={`${promptGenerationLabel}风格`}
              value={options.style}
              templates={VIDEO_STYLE_TEMPLATES}
              rows={4}
              onChange={(style) => onOptionsChange((prev) => ({ ...prev, style }))}
            />
            <details className="group mt-3 border-y border-book-border/40 py-2">
              <summary className="flex min-h-8 cursor-pointer list-none items-center gap-2 text-xs [&::-webkit-details-marker]:hidden">
                <span className="font-medium text-book-text-main">生成选项</span>
                <span className="min-w-0 flex-1 truncate text-right text-[10px] text-book-text-muted">
                  每批 {promptBatchInputValue} · {options.durationSecs} 秒 · {options.aspectRatio} · {PROMPT_OUTPUT_LANGUAGE_LABELS[options.promptOutputLanguage]}
                </span>
              </summary>
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
            </details>

            {durationLimits.configError && (
              <div role="alert" className="mt-2 rounded-lg border border-error/35 bg-error-bg px-2.5 py-2 text-[11px] text-error">
                {durationLimits.configError}
              </div>
            )}
            {promptInputSelectionError && (
              <div role="alert" className="mt-2 rounded-lg border border-error/35 bg-error-bg px-2.5 py-2 text-[11px] text-error">
                {promptInputSelectionError}
              </div>
            )}

            {generationProgressItems.length > 0 ? (
              <div className="mt-3 text-[11px] leading-relaxed text-book-text-muted">
                {generationProgressItems.join(" · ")}
              </div>
            ) : null}
          </div>

          <div className="mt-3 shrink-0 border-t border-book-border/40 pt-3">
            {pendingVideo && (
              <div className="mb-2 text-[11px] text-book-text-muted">
                有一个已提交的视频任务等待同步。
              </div>
            )}

            {videoPromptGenerationBlockedReason && (
              <div role="status" className="mb-2 rounded-lg border border-warning/35 bg-warning-bg px-2.5 py-2 text-[11px] text-warning">
                {videoPromptGenerationBlockedReason}
              </div>
            )}

            <div className="flex items-center gap-2">
              {showGeneratePrimary && (
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={() => void onGeneratePrompts()}
                  disabled={generatingPrompts}
                  title={videoPromptGenerationBlockedReason}
                  className="min-h-10 flex-1"
                >
                  <span className="inline-flex items-center gap-1">
                    {generatingPrompts ? (
                      <Loader2 size={13} className="animate-spin" />
                    ) : (
                      <Wand2 size={13} />
                    )}
                    {generatingPrompts ? "正在生成" : `生成下一批${promptGenerationLabel}`}
                  </span>
                </BookButton>
              )}
              {(pendingVideo || showPromptRegenerateAction || showPromptPreviewAction) && (
                <Dropdown
                  label="更多"
                  items={[
                    ...(pendingVideo
                      ? [{
                          label: acting === `poll-${pendingVideo.id}` ? "正在同步视频任务" : "同步上次视频任务",
                          icon: acting === `poll-${pendingVideo.id}`
                            ? <Loader2 size={13} className="animate-spin" />
                            : <RefreshCw size={13} />,
                          disabled: acting === `poll-${pendingVideo.id}`,
                          onClick: () => void onPollPendingVideo(pendingVideo),
                        }]
                      : []),
                    ...(showPromptRegenerateAction
                      ? [{
                          label: "重新生成视频提示词",
                          icon: <RotateCcw size={13} />,
                          disabled:
                            generatingPrompts || videoPromptRegenerateBlockedReason != null,
                          onClick: () => void onGeneratePrompts({ regenerateExisting: true }),
                        }]
                      : []),
                    ...(showPromptPreviewAction
                      ? [{
                          label: previewingPromptRequests ? "正在准备模型请求" : "查看完整模型请求",
                          icon: previewingPromptRequests
                            ? <Loader2 size={13} className="animate-spin" />
                            : <Eye size={13} />,
                          disabled: previewingPromptRequests || promptInputSelectionError != null,
                          onClick: () => void onPreviewPromptRequests(),
                        }]
                      : []),
                  ]}
                />
              )}
            </div>
          </div>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
          {prompts.length > 0 ? (
            <div className="mb-2 flex min-w-0 items-center gap-2">
              <span className="min-w-0 flex-1 text-[11px] text-book-text-muted">
                {prompts.length} 条提示词
                {generatedVideoPromptCount > 0
                  ? ` · ${generatedVideoPromptCount} 条已生成视频`
                  : ""}
              </span>
              {!chatVideoMode &&
                (acting === "generate-all-videos" || generateAllVideosBlockedReason == null) && (
                  <BookButton
                    variant="primary"
                    size="sm"
                    onClick={() => void onGenerateAllVideos()}
                    disabled={acting === "generate-all-videos"}
                    title={generateAllVideosBlockedReason ?? "为所有未成片的视频提示词增量生成视频"}
                    className="!px-2.5"
                  >
                    <span className="inline-flex items-center gap-1">
                      {acting === "generate-all-videos" ? (
                        <Loader2 size={12} className="animate-spin" />
                      ) : (
                        <Play size={12} />
                      )}
                      {acting === "generate-all-videos" ? "生成中" : "生成全部"}
                    </span>
                  </BookButton>
                )}
              {totalVideoChainPromptCount > 0 && (
                <Dropdown
                  items={[
                    {
                      label: "清空视频提示词链路",
                      icon: <Trash2 size={12} />,
                      danger: true,
                      disabled:
                        acting === "generate-all-videos" ||
                        acting === "delete-video-prompts" ||
                        deleteAllVideoPromptsBlockedReason != null,
                      onClick: () => void onDeleteAllVideoPrompts(),
                    },
                  ]}
                />
              )}
            </div>
          ) : null}
          {!chatVideoMode && generateAllVideosBlockedReason && prompts.length > 0 && (
            <div className="mb-2 text-[11px] text-book-text-muted">
              {generateAllVideosBlockedReason}
            </div>
          )}
          {prompts.length === 0 ? (
            <EmptyHint text="本章还没有视频提示词。" />
          ) : (
            <div className="space-y-2">
              {promptGroups.map(([shotNumber, variants]) => {
                const currentVariant = variants.find(
                  (prompt) => prompt.workflow_mode === workflowMode,
                );
                const canGenerateCurrentVariant =
                  workflowMode !== "reference_media";
                return (
                  <section key={shotNumber} className="a4n-flat-surface overflow-hidden">
                    <div className="border-b border-book-border/40 px-3 py-2">
                      <div className="mb-2 flex items-center justify-between gap-2">
                        <span className="text-xs font-semibold text-book-text-main">片段 {shotNumber}</span>
                        <span className="text-[10px] text-book-text-muted">{variants.length} 个执行方案</span>
                      </div>
                      <div className="flex min-w-0 flex-wrap items-center gap-2">
                        <div className="max-w-full">
                          <SegmentedControl
                            ariaLabel={`片段 ${shotNumber} 生成方案`}
                            value={workflowMode}
                            onChange={(mode) => onWorkflowModeChange(mode, shotNumber)}
                            className="a4n-seg--segment-workflows"
                            options={[
                              { id: "boundary_frames", label: "首尾帧" },
                              { id: "first_frame_chain", label: "首帧续接" },
                              { id: "storyboard", label: "分镜" },
                              { id: "reference_media", label: "参考媒体" },
                              { id: "chat_video", label: "Chat" },
                            ]}
                          />
                        </div>
                        {!currentVariant && canGenerateCurrentVariant && (
                          <BookButton
                            variant="secondary"
                            size="sm"
                            disabled={acting != null || chapterGenerating}
                            onClick={() =>
                              void onGeneratePrompts({
                                targetShotNumber: shotNumber,
                                promptBatchSize: 1,
                              })
                            }
                          >
                            <span className="inline-flex items-center gap-1">
                              <Wand2 size={11} />
                              生成此方案提示词
                            </span>
                          </BookButton>
                        )}
                      </div>
                    </div>
                    <ul className="divide-y divide-book-border/35">
                      {variants.map(renderPromptRow)}
                    </ul>
                  </section>
                );
              })}
            </div>
          )}
        </div>
      )}
    </section>
  );
};
