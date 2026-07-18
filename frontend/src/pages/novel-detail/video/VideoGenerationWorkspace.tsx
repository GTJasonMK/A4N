import React, { useMemo, useState } from "react";
import {
  Eye,
  Image as ImageIcon,
  Images,
  ListChecks,
  Loader2,
  Copy,
  PanelTop,
  Play,
  Save,
  Trash2,
  Upload,
  Wand2,
} from "lucide-react";
import type { GeneratedImage } from "../../../api/image";
import type {
  ChapterVideoPrompt,
  GeneratedVideo,
  UpdateVideoInput,
  VideoWorkflowMode,
} from "../../../api/video";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { EmptyHint, type ChapterRow } from "../shared";
import { imageTypeLabel } from "../images/imageTabUtils";
import {
  parseNumberInput,
  promptByteCount,
  promptCharCount,
  staleVideoPromptMessage,
  videoPromptUnitFrameNumbers,
  videoFrameImageRoleLabel,
  videoFrameImageRoleOfImage,
  videoImageGenerationPromptId,
  videoTailFrameBoundaryNumber,
  isVideoPromptFrameImage,
  type PromptDraft,
  type VideoDurationLimits,
  type VideoGenOptions,
} from "./videoTabUtils";
import { VideoPromptSetupPanel } from "./VideoPromptSetupPanel";
import { VideoResultsPane } from "./VideoResultsPane";
import {
  directorSections,
  isVideoTailFrameImage,
  promptTitle,
} from "./videoGenerationDisplay";

interface VideoGenerationWorkspaceProps {
  chapter: ChapterRow;
  prompts: ChapterVideoPrompt[];
  generationPrompts: ChapterVideoPrompt[];
  selectedPrompt: ChapterVideoPrompt | null;
  selectedPromptId: number | null;
  workflowMode: VideoWorkflowMode;
  pendingVideo: GeneratedVideo | null;
  selectedPromptStale: boolean;
  expectedBoundaryFrameCount: number | null;
  expectedVideoPromptCount: number | null;
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  providerBackend: string | null;
  providerOptionsJson: unknown;
  promptInputSelectionError: string | null;
  loading: boolean;
  acting: string | null;
  chapterGenerating: boolean;
  chapterContentReady: boolean;
  previewingPromptRequests: boolean;
  draft: PromptDraft;
  selectedPromptSceneSummary: string;
  selectedPromptRunning: boolean;
  selectedPromptDirty: boolean;
  staleSavedReferenceCount: number;
  savedReferenceIdsError: string | null;
  invalidPromptReferenceIds: number[];
  systemReferenceIds: Set<number>;
  validPromptReferenceIds: Set<number>;
  boundReferenceImages: GeneratedImage[];
  framePrompts: ChapterVideoPrompt[];
  frameImages: GeneratedImage[];
  videos: GeneratedVideo[];
  totalVideoAssetCount: number;
  totalVideoChainPromptCount: number;
  selectedVideo: GeneratedVideo | null;
  selectedVideoId: number | null;
  selectedVideoPath: string;
  trimStart: string;
  trimEnd: string;
  referenceImageScopeLabel: (img: GeneratedImage) => string;
  onLoad: () => void | Promise<void>;
  onOptionsChange: React.Dispatch<React.SetStateAction<VideoGenOptions>>;
  onGeneratePromptRequests: (options?: {
    regenerateExisting?: boolean;
    promptBatchSize?: number;
  }) => void | Promise<void>;
  onPreviewPromptRequests: () => void | Promise<void>;
  onSelectPrompt: (id: number) => void;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onDeleteAllVideoPrompts: () => void | Promise<void>;
  onPollPendingVideo: (video: GeneratedVideo) => void | Promise<void>;
  onOpenFrames: () => void;
  onDraftChange: (draft: PromptDraft) => void;
  onClearInvalidPromptReferences: () => void;
  onSavePrompt: () => void | Promise<void>;
  onOptimizePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onPreviewGenerateVideo: () => void | Promise<void>;
  onCopyPrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onPreviewVideoRequest: (video: GeneratedVideo) => void | Promise<void>;
  onGenerateVideo: (prompt?: ChapterVideoPrompt) => void | Promise<void>;
  onGenerateAllVideos: () => void | Promise<void>;
  onImportVideo: () => boolean | Promise<boolean>;
  onSelectVideo: (id: number) => void;
  onPollVideo: (video: GeneratedVideo) => void | Promise<void>;
  onToggleVideoFavorite: (video: GeneratedVideo) => void | Promise<void>;
  onUpdateVideo: (
    video: GeneratedVideo,
    input: UpdateVideoInput,
  ) => void | Promise<void>;
  onDeleteVideo: (video: GeneratedVideo) => void | Promise<void>;
  onDeleteAllVideoAssets: () => void | Promise<void>;
  onTrimStartChange: (value: string) => void;
  onTrimEndChange: (value: string) => void;
  onTrim: () => void | Promise<void>;
}

const DraftDurationControl: React.FC<{
  value: number;
  durationLimits: VideoDurationLimits;
  disabled?: boolean;
  onChange: (value: number) => void;
}> = ({ value, durationLimits, disabled = false, onChange }) => {
  if (
    durationLimits.durationMode === "fixed" &&
    durationLimits.supportedDurations.length > 0
  ) {
    return (
      <select
        value={value}
        onChange={(event) => {
          if (disabled) return;
          const durationSecs = parseNumberInput(event.target.value);
          if (
            durationSecs !== null &&
            durationLimits.supportedDurations.includes(durationSecs)
          ) {
            onChange(durationSecs);
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
      value={value}
      onChange={(event) => {
        if (disabled) return;
        const durationSecs = parseNumberInput(event.target.value);
        if (durationSecs !== null) onChange(durationSecs);
      }}
      disabled={disabled}
      className="pyqt-input !px-2 !py-1.5"
    />
  );
};

export const VideoGenerationWorkspace: React.FC<
  VideoGenerationWorkspaceProps
> = ({
  chapter,
  prompts,
  generationPrompts,
  selectedPrompt,
  selectedPromptId,
  workflowMode,
  pendingVideo,
  selectedPromptStale,
  expectedBoundaryFrameCount,
  expectedVideoPromptCount,
  options,
  durationLimits,
  providerBackend,
  providerOptionsJson,
  promptInputSelectionError,
  loading,
  acting,
  chapterGenerating,
  chapterContentReady,
  previewingPromptRequests,
  draft,
  selectedPromptSceneSummary,
  selectedPromptRunning,
  selectedPromptDirty,
  staleSavedReferenceCount,
  savedReferenceIdsError,
  invalidPromptReferenceIds,
  systemReferenceIds,
  validPromptReferenceIds,
  boundReferenceImages,
  framePrompts,
  frameImages,
  videos,
  totalVideoAssetCount,
  totalVideoChainPromptCount,
  selectedVideo,
  selectedVideoId,
  selectedVideoPath,
  trimStart,
  trimEnd,
  referenceImageScopeLabel,
  onLoad,
  onOptionsChange,
  onGeneratePromptRequests,
  onPreviewPromptRequests,
  onSelectPrompt,
  onDeletePrompt,
  onDeleteAllVideoPrompts,
  onPollPendingVideo,
  onOpenFrames,
  onDraftChange,
  onClearInvalidPromptReferences,
  onSavePrompt,
  onOptimizePrompt,
  onPreviewGenerateVideo,
  onCopyPrompt,
  onPreviewVideoRequest,
  onGenerateVideo,
  onGenerateAllVideos,
  onImportVideo,
  onSelectVideo,
  onPollVideo,
  onToggleVideoFavorite,
  onUpdateVideo,
  onDeleteVideo,
  onDeleteAllVideoAssets,
  onTrimStartChange,
  onTrimEndChange,
  onTrim,
}) => {
  const [promptDetailOpen, setPromptDetailOpen] = useState(false);
  const [mobilePane, setMobilePane] = useState<"setup" | "results">("setup");

  const updateDraft = (patch: Partial<PromptDraft>) =>
    onDraftChange({ ...draft, ...patch });

  const framePromptByNumber = useMemo(() => {
    const map = new Map<number, ChapterVideoPrompt>();
    for (const prompt of framePrompts) map.set(prompt.shot_number, prompt);
    return map;
  }, [framePrompts]);
  const frameImageIndexes = useMemo(() => {
    const frameNumberByPromptId = new Map(
      framePrompts.map((prompt) => [prompt.id, prompt.shot_number]),
    );
    const videoPromptIds = new Set(prompts.map((prompt) => prompt.id));
    const boundaryByFrameNumber = new Map<number, GeneratedImage>();
    const tailByPromptId = new Map<number, GeneratedImage>();
    const sorted = [...frameImages].sort((a, b) => b.id - a.id);
    for (const image of sorted) {
      const promptId = videoImageGenerationPromptId(image);
      const promptFrameImage = promptId != null && isVideoPromptFrameImage(image);
      if (promptFrameImage) {
        const frameNumber = frameNumberByPromptId.get(promptId);
        if (frameNumber != null && !boundaryByFrameNumber.has(frameNumber))
          boundaryByFrameNumber.set(frameNumber, image);
      }
      if (!promptFrameImage) {
        const tailBoundaryNumber = videoTailFrameBoundaryNumber(image);
        if (
          tailBoundaryNumber != null &&
          !boundaryByFrameNumber.has(tailBoundaryNumber)
        ) {
          boundaryByFrameNumber.set(tailBoundaryNumber, image);
        }
      }
      if (
        promptId != null &&
        isVideoTailFrameImage(image) &&
        videoPromptIds.has(promptId) &&
        !tailByPromptId.has(promptId)
      ) {
        tailByPromptId.set(promptId, image);
      }
    }
    return { boundaryByFrameNumber, tailByPromptId };
  }, [frameImages, framePrompts, prompts]);
  const latestPromptByShotNumber = useMemo(() => {
    const map = new Map<number, ChapterVideoPrompt>();
    for (const prompt of prompts) {
      const current = map.get(prompt.shot_number);
      if (!current || prompt.id > current.id) {
        map.set(prompt.shot_number, prompt);
      }
    }
    return map;
  }, [prompts]);

  const selectPrompt = (id: number) => {
    onSelectPrompt(id);
    setPromptDetailOpen(true);
  };

  const selectVideo = (id: number) => {
    onSelectVideo(id);
  };

  const renderBoundReferenceImages = () => {
    if (boundReferenceImages.length === 0) {
      return (
        <div className="text-[11px] text-book-text-muted">
          当前提示词未绑定参考图。
        </div>
      );
    }

    return (
      <ul className="grid grid-cols-4 gap-2 md:grid-cols-6 xl:grid-cols-8">
        {boundReferenceImages.map((image) => {
          const role = videoFrameImageRoleOfImage(image);
          const src = toMediaSrc(image.file_path);
          const scopeLabel = referenceImageScopeLabel(image);
          const stale =
            !systemReferenceIds.has(image.id) && scopeLabel === "指纹不一致";
          return (
            <li key={image.id} className="min-w-0 text-[10px]">
              <div
                className={`relative flex aspect-square items-center justify-center overflow-hidden rounded-xl border bg-book-bg-tertiary ${
                  stale ? "border-error/60" : "border-book-border/50"
                }`}
              >
                {src ? (
                  <img
                    src={src}
                    alt=""
                    className="h-full w-full object-cover"
                  />
                ) : (
                  "无图"
                )}
              </div>
              <div className="truncate text-book-text-muted">
                {videoFrameImageRoleLabel(role, imageTypeLabel(image.image_type))}
              </div>
            </li>
          );
        })}
      </ul>
    );
  };

  const renderBoundaryFrameSlot = (
    label: string,
    prompt: ChapterVideoPrompt | null,
    image: GeneratedImage | null,
    emptyText = "等待首尾帧生成",
  ) => {
    const src = image ? toMediaSrc(image.file_path) : "";
    return (
      <div className="min-w-0">
        <div className="mb-1.5 min-w-0 text-xs">
          <span className="font-medium text-book-text-main">{label}</span>
          <div className="mt-0.5 text-[10px] text-book-text-muted">
            {prompt ? `帧 ${prompt.shot_number}` : "尚无提示词"}
            <span className="mx-1">·</span>
            {image ? "已生成" : "等待生成"}
          </div>
        </div>
        <div className="flex aspect-video items-center justify-center overflow-hidden rounded-xl bg-book-bg-tertiary text-[11px] text-book-text-muted">
          {src ? (
            <img src={src} alt={label} className="h-full w-full object-cover" />
          ) : (
            "未生成"
          )}
        </div>
        <div className="mt-1 line-clamp-1 text-[10px] text-book-text-sub">
          {prompt?.title || prompt?.scene_summary || emptyText}
        </div>
      </div>
    );
  };

  const frameInputStateForPrompt = (prompt: ChapterVideoPrompt) => {
    const firstFrameChainMode = workflowMode === "first_frame_chain";
    const chatVideoMode = workflowMode === "chat_video";
    const storyboardMode = workflowMode === "storyboard";
    const { startFrameNumber, endFrameNumber } =
      videoPromptUnitFrameNumbers(prompt);
    const startFramePrompt =
      framePromptByNumber.get(startFrameNumber ?? prompt.shot_number) ?? null;
    const endFramePrompt = firstFrameChainMode || storyboardMode
      ? null
      : (endFrameNumber == null
          ? null
          : (framePromptByNumber.get(endFrameNumber) ?? null));
    const generatedStartFrameImage =
      startFramePrompt == null
        ? null
        : (frameImageIndexes.boundaryByFrameNumber.get(
            startFramePrompt.shot_number,
          ) ??
          null);
    const previousVideoPrompt =
      firstFrameChainMode && prompt.shot_number > 1
        ? (latestPromptByShotNumber.get(prompt.shot_number - 1) ?? null)
        : null;
    const previousTailFrameImage =
      previousVideoPrompt == null
        ? null
        : (frameImageIndexes.tailByPromptId.get(previousVideoPrompt.id) ??
          null);
    const actualStartFrameImage =
      firstFrameChainMode && prompt.shot_number > 1
        ? previousTailFrameImage
        : generatedStartFrameImage;
    const endFrameImage =
      endFramePrompt == null
        ? null
        : (frameImageIndexes.boundaryByFrameNumber.get(
            endFramePrompt.shot_number,
          ) ??
          null);
    const startFrameMissingMessage =
      firstFrameChainMode && !actualStartFrameImage
        ? prompt.shot_number <= 1
          ? "请先在首帧生成里生成第 1 段首帧图片。"
          : `请先生成并完成第 ${prompt.shot_number - 1} 个视频单元，等待系统采集尾帧后再生成本单元。`
        : null;
    const boundaryFrameMissingMessage =
      workflowMode === "boundary_frames"
        ? [
            startFramePrompt
              ? generatedStartFrameImage
                ? null
                : `第 ${startFrameNumber ?? prompt.shot_number} 张起始边界图`
              : `第 ${startFrameNumber ?? prompt.shot_number} 张边界帧提示词`,
            endFramePrompt
              ? endFrameImage
                ? null
                : `第 ${endFrameNumber ?? prompt.shot_number} 张结束边界图`
              : `第 ${endFrameNumber ?? prompt.shot_number} 张边界帧提示词`,
          ]
            .filter((value): value is string => Boolean(value))
            .join("、")
        : "";
    const storyboardFrameMissingMessage = storyboardMode
      ? startFramePrompt
        ? generatedStartFrameImage
          ? null
          : `第 ${prompt.shot_number} 段分镜板`
        : `第 ${prompt.shot_number} 段分镜提示词`
      : null;
    const frameInputMissingMessage = chatVideoMode
      ? null
      : storyboardFrameMissingMessage
        ? `请先在分镜生成里生成${storyboardFrameMissingMessage}。`
        : startFrameMissingMessage ||
        (boundaryFrameMissingMessage
          ? `请先在首尾帧生成里生成${boundaryFrameMissingMessage}。`
          : null);

    return {
      startFramePrompt,
      endFramePrompt,
      generatedStartFrameImage,
      previousVideoPrompt,
      actualStartFrameImage,
      endFrameImage,
      startFrameMissingMessage,
      frameInputMissingMessage,
    };
  };

  const selectedPromptFrameInputState = selectedPrompt
    ? frameInputStateForPrompt(selectedPrompt)
    : null;

  const selectedPromptCharStats = useMemo(() => {
    if (!selectedPrompt) return null;
    const visualPrompt = draft.visual_prompt ?? selectedPrompt.visual_prompt;
    const motionPrompt =
      draft.motion_prompt ?? selectedPrompt.motion_prompt ?? "";
    const negativePrompt =
      draft.negative_prompt ?? selectedPrompt.negative_prompt ?? "";
    return {
      visualChars: promptCharCount(visualPrompt),
      visualBytes: promptByteCount(visualPrompt),
      motionChars: promptCharCount(motionPrompt),
      motionBytes: promptByteCount(motionPrompt),
      negativeChars: promptCharCount(negativePrompt),
      negativeBytes: promptByteCount(negativePrompt),
    };
  }, [selectedPrompt, draft]);

  const promptLengthText = (chars: number, bytes: number): string =>
    `${chars.toLocaleString()} 字符 / ${bytes.toLocaleString()} 字节`;

  const renderEditor = () => {
    if (!selectedPrompt) {
      return (
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <EmptyHint text="选择左侧视频提示词后编辑和生成视频。" />
        </div>
      );
    }

    const chatVideoMode = workflowMode === "chat_video";
    const firstFrameChainMode = workflowMode === "first_frame_chain";
    const storyboardMode = workflowMode === "storyboard";
    const frameStageLabel = chatVideoMode
      ? "Chat 生视频"
      : storyboardMode
        ? "分镜生成"
      : firstFrameChainMode
        ? "首帧生成"
        : "首尾帧生成";
    const {
      startFramePrompt,
      endFramePrompt,
      generatedStartFrameImage,
      previousVideoPrompt,
      actualStartFrameImage,
      endFrameImage,
      startFrameMissingMessage,
      frameInputMissingMessage,
    } =
      selectedPromptFrameInputState ?? frameInputStateForPrompt(selectedPrompt);
    const selectedPromptHasVideo = selectedPrompt.generated_video_id != null;
    const selectedPromptLockedReason = selectedPromptRunning
      ? "该视频提示词正在运行，不能编辑或保存"
      : null;
    const selectedPromptDownstreamNotice = selectedPromptHasVideo
      ? "该视频提示词已有下游视频；保存、优化或删除时需要选择删除下游或保留下游继续"
      : null;
    const durationConfigError = durationLimits.configError;
    const saveBlockedReason =
      durationConfigError ??
      (chapterGenerating
        ? "当前章节正在生成，不能保存视频提示词"
        : undefined) ??
      (invalidPromptReferenceIds.length > 0
        ? `当前草稿中有 ${invalidPromptReferenceIds.length} 张参考图已不可用，请重新选择`
        : undefined) ??
      selectedPromptLockedReason ??
      undefined;
    const optimizeBlockedReason =
      (selectedPromptDirty
        ? "当前编辑内容未保存，请先保存或放弃草稿后再优化"
        : undefined) ??
      selectedPromptLockedReason ??
      (chapterGenerating
        ? "当前章节正在生成，不能优化视频提示词"
        : undefined) ??
      undefined;
    const videoGenerationInFlight =
      acting === `generate-video-${selectedPrompt.id}`;
    const selectedGenerateBlockedReason =
      durationConfigError ??
      (selectedPrompt.prompt_type !== "video_prompt"
        ? "当前记录不是视频生成提示词"
        : undefined) ??
      (selectedPromptStale
        ? "当前视频提示词已过期，请重新生成"
        : undefined) ??
      (savedReferenceIdsError
        ? `保存的参考图数据异常：${savedReferenceIdsError}`
        : undefined) ??
      (invalidPromptReferenceIds.length > 0
        ? `当前草稿中有 ${invalidPromptReferenceIds.length} 张参考图已不可用，请重新选择`
        : undefined) ??
      (selectedPromptDirty ? "请先保存当前更改" : undefined) ??
      frameInputMissingMessage ??
      (selectedPromptRunning ? "当前视频提示词正在运行" : undefined) ??
      (selectedPromptHasVideo ? "当前视频提示词已生成视频" : undefined) ??
      (chapterGenerating ? "当前章节正在生成，暂不能生成视频" : undefined) ??
      (!chapterContentReady ? "当前章节正文为空，不能生成视频" : undefined);
    const selectedImportBlockedReason =
      (selectedPrompt.prompt_type !== "video_prompt"
        ? "当前记录不是视频生成提示词"
        : undefined) ??
      (selectedPromptStale
        ? "当前视频提示词已过期，请重新生成"
        : undefined) ??
      (selectedPromptDirty ? "请先保存当前更改" : undefined) ??
      (selectedPromptRunning ? "当前视频提示词正在运行" : undefined) ??
      (selectedPromptHasVideo ? "当前视频提示词已关联视频" : undefined) ??
      (chapterGenerating ? "当前章节正在生成，暂不能导入视频" : undefined) ??
      (!chapterContentReady ? "当前章节正文为空，不能导入视频" : undefined);
    const deletePromptBlockedReason =
      (chapterGenerating ? "当前章节正在生成，暂不能删除提示词" : undefined) ??
      (selectedPromptRunning ? "当前视频提示词正在运行" : undefined);
    const savingPrompt = acting === `save-prompt-${selectedPrompt.id}`;
    const showSavePrimary = savingPrompt || selectedPromptDirty;
    const showCopyPrimary = !showSavePrimary && chatVideoMode;
    const showGeneratePrimary =
      !showSavePrimary &&
      !chatVideoMode &&
      (videoGenerationInFlight || selectedGenerateBlockedReason == null);
    const editorMetadata = [
      `${selectedPrompt.duration_secs} 秒`,
      getOperationStatusLabel(selectedPrompt.status),
      selectedPromptStale ? "正文已变化" : null,
      selectedPrompt.generated_video_id != null ? "已生成视频" : null,
      selectedPromptDirty ? "有未保存更改" : null,
    ].filter((value): value is string => Boolean(value));
    const sections = directorSections(selectedPrompt.metadata_json);

    return (
      <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
        <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2 border-b border-book-border/40 pb-3">
          <span className="min-w-0 flex-1 text-[11px] text-book-text-muted">
            {editorMetadata.join(" · ")}
          </span>
          <div className="flex shrink-0 items-center gap-2">
            {showSavePrimary && (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void onSavePrompt()}
                disabled={
                  savingPrompt ||
                  selectedPromptRunning ||
                  chapterGenerating ||
                  invalidPromptReferenceIds.length > 0 ||
                  durationConfigError != null
                }
                title={saveBlockedReason}
                className="!min-h-9 !px-3 sm:!min-h-7"
              >
                <span className="inline-flex items-center gap-1">
                  {savingPrompt ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Save size={12} />
                  )}
                  {savingPrompt ? "保存中" : "保存更改"}
                </span>
              </BookButton>
            )}
            {showCopyPrimary && (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void onCopyPrompt(selectedPrompt)}
                disabled={selectedPrompt.prompt_type !== "video_prompt"}
                className="!min-h-9 !px-3 sm:!min-h-7"
              >
                <span className="inline-flex items-center gap-1">
                  <Copy size={12} />
                  复制提示词
                </span>
              </BookButton>
            )}
            {showGeneratePrimary && (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void onGenerateVideo(selectedPrompt)}
                disabled={videoGenerationInFlight}
                title={selectedGenerateBlockedReason}
                className="!min-h-9 !px-3 sm:!min-h-7"
              >
                <span className="inline-flex items-center gap-1">
                  {videoGenerationInFlight ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Play size={12} />
                  )}
                  {videoGenerationInFlight ? "生成中" : "生成视频"}
                </span>
              </BookButton>
            )}
            <Dropdown
              items={[
                {
                  label:
                    acting === `optimize-prompt-${selectedPrompt.id}`
                      ? "正在优化提示词"
                      : "优化提示词",
                  icon:
                    acting === `optimize-prompt-${selectedPrompt.id}`
                      ? <Loader2 size={13} className="animate-spin" />
                      : <Wand2 size={13} />,
                  disabled:
                    acting === `optimize-prompt-${selectedPrompt.id}` ||
                    optimizeBlockedReason != null,
                  onClick: () => void onOptimizePrompt(selectedPrompt),
                },
                ...(!chatVideoMode
                  ? [
                      {
                        label: "查看模型请求",
                        icon: <Eye size={13} />,
                        disabled:
                          selectedPrompt.prompt_type !== "video_prompt" ||
                          invalidPromptReferenceIds.length > 0,
                        onClick: () => void onPreviewGenerateVideo(),
                      },
                      {
                        label:
                          acting === `import-video-${selectedPrompt.id}`
                            ? "正在导入视频"
                            : "导入外部视频",
                        icon:
                          acting === `import-video-${selectedPrompt.id}`
                            ? <Loader2 size={13} className="animate-spin" />
                            : <Upload size={13} />,
                        disabled:
                          acting === `import-video-${selectedPrompt.id}` ||
                          selectedImportBlockedReason != null,
                        onClick: () => void onImportVideo(),
                      },
                    ]
                  : []),
                ...(deletePromptBlockedReason == null
                  ? [
                      { type: "divider" as const },
                      {
                        label: "删除视频提示词",
                        icon: <Trash2 size={13} />,
                        danger: true,
                        disabled: acting === `delete-prompt-${selectedPrompt.id}`,
                        onClick: () => void onDeletePrompt(selectedPrompt),
                      },
                    ]
                  : []),
              ]}
            />
          </div>
        </div>
        {selectedPromptStale && (
          <div className="mb-2 text-[11px] text-error">
            {staleVideoPromptMessage(frameStageLabel)}。
          </div>
        )}
        {frameInputMissingMessage && (
          <div className="mb-2 text-[11px] text-warning">
            {frameInputMissingMessage}
          </div>
        )}
        {selectedPromptLockedReason && (
          <div className="mb-2 text-[11px] text-warning">
            {selectedPromptLockedReason}
          </div>
        )}
        {selectedPromptDownstreamNotice && (
          <div className="mb-2 text-[11px] text-warning">
            {selectedPromptDownstreamNotice}
          </div>
        )}
        {durationConfigError && (
          <div className="mb-2 text-[11px] text-error">
            {durationConfigError}
          </div>
        )}
        {selectedPromptDirty && (
          <div className="mb-2 text-[11px] text-warning">
            {savedReferenceIdsError
              ? "当前保存的参考图数据异常；模型请求预览会使用草稿绑定，生成视频前需要先保存当前绑定。"
              : "当前编辑内容未保存，模型请求预览会使用草稿覆盖，生成视频前需要先保存。"}
          </div>
        )}

        {selectedPromptRunning ? (
          <div className="a4n-flat-surface flex min-h-40 flex-col items-center justify-center gap-2 px-4 py-8 text-center">
            <Loader2 size={18} className="animate-spin text-book-primary" />
            <div className="text-sm font-medium text-book-text-main">
              当前提示词正在运行
            </div>
            <div className="max-w-md text-[11px] leading-relaxed text-book-text-muted">
              运行结束后再继续编辑。生成前设置和辅助操作已暂时收起，避免误改当前任务输入。
            </div>
          </div>
        ) : (
          <>
        {chatVideoMode ? null : firstFrameChainMode ? (
          <div className="a4n-flat-surface mb-3 p-3">
            <div className="mb-2 flex items-center gap-2">
              <div className="inline-flex items-center gap-1.5 text-xs text-book-text-main">
                <ImageIcon size={12} className="text-book-primary" />
                起始帧
                <span className="text-[10px] text-book-text-muted">
                  · {actualStartFrameImage ? "已生成" : "等待生成"}
                </span>
              </div>
            </div>
            {startFrameMissingMessage && (
              <div className="mb-2 text-[10px] text-warning">
                {startFrameMissingMessage}
              </div>
            )}
            <div className="grid gap-2 md:grid-cols-2">
              {selectedPrompt.shot_number <= 1
                ? renderBoundaryFrameSlot(
                    "首段专用首帧",
                    startFramePrompt,
                    actualStartFrameImage,
                    "等待首帧生成",
                  )
                : renderBoundaryFrameSlot(
                    "上一段尾帧",
                    previousVideoPrompt,
                    actualStartFrameImage,
                    `等待第 ${selectedPrompt.shot_number - 1} 个视频单元尾帧采集`,
                  )}
            </div>
          </div>
        ) : storyboardMode ? (
          <div className="a4n-flat-surface mb-3 p-3">
            <div className="mb-2 flex min-w-0 items-center gap-2">
              <ImageIcon size={12} className="shrink-0 text-book-primary" />
              <div className="min-w-0 flex-1">
                <div className="text-xs font-medium text-book-text-main">分镜板</div>
                <div className="mt-0.5 text-[10px] text-book-text-muted">
                  {generatedStartFrameImage ? "已生成" : "等待生成"}
                </div>
              </div>
              <button
                type="button"
                onClick={onOpenFrames}
                className="pyqt-btn-secondary !min-h-9 !px-2.5 text-[10px] sm:!min-h-7"
              >
                前往生成
              </button>
            </div>
            {frameInputMissingMessage && (
              <div className="mb-2 text-[10px] text-warning">
                {frameInputMissingMessage}
              </div>
            )}
            <div className="grid gap-2 md:grid-cols-2">
              {renderBoundaryFrameSlot(
                "分镜板",
                startFramePrompt,
                generatedStartFrameImage,
                "等待分镜板生成",
              )}
            </div>
          </div>
        ) : (
          <div className="a4n-flat-surface mb-3 p-3">
            <div className="mb-2 flex min-w-0 items-center gap-2">
              <PanelTop size={12} className="shrink-0 text-book-primary" />
              <div className="min-w-0 flex-1">
                <div className="text-xs font-medium text-book-text-main">首尾帧</div>
                <div className="mt-0.5 text-[10px] text-book-text-muted">
                  {Number(Boolean(generatedStartFrameImage)) + Number(Boolean(endFrameImage))} / 2 已生成
                </div>
              </div>
              <button
                type="button"
                onClick={onOpenFrames}
                className="pyqt-btn-secondary !min-h-9 !px-2.5 text-[10px] sm:!min-h-7"
              >
                前往生成
              </button>
            </div>
            <div className="grid grid-cols-2 gap-2">
              {renderBoundaryFrameSlot(
                "首帧",
                startFramePrompt,
                generatedStartFrameImage,
              )}
              {renderBoundaryFrameSlot("尾帧", endFramePrompt, endFrameImage)}
            </div>
          </div>
        )}

        <div className="a4n-flat-surface mb-3 p-3">
          <div className="mb-2 flex min-w-0 items-center gap-2">
            <Images size={12} className="shrink-0 text-book-primary" />
            <div className="min-w-0 flex-1">
              <div className="text-xs font-medium text-book-text-main">
                {chatVideoMode ? "建议上传图片" : "参考图"}
              </div>
              <div className="mt-0.5 text-[10px] text-book-text-muted">
                {boundReferenceImages.length > 0
                  ? `${boundReferenceImages.length} 张已绑定`
                  : "尚未绑定图片"}
              </div>
            </div>
          </div>
          {staleSavedReferenceCount > 0 && (
            <div className="mb-2 flex flex-wrap items-center gap-2 text-[10px] text-error">
              <span>
                已保存参考图里有 {staleSavedReferenceCount} 张失效图片
                。
              </span>
              {invalidPromptReferenceIds.length > 0 ? (
                <button
                  type="button"
                  onClick={onClearInvalidPromptReferences}
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[22px]"
                >
                  从草稿清除
                </button>
              ) : (
                <span className="text-book-text-muted">
                  已从草稿移除，保存后生效。
                </span>
              )}
            </div>
          )}
          {savedReferenceIdsError && (
            <div className="mb-2 text-[10px] text-error">
              已保存参考图数据异常：{savedReferenceIdsError}
              。请保存当前绑定后再生成。
            </div>
          )}
          {renderBoundReferenceImages()}
        </div>

        <div
          className={`mb-3 grid gap-2 text-xs ${
            selectedPromptSceneSummary
              ? "md:grid-cols-[minmax(0,1fr)_120px]"
              : "md:grid-cols-[120px]"
          }`}
        >
          {selectedPromptSceneSummary && (
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-book-text-muted">场景摘要</span>
              <div className="min-h-[34px] border-l-2 border-book-border/55 py-1 pl-2.5 leading-relaxed text-book-text-sub">
                {selectedPromptSceneSummary}
              </div>
            </label>
          )}
          <label className="flex min-w-0 flex-col gap-1">
            <span className="text-book-text-muted">时长</span>
            <DraftDurationControl
              value={draft.duration_secs}
              durationLimits={durationLimits}
              disabled={
                selectedPromptRunning ||
                selectedPromptHasVideo ||
                durationConfigError != null
              }
              onChange={(durationSecs) =>
                updateDraft({ duration_secs: durationSecs })
              }
            />
          </label>
        </div>

        {sections.length > 0 && (
          <details className="a4n-flat-surface mb-3 p-2">
            <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-2 text-xs text-book-text-main [&::-webkit-details-marker]:hidden">
              <span className="inline-flex min-w-0 items-center gap-1.5">
                <ListChecks size={12} className="text-book-primary" />
                导演约束
              </span>
              <span className="text-[10px] text-book-text-muted">
                {sections.length} 项
              </span>
            </summary>
            <dl className="mt-2 divide-y divide-book-border/40 border-y border-book-border/40">
              {sections.map((section) => (
                <div
                  key={section.key}
                  className="grid min-w-0 gap-1 px-2.5 py-2 md:grid-cols-[110px_minmax(0,1fr)] md:gap-3"
                >
                  <dt className="text-[10px] text-book-text-muted">
                    {section.label}
                  </dt>
                  <dd className="line-clamp-4 whitespace-pre-wrap break-words text-[11px] leading-relaxed text-book-text-sub">
                    {section.text}
                  </dd>
                </div>
              ))}
            </dl>
          </details>
        )}

        {selectedPromptCharStats && (
          <details className="mb-3 text-[10px] text-book-text-muted">
            <summary className="w-fit cursor-pointer hover:text-book-text-main">诊断信息</summary>
            <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 border-l-2 border-book-border/60 pl-2 text-[10px]">
              <span className="text-book-text-muted">
                {chatVideoMode ? "Chat 提示词" : "画面"}{" "}
                {promptLengthText(
                  selectedPromptCharStats.visualChars,
                  selectedPromptCharStats.visualBytes,
                )}
              </span>
              {!chatVideoMode && (
                <span className="text-book-text-muted">
                  运动{" "}
                  {promptLengthText(
                    selectedPromptCharStats.motionChars,
                    selectedPromptCharStats.motionBytes,
                  )}
                </span>
              )}
              <span className="text-book-text-muted">
                限制{" "}
                {promptLengthText(
                  selectedPromptCharStats.negativeChars,
                  selectedPromptCharStats.negativeBytes,
                )}
              </span>
            </div>
          </details>
        )}

        <div className="grid gap-3">
          <label className="flex min-w-0 flex-col gap-1">
            <span className="text-xs text-book-text-muted">
              {chatVideoMode ? "Chat 视频提示词" : "画面提示词"}
            </span>
            <textarea
              value={draft.visual_prompt}
              onChange={(event) =>
                updateDraft({ visual_prompt: event.target.value })
              }
              disabled={selectedPromptRunning || selectedPromptHasVideo}
              className="book-control min-h-[220px] resize-y p-2 text-xs leading-relaxed"
            />
          </label>
          {!chatVideoMode && (
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-xs text-book-text-muted">
                运动提示词
              </span>
              <textarea
                value={draft.motion_prompt}
                onChange={(event) =>
                  updateDraft({ motion_prompt: event.target.value })
                }
                disabled={selectedPromptRunning || selectedPromptHasVideo}
                className="book-control min-h-[140px] resize-y p-2 text-xs leading-relaxed"
              />
            </label>
          )}
          <label className="flex min-w-0 flex-col gap-1">
            <span className="text-xs text-book-text-muted">
              限制与避免项
            </span>
            <textarea
              value={draft.negative_prompt}
              onChange={(event) =>
                updateDraft({ negative_prompt: event.target.value })
              }
              disabled={selectedPromptRunning || selectedPromptHasVideo}
              className="book-control min-h-[96px] resize-y p-2 text-xs leading-relaxed"
            />
          </label>
        </div>
          </>
        )}
      </div>
    );
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="shrink-0 xl:hidden">
        <SegmentedControl
          ariaLabel="视频生成移动工作区"
          value={mobilePane}
          onChange={setMobilePane}
          options={[
            { id: "setup", label: "配置" },
            {
              id: "results",
              label: `成片${videos.length > 0 ? ` ${videos.length}` : ""}`,
            },
          ]}
        />
      </div>
      <div className="grid min-h-0 flex-1 gap-3 xl:grid-cols-[300px_minmax(0,1fr)]">
        <div
          className={`min-h-0 ${mobilePane === "setup" ? "block" : "hidden xl:block"}`}
        >
          <VideoPromptSetupPanel
            chapter={chapter}
            prompts={prompts}
            generationPrompts={generationPrompts}
            selectedPrompt={selectedPrompt}
            selectedPromptId={selectedPromptId}
            workflowMode={workflowMode}
            providerBackend={providerBackend}
            providerOptionsJson={providerOptionsJson}
            pendingVideo={pendingVideo}
            framePromptCount={framePrompts.length}
            expectedBoundaryFrameCount={expectedBoundaryFrameCount}
            expectedVideoPromptCount={expectedVideoPromptCount}
            totalVideoChainPromptCount={totalVideoChainPromptCount}
            options={options}
            durationLimits={durationLimits}
            promptInputSelectionError={promptInputSelectionError}
            loading={loading}
            acting={acting}
            chapterGenerating={chapterGenerating}
            chapterContentReady={chapterContentReady}
            previewingPromptRequests={previewingPromptRequests}
            selectedPromptDirty={selectedPromptDirty}
            systemReferenceIds={systemReferenceIds}
            validPromptReferenceIds={validPromptReferenceIds}
            onLoad={onLoad}
            onOptionsChange={onOptionsChange}
            onGeneratePrompts={onGeneratePromptRequests}
            onPreviewPromptRequests={onPreviewPromptRequests}
            onSelectPrompt={selectPrompt}
            onDeletePrompt={onDeletePrompt}
            onDeleteAllVideoPrompts={onDeleteAllVideoPrompts}
            onPollPendingVideo={onPollPendingVideo}
            onGenerateAllVideos={onGenerateAllVideos}
          />
        </div>

        <section
          aria-label="视频结果"
          className={`min-h-0 flex-col ${mobilePane === "results" ? "flex" : "hidden xl:flex"}`}
        >
          <VideoResultsPane
            videos={videos}
            selectedVideo={selectedVideo}
            selectedVideoId={selectedVideoId}
            totalVideoAssetCount={totalVideoAssetCount}
            selectedVideoPath={selectedVideoPath}
            trimStart={trimStart}
            trimEnd={trimEnd}
            acting={acting}
            onSelectVideo={selectVideo}
            onPollVideo={onPollVideo}
            onPreviewVideoRequest={onPreviewVideoRequest}
            onToggleVideoFavorite={onToggleVideoFavorite}
            onUpdateVideo={onUpdateVideo}
            onDeleteVideo={onDeleteVideo}
            onDeleteAllVideoAssets={onDeleteAllVideoAssets}
            onTrimStartChange={onTrimStartChange}
            onTrimEndChange={onTrimEndChange}
            onTrim={onTrim}
            onOpenSetup={() => setMobilePane("setup")}
          />
        </section>
      </div>

      <Modal
        isOpen={promptDetailOpen && selectedPrompt != null}
        onClose={() => setPromptDetailOpen(false)}
        title={
          selectedPrompt
            ? `镜头 ${selectedPrompt.shot_number} · 视频提示词`
            : "视频提示词"
        }
        subtitle={selectedPrompt ? promptTitle(selectedPrompt) : undefined}
        icon={<Wand2 size={15} />}
        maxWidthClassName="max-w-6xl"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4"
      >
        {renderEditor()}
      </Modal>
    </div>
  );
};
