import React, { useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  CheckCircle2,
  Eye,
  Image as ImageIcon,
  ListChecks,
  Copy,
  PanelTop,
  Play,
  Plus,
  Save,
  SlidersHorizontal,
  Trash2,
  Upload,
  Wand2,
} from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { GeneratedImage } from "../../../api/image";
import {
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type UpdateVideoInput,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentWorkflowMode,
  type VideoTimelineSummary,
} from "../../../api/video";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { EmptyHint } from "../shared";
import {
  durationChoicesForLimits,
  durationLimitsLabel,
  isDurationAllowed,
  promptByteCount,
  promptCharCount,
  staleVideoPromptMessage,
  videoGuidanceConfigurationError,
  videoPromptSourceFrameNumbers,
  videoImageGenerationPromptId,
  videoTailFrameSourceVideoId,
  videoTailFrameBoundaryNumber,
  isVideoPromptFrameImage,
  VIDEO_FINAL_PROMPT_TYPE,
  VIDEO_WORKFLOW_MODE_LABELS,
  type PromptDraft,
  type VideoDurationLimits,
  type VideoSegmentCardPlan,
} from "./videoTabUtils";
import { VideoPromptSetupPanel } from "./VideoPromptSetupPanel";
import type { InsertedSegmentGap } from "./InsertedSegmentTimelinePanel";
import { VideoResultsPane } from "./VideoResultsPane";
import { directorSections } from "./videoGenerationDisplay";

interface VideoGenerationWorkspaceProps {
  prompts: ChapterVideoPrompt[];
  segmentCards: VideoSegmentCardPlan[];
  segmentRecipes: Map<number, VideoSegmentExecutionRecipe>;
  selectedSegmentNumber: number | null;
  planningDraftVisible: boolean;
  finalPromptBlockedReasons: Map<number, string>;
  timeline: VideoTimelineSummary | null;
  selectedPrompt: ChapterVideoPrompt | null;
  selectedPromptId: number | null;
  selectedPromptStale: boolean;
  durationLimits: VideoDurationLimits;
  loading: boolean;
  acting: string | null;
  chapterContentReady: boolean;
  savingRecipeSegments: Set<number>;
  draft: PromptDraft;
  selectedPromptRunning: boolean;
  selectedPromptDirty: boolean;
  staleSavedReferenceCount: number;
  savedReferenceIdsError: string | null;
  framePrompts: ChapterVideoPrompt[];
  frameImages: GeneratedImage[];
  imageBusyId: number | null;
  videos: GeneratedVideo[];
  previousAdoptedVideos: GeneratedVideo[];
  referenceInputs?: React.ReactNode;
  selectedVideo: GeneratedVideo | null;
  selectedVideoPath: string;
  trimStart: string;
  trimEnd: string;
  onLoad: () => void | Promise<void>;
  onGeneratePromptRequests: (segmentNumber: number) => void | Promise<void>;
  onSelectPrompt: (id: number) => void | Promise<void>;
  segmentConfigOpen: boolean;
  onActivateSegment: (segmentNumber: number) => void | Promise<void>;
  onCloseSegmentConfig: () => void | Promise<void>;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onInsertSegment: (gap: InsertedSegmentGap) => void;
  onOpenInsertedSegment: (timelineItemId: number) => void;
  onWorkflowModeChange: (
    mode: VideoSegmentWorkflowMode,
    shotNumber: number,
  ) => void | Promise<void>;
  onFirstFrameSourceChange: (
    segmentNumber: number,
    guidanceMode: Extract<
      VideoSegmentExecutionRecipe["guidance_mode"],
      "continuity" | "system_frames"
    >,
  ) => void | Promise<void>;
  onContinuitySourceChange: (
    segmentNumber: number,
    continuitySourceVideoId: number | null,
  ) => void | Promise<void>;
  onGenerateFramePrompts: (segmentNumber: number) => void | Promise<void>;
  onPreviewFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onGenerateFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onImportFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onDeleteFrameImage: (image: GeneratedImage) => void | Promise<void>;
  onDraftChange: (draft: PromptDraft) => void;
  onSavePrompt: () => void | Promise<void>;
  onSaveFramePrompt: (
    prompt: ChapterVideoPrompt,
    visualPrompt: string,
  ) => Promise<ChapterVideoPrompt | null>;
  onOptimizePrompt: (
    prompt: ChapterVideoPrompt,
  ) => Promise<ChapterVideoPrompt | null>;
  onPreviewGenerateVideo: () => void | Promise<void>;
  onCopyPrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onPreviewVideoRequest: (video: GeneratedVideo) => void | Promise<void>;
  onGenerateVideo: () => void | Promise<void>;
  onImportVideo: () => void | Promise<void>;
  onAdoptVideo: (video: GeneratedVideo) => void | Promise<void>;
  onPollVideo: (video: GeneratedVideo) => void | Promise<void>;
  onToggleVideoFavorite: (video: GeneratedVideo) => void | Promise<void>;
  onUpdateVideo: (
    video: GeneratedVideo,
    input: UpdateVideoInput,
  ) => void | Promise<void>;
  onDeleteVideo: (video: GeneratedVideo) => void | Promise<void>;
  onTrimStartChange: (value: string) => void;
  onTrimEndChange: (value: string) => void;
  onTrim: () => void | Promise<void>;
  referenceMediaWorkspace: React.ReactNode;
}

type SegmentInputDialog =
  | {
      kind: "frame";
      label: string;
      promptId: number | null;
      emptyText: string;
    }
  | { kind: "continuity" }
  | null;

const MEDIA_SLOT_GRID_CLASS =
  "grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5";

export const VideoGenerationWorkspace: React.FC<
  VideoGenerationWorkspaceProps
> = ({
  prompts,
  segmentCards,
  segmentRecipes,
  selectedSegmentNumber,
  planningDraftVisible,
  finalPromptBlockedReasons,
  timeline,
  selectedPrompt,
  selectedPromptId,
  selectedPromptStale,
  durationLimits,
  loading,
  acting,
  chapterContentReady,
  savingRecipeSegments,
  draft,
  selectedPromptRunning,
  selectedPromptDirty,
  staleSavedReferenceCount,
  savedReferenceIdsError,
  framePrompts,
  frameImages,
  imageBusyId,
  videos,
  previousAdoptedVideos,
  referenceInputs,
  selectedVideo,
  selectedVideoPath,
  trimStart,
  trimEnd,
  onLoad,
  onGeneratePromptRequests,
  onSelectPrompt,
  segmentConfigOpen,
  onActivateSegment,
  onCloseSegmentConfig,
  onDeletePrompt,
  onInsertSegment,
  onOpenInsertedSegment,
  onWorkflowModeChange,
  onFirstFrameSourceChange,
  onContinuitySourceChange,
  onGenerateFramePrompts,
  onPreviewFrameImage,
  onGenerateFrameImage,
  onImportFrameImage,
  onDeleteFrameImage,
  onDraftChange,
  onSavePrompt,
  onSaveFramePrompt,
  onOptimizePrompt,
  onPreviewGenerateVideo,
  onCopyPrompt,
  onPreviewVideoRequest,
  onGenerateVideo,
  onImportVideo,
  onAdoptVideo,
  onPollVideo,
  onToggleVideoFavorite,
  onUpdateVideo,
  onDeleteVideo,
  onTrimStartChange,
  onTrimEndChange,
  onTrim,
  referenceMediaWorkspace,
}) => {
  const [mobilePane, setMobilePane] = useState<"segments" | "results">(
    "segments",
  );
  const [inputDialog, setInputDialog] = useState<SegmentInputDialog>(null);
  const [framePromptDraft, setFramePromptDraft] = useState("");
  const [failedFrameMediaSrcs, setFailedFrameMediaSrcs] = useState<Set<string>>(
    () => new Set(),
  );
  const segmentRecipe =
    selectedSegmentNumber == null
      ? null
      : (segmentRecipes.get(selectedSegmentNumber) ?? null);
  const workflowMode = segmentRecipe?.workflow_mode ?? null;
  const finalPromptBlockedReason =
    selectedSegmentNumber == null
      ? null
      : (finalPromptBlockedReasons.get(selectedSegmentNumber) ?? null);
  const selectedPromptSceneSummary =
    selectedPrompt?.scene_summary?.trim() ?? "";

  useEffect(() => {
    if (!segmentConfigOpen) setInputDialog(null);
  }, [segmentConfigOpen]);

  const inputDialogPromptId =
    inputDialog?.kind === "frame" ? inputDialog.promptId : null;
  const inputDialogPrompt =
    inputDialogPromptId == null
      ? null
      : (framePrompts.find((prompt) => prompt.id === inputDialogPromptId) ??
        null);

  useEffect(() => {
    setFailedFrameMediaSrcs(new Set());
  }, [frameImages]);

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
    const boundaryByFrameNumber = new Map<number, GeneratedImage>();
    const tailByVideoId = new Map<number, GeneratedImage>();
    const sorted = [...frameImages].sort((a, b) => b.id - a.id);
    for (const image of sorted) {
      const promptId = videoImageGenerationPromptId(image);
      const promptFrameImage =
        promptId != null && isVideoPromptFrameImage(image);
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
      const sourceVideoId = videoTailFrameSourceVideoId(image);
      if (sourceVideoId != null && !tailByVideoId.has(sourceVideoId)) {
        tailByVideoId.set(sourceVideoId, image);
      }
    }
    return { boundaryByFrameNumber, tailByVideoId };
  }, [frameImages, framePrompts]);

  const selectedSegmentCard = segmentCards.find(
    (card) => card.segmentNumber === selectedSegmentNumber,
  );
  const selectedContinuitySourceVideoId =
    segmentRecipe?.continuity_source_video_id ?? null;
  const selectedContinuityTailFrame =
    selectedContinuitySourceVideoId == null
      ? null
      : (frameImageIndexes.tailByVideoId.get(selectedContinuitySourceVideoId) ??
        null);
  const selectedContinuityTailFrameSrc = selectedContinuityTailFrame
    ? toMediaSrc(selectedContinuityTailFrame.file_path)
    : "";
  const inputDialogImage =
    inputDialog?.kind === "frame" && inputDialogPrompt
      ? ([...frameImages]
          .sort((left, right) => right.id - left.id)
          .find(
            (image) =>
              isVideoPromptFrameImage(image) &&
              videoImageGenerationPromptId(image) === inputDialogPrompt.id,
          ) ?? null)
      : null;
  const inputDialogFrameSrc = inputDialogImage
    ? toMediaSrc(inputDialogImage.file_path)
    : "";
  const framePromptDirty = Boolean(
    inputDialogPrompt &&
    framePromptDraft.trim() !== inputDialogPrompt.visual_prompt.trim(),
  );
  const frameImageLocked = inputDialogImage != null;
  const selectedContinuitySourceAvailable = previousAdoptedVideos.some(
    (video) => video.id === selectedContinuitySourceVideoId,
  );

  const renderFrameSlot = (
    label: string,
    prompt: ChapterVideoPrompt | null,
    image: GeneratedImage | null,
    emptyText = "等待首尾帧生成",
  ) => {
    const src = image ? toMediaSrc(image.file_path) : "";
    const mediaReady = Boolean(src && !failedFrameMediaSrcs.has(src));
    const mediaUnavailable = image != null && !mediaReady;
    return (
      <button
        type="button"
        aria-label={`配置${label}`}
        onClick={() => {
          setFramePromptDraft(prompt?.visual_prompt ?? "");
          setInputDialog({
            kind: "frame",
            label,
            promptId: prompt?.id ?? null,
            emptyText,
          });
        }}
        className={`group min-w-0 overflow-hidden rounded-xl border text-left transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary ${
          mediaReady
            ? "border-success/35 bg-book-bg-paper/45 hover:border-book-primary/65"
            : mediaUnavailable
              ? "border-error/45 bg-error-bg/25 hover:border-book-primary/65"
              : "border-warning/45 bg-warning-bg/35 hover:border-book-primary/65"
        }`}
      >
        <span className="relative flex aspect-video items-center justify-center overflow-hidden bg-book-bg-tertiary text-small text-book-text-muted">
          {mediaReady ? (
            <img
              src={src}
              alt=""
              className="h-full w-full object-cover transition-transform group-hover:scale-[1.02]"
              onError={() =>
                setFailedFrameMediaSrcs((current) => new Set(current).add(src))
              }
            />
          ) : (
            <span className="flex flex-col items-center gap-1.5">
              {mediaUnavailable ? (
                <AlertCircle size={18} />
              ) : (
                <Plus size={18} />
              )}
              <span>
                {mediaUnavailable
                  ? "图片无法加载，点击处理"
                  : prompt
                    ? `生成或导入${label}`
                    : `准备${label}`}
              </span>
            </span>
          )}
        </span>
        <span className="block min-w-0 p-2">
          <span className="flex min-w-0 items-center gap-2">
            <span className="min-w-0 flex-1 truncate text-small font-medium text-book-text-main">
              {label}
            </span>
            <span
              className={`shrink-0 text-caption ${
                mediaReady
                  ? "text-success"
                  : mediaUnavailable
                    ? "text-error"
                    : "text-warning"
              }`}
            >
              {mediaReady ? "已填充" : mediaUnavailable ? "不可用" : "必填"}
            </span>
          </span>
          <span className="mt-0.5 block truncate text-caption text-book-text-muted">
            {prompt?.title || prompt?.scene_summary || emptyText}
          </span>
        </span>
      </button>
    );
  };

  const renderContinuitySlot = () => {
    const mediaReady = Boolean(
      selectedContinuityTailFrameSrc &&
      !failedFrameMediaSrcs.has(selectedContinuityTailFrameSrc),
    );
    const mediaUnavailable = selectedContinuityTailFrame != null && !mediaReady;
    return (
      <button
        type="button"
        aria-label="配置续接首帧"
        onClick={() => setInputDialog({ kind: "continuity" })}
        className={`group min-w-0 overflow-hidden rounded-xl border text-left transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary ${
          mediaReady
            ? "border-success/35 bg-book-bg-paper/45 hover:border-book-primary/65"
            : mediaUnavailable
              ? "border-error/45 bg-error-bg/25 hover:border-book-primary/65"
              : "border-warning/45 bg-warning-bg/35 hover:border-book-primary/65"
        }`}
      >
        <span className="flex aspect-video items-center justify-center overflow-hidden bg-book-bg-tertiary text-small text-book-text-muted">
          {mediaReady ? (
            <img
              src={selectedContinuityTailFrameSrc}
              alt=""
              className="h-full w-full object-cover transition-transform group-hover:scale-[1.02]"
              onError={() =>
                setFailedFrameMediaSrcs((current) =>
                  new Set(current).add(selectedContinuityTailFrameSrc),
                )
              }
            />
          ) : (
            <span className="flex flex-col items-center gap-1.5">
              {mediaUnavailable ? (
                <AlertCircle size={18} />
              ) : (
                <Plus size={18} />
              )}
              <span>
                {mediaUnavailable
                  ? "图片无法加载，点击处理"
                  : "选择更早片段的视频"}
              </span>
            </span>
          )}
        </span>
        <span className="block min-w-0 p-2">
          <span className="flex min-w-0 items-center gap-2">
            <span className="min-w-0 flex-1 truncate text-small font-medium text-book-text-main">
              续接首帧
            </span>
            <span
              className={`shrink-0 text-caption ${
                mediaReady
                  ? "text-success"
                  : mediaUnavailable
                    ? "text-error"
                    : "text-warning"
              }`}
            >
              {mediaReady ? "已填充" : mediaUnavailable ? "不可用" : "必填"}
            </span>
          </span>
          <span className="mt-0.5 block truncate text-caption text-book-text-muted">
            {selectedContinuitySourceVideoId == null
              ? "尚未选择来源视频"
              : `来自视频 #${selectedContinuitySourceVideoId} 的尾帧`}
          </span>
        </span>
      </button>
    );
  };

  const renderFirstFrameSourceSelector = () => {
    if (
      workflowMode !== VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN ||
      segmentRecipe == null ||
      selectedSegmentNumber == null ||
      selectedSegmentNumber <= 1
    ) {
      return null;
    }
    const value =
      segmentRecipe.guidance_mode === "system_frames"
        ? "system_frames"
        : "continuity";
    const disabled =
      acting != null || savingRecipeSegments.has(selectedSegmentNumber);
    return (
      <div className="mb-3 flex min-w-0 flex-wrap items-center justify-between gap-2 rounded-xl border border-book-border/45 bg-book-bg-paper/45 px-3 py-2.5">
        <div className="min-w-0">
          <div className="text-xs font-medium text-book-text-main">
            首帧来源
          </div>
          <div className="mt-0.5 text-caption text-book-text-muted">
            选择沿用更早视频尾帧，或为当前片段单独生成首帧。
          </div>
        </div>
        <SegmentedControl<"continuity" | "system_frames">
          ariaLabel="首帧来源"
          value={value}
          options={[
            { id: "continuity", label: "尾帧续接", disabled },
            { id: "system_frames", label: "独立生成", disabled },
          ]}
          onChange={(guidanceMode) =>
            void onFirstFrameSourceChange(selectedSegmentNumber, guidanceMode)
          }
          className={disabled ? "opacity-60" : ""}
        />
      </div>
    );
  };

  const frameInputStateForPrompt = (
    prompt: ChapterVideoPrompt,
    recipe: VideoSegmentExecutionRecipe,
  ) => {
    const firstFrameChainMode =
      workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN;
    const chatVideoMode = workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO;
    const storyboardMode = workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD;
    const usesContinuity =
      firstFrameChainMode && recipe.guidance_mode === "continuity";
    const { startFrameNumber, endFrameNumber } = videoPromptSourceFrameNumbers(
      prompt,
      workflowMode,
      recipe.guidance_mode,
    );
    const startFramePrompt = usesContinuity
      ? null
      : (framePromptByNumber.get(startFrameNumber ?? prompt.shot_number) ??
        null);
    const endFramePrompt =
      firstFrameChainMode || storyboardMode
        ? null
        : endFrameNumber == null
          ? null
          : (framePromptByNumber.get(endFrameNumber) ?? null);
    const generatedStartFrameImage =
      startFramePrompt == null
        ? null
        : (frameImageIndexes.boundaryByFrameNumber.get(
            startFramePrompt.shot_number,
          ) ?? null);
    const continuitySourceVideoId = usesContinuity
      ? recipe.continuity_source_video_id
      : null;
    const continuityTailFrameImage =
      continuitySourceVideoId == null
        ? null
        : (frameImageIndexes.tailByVideoId.get(continuitySourceVideoId) ??
          null);
    const actualStartFrameImage = usesContinuity
      ? continuityTailFrameImage
      : generatedStartFrameImage;
    const endFrameImage =
      endFramePrompt == null
        ? null
        : (frameImageIndexes.boundaryByFrameNumber.get(
            endFramePrompt.shot_number,
          ) ?? null);
    const startFrameMissingMessage =
      firstFrameChainMode && !actualStartFrameImage
        ? usesContinuity
          ? continuitySourceVideoId == null
            ? "请先点击续接首帧槽位，选择一个更早片段已采用的视频。"
            : `续接来源视频 #${continuitySourceVideoId} 尚无可用尾帧，请等待尾帧采集完成或重新选择来源。`
          : "请先点击首帧槽位生成当前片段首帧图片。"
        : null;
    const boundaryFrameMissingMessage =
      workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
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
        ? `请先点击分镜板槽位生成${storyboardFrameMissingMessage}。`
        : startFrameMissingMessage ||
          (boundaryFrameMissingMessage
            ? `请先点击首帧或尾帧槽位生成${boundaryFrameMissingMessage}。`
            : null);

    return {
      startFramePrompt,
      endFramePrompt,
      generatedStartFrameImage,
      actualStartFrameImage,
      endFrameImage,
      startFrameMissingMessage,
      frameInputMissingMessage,
      usesContinuity,
    };
  };

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
    if (selectedSegmentNumber == null) {
      return (
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <EmptyHint text="请先选择片段。" />
        </div>
      );
    }
    if (!selectedSegmentCard) {
      return (
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <EmptyHint text="当前片段不在最新片段规划中，请刷新后重新选择。" />
        </div>
      );
    }
    if (workflowMode == null || segmentRecipe == null) {
      return (
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <EmptyHint
            text={
              planningDraftVisible
                ? "当前片段已生成，片段规划完成落库后即可选择执行方案。"
                : "当前片段缺少有效执行方案，请刷新或重新生成片段规划。"
            }
          />
        </div>
      );
    }
    if (!selectedPrompt) {
      const mode = segmentRecipe.workflow_mode;
      const startFramePrompt =
        framePromptByNumber.get(
          mode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES ||
            mode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
            ? 1
            : selectedSegmentNumber,
        ) ?? null;
      const endFramePrompt =
        mode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
          ? (framePromptByNumber.get(2) ?? null)
          : null;
      const startFrameImage = startFramePrompt
        ? (frameImageIndexes.boundaryByFrameNumber.get(
            startFramePrompt.shot_number,
          ) ?? null)
        : null;
      const endFrameImage = endFramePrompt
        ? (frameImageIndexes.boundaryByFrameNumber.get(
            endFramePrompt.shot_number,
          ) ?? null)
        : null;
      const ready = finalPromptBlockedReason == null;

      return (
        <div className="min-h-0 flex-1 overflow-y-auto overflow-x-visible custom-scrollbar pr-2">
          <div className="w-full space-y-3">
            <section className="a4n-flat-surface p-4 sm:p-5">
              <div className="flex min-w-0 flex-wrap items-start gap-3">
                <div className="min-w-0 flex-1">
                  <div className="text-caption font-medium text-book-primary">
                    当前模式 · {VIDEO_WORKFLOW_MODE_LABELS[mode]}
                  </div>
                  <h3 className="mt-1 text-base font-semibold text-book-text-main">
                    片段 {selectedSegmentNumber} · {selectedSegmentCard.title}
                  </h3>
                  {selectedSegmentCard.sceneSummary ? (
                    <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-muted">
                      {selectedSegmentCard.sceneSummary}
                    </p>
                  ) : null}
                </div>
                <span
                  className={`inline-flex shrink-0 items-center gap-1 rounded-full px-2.5 py-1 text-caption font-medium ${
                    ready
                      ? "bg-success/10 text-success"
                      : "bg-warning-bg text-warning"
                  }`}
                >
                  {ready ? (
                    <CheckCircle2 size={12} />
                  ) : (
                    <AlertCircle size={12} />
                  )}
                  {ready ? "输入已就绪" : "仍有必填项"}
                </span>
              </div>
              <div
                role="status"
                className={`mt-3 rounded-xl border px-3 py-2.5 text-small leading-relaxed ${
                  ready
                    ? "border-success/25 bg-success/5 text-book-text-sub"
                    : "border-warning/35 bg-warning-bg text-warning"
                }`}
              >
                {finalPromptBlockedReason ??
                  "必需输入已填充，可以生成最终提示词。"}
              </div>
            </section>

            {mode !== VIDEO_WORKFLOW_MODE_CHAT_VIDEO ? (
              <section
                aria-label="必需输入"
                className="a4n-flat-surface p-3 sm:p-4"
              >
                <div className="mb-3">
                  <h3 className="text-xs font-medium text-book-text-main">
                    必需输入
                  </h3>
                  <div className="mt-0.5 text-caption text-book-text-muted">
                    点击对应槽位生成、导入或选择素材。
                  </div>
                </div>
                {renderFirstFrameSourceSelector()}
                <div className={MEDIA_SLOT_GRID_CLASS}>
                  {mode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES ? (
                    <>
                      {renderFrameSlot(
                        "首帧",
                        startFramePrompt,
                        startFrameImage,
                      )}
                      {renderFrameSlot("尾帧", endFramePrompt, endFrameImage)}
                    </>
                  ) : mode === VIDEO_WORKFLOW_MODE_STORYBOARD ? (
                    renderFrameSlot(
                      "分镜板",
                      startFramePrompt,
                      startFrameImage,
                      "等待分镜板生成",
                    )
                  ) : segmentRecipe.guidance_mode === "system_frames" ? (
                    renderFrameSlot(
                      "首帧",
                      startFramePrompt,
                      startFrameImage,
                      "等待首帧生成",
                    )
                  ) : (
                    renderContinuitySlot()
                  )}
                </div>
              </section>
            ) : null}

            {referenceInputs}

            <section className="a4n-flat-surface flex flex-wrap items-center justify-between gap-3 p-3 sm:p-4">
              <span className="text-small text-book-text-muted">
                生成后可继续编辑提示词并创建候选视频。
              </span>
              <BookButton
                variant={ready ? "primary" : "secondary"}
                size="sm"
                disabled={acting != null || !ready}
                title={finalPromptBlockedReason ?? undefined}
                onClick={() =>
                  void onGeneratePromptRequests(selectedSegmentNumber)
                }
              >
                <Wand2 size={12} />
                生成最终提示词
              </BookButton>
            </section>
          </div>
        </div>
      );
    }

    const chatVideoMode = workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO;
    const firstFrameChainMode =
      workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN;
    const storyboardMode = workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD;
    const guidanceConfigurationError = videoGuidanceConfigurationError(
      workflowMode,
      selectedSegmentNumber,
      segmentRecipe.guidance_mode,
    );
    const frameStageLabel = VIDEO_WORKFLOW_MODE_LABELS[workflowMode];
    const {
      startFramePrompt,
      endFramePrompt,
      generatedStartFrameImage,
      actualStartFrameImage,
      endFrameImage,
      startFrameMissingMessage,
      frameInputMissingMessage,
      usesContinuity,
    } = frameInputStateForPrompt(selectedPrompt, segmentRecipe);
    const selectedPromptHasVideo = selectedPrompt.generated_video_id != null;
    const selectedPromptLockedReason = selectedPromptRunning
      ? "该视频提示词正在运行，不能编辑或保存"
      : null;
    const selectedPromptDownstreamNotice = selectedPromptHasVideo
      ? "该视频提示词已有下游视频；保存、优化或删除时需要选择删除下游或保留下游继续"
      : null;
    const durationConfigError = durationLimits.configError;
    const saveBlockedReason = selectedPromptLockedReason ?? undefined;
    const optimizeBlockedReason =
      (selectedPromptDirty
        ? "当前编辑内容未保存，请先保存或放弃草稿后再优化"
        : undefined) ??
      selectedPromptLockedReason ??
      undefined;
    const videoGenerationInFlight =
      acting === `generate-video-${selectedPrompt.id}`;
    const selectedGenerateBlockedReason =
      durationConfigError ??
      (selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE
        ? "当前记录不是视频生成提示词"
        : undefined) ??
      (selectedPromptStale ? "当前视频提示词已过期，请重新生成" : undefined) ??
      (savedReferenceIdsError
        ? `保存的参考图数据异常：${savedReferenceIdsError}`
        : undefined) ??
      (staleSavedReferenceCount > 0
        ? `最终提示词有 ${staleSavedReferenceCount} 张冻结参考图已失效，请修改执行方案并重新生成提示词`
        : undefined) ??
      (selectedPromptDirty ? "请先保存当前更改" : undefined) ??
      guidanceConfigurationError ??
      frameInputMissingMessage ??
      (selectedPromptRunning ? "当前视频提示词正在运行" : undefined) ??
      (!chapterContentReady ? "当前章节正文为空，不能生成视频" : undefined);
    const selectedImportBlockedReason =
      (selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE
        ? "当前记录不是视频生成提示词"
        : undefined) ??
      (selectedPromptStale ? "当前视频提示词已过期，请重新生成" : undefined) ??
      (selectedPromptDirty ? "请先保存当前更改" : undefined) ??
      (selectedPromptRunning ? "当前视频提示词正在运行" : undefined) ??
      (!chapterContentReady ? "当前章节正文为空，不能导入视频" : undefined);
    const deletePromptBlockedReason = selectedPromptRunning
      ? "当前视频提示词正在运行"
      : undefined;
    const savingPrompt = acting === `save-prompt-${selectedPrompt.id}`;
    const showSavePrimary = savingPrompt || selectedPromptDirty;
    const showCopyPrimary = !showSavePrimary && chatVideoMode;
    const showGeneratePrimary =
      !showSavePrimary &&
      !chatVideoMode &&
      (videoGenerationInFlight || selectedGenerateBlockedReason == null);
    const editorMetadata = [
      `${draft.duration_secs} 秒`,
      getOperationStatusLabel(selectedPrompt.status),
      selectedPromptStale ? "正文已变化" : null,
      selectedPrompt.generated_video_id != null ? "已生成视频" : null,
      selectedPromptDirty ? "有未保存更改" : null,
    ].filter((value): value is string => Boolean(value));
    const sections = directorSections(selectedPrompt.metadata_json);

    return (
      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-visible custom-scrollbar pr-2">
        <div className="mb-3 flex min-w-0 flex-wrap items-center gap-2 border-b border-book-border/40 pb-3">
          <span className="min-w-0 flex-1 text-small text-book-text-muted">
            {editorMetadata.join(" · ")}
          </span>
          <div className="flex shrink-0 items-center gap-2">
            {showSavePrimary && (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void onSavePrompt()}
                disabled={savingPrompt || selectedPromptRunning}
                title={saveBlockedReason}
                className="!min-h-9 !px-3 sm:!min-h-7"
              >
                <span className="inline-flex items-center gap-1">
                  {savingPrompt ? (
                    <LoadingSpinner size="sm" />
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
                disabled={selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE}
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
                onClick={() => void onGenerateVideo()}
                disabled={videoGenerationInFlight}
                title={selectedGenerateBlockedReason}
                className="!min-h-9 !px-3 sm:!min-h-7"
              >
                <span className="inline-flex items-center gap-1">
                  {videoGenerationInFlight ? (
                    <LoadingSpinner size="sm" />
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
                    acting === `optimize-prompt-${selectedPrompt.id}` ? (
                      <LoadingSpinner size="sm" />
                    ) : (
                      <Wand2 size={13} />
                    ),
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
                          selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE ||
                          staleSavedReferenceCount > 0 ||
                          savedReferenceIdsError != null,
                        onClick: () => void onPreviewGenerateVideo(),
                      },
                      {
                        label:
                          acting === `import-video-${selectedPrompt.id}`
                            ? "正在导入视频"
                            : "导入外部视频",
                        icon:
                          acting === `import-video-${selectedPrompt.id}` ? (
                            <LoadingSpinner size="sm" />
                          ) : (
                            <Upload size={13} />
                          ),
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
                        disabled:
                          acting === `delete-prompt-${selectedPrompt.id}`,
                        onClick: () => void onDeletePrompt(selectedPrompt),
                      },
                    ]
                  : []),
              ]}
            />
          </div>
        </div>
        {selectedPromptStale && (
          <div className="mb-2 text-small text-error">
            {staleVideoPromptMessage(frameStageLabel)}。
          </div>
        )}
        {frameInputMissingMessage && (
          <div className="mb-2 text-small text-warning">
            {frameInputMissingMessage}
          </div>
        )}
        {selectedPromptLockedReason && (
          <div className="mb-2 text-small text-warning">
            {selectedPromptLockedReason}
          </div>
        )}
        {selectedPromptDownstreamNotice && (
          <div className="mb-2 text-small text-warning">
            {selectedPromptDownstreamNotice}
          </div>
        )}
        {durationConfigError && (
          <div className="mb-2 text-small text-error">
            {durationConfigError}
          </div>
        )}
        {selectedPromptDirty && (
          <div className="mb-2 text-small text-warning">
            当前编辑内容未保存，模型请求预览会使用文字草稿覆盖，生成视频前需要先保存。
          </div>
        )}

        {selectedPromptRunning ? (
          <div className="a4n-flat-surface mb-3 flex items-center gap-2 px-3 py-2.5 text-small text-book-text-muted">
            <LoadingSpinner size="sm" />
            当前提示词正在运行，以下内容暂时只读。
          </div>
        ) : null}
        <fieldset
          disabled={selectedPromptRunning}
          className="m-0 min-w-0 border-0 p-0"
        >
          {chatVideoMode ? null : firstFrameChainMode ? (
            <div className="a4n-flat-surface mb-3 p-3">
              {renderFirstFrameSourceSelector()}
              <div className="mb-2 flex items-center gap-2">
                <div className="inline-flex items-center gap-1.5 text-xs text-book-text-main">
                  <ImageIcon size={12} className="text-book-primary" />
                  起始帧
                  <span className="text-caption text-book-text-muted">
                    · {actualStartFrameImage ? "已生成" : "等待生成"}
                  </span>
                </div>
              </div>
              {startFrameMissingMessage && (
                <div className="mb-2 text-caption text-warning">
                  {startFrameMissingMessage}
                </div>
              )}
              <div className={MEDIA_SLOT_GRID_CLASS}>
                {usesContinuity
                  ? renderContinuitySlot()
                  : renderFrameSlot(
                      "首帧",
                      startFramePrompt,
                      actualStartFrameImage,
                      "等待首帧生成",
                    )}
              </div>
            </div>
          ) : storyboardMode ? (
            <div className="a4n-flat-surface mb-3 p-3">
              <div className="mb-2 flex min-w-0 items-center gap-2">
                <ImageIcon size={12} className="shrink-0 text-book-primary" />
                <div className="min-w-0 flex-1">
                  <div className="text-xs font-medium text-book-text-main">
                    分镜板
                  </div>
                  <div className="mt-0.5 text-caption text-book-text-muted">
                    {generatedStartFrameImage ? "已生成" : "等待生成"}
                  </div>
                </div>
              </div>
              {frameInputMissingMessage && (
                <div className="mb-2 text-caption text-warning">
                  {frameInputMissingMessage}
                </div>
              )}
              <div className={MEDIA_SLOT_GRID_CLASS}>
                {renderFrameSlot(
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
                  <div className="text-xs font-medium text-book-text-main">
                    首尾帧
                  </div>
                  <div className="mt-0.5 text-caption text-book-text-muted">
                    {Number(Boolean(generatedStartFrameImage)) +
                      Number(Boolean(endFrameImage))}{" "}
                    / 2 已生成
                  </div>
                </div>
              </div>
              <div className={MEDIA_SLOT_GRID_CLASS}>
                {renderFrameSlot(
                  "首帧",
                  startFramePrompt,
                  generatedStartFrameImage,
                )}
                {renderFrameSlot("尾帧", endFramePrompt, endFrameImage)}
              </div>
            </div>
          )}

          {staleSavedReferenceCount > 0 ? (
            <div className="mb-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
              最终提示词冻结的参考图里有 {staleSavedReferenceCount}{" "}
              张已失效。请调整下方参考素材并重新生成最终提示词。
            </div>
          ) : null}
          {savedReferenceIdsError ? (
            <div className="mb-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
              已保存参考图数据异常：{savedReferenceIdsError}
              。请调整下方参考素材并重新生成最终提示词。
            </div>
          ) : null}
          {referenceInputs ? (
            <div className="mb-3">{referenceInputs}</div>
          ) : null}

          {selectedPromptSceneSummary ? (
            <div className="mb-3 text-xs">
              <span className="text-book-text-muted">场景摘要</span>
              <div className="mt-1 min-h-[34px] border-l-2 border-book-border/55 py-1 pl-2.5 leading-relaxed text-book-text-sub">
                {selectedPromptSceneSummary}
              </div>
            </div>
          ) : null}

          {sections.length > 0 && (
            <details className="a4n-flat-surface mb-3 p-2">
              <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-2 text-xs text-book-text-main [&::-webkit-details-marker]:hidden">
                <span className="inline-flex min-w-0 items-center gap-1.5">
                  <ListChecks size={12} className="text-book-primary" />
                  导演约束
                </span>
                <span className="text-caption text-book-text-muted">
                  {sections.length} 项
                </span>
              </summary>
              <dl className="mt-2 divide-y divide-book-border/40 border-y border-book-border/40">
                {sections.map((section) => (
                  <div
                    key={section.key}
                    className="grid min-w-0 gap-1 px-2.5 py-2 md:grid-cols-[110px_minmax(0,1fr)] md:gap-3"
                  >
                    <dt className="text-caption text-book-text-muted">
                      {section.label}
                    </dt>
                    <dd className="line-clamp-4 whitespace-pre-wrap break-words text-small leading-relaxed text-book-text-sub">
                      {section.text}
                    </dd>
                  </div>
                ))}
              </dl>
            </details>
          )}

          {selectedPromptCharStats && (
            <details className="mb-3 text-caption text-book-text-muted">
              <summary className="w-fit cursor-pointer hover:text-book-text-main">
                诊断信息
              </summary>
              <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 border-l-2 border-book-border/60 pl-2 text-caption">
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
              <span className="text-xs text-book-text-muted">视频时长</span>
              {durationLimits.durationMode === "fixed" ? (
                <select
                  value={draft.duration_secs}
                  onChange={(event) =>
                    updateDraft({ duration_secs: Number(event.target.value) })
                  }
                  disabled={selectedPromptRunning}
                  className="book-control h-9 px-2 text-xs"
                >
                  {!isDurationAllowed(draft.duration_secs, durationLimits) && (
                    <option value={draft.duration_secs} disabled>
                      {draft.duration_secs} 秒（当前配置不支持）
                    </option>
                  )}
                  {durationChoicesForLimits(durationLimits).map((duration) => (
                    <option key={duration} value={duration}>
                      {duration} 秒
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  type="number"
                  min={durationLimits.minDurationSecs}
                  max={durationLimits.maxDurationSecs}
                  step={1}
                  value={draft.duration_secs}
                  onChange={(event) =>
                    updateDraft({ duration_secs: Number(event.target.value) })
                  }
                  disabled={selectedPromptRunning}
                  className="book-control h-9 px-2 text-xs"
                />
              )}
              <span className="text-caption text-book-text-muted">
                当前模型支持：{durationLimitsLabel(durationLimits)}
              </span>
            </label>
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-xs text-book-text-muted">
                {chatVideoMode ? "Chat 视频提示词" : "画面提示词"}
              </span>
              <textarea
                value={draft.visual_prompt}
                onChange={(event) =>
                  updateDraft({ visual_prompt: event.target.value })
                }
                disabled={selectedPromptRunning}
                className="book-control min-h-[220px] resize-y p-2 text-xs leading-relaxed"
              />
            </label>
            {!chatVideoMode && (
              <label className="flex min-w-0 flex-col gap-1">
                <span className="text-xs text-book-text-muted">运动提示词</span>
                <textarea
                  value={draft.motion_prompt}
                  onChange={(event) =>
                    updateDraft({ motion_prompt: event.target.value })
                  }
                  disabled={selectedPromptRunning}
                  className="book-control min-h-[140px] resize-y p-2 text-xs leading-relaxed"
                />
              </label>
            )}
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-xs text-book-text-muted">限制与避免项</span>
              <textarea
                value={draft.negative_prompt}
                onChange={(event) =>
                  updateDraft({ negative_prompt: event.target.value })
                }
                disabled={selectedPromptRunning}
                className="book-control min-h-[96px] resize-y p-2 text-xs leading-relaxed"
              />
            </label>
          </div>
        </fieldset>
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
            { id: "segments", label: "片段" },
            {
              id: "results",
              label: `视频${videos.length > 0 ? ` ${videos.length}` : ""}`,
            },
          ]}
        />
      </div>
      <div className="grid min-h-0 flex-1 gap-3 xl:grid-cols-[340px_minmax(0,1fr)]">
        <div
          className={`min-h-0 ${mobilePane === "segments" ? "block" : "hidden xl:block"}`}
        >
          <VideoPromptSetupPanel
            prompts={prompts}
            segmentCards={segmentCards}
            segmentRecipes={segmentRecipes}
            selectedSegmentNumber={selectedSegmentNumber}
            planningDraftVisible={planningDraftVisible}
            selectedPromptId={selectedPromptId}
            acting={acting}
            loading={loading}
            savingRecipeSegments={savingRecipeSegments}
            finalPromptBlockedReasons={finalPromptBlockedReasons}
            timeline={timeline}
            layout="index"
            onLoad={onLoad}
            onSelectPrompt={onSelectPrompt}
            onSelectSegment={onActivateSegment}
            onWorkflowModeChange={onWorkflowModeChange}
            onDeletePrompt={onDeletePrompt}
            onInsertSegment={onInsertSegment}
            onOpenInsertedSegment={onOpenInsertedSegment}
          />
        </div>

        <section
          aria-label="视频结果"
          className={`min-h-0 flex-col ${mobilePane === "results" ? "flex" : "hidden xl:flex"}`}
        >
          <VideoResultsPane
            selectedVideo={selectedVideo}
            adoptedVideoId={segmentRecipe?.adopted_video_id ?? null}
            selectedVideoPath={selectedVideoPath}
            trimStart={trimStart}
            trimEnd={trimEnd}
            acting={acting}
            onAdoptVideo={onAdoptVideo}
            onPollVideo={onPollVideo}
            onPreviewVideoRequest={onPreviewVideoRequest}
            onToggleVideoFavorite={onToggleVideoFavorite}
            onUpdateVideo={onUpdateVideo}
            onDeleteVideo={onDeleteVideo}
            onTrimStartChange={onTrimStartChange}
            onTrimEndChange={onTrimEndChange}
            onTrim={onTrim}
            onOpenSegmentConfig={() => {
              setMobilePane("segments");
              if (selectedSegmentNumber != null) {
                void onActivateSegment(selectedSegmentNumber);
              }
            }}
          />
        </section>
      </div>

      {selectedSegmentNumber != null ? (
        <Modal
          isOpen={segmentConfigOpen}
          onClose={() => {
            setInputDialog(null);
            void onCloseSegmentConfig();
          }}
          title={`片段 ${selectedSegmentNumber} · 配置`}
          subtitle={selectedSegmentCard?.title ?? "当前片段不在最新片段规划中"}
          icon={<SlidersHorizontal size={15} />}
          maxWidthClassName="max-w-none sm:max-w-[min(96vw,90rem)]"
          className="h-[calc(100dvh-0.75rem)] sm:h-[90vh]"
          bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar"
        >
          <div className="flex min-h-full flex-col gap-3 p-3 lg:h-full lg:min-h-0 lg:p-4">
            <section
              aria-label="片段配置选项"
              className="shrink-0 overflow-hidden rounded-[var(--pyqt-radius-lg)] border border-book-border/55 bg-book-bg-paper/45"
            >
              <div className="p-3">
                <VideoPromptSetupPanel
                  prompts={prompts}
                  segmentCards={segmentCards}
                  segmentRecipes={segmentRecipes}
                  selectedSegmentNumber={selectedSegmentNumber}
                  planningDraftVisible={planningDraftVisible}
                  selectedPromptId={selectedPromptId}
                  acting={acting}
                  loading={loading}
                  savingRecipeSegments={savingRecipeSegments}
                  finalPromptBlockedReasons={finalPromptBlockedReasons}
                  timeline={timeline}
                  layout="selected"
                  onLoad={onLoad}
                  onSelectPrompt={onSelectPrompt}
                  onSelectSegment={onActivateSegment}
                  onWorkflowModeChange={onWorkflowModeChange}
                  onDeletePrompt={onDeletePrompt}
                  onInsertSegment={onInsertSegment}
                  onOpenInsertedSegment={onOpenInsertedSegment}
                />
              </div>
            </section>
            <section
              aria-label="片段生成工作区"
              className="flex min-h-[36rem] flex-1 flex-col overflow-hidden lg:min-h-0"
            >
              {workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA ? (
                referenceMediaWorkspace
              ) : (
                <div className="flex h-full min-h-0 flex-col overflow-hidden p-3">
                  {renderEditor()}
                </div>
              )}
            </section>
          </div>
        </Modal>
      ) : null}

      <Modal
        isOpen={segmentConfigOpen && inputDialog?.kind === "frame"}
        onClose={() => setInputDialog(null)}
        title={
          inputDialog?.kind === "frame" ? `${inputDialog.label}配置` : "帧配置"
        }
        subtitle="在这里准备提示词，并生成或导入对应图片"
        icon={<ImageIcon size={15} />}
        maxWidthClassName="max-w-2xl"
        zIndexClassName="z-[70]"
        footer={
          inputDialog?.kind === "frame" && selectedSegmentNumber != null ? (
            inputDialogPrompt ? (
              <>
                {inputDialogImage ? (
                  <BookButton
                    variant="danger"
                    size="sm"
                    aria-label={`删除${inputDialog.label}图片`}
                    disabled={
                      acting != null || imageBusyId === inputDialogImage.id
                    }
                    onClick={() => void onDeleteFrameImage(inputDialogImage)}
                  >
                    {imageBusyId === inputDialogImage.id ? (
                      <LoadingSpinner size="sm" />
                    ) : (
                      <Trash2 size={12} />
                    )}
                    删除图片
                  </BookButton>
                ) : null}
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={
                    acting != null ||
                    frameImageLocked ||
                    framePromptDirty ||
                    inputDialogPrompt.status === "running"
                  }
                  title={
                    frameImageLocked
                      ? "请先删除当前图片，再优化提示词"
                      : framePromptDirty
                        ? "当前提示词有未保存编辑，请先保存"
                        : undefined
                  }
                  onClick={async () => {
                    const updated = await onOptimizePrompt(inputDialogPrompt);
                    if (updated) setFramePromptDraft(updated.visual_prompt);
                  }}
                >
                  <Wand2 size={12} />
                  优化提示词
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={
                    acting != null ||
                    frameImageLocked ||
                    !framePromptDirty ||
                    inputDialogPrompt.status === "running" ||
                    !framePromptDraft.trim()
                  }
                  title={
                    frameImageLocked
                      ? "请先删除当前图片，再编辑提示词"
                      : undefined
                  }
                  onClick={async () => {
                    const updated = await onSaveFramePrompt(
                      inputDialogPrompt,
                      framePromptDraft,
                    );
                    if (updated) setFramePromptDraft(updated.visual_prompt);
                  }}
                >
                  <Save size={12} />
                  保存提示词
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={acting != null || framePromptDirty}
                  title={
                    framePromptDirty
                      ? "请先保存提示词，再预览图片请求"
                      : undefined
                  }
                  onClick={() => {
                    setInputDialog(null);
                    void onPreviewFrameImage(inputDialogPrompt);
                  }}
                >
                  <Eye size={12} />
                  预览请求
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={
                    acting != null || frameImageLocked || framePromptDirty
                  }
                  title={
                    frameImageLocked
                      ? "请先删除当前图片，再导入新图片"
                      : framePromptDirty
                        ? "请先保存提示词，再导入图片"
                        : undefined
                  }
                  onClick={() => {
                    setInputDialog(null);
                    void onImportFrameImage(inputDialogPrompt);
                  }}
                >
                  <Upload size={12} />
                  导入图片
                </BookButton>
                <BookButton
                  variant="primary"
                  size="sm"
                  disabled={
                    acting != null || frameImageLocked || framePromptDirty
                  }
                  title={
                    frameImageLocked
                      ? "请先删除当前图片，再重新生成"
                      : framePromptDirty
                        ? "请先保存提示词，再生成图片"
                        : undefined
                  }
                  onClick={() => {
                    setInputDialog(null);
                    void onGenerateFrameImage(inputDialogPrompt);
                  }}
                >
                  <Wand2 size={12} />
                  生成图片
                </BookButton>
              </>
            ) : (
              <BookButton
                variant="primary"
                size="sm"
                disabled={
                  acting != null ||
                  savingRecipeSegments.has(selectedSegmentNumber)
                }
                onClick={() => {
                  setInputDialog(null);
                  void onGenerateFramePrompts(selectedSegmentNumber);
                }}
              >
                <Wand2 size={12} />
                准备{inputDialog.label}提示词
              </BookButton>
            )
          ) : undefined
        }
      >
        {inputDialog?.kind === "frame" ? (
          <div className="space-y-3">
            <div className="flex aspect-video items-center justify-center overflow-hidden rounded-xl bg-book-bg-tertiary text-small text-book-text-muted">
              {inputDialogFrameSrc &&
              !failedFrameMediaSrcs.has(inputDialogFrameSrc) ? (
                <img
                  src={inputDialogFrameSrc}
                  alt=""
                  className="h-full w-full object-cover"
                  onError={() =>
                    setFailedFrameMediaSrcs((current) =>
                      new Set(current).add(inputDialogFrameSrc),
                    )
                  }
                />
              ) : (
                <span className="flex flex-col items-center gap-2">
                  {inputDialogImage ? (
                    <AlertCircle size={24} />
                  ) : (
                    <Plus size={24} />
                  )}
                  <span>
                    {inputDialogImage
                      ? "图片无法加载，请先删除后重新生成或导入"
                      : inputDialog.emptyText}
                  </span>
                </span>
              )}
            </div>
            <div className="rounded-xl border border-book-border/45 bg-book-bg-paper/45 px-3 py-2.5">
              <label className="block">
                <span className="text-xs font-medium text-book-text-main">
                  {inputDialogPrompt
                    ? `帧 ${inputDialogPrompt.shot_number} · 图片提示词`
                    : "尚无帧提示词"}
                </span>
                {inputDialogPrompt ? (
                  <textarea
                    aria-label={`${inputDialog.label}图片提示词`}
                    value={framePromptDraft}
                    onChange={(event) =>
                      setFramePromptDraft(event.target.value)
                    }
                    disabled={
                      acting != null ||
                      frameImageLocked ||
                      inputDialogPrompt.status === "running"
                    }
                    title={
                      frameImageLocked
                        ? "请先删除当前图片，再编辑提示词"
                        : undefined
                    }
                    className="book-control mt-2 min-h-[150px] w-full resize-y p-2 text-xs leading-relaxed"
                  />
                ) : (
                  <span className="mt-1 block text-small leading-relaxed text-book-text-muted">
                    点击下方按钮先准备{inputDialog.label}提示词。
                  </span>
                )}
              </label>
            </div>
          </div>
        ) : null}
      </Modal>

      <Modal
        isOpen={segmentConfigOpen && inputDialog?.kind === "continuity"}
        onClose={() => setInputDialog(null)}
        title="续接首帧配置"
        subtitle="选择更早片段已采用的视频，并使用它的尾帧作为当前片段首帧"
        icon={<PanelTop size={15} />}
        maxWidthClassName="max-w-2xl"
        zIndexClassName="z-[70]"
      >
        {inputDialog?.kind === "continuity" ? (
          <div className="space-y-3">
            <div className="flex aspect-video items-center justify-center overflow-hidden rounded-xl bg-book-bg-tertiary text-small text-book-text-muted">
              {selectedContinuityTailFrameSrc &&
              !failedFrameMediaSrcs.has(selectedContinuityTailFrameSrc) ? (
                <img
                  src={selectedContinuityTailFrameSrc}
                  alt=""
                  className="h-full w-full object-cover"
                  onError={() =>
                    setFailedFrameMediaSrcs((current) =>
                      new Set(current).add(selectedContinuityTailFrameSrc),
                    )
                  }
                />
              ) : (
                <span className="flex flex-col items-center gap-2 text-center">
                  {selectedContinuitySourceVideoId != null ? (
                    <AlertCircle size={24} />
                  ) : (
                    <Plus size={24} />
                  )}
                  <span>
                    {selectedContinuitySourceVideoId == null
                      ? "尚未选择来源视频"
                      : selectedContinuityTailFrame
                        ? "图片无法加载，请重新选择来源视频"
                        : `视频 #${selectedContinuitySourceVideoId} 尚无可用尾帧`}
                  </span>
                </span>
              )}
            </div>
            <label className="block">
              <span className="mb-1 block text-xs text-book-text-muted">
                来源视频
              </span>
              <select
                value={selectedContinuitySourceVideoId ?? ""}
                disabled={
                  acting != null ||
                  selectedSegmentNumber == null ||
                  savingRecipeSegments.has(selectedSegmentNumber)
                }
                onChange={(event) => {
                  if (selectedSegmentNumber == null) return;
                  void onContinuitySourceChange(
                    selectedSegmentNumber,
                    Number(event.target.value) || null,
                  );
                }}
                className="book-control h-10 w-full px-3 text-small"
              >
                <option value="">选择更早片段已采用的视频…</option>
                {selectedContinuitySourceVideoId != null &&
                !selectedContinuitySourceAvailable ? (
                  <option value={selectedContinuitySourceVideoId}>
                    当前来源视频 #{selectedContinuitySourceVideoId}（已失效）
                  </option>
                ) : null}
                {previousAdoptedVideos.map((video) => (
                  <option key={video.id} value={video.id}>
                    片段{" "}
                    {String(
                      (video.metadata_json as Record<string, unknown> | null)
                        ?.shot_number ?? "?",
                    )}{" "}
                    · 视频 #{video.id}
                  </option>
                ))}
              </select>
            </label>
            <div className="text-caption leading-relaxed text-book-text-muted">
              选择会即时保存；尾帧尚未采集时，最终提示词和视频生成会保持阻止状态并显示明确错误。
            </div>
          </div>
        ) : null}
      </Modal>
    </div>
  );
};
