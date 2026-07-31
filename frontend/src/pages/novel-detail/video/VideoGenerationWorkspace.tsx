import React, { useMemo, useState } from "react";
import {
  AlertCircle,
  CheckCircle2,
  Eye,
  Image as ImageIcon,
  Images,
  ListChecks,
  Copy,
  PanelTop,
  Play,
  Save,
  Trash2,
  Upload,
  Wand2,
} from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { GeneratedImage } from "../../../api/image";
import type {
  ChapterVideoPrompt,
  GeneratedVideo,
  UpdateVideoInput,
  SegmentGuidanceMode,
  VideoSegmentExecutionRecipe,
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
  promptByteCount,
  promptCharCount,
  staleVideoPromptMessage,
  videoGuidanceConfigurationError,
  videoPromptUnitFrameNumbers,
  videoFrameImageRoleLabel,
  videoFrameImageRoleOfImage,
  videoImageGenerationPromptId,
  videoTailFrameSourceVideoId,
  videoTailFrameBoundaryNumber,
  isVideoPromptFrameImage,
  VIDEO_WORKFLOW_MODE_LABELS,
  type ChapterVideoWorkflowMode,
  type PromptDraft,
  type VideoDurationLimits,
  type VideoSegmentCardPlan,
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
  segmentCards: VideoSegmentCardPlan[];
  segmentRecipes: Map<number, VideoSegmentExecutionRecipe>;
  selectedSegmentNumber: number | null;
  planningDraftVisible: boolean;
  segmentRecipe: VideoSegmentExecutionRecipe | null;
  finalPromptBlockedReason: string | null;
  finalPromptBlockedReasons: Map<number, string>;
  selectedPrompt: ChapterVideoPrompt | null;
  selectedPromptId: number | null;
  workflowMode: ChapterVideoWorkflowMode | null;
  selectedPromptStale: boolean;
  durationLimits: VideoDurationLimits;
  selectableReferenceImages: GeneratedImage[];
  loading: boolean;
  acting: string | null;
  chapterGenerating: boolean;
  chapterContentReady: boolean;
  savingRecipeSegments: Set<number>;
  draft: PromptDraft;
  selectedPromptSceneSummary: string;
  selectedPromptRunning: boolean;
  selectedPromptDirty: boolean;
  staleSavedReferenceCount: number;
  savedReferenceIdsError: string | null;
  systemReferenceIds: Set<number>;
  boundReferenceImages: GeneratedImage[];
  framePrompts: ChapterVideoPrompt[];
  frameImages: GeneratedImage[];
  videos: GeneratedVideo[];
  previousAdoptedVideos: GeneratedVideo[];
  selectedSegmentMaterialsControl?: React.ReactNode;
  selectedVideo: GeneratedVideo | null;
  selectedVideoId: number | null;
  selectedVideoPath: string;
  trimStart: string;
  trimEnd: string;
  referenceImageScopeLabel: (img: GeneratedImage) => string;
  onLoad: () => void | Promise<void>;
  onGeneratePromptRequests: (options?: {
    regenerateExisting?: boolean;
    targetShotNumber?: number;
  }) => void | Promise<void>;
  onSelectPrompt: (id: number) => boolean | Promise<boolean>;
  onSelectSegment: (segmentNumber: number) => boolean | Promise<boolean>;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onWorkflowModeChange: (
    mode: ChapterVideoWorkflowMode,
    shotNumber: number,
  ) => void | Promise<void>;
  onUpdateGuidance: (
    segmentNumber: number,
    guidanceMode: SegmentGuidanceMode,
    referenceImageIds: number[],
    continuitySourceVideoId: number | null,
  ) => void | Promise<void>;
  onGenerateFramePrompts: (options?: {
    regenerateExisting?: boolean;
    targetShotNumber?: number;
  }) => void | Promise<void>;
  onPreviewFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onGenerateFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onImportFrameImage: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onDraftChange: (draft: PromptDraft) => void;
  onSavePrompt: () => void | Promise<void>;
  onOptimizePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onPreviewGenerateVideo: () => void | Promise<void>;
  onCopyPrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onPreviewVideoRequest: (video: GeneratedVideo) => void | Promise<void>;
  onGenerateVideo: (prompt?: ChapterVideoPrompt) => void | Promise<void>;
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
  onTrimStartChange: (value: string) => void;
  onTrimEndChange: (value: string) => void;
  onTrim: () => void | Promise<void>;
  selectedSegmentWorkspace?: React.ReactNode;
}

export const VideoGenerationWorkspace: React.FC<
  VideoGenerationWorkspaceProps
> = ({
  chapter,
  prompts,
  segmentCards,
  segmentRecipes,
  selectedSegmentNumber,
  planningDraftVisible,
  segmentRecipe,
  finalPromptBlockedReason,
  finalPromptBlockedReasons,
  selectedPrompt,
  selectedPromptId,
  workflowMode,
  selectedPromptStale,
  durationLimits,
  selectableReferenceImages,
  loading,
  acting,
  chapterGenerating,
  chapterContentReady,
  savingRecipeSegments,
  draft,
  selectedPromptSceneSummary,
  selectedPromptRunning,
  selectedPromptDirty,
  staleSavedReferenceCount,
  savedReferenceIdsError,
  systemReferenceIds,
  boundReferenceImages,
  framePrompts,
  frameImages,
  videos,
  previousAdoptedVideos,
  selectedSegmentMaterialsControl,
  selectedVideo,
  selectedVideoId,
  selectedVideoPath,
  trimStart,
  trimEnd,
  referenceImageScopeLabel,
  onLoad,
  onGeneratePromptRequests,
  onSelectPrompt,
  onSelectSegment,
  onDeletePrompt,
  onWorkflowModeChange,
  onUpdateGuidance,
  onGenerateFramePrompts,
  onPreviewFrameImage,
  onGenerateFrameImage,
  onImportFrameImage,
  onDraftChange,
  onSavePrompt,
  onOptimizePrompt,
  onPreviewGenerateVideo,
  onCopyPrompt,
  onPreviewVideoRequest,
  onGenerateVideo,
  onImportVideo,
  onSelectVideo,
  onAdoptVideo,
  onPollVideo,
  onToggleVideoFavorite,
  onUpdateVideo,
  onDeleteVideo,
  onTrimStartChange,
  onTrimEndChange,
  onTrim,
  selectedSegmentWorkspace,
}) => {
  const [promptDetailOpen, setPromptDetailOpen] = useState(false);
  const [mobilePane, setMobilePane] = useState<"setup" | "results">("setup");
  const guidanceMode = segmentRecipe?.guidance_mode ?? "none";

  const segmentVideos = videos;

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
      const sourceVideoId = isVideoTailFrameImage(image)
        ? videoTailFrameSourceVideoId(image)
        : null;
      if (sourceVideoId != null && !tailByVideoId.has(sourceVideoId)) {
        tailByVideoId.set(sourceVideoId, image);
      }
    }
    return { boundaryByFrameNumber, tailByVideoId };
  }, [frameImages, framePrompts]);

  const selectPrompt = async (id: number) => {
    const prompt = prompts.find((row) => row.id === id);
    if (!prompt) return;
    if ((await onSelectSegment(prompt.shot_number)) === false) return;
    if ((await onSelectPrompt(id)) === false) return;
    setPromptDetailOpen(true);
  };

  const selectVideo = (id: number) => {
    onSelectVideo(id);
  };

  const renderBoundReferenceImages = () => {
    if (boundReferenceImages.length === 0) {
      return (
        <div className="text-small text-book-text-muted">
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
            <li key={image.id} className="min-w-0 text-caption">
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
                {videoFrameImageRoleLabel(
                  role,
                  imageTypeLabel(image.image_type),
                )}
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
    sourceLabel?: string,
  ) => {
    const src = image ? toMediaSrc(image.file_path) : "";
    return (
      <div className="min-w-0">
        <div className="mb-1.5 min-w-0 text-xs">
          <span className="font-medium text-book-text-main">{label}</span>
          <div className="mt-0.5 text-caption text-book-text-muted">
            {sourceLabel ??
              (prompt ? `帧 ${prompt.shot_number}` : "尚无提示词")}
            <span className="mx-1">·</span>
            {image ? "已生成" : "等待生成"}
          </div>
        </div>
        <div className="flex aspect-video items-center justify-center overflow-hidden rounded-xl bg-book-bg-tertiary text-small text-book-text-muted">
          {src ? (
            <img src={src} alt={label} className="h-full w-full object-cover" />
          ) : (
            "未生成"
          )}
        </div>
        <div className="mt-1 line-clamp-1 text-caption text-book-text-sub">
          {prompt?.title || prompt?.scene_summary || emptyText}
        </div>
        {prompt ? (
          <div className="mt-2 flex flex-wrap items-center gap-1.5">
            <BookButton
              variant={image ? "secondary" : "primary"}
              size="sm"
              onClick={() => void onPreviewFrameImage(prompt)}
              className="!min-h-7 !px-2 text-caption"
              title={`预览${label}生成请求`}
            >
              <Eye size={11} />
              预览请求
            </BookButton>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void onGenerateFrameImage(prompt)}
              className="!min-h-7 !px-2 text-caption"
              title={image ? `重新生成${label}` : `生成${label}`}
            >
              <Wand2 size={11} />
              {image ? "重新生成" : "生成图片"}
            </BookButton>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void onImportFrameImage(prompt)}
              className="!min-h-7 !px-2 text-caption"
              title={`导入${label}`}
            >
              <Upload size={11} />
              导入图片
            </BookButton>
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
    const continuitySourceVideoId =
      firstFrameChainMode &&
      prompt.shot_number > 1 &&
      guidanceMode === "continuity"
        ? (segmentRecipe?.continuity_source_video_id ?? null)
        : null;
    const continuityTailFrameImage =
      continuitySourceVideoId == null
        ? null
        : (frameImageIndexes.tailByVideoId.get(continuitySourceVideoId) ??
          null);
    const actualStartFrameImage =
      firstFrameChainMode && prompt.shot_number > 1
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
        ? prompt.shot_number <= 1
          ? "请先在首帧生成里生成第 1 段首帧图片。"
          : continuitySourceVideoId == null
            ? "请先在片段卡片里选择一个更早片段已采用的视频作为续接来源。"
            : `续接来源视频 #${continuitySourceVideoId} 尚无可用尾帧，请等待尾帧采集完成或重新选择来源。`
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
      continuitySourceVideoId,
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
      if (selectedSegmentNumber != null && segmentRecipe) {
        const mode = segmentRecipe.workflow_mode;
        const startFramePrompt =
          framePromptByNumber.get(
            mode === "boundary_frames" ? 1 : selectedSegmentNumber,
          ) ?? null;
        const endFramePrompt =
          mode === "boundary_frames"
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
        const needsFramePreparation =
          mode === "boundary_frames" ||
          mode === "storyboard" ||
          (mode === "first_frame_chain" && selectedSegmentNumber === 1);
        const usesSystemFrames =
          segmentRecipe.guidance_mode === "system_frames";
        const continuitySourceVideoId =
          segmentRecipe.continuity_source_video_id;
        const textFramePromptStatus =
          mode === "boundary_frames"
            ? `${Number(Boolean(startFramePrompt)) + Number(Boolean(endFramePrompt))} / 2 条边界帧提示词已准备`
            : mode === "storyboard"
              ? startFramePrompt
                ? "分镜提示词已准备"
                : "分镜提示词尚未准备"
              : startFramePrompt
                ? "首帧提示词已准备"
                : "首帧提示词尚未准备";
        const guidanceStatus =
          segmentRecipe.guidance_mode === "system_frames"
            ? mode === "storyboard"
              ? "系统分镜板"
              : mode === "first_frame_chain"
                ? "系统首帧"
                : "系统首尾帧"
            : segmentRecipe.guidance_mode === "keyframes"
              ? `关键帧引导 · 已选择 ${segmentRecipe.guidance_reference_image_ids.length} 张`
              : segmentRecipe.guidance_mode === "continuity"
                ? continuitySourceVideoId == null
                  ? "续接视频 · 尚未选择来源"
                  : `续接视频 · 来源 #${continuitySourceVideoId}`
                : "纯文本引导 · 不生成系统图片";
        const preparationMessage =
          finalPromptBlockedReason ??
          (mode === "chat_video"
            ? "Chat 方案不生成系统帧，可直接生成最终提示词。"
            : mode === "reference_media"
              ? "请在参考媒体工作区保存当前片段的提示词、媒体槽和用途。"
              : mode === "first_frame_chain" && selectedSegmentNumber > 1
                ? segmentRecipe.guidance_mode === "continuity"
                  ? continuitySourceVideoId != null
                    ? `将从已采用视频 #${continuitySourceVideoId} 续接。`
                    : `请先为片段 ${selectedSegmentNumber} 选择一个更早片段已采用的视频。`
                  : segmentRecipe.guidance_mode === "keyframes"
                    ? "先确认卡片中的关键帧，再生成当前片段的最终提示词。"
                    : "当前片段使用纯文本引导，可直接生成最终提示词。"
                : mode === "storyboard"
                  ? usesSystemFrames
                    ? "先准备当前片段的分镜提示词和分镜板，再生成最终提示词。"
                    : "先准备当前片段的分镜文字提示词，再使用卡片所选媒体引导生成最终提示词。"
                  : mode === "first_frame_chain"
                    ? usesSystemFrames
                      ? "先准备首段首帧提示词和图片，再生成最终提示词。"
                      : "先准备首段首帧文字提示词，再使用卡片所选媒体引导生成最终提示词。"
                    : usesSystemFrames
                      ? "先准备当前片段的首尾帧提示词和图片，再生成最终提示词。"
                      : "先准备当前片段的首尾帧文字提示词，再使用卡片所选媒体引导生成最终提示词。");

        const selectedCard = segmentCards.find(
          (card) => card.segmentNumber === selectedSegmentNumber,
        );
        const referenceCount = new Set([
          ...segmentRecipe.reference_image_ids,
          ...segmentRecipe.video_library_image_ids,
        ]).size;
        const inputSummary =
          mode === "chat_video"
            ? referenceCount > 0
              ? `${referenceCount} 张 Chat 参考图`
              : "纯文本 Chat 输入"
            : guidanceStatus;
        const frameActionLabel =
          mode === "storyboard"
            ? "准备分镜提示词"
            : mode === "first_frame_chain"
              ? "准备首帧提示词"
              : "准备首尾帧提示词";
        const framePromptsReady =
          mode === "boundary_frames"
            ? startFramePrompt != null && endFramePrompt != null
            : startFramePrompt != null;
        const ready = finalPromptBlockedReason == null;

        return (
          <div className="min-h-0 flex-1 overflow-y-auto overflow-x-visible custom-scrollbar pr-2">
            <div className="mx-auto w-full max-w-3xl space-y-3">
              <section className="a4n-flat-surface overflow-hidden">
                <div className="p-4 sm:p-5">
                  <div className="flex min-w-0 flex-wrap items-start gap-3">
                    <div className="min-w-0 flex-1">
                      <div className="text-caption font-medium text-book-primary">
                        当前片段 · {VIDEO_WORKFLOW_MODE_LABELS[mode]}
                      </div>
                      <h3 className="mt-1 text-base font-semibold text-book-text-main">
                        片段 {selectedSegmentNumber} ·{" "}
                        {selectedCard?.title || "准备生成"}
                      </h3>
                      {selectedCard?.sceneSummary ? (
                        <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-muted">
                          {selectedCard.sceneSummary}
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
                      {ready ? "可以生成" : "需要处理"}
                    </span>
                  </div>

                  <div
                    role="status"
                    className={`mt-4 rounded-xl border px-3 py-2.5 text-small leading-relaxed ${
                      ready
                        ? "border-success/25 bg-success/5 text-book-text-sub"
                        : "border-warning/35 bg-warning-bg text-warning"
                    }`}
                  >
                    {preparationMessage}
                  </div>

                  <ol className="mt-4 divide-y divide-book-border/35 border-y border-book-border/35">
                    <li className="grid grid-cols-[24px_minmax(0,1fr)_auto] items-center gap-3 py-3">
                      <span className="flex h-6 w-6 items-center justify-center rounded-full bg-success/10 text-caption font-semibold text-success">
                        1
                      </span>
                      <div className="min-w-0">
                        <div className="text-xs font-medium text-book-text-main">
                          选择生成方式
                        </div>
                        <div className="mt-0.5 text-caption text-book-text-muted">
                          {VIDEO_WORKFLOW_MODE_LABELS[mode]}
                        </div>
                      </div>
                      <span className="text-caption text-success">已完成</span>
                    </li>
                    <li className="grid grid-cols-[24px_minmax(0,1fr)_auto] items-center gap-3 py-3">
                      <span
                        className={`flex h-6 w-6 items-center justify-center rounded-full text-caption font-semibold ${
                          ready
                            ? "bg-success/10 text-success"
                            : "bg-warning-bg text-warning"
                        }`}
                      >
                        2
                      </span>
                      <div className="min-w-0">
                        <div className="text-xs font-medium text-book-text-main">
                          准备生成输入
                        </div>
                        <div className="mt-0.5 text-caption text-book-text-muted">
                          {inputSummary}
                          {needsFramePreparation
                            ? ` · ${textFramePromptStatus}`
                            : ""}
                        </div>
                      </div>
                      <span
                        className={`text-caption ${ready ? "text-success" : "text-warning"}`}
                      >
                        {ready ? "已就绪" : "待处理"}
                      </span>
                    </li>
                    <li className="grid grid-cols-[24px_minmax(0,1fr)_auto] items-center gap-3 py-3">
                      <span className="flex h-6 w-6 items-center justify-center rounded-full bg-book-bg-tertiary text-caption font-semibold text-book-text-muted">
                        3
                      </span>
                      <div className="min-w-0">
                        <div className="text-xs font-medium text-book-text-main">
                          生成最终提示词
                        </div>
                        <div className="mt-0.5 text-caption text-book-text-muted">
                          生成后可编辑提示词并创建候选视频
                        </div>
                      </div>
                      <span className="text-caption text-book-text-muted">
                        未生成
                      </span>
                    </li>
                  </ol>

                  <div className="mt-4 flex flex-wrap justify-end gap-2">
                    {needsFramePreparation ? (
                      <BookButton
                        variant={
                          ready || framePromptsReady ? "secondary" : "primary"
                        }
                        size="sm"
                        disabled={acting != null || chapterGenerating}
                        onClick={() =>
                          void onGenerateFramePrompts({
                            targetShotNumber: selectedSegmentNumber,
                          })
                        }
                      >
                        <Wand2 size={12} />
                        {framePromptsReady
                          ? `重新${frameActionLabel}`
                          : frameActionLabel}
                      </BookButton>
                    ) : null}
                    {mode !== "reference_media" ? (
                      <BookButton
                        variant={ready ? "primary" : "secondary"}
                        size="sm"
                        disabled={acting != null || chapterGenerating || !ready}
                        title={finalPromptBlockedReason ?? undefined}
                        onClick={() =>
                          void onGeneratePromptRequests({
                            targetShotNumber: selectedSegmentNumber,
                          })
                        }
                      >
                        <Wand2 size={12} />
                        生成最终提示词
                      </BookButton>
                    ) : null}
                  </div>
                </div>

                {usesSystemFrames && mode === "boundary_frames" ? (
                  <div className="grid grid-cols-1 gap-3 border-t border-book-border/40 p-4 sm:grid-cols-2 sm:p-5">
                    {renderBoundaryFrameSlot(
                      "首帧",
                      startFramePrompt,
                      startFrameImage,
                    )}
                    {renderBoundaryFrameSlot(
                      "尾帧",
                      endFramePrompt,
                      endFrameImage,
                    )}
                  </div>
                ) : usesSystemFrames && mode === "storyboard" ? (
                  <div className="border-t border-book-border/40 p-4 sm:p-5">
                    {renderBoundaryFrameSlot(
                      "分镜板",
                      startFramePrompt,
                      startFrameImage,
                      "等待分镜板生成",
                    )}
                  </div>
                ) : usesSystemFrames &&
                  mode === "first_frame_chain" &&
                  selectedSegmentNumber === 1 ? (
                  <div className="border-t border-book-border/40 p-4 sm:p-5">
                    {renderBoundaryFrameSlot(
                      "首段首帧",
                      startFramePrompt,
                      startFrameImage,
                      "等待首帧生成",
                    )}
                  </div>
                ) : null}
              </section>
            </div>
          </div>
        );
      }
      return (
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <EmptyHint text="请先选择左侧片段卡片。" />
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

    const chatVideoMode = workflowMode === "chat_video";
    const firstFrameChainMode = workflowMode === "first_frame_chain";
    const storyboardMode = workflowMode === "storyboard";
    const guidanceConfigurationError = videoGuidanceConfigurationError(
      workflowMode,
      selectedSegmentNumber ?? selectedPrompt.shot_number,
      guidanceMode,
    );
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
      continuitySourceVideoId,
      actualStartFrameImage,
      endFrameImage,
      startFrameMissingMessage,
      frameInputMissingMessage,
    } =
      selectedPromptFrameInputState ?? frameInputStateForPrompt(selectedPrompt);
    const providerFrameInputMissingMessage =
      guidanceMode === "system_frames" || guidanceMode === "continuity"
        ? frameInputMissingMessage
        : null;
    const selectedPromptHasVideo = selectedPrompt.generated_video_id != null;
    const selectedPromptLockedReason = selectedPromptRunning
      ? "该视频提示词正在运行，不能编辑或保存"
      : null;
    const selectedPromptDownstreamNotice = selectedPromptHasVideo
      ? "该视频提示词已有下游视频；保存、优化或删除时需要选择删除下游或保留下游继续"
      : null;
    const durationConfigError = durationLimits.configError;
    const saveBlockedReason =
      (chapterGenerating
        ? "当前章节正在生成，不能保存视频提示词"
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
      (selectedPromptStale ? "当前视频提示词已过期，请重新生成" : undefined) ??
      (savedReferenceIdsError
        ? `保存的参考图数据异常：${savedReferenceIdsError}`
        : undefined) ??
      (staleSavedReferenceCount > 0
        ? `最终提示词有 ${staleSavedReferenceCount} 张冻结参考图已失效，请修改执行方案并重新生成提示词`
        : undefined) ??
      (selectedPromptDirty ? "请先保存当前更改" : undefined) ??
      guidanceConfigurationError ??
      providerFrameInputMissingMessage ??
      (selectedPromptRunning ? "当前视频提示词正在运行" : undefined) ??
      (chapterGenerating ? "当前章节正在生成，暂不能生成视频" : undefined) ??
      (!chapterContentReady ? "当前章节正文为空，不能生成视频" : undefined);
    const selectedImportBlockedReason =
      (selectedPrompt.prompt_type !== "video_prompt"
        ? "当前记录不是视频生成提示词"
        : undefined) ??
      (selectedPromptStale ? "当前视频提示词已过期，请重新生成" : undefined) ??
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
                disabled={
                  savingPrompt || selectedPromptRunning || chapterGenerating
                }
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
                          selectedPrompt.prompt_type !== "video_prompt" ||
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
        {providerFrameInputMissingMessage && (
          <div className="mb-2 text-small text-warning">
            {providerFrameInputMissingMessage}
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
          <div className="a4n-flat-surface flex min-h-40 flex-col items-center justify-center gap-2 px-4 py-8 text-center">
            <LoadingSpinner size="md" />
            <div className="text-sm font-medium text-book-text-main">
              当前提示词正在运行
            </div>
            <div className="max-w-md text-small leading-relaxed text-book-text-muted">
              运行结束后再继续编辑。生成前设置和辅助操作已暂时收起，避免误改当前任务输入。
            </div>
          </div>
        ) : (
          <>
            {chatVideoMode ||
            (guidanceMode !== "system_frames" &&
              guidanceMode !== "continuity") ? null : firstFrameChainMode ? (
              <div className="a4n-flat-surface mb-3 p-3">
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
                <div className="grid gap-2 md:grid-cols-2">
                  {selectedPrompt.shot_number <= 1
                    ? renderBoundaryFrameSlot(
                        "首段专用首帧",
                        startFramePrompt,
                        actualStartFrameImage,
                        "等待首帧生成",
                      )
                    : renderBoundaryFrameSlot(
                        "续接来源尾帧",
                        null,
                        actualStartFrameImage,
                        continuitySourceVideoId == null
                          ? "尚未选择续接来源视频"
                          : `等待视频 #${continuitySourceVideoId} 尾帧采集`,
                        continuitySourceVideoId == null
                          ? undefined
                          : `视频 #${continuitySourceVideoId}`,
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
                  <button
                    type="button"
                    onClick={() =>
                      startFramePrompt
                        ? void onGenerateFrameImage(startFramePrompt)
                        : void onGenerateFramePrompts()
                    }
                    className="pyqt-btn-secondary !min-h-9 !px-2.5 text-caption sm:!min-h-7"
                  >
                    {startFramePrompt ? "生成分镜板" : "准备分镜提示词"}
                  </button>
                </div>
                {frameInputMissingMessage && (
                  <div className="mb-2 text-caption text-warning">
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
                    <div className="text-xs font-medium text-book-text-main">
                      首尾帧
                    </div>
                    <div className="mt-0.5 text-caption text-book-text-muted">
                      {Number(Boolean(generatedStartFrameImage)) +
                        Number(Boolean(endFrameImage))}{" "}
                      / 2 已生成
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={() =>
                      endFramePrompt
                        ? void onGenerateFrameImage(endFramePrompt)
                        : void onGenerateFramePrompts()
                    }
                    className="pyqt-btn-secondary !min-h-9 !px-2.5 text-caption sm:!min-h-7"
                  >
                    {startFramePrompt && endFramePrompt
                      ? "生成缺失帧"
                      : "准备首尾帧提示词"}
                  </button>
                </div>
                <div className="grid grid-cols-2 gap-2">
                  {renderBoundaryFrameSlot(
                    "首帧",
                    startFramePrompt,
                    generatedStartFrameImage,
                  )}
                  {renderBoundaryFrameSlot(
                    "尾帧",
                    endFramePrompt,
                    endFrameImage,
                  )}
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
                  <div className="mt-0.5 text-caption text-book-text-muted">
                    {boundReferenceImages.length > 0
                      ? `${boundReferenceImages.length} 张已绑定`
                      : "尚未绑定图片"}
                  </div>
                </div>
              </div>
              {staleSavedReferenceCount > 0 && (
                <div className="mb-2 flex flex-wrap items-center gap-2 text-caption text-error">
                  <span>
                    最终提示词冻结的参考图里有 {staleSavedReferenceCount}{" "}
                    张失效图片。
                  </span>
                  <span className="text-book-text-muted">
                    请修改当前片段的参考素材并重新生成最终提示词；文字保存不会改写冻结引用。
                  </span>
                </div>
              )}
              {savedReferenceIdsError && (
                <div className="mb-2 text-caption text-error">
                  已保存参考图数据异常：{savedReferenceIdsError}
                  。请重新生成当前片段的最终提示词。
                </div>
              )}
              {renderBoundReferenceImages()}
            </div>

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
      {!selectedSegmentWorkspace ? (
        <div className="shrink-0 xl:hidden">
          <SegmentedControl
            ariaLabel="视频生成移动工作区"
            value={mobilePane}
            onChange={setMobilePane}
            options={[
              { id: "setup", label: "配置" },
              {
                id: "results",
                label: `候选${segmentVideos.length > 0 ? ` ${segmentVideos.length}` : ""}`,
              },
            ]}
          />
        </div>
      ) : null}
      <div
        className={`grid min-h-0 flex-1 gap-3 ${selectedSegmentWorkspace ? "grid-cols-1" : "xl:grid-cols-[340px_minmax(0,1fr)]"}`}
      >
        <div
          className={`min-h-0 ${selectedSegmentWorkspace || mobilePane === "setup" ? "block" : "hidden xl:block"}`}
        >
          <VideoPromptSetupPanel
            chapter={chapter}
            prompts={prompts}
            segmentCards={segmentCards}
            segmentRecipes={segmentRecipes}
            selectedSegmentNumber={selectedSegmentNumber}
            planningDraftVisible={planningDraftVisible}
            selectedPromptId={selectedPromptId}
            acting={acting}
            chapterGenerating={chapterGenerating}
            loading={loading}
            savingRecipeSegments={savingRecipeSegments}
            selectableReferenceImages={selectableReferenceImages}
            previousAdoptedVideos={previousAdoptedVideos}
            finalPromptBlockedReasons={finalPromptBlockedReasons}
            selectedSegmentMaterialsControl={selectedSegmentMaterialsControl}
            selectedSegmentWorkspace={selectedSegmentWorkspace}
            onLoad={onLoad}
            onSelectPrompt={selectPrompt}
            onSelectSegment={onSelectSegment}
            onWorkflowModeChange={onWorkflowModeChange}
            onUpdateGuidance={onUpdateGuidance}
            onDeletePrompt={onDeletePrompt}
          />
        </div>

        {!selectedSegmentWorkspace ? (
          <section
            aria-label="视频结果"
            className={`min-h-0 flex-col ${mobilePane === "results" ? "flex" : "hidden xl:flex"}`}
          >
            {!selectedPrompt ? (
              renderEditor()
            ) : (
              <VideoResultsPane
                videos={segmentVideos}
                selectedVideo={selectedVideo}
                selectedVideoId={selectedVideoId}
                adoptedVideoId={segmentRecipe?.adopted_video_id}
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
                onTrimStartChange={onTrimStartChange}
                onTrimEndChange={onTrimEndChange}
                onTrim={onTrim}
                onOpenSetup={() => setMobilePane("setup")}
              />
            )}
          </section>
        ) : null}
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
