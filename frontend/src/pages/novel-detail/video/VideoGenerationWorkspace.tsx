import React, { useEffect, useMemo, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
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
  X,
} from "lucide-react";
import type { GeneratedImage } from "../../../api/image";
import type {
  ChapterVideoPrompt,
  GeneratedVideo,
  UpdateVideoInput,
  SegmentGuidanceMode,
  VideoGenConfig,
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
  readSavedReferenceImageIds,
  staleVideoPromptMessage,
  videoPromptUnitFrameNumbers,
  videoFrameImageRoleLabel,
  videoFrameImageRoleOfImage,
  videoImageGenerationPromptId,
  videoTailFrameBoundaryNumber,
  videoWorkflowModeOfPrompt,
  isVideoPromptFrameImage,
  type ChapterVideoWorkflowMode,
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
  workflowMode: ChapterVideoWorkflowMode;
  pendingVideo: GeneratedVideo | null;
  selectedPromptStale: boolean;
  expectedBoundaryFrameCount: number | null;
  expectedVideoPromptCount: number | null;
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  providerBackend: string | null;
  providerOptionsJson: unknown;
  providerCapabilities?: VideoGenConfig["capabilities"];
  selectableReferenceImages: GeneratedImage[];
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
    targetShotNumber?: number;
  }) => void | Promise<void>;
  onPreviewPromptRequests: () => void | Promise<void>;
  onSelectPrompt: (id: number) => void;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onDeleteAllVideoPrompts: () => void | Promise<void>;
  onPollPendingVideo: (video: GeneratedVideo) => void | Promise<void>;
  onWorkflowModeChange: (mode: ChapterVideoWorkflowMode, shotNumber?: number) => void;
  onUpdateGuidance: (
    prompt: ChapterVideoPrompt,
    guidanceMode: SegmentGuidanceMode,
    referenceImageIds: number[],
    continuitySourceVideoId: number | null,
  ) => void | Promise<void>;
  onGenerateFramePrompts: () => void | Promise<void>;
  onPreviewFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onGenerateFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onImportFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
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
  onAdoptVideo: (video: GeneratedVideo) => void | Promise<void>;
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
  workspaceOverride?: React.ReactNode;
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
  providerCapabilities,
  selectableReferenceImages,
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
  onWorkflowModeChange,
  onUpdateGuidance,
  onGenerateFramePrompts,
  onPreviewFrameImage,
  onGenerateFrameImage,
  onImportFrameImage,
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
  onAdoptVideo,
  onPollVideo,
  onToggleVideoFavorite,
  onUpdateVideo,
  onDeleteVideo,
  onDeleteAllVideoAssets,
  onTrimStartChange,
  onTrimEndChange,
  onTrim,
  workspaceOverride,
}) => {
  const [promptDetailOpen, setPromptDetailOpen] = useState(false);
  const [mobilePane, setMobilePane] = useState<"setup" | "results">("setup");
  const [guidanceMode, setGuidanceMode] = useState<SegmentGuidanceMode>("system_frames");
  const [keyframeImageIds, setKeyframeImageIds] = useState<number[]>([]);
  const [continuitySourceVideoId, setContinuitySourceVideoId] = useState<number | null>(null);

  useEffect(() => {
    const metadata =
      selectedPrompt?.metadata_json &&
      typeof selectedPrompt.metadata_json === "object" &&
      !Array.isArray(selectedPrompt.metadata_json)
        ? (selectedPrompt.metadata_json as Record<string, unknown>)
        : null;
    const storedMode = metadata?.guidance_mode;
    setGuidanceMode(
      storedMode === "keyframes" || storedMode === "none" || storedMode === "continuity"
        ? storedMode
        : "system_frames",
    );
    setKeyframeImageIds(readSavedReferenceImageIds(selectedPrompt?.reference_image_ids_json).ids);
    const continuity = metadata?.continuity_source;
    setContinuitySourceVideoId(
      continuity && typeof continuity === "object" && !Array.isArray(continuity)
        ? (typeof (continuity as Record<string, unknown>).video_id === "number"
            ? ((continuity as Record<string, unknown>).video_id as number)
            : null)
        : null,
    );
  }, [selectedPrompt?.id, selectedPrompt?.metadata_json, selectedPrompt?.reference_image_ids_json]);

  const previousCompletedVideos = useMemo(() => {
    if (!selectedPrompt) return [];
    return videos.filter((video) => {
      if (video.status !== "completed" || !video.file_path) return false;
      const metadata =
        video.metadata_json && typeof video.metadata_json === "object" && !Array.isArray(video.metadata_json)
          ? (video.metadata_json as Record<string, unknown>)
          : null;
      return typeof metadata?.shot_number === "number" && metadata.shot_number < selectedPrompt.shot_number;
    });
  }, [selectedPrompt, videos]);

  const segmentVideos = useMemo(() => {
    if (!selectedPrompt) return videos;
    return videos.filter((video) => {
      const metadata =
        video.metadata_json && typeof video.metadata_json === "object" && !Array.isArray(video.metadata_json)
          ? (video.metadata_json as Record<string, unknown>)
          : null;
      return metadata?.shot_number === selectedPrompt.shot_number;
    });
  }, [selectedPrompt, videos]);

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
    const prompt = prompts.find((row) => row.id === id);
    const promptMode = prompt ? videoWorkflowModeOfPrompt(prompt) : null;
    if (prompt && promptMode && promptMode !== workflowMode) {
      onWorkflowModeChange(promptMode, prompt.shot_number);
    } else {
      onSelectPrompt(id);
    }
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
        {prompt ? (
          <div className="mt-1.5 flex items-center gap-1">
            <button
              type="button"
              onClick={() => void onPreviewFrameImage(prompt)}
              className="pyqt-btn-icon !h-7 !w-7"
              title={`预览${label}生成请求`}
            >
              <Eye size={11} />
            </button>
            <button
              type="button"
              onClick={() => void onGenerateFrameImage(prompt)}
              className="pyqt-btn-icon !h-7 !w-7"
              title={image ? `重新生成${label}` : `生成${label}`}
            >
              <Wand2 size={11} />
            </button>
            <button
              type="button"
              onClick={() => void onImportFrameImage(prompt)}
              className="pyqt-btn-icon !h-7 !w-7"
              title={`导入${label}`}
            >
              <Upload size={11} />
            </button>
          </div>
        ) : null}
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
    const keyframeRangeValid =
      providerCapabilities?.supports_keyframes === true &&
      Number.isInteger(providerCapabilities.keyframe_min) &&
      Number.isInteger(providerCapabilities.keyframe_max) &&
      providerCapabilities.keyframe_min > 0 &&
      providerCapabilities.keyframe_max >= providerCapabilities.keyframe_min;
    const keyframeMin = keyframeRangeValid ? providerCapabilities.keyframe_min : 0;
    const keyframeMax = keyframeRangeValid ? providerCapabilities.keyframe_max : 0;
    const systemFramesSupported = storyboardMode
      ? providerCapabilities?.supports_storyboard_reference === true
      : firstFrameChainMode
        ? providerCapabilities?.supports_first_frame === true
        : providerCapabilities?.supports_first_frame === true &&
          providerCapabilities.supports_last_frame === true;
    const guidanceCapabilityError =
      providerCapabilities == null
        ? "当前视频服务未返回媒体能力合同"
        : guidanceMode === "system_frames" && !systemFramesSupported
          ? "当前视频服务不支持此系统帧输入"
          : guidanceMode === "continuity" && providerCapabilities.supports_first_frame !== true
            ? "当前视频服务不支持首帧续接输入"
            : guidanceMode === "keyframes" && !keyframeRangeValid
              ? "当前视频服务不支持有效的关键帧范围"
              : guidanceMode === "none" && providerCapabilities.supports_text_only !== true
                ? "当前视频服务不支持纯文本生成"
                : null;
    const guidanceSaveBlockedReason =
      guidanceCapabilityError ??
      (guidanceMode === "keyframes" &&
      (keyframeImageIds.length < keyframeMin || keyframeImageIds.length > keyframeMax)
        ? `关键帧数量必须为 ${keyframeMin}-${keyframeMax} 张`
        : null) ??
      (guidanceMode === "continuity" && continuitySourceVideoId == null
        ? "请选择续接来源视频"
        : null);
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
      guidanceCapabilityError ??
      ((guidanceMode === "system_frames" || guidanceMode === "continuity")
        ? frameInputMissingMessage
        : undefined) ??
      (selectedPromptRunning ? "当前视频提示词正在运行" : undefined) ??
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
        {!chatVideoMode && (
          <div className="a4n-flat-surface mb-3 p-3">
            <div className="mb-2 text-xs font-medium text-book-text-main">媒体引导</div>
            <SegmentedControl
              ariaLabel="片段媒体引导"
              value={guidanceMode}
              onChange={(value) => setGuidanceMode(value as SegmentGuidanceMode)}
              options={[
                {
                  id: "system_frames",
                  label: storyboardMode ? "分镜板" : firstFrameChainMode ? "系统首帧" : "系统首尾帧",
                  disabled: !systemFramesSupported,
                  title: "当前视频模型不支持该系统帧输入合同",
                },
                ...(firstFrameChainMode && selectedPrompt.shot_number > 1
                  ? [{
                      id: "continuity" as const,
                      label: "指定续接",
                      disabled: providerCapabilities?.supports_first_frame !== true,
                      title: "当前视频模型不支持首帧输入",
                    }]
                  : []),
                {
                  id: "keyframes",
                  label: keyframeRangeValid ? `关键帧 ${keyframeMin}-${keyframeMax}` : "关键帧",
                  disabled: !keyframeRangeValid,
                  title: "当前视频模型不支持多关键帧输入",
                },
                {
                  id: "none",
                  label: "纯文本",
                  disabled: providerCapabilities?.supports_text_only !== true,
                  title: "当前视频模型要求图像输入",
                },
              ]}
            />

            {guidanceMode === "keyframes" && (
              <div className="mt-3">
                <div className="mb-2 flex items-center justify-between gap-2 text-[10px] text-book-text-muted">
                  <span>按此顺序发送给视频 provider</span>
                  <span>{keyframeImageIds.length} / {keyframeMax || "-"}</span>
                </div>
                <div className="space-y-1.5">
                  {keyframeImageIds.map((imageId, index) => {
                    const image = selectableReferenceImages.find((item) => item.id === imageId);
                    return (
                      <div key={imageId} className="flex min-w-0 items-center gap-2 border-b border-book-border/35 pb-1.5 text-[11px]">
                        <span className="w-5 shrink-0 text-book-text-muted">{index + 1}</span>
                        <span className="min-w-0 flex-1 truncate text-book-text-main">
                          {image?.prompt || `图片 #${imageId}`}
                        </span>
                        <button type="button" className="pyqt-btn-icon !h-7 !w-7" title="上移"
                          disabled={index === 0}
                          onClick={() => setKeyframeImageIds((ids) => ids.map((id, current) => current === index - 1 ? imageId : current === index ? ids[index - 1] : id))}>
                          <ArrowUp size={11} />
                        </button>
                        <button type="button" className="pyqt-btn-icon !h-7 !w-7" title="下移"
                          disabled={index === keyframeImageIds.length - 1}
                          onClick={() => setKeyframeImageIds((ids) => ids.map((id, current) => current === index + 1 ? imageId : current === index ? ids[index + 1] : id))}>
                          <ArrowDown size={11} />
                        </button>
                        <button type="button" className="pyqt-btn-icon !h-7 !w-7" title="移除"
                          onClick={() => setKeyframeImageIds((ids) => ids.filter((id) => id !== imageId))}>
                          <X size={11} />
                        </button>
                      </div>
                    );
                  })}
                </div>
                {keyframeRangeValid && keyframeImageIds.length < keyframeMax && (
                  <select
                    value=""
                    onChange={(event) => {
                      const id = Number(event.target.value);
                      if (id > 0) setKeyframeImageIds((ids) => [...ids, id]);
                    }}
                    className="pyqt-input mt-2 w-full !px-2 !py-1.5 text-xs"
                  >
                    <option value="">添加项目参考图...</option>
                    {selectableReferenceImages
                      .filter((image) => !keyframeImageIds.includes(image.id))
                      .map((image) => <option key={image.id} value={image.id}>{image.prompt || `图片 #${image.id}`}</option>)}
                  </select>
                )}
                {keyframeRangeValid && keyframeImageIds.length < keyframeMin && (
                  <div className="mt-2 text-[10px] text-warning">至少选择 {keyframeMin} 张关键帧。</div>
                )}
              </div>
            )}

            {guidanceMode === "continuity" && (
              <label className="mt-3 flex flex-col gap-1 text-xs">
                <span className="text-book-text-muted">续接来源视频</span>
                <select
                  value={continuitySourceVideoId ?? ""}
                  onChange={(event) => setContinuitySourceVideoId(Number(event.target.value) || null)}
                  className="pyqt-input !px-2 !py-1.5"
                >
                  <option value="">选择更早片段的已完成视频</option>
                  {previousCompletedVideos.map((video) => {
                    const metadata = video.metadata_json as Record<string, unknown> | null;
                    return <option key={video.id} value={video.id}>片段 {String(metadata?.shot_number ?? "?")} · 视频 #{video.id}</option>;
                  })}
                </select>
              </label>
            )}

            {guidanceMode === "none" && (
              <div className="mt-2 text-[10px] text-book-text-muted">生成时不发送系统帧或参考图；provider 不支持纯文本时服务端会明确拒绝。</div>
            )}

            <div className="mt-3 flex justify-end">
              <BookButton
                variant="secondary"
                size="sm"
                disabled={
                  acting === `guidance-${selectedPrompt.id}` ||
                  guidanceSaveBlockedReason != null
                }
                title={guidanceSaveBlockedReason ?? undefined}
                onClick={() => void onUpdateGuidance(
                  selectedPrompt,
                  guidanceMode,
                  guidanceMode === "none" ? [] : keyframeImageIds,
                  guidanceMode === "continuity" ? continuitySourceVideoId : null,
                )}
              >
                {acting === `guidance-${selectedPrompt.id}` ? "保存中" : "保存媒体引导"}
              </BookButton>
            </div>
          </div>
        )}
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
        {chatVideoMode || (guidanceMode !== "system_frames" && guidanceMode !== "continuity") ? null : firstFrameChainMode ? (
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
                onClick={() =>
                  startFramePrompt
                    ? void onGenerateFrameImage(startFramePrompt)
                    : void onGenerateFramePrompts()
                }
                className="pyqt-btn-secondary !min-h-9 !px-2.5 text-[10px] sm:!min-h-7"
              >
                {startFramePrompt ? "生成分镜板" : "准备分镜提示词"}
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
                onClick={() =>
                  endFramePrompt
                    ? void onGenerateFrameImage(endFramePrompt)
                    : void onGenerateFramePrompts()
                }
                className="pyqt-btn-secondary !min-h-9 !px-2.5 text-[10px] sm:!min-h-7"
              >
                {startFramePrompt && endFramePrompt ? "生成缺失帧" : "准备首尾帧提示词"}
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
              label: workspaceOverride
                ? "参考媒体"
                : `候选${segmentVideos.length > 0 ? ` ${segmentVideos.length}` : ""}`,
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
            onWorkflowModeChange={onWorkflowModeChange}
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
          {workspaceOverride ?? (
            <VideoResultsPane
              videos={segmentVideos}
              selectedVideo={selectedVideo}
              selectedVideoId={selectedVideoId}
              totalVideoAssetCount={totalVideoAssetCount}
              selectedVideoPath={selectedVideoPath}
              trimStart={trimStart}
              trimEnd={trimEnd}
              acting={acting}
              onSelectVideo={selectVideo}
              onAdoptVideo={onAdoptVideo}
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
          )}
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
