import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  Eye,
  Image as ImageIcon,
  PanelTop,
  Plus,
  Save,
  SlidersHorizontal,
  Trash2,
  Upload,
  Wand2,
} from "lucide-react";
import { useI18n } from "../../../i18n";
import type { GeneratedImage } from "../../../api/image";
import {
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type UpdateVideoInput,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentWorkflowMode,
  type VideoTimelineSummary,
} from "../../../api/video";
import { BookButton } from "../../../components/ui/BookButton";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { Dropdown } from "../../../components/ui/Dropdown";
import { ImageZoomButton, ZoomableImage } from "../../../components/ui/ImageLightbox";
import { Modal } from "../../../components/ui/Modal";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { indexVideoFrameImages } from "./videoImageUtils";
import { type PromptDraft, type VideoDurationLimits } from "./videoTabConstants";
import { type VideoSegmentCardPlan } from "./videoSegmentPlan";
import { VideoPromptSetupPanel } from "./VideoPromptSetupPanel";
import type { InsertedSegmentGap } from "./InsertedSegmentTimelinePanel";
import { VideoResultsPane } from "./VideoResultsPane";
import {
  VideoGenerationEditor,
  type SegmentInputDialog,
} from "./VideoGenerationEditor";

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
  onLoad: () => void | boolean | Promise<boolean | void>;
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
  const { t } = useI18n();
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
  useEffect(() => {
    if (!segmentConfigOpen) setInputDialog(null);
  }, [segmentConfigOpen]);

  const inputDialogPromptId =
    inputDialog?.kind === "frame" ? inputDialog.promptId : null;
  const framePromptById = useMemo(
    () => new Map(framePrompts.map((prompt) => [prompt.id, prompt])),
    [framePrompts],
  );
  const inputDialogPrompt =
    inputDialogPromptId == null
      ? null
      : (framePromptById.get(inputDialogPromptId) ?? null);

  useEffect(() => {
    setFailedFrameMediaSrcs(new Set());
  }, [frameImages]);

  const framePromptByNumber = useMemo(() => {
    const map = new Map<number, ChapterVideoPrompt>();
    for (const prompt of framePrompts) map.set(prompt.shot_number, prompt);
    return map;
  }, [framePrompts]);
  const frameImageIndexes = useMemo(() => {
    return indexVideoFrameImages(framePrompts, frameImages);
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
      ? (frameImageIndexes.frameImageByPromptId.get(inputDialogPrompt.id) ?? null)
      : null;
  const inputDialogFrameSrc = inputDialogImage
    ? toMediaSrc(inputDialogImage.file_path)
    : "";
  const framePromptDirty = Boolean(
    inputDialogPrompt &&
    framePromptDraft.trim() !== inputDialogPrompt.visual_prompt.trim(),
  );
  const closeFrameDialog = async () => {
    if (framePromptDirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("video.chapter.imagePrompt"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    setInputDialog(null);
  };
  const frameImageLocked = inputDialogImage != null;
  const selectedContinuitySourceAvailable = previousAdoptedVideos.some(
    (video) => video.id === selectedContinuitySourceVideoId,
  );

  const renderFrameSlot = (
    label: string,
    prompt: ChapterVideoPrompt | null,
    image: GeneratedImage | null,
    emptyText = t("video.chapter.waitingBoundary"),
  ) => {
    const src = image ? toMediaSrc(image.file_path) : "";
    const mediaReady = Boolean(src && !failedFrameMediaSrcs.has(src));
    const mediaUnavailable = image != null && !mediaReady;
    return (
      <div className="relative min-w-0">
      <button
        type="button"
        aria-label={t("video.chapter.configureFrame", { label })}
        onClick={() => {
          setFramePromptDraft(prompt?.visual_prompt ?? "");
          setInputDialog({
            kind: "frame",
            label,
            promptId: prompt?.id ?? null,
            emptyText,
          });
        }}
        className={`group w-full min-w-0 overflow-hidden rounded-xl border text-left transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary ${
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
              className="h-full w-full object-cover"
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
                  ? t("video.chapter.imageLoadFailedShort")
                  : prompt
                    ? t("video.chapter.generateOrImport", { label })
                    : t("video.chapter.prepare", { label })}
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
              {mediaReady ? t("video.chapter.filled") : mediaUnavailable ? t("video.chapter.unavailable") : t("video.chapter.required")}
            </span>
          </span>
          <span className="mt-0.5 block truncate text-caption text-book-text-muted">
            {prompt?.title || prompt?.scene_summary || emptyText}
          </span>
        </span>
      </button>
      {mediaReady ? <ImageZoomButton src={src} alt={label} className="absolute right-2 top-2" /> : null}
      </div>
    );
  };

  const renderContinuitySlot = () => {
    const mediaReady = Boolean(
      selectedContinuityTailFrameSrc &&
      !failedFrameMediaSrcs.has(selectedContinuityTailFrameSrc),
    );
    const mediaUnavailable = selectedContinuityTailFrame != null && !mediaReady;
    return (
      <div className="relative min-w-0">
      <button
        type="button"
        aria-label={t("video.chapter.configureContinuity")}
        onClick={() => setInputDialog({ kind: "continuity" })}
        className={`group w-full min-w-0 overflow-hidden rounded-xl border text-left transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary ${
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
              className="h-full w-full object-cover"
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
                  ? t("video.chapter.imageLoadFailedShort")
                  : t("video.chapter.chooseEarlierVideo")}
              </span>
            </span>
          )}
        </span>
        <span className="block min-w-0 p-2">
          <span className="flex min-w-0 items-center gap-2">
            <span className="min-w-0 flex-1 truncate text-small font-medium text-book-text-main">
              {t("video.chapter.configureContinuity")}
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
              {mediaReady ? t("video.chapter.filled") : mediaUnavailable ? t("video.chapter.unavailable") : t("video.chapter.required")}
            </span>
          </span>
          <span className="mt-0.5 block truncate text-caption text-book-text-muted">
            {selectedContinuitySourceVideoId == null
              ? t("video.chapter.noSourceVideo")
              : t("video.chapter.sourceVideoTail", { id: selectedContinuitySourceVideoId })}
          </span>
        </span>
      </button>
      {mediaReady ? <ImageZoomButton src={selectedContinuityTailFrameSrc} alt={t("video.chapter.configureContinuity")} className="absolute right-2 top-2" /> : null}
      </div>
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
            {t("video.chapter.startFrameSource")}
          </div>
          <div className="mt-0.5 text-caption text-book-text-muted">
            {t("video.chapter.startFrameSourceHelp")}
          </div>
        </div>
        <SegmentedControl<"continuity" | "system_frames">
          ariaLabel={t("video.chapter.startFrameSource")}
          value={value}
          options={[
            { id: "continuity", label: t("video.chapter.continuityTail"), disabled },
            { id: "system_frames", label: t("video.chapter.independentFrame"), disabled },
          ]}
          onChange={(guidanceMode) =>
            void onFirstFrameSourceChange(selectedSegmentNumber, guidanceMode)
          }
          className={disabled ? "opacity-60" : ""}
        />
      </div>
    );
  };


  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="shrink-0 xl:hidden">
        <SegmentedControl
          ariaLabel={t("video.chapter.mobileWorkspace")}
          value={mobilePane}
          onChange={setMobilePane}
          options={[
            { id: "segments", label: t("video.chapter.segmentTab") },
            {
              id: "results",
              label: videos.length > 0 ? t("video.chapter.videoTabCount", { count: videos.length }) : t("video.chapter.videoTab"),
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
          aria-label={t("video.chapter.videoPreview")}
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
          title={t("video.chapter.segmentConfig", { number: selectedSegmentNumber })}
          subtitle={selectedSegmentCard?.title ?? t("video.chapter.segmentNotInPlan")}
          icon={<SlidersHorizontal size={15} />}
          maxWidthClassName="max-w-6xl"
          className="h-[calc(100dvh-0.75rem)] sm:h-[90vh]"
          bodyClassName="min-h-0 flex-1 overflow-hidden"
        >
          <div className="flex h-full min-h-0 flex-col gap-3 p-3 lg:p-4">
            <section
              aria-label={t("video.chapter.segmentConfigOptions")}
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
              aria-label={t("video.chapter.segmentGenerationWorkspace")}
              className="flex min-h-0 flex-1 flex-col overflow-hidden"
            >
              {workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA ? (
                referenceMediaWorkspace
              ) : (
                <div className="flex h-full min-h-0 flex-col overflow-hidden">
                  <VideoGenerationEditor
                    selectedSegmentNumber={selectedSegmentNumber}
                    selectedSegmentCard={selectedSegmentCard}
                    planningDraftVisible={planningDraftVisible}
                    finalPromptBlockedReason={finalPromptBlockedReason}
                    workflowMode={workflowMode}
                    segmentRecipe={segmentRecipe}
                    selectedPrompt={selectedPrompt}
                    selectedPromptStale={selectedPromptStale}
                    framePromptByNumber={framePromptByNumber}
                    frameImageIndexes={frameImageIndexes}
                    referenceInputs={referenceInputs}
                    acting={acting}
                    selectedPromptRunning={selectedPromptRunning}
                    selectedPromptDirty={selectedPromptDirty}
                    staleSavedReferenceCount={staleSavedReferenceCount}
                    savedReferenceIdsError={savedReferenceIdsError}
                    chapterContentReady={chapterContentReady}
                    durationLimits={durationLimits}
                    draft={draft}
                    renderFrameSlot={renderFrameSlot}
                    renderContinuitySlot={renderContinuitySlot}
                    renderFirstFrameSourceSelector={renderFirstFrameSourceSelector}
                    onGeneratePromptRequests={onGeneratePromptRequests}
                    onSavePrompt={onSavePrompt}
                    onCopyPrompt={onCopyPrompt}
                    onGenerateVideo={onGenerateVideo}
                    onOptimizePrompt={onOptimizePrompt}
                    onPreviewGenerateVideo={onPreviewGenerateVideo}
                    onImportVideo={onImportVideo}
                    onDeletePrompt={onDeletePrompt}
                    onDraftChange={onDraftChange}
                  />
                </div>
              )}
            </section>
          </div>
        </Modal>
      ) : null}

      <Modal
        isOpen={segmentConfigOpen && inputDialog?.kind === "frame"}
        onClose={() => void closeFrameDialog()}
        title={
          inputDialog?.kind === "frame" ? t("video.chapter.frameConfigTitle", { label: inputDialog.label }) : t("video.chapter.frameConfig")
        }
        subtitle={t("video.chapter.frameConfigSubtitle")}
        icon={<ImageIcon size={15} />}
        maxWidthClassName="max-w-2xl"
        zIndexClassName="z-[70]"
        footer={
          inputDialog?.kind === "frame" && selectedSegmentNumber != null ? (
            inputDialogPrompt ? (
              <div className="flex w-full flex-wrap items-center justify-end gap-2">
                <Dropdown items={[
                  {
                    label: t("video.chapter.optimizePrompt"),
                    icon: <Wand2 size={12} />,
                    disabled: acting != null || frameImageLocked || framePromptDirty || inputDialogPrompt.status === "running",
                    onClick: async () => {
                      const updated = await onOptimizePrompt(inputDialogPrompt);
                      if (updated) setFramePromptDraft(updated.visual_prompt);
                    },
                  },
                  ...(inputDialogImage ? [{
                    label: t("video.chapter.deleteFrameImage", { label: inputDialog.label }),
                    icon: <Trash2 size={12} />,
                    danger: true,
                    disabled: acting != null || imageBusyId === inputDialogImage.id,
                    onClick: () => void onDeleteFrameImage(inputDialogImage),
                  }] : []),
                ]} />
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
                      ? t("video.chapter.deleteImageBefore", { action: t("common.edit") })
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
                  {t("video.chapter.saveChanges")}
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={acting != null || framePromptDirty}
                  title={
                    framePromptDirty
                      ? t("video.chapter.saveBeforeAction")
                      : undefined
                  }
                  onClick={() => {
                    setInputDialog(null);
                    void onPreviewFrameImage(inputDialogPrompt);
                  }}
                >
                  <Eye size={12} />
                  {t("video.chapter.previewRequest")}
                </BookButton>
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={
                    acting != null || frameImageLocked || framePromptDirty
                  }
                  title={
                    frameImageLocked
                      ? t("video.chapter.deleteImageBefore", { action: t("common.import") })
                      : framePromptDirty
                        ? t("video.chapter.saveBeforeAction")
                        : undefined
                  }
                  onClick={() => {
                    setInputDialog(null);
                    void onImportFrameImage(inputDialogPrompt);
                  }}
                >
                  <Upload size={12} />
                  {t("common.import")} {t("video.chapter.image")}
                </BookButton>
                <BookButton
                  variant="primary"
                  size="sm"
                  disabled={
                    acting != null || frameImageLocked || framePromptDirty
                  }
                  title={
                    frameImageLocked
                      ? t("video.chapter.deleteImageBefore", { action: t("video.chapter.generateVideo") })
                      : framePromptDirty
                        ? t("video.chapter.saveBeforeAction")
                        : undefined
                  }
                  onClick={() => {
                    setInputDialog(null);
                    void onGenerateFrameImage(inputDialogPrompt);
                  }}
                >
                  <Wand2 size={12} />
                  {t("video.chapter.generateImage")}
                </BookButton>
              </div>
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
                {t("video.chapter.preparePrompt", { label: inputDialog.label })}
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
                <ZoomableImage
                  src={inputDialogFrameSrc}
                  alt=""
                  className="h-full w-full object-contain"
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
                      ? t("video.chapter.imageLoadFailed")
                      : inputDialog.emptyText}
                  </span>
                </span>
              )}
            </div>
            <div className="rounded-xl border border-book-border/45 bg-book-bg-paper/45 px-3 py-2.5">
              <label className="block">
                <span className="text-xs font-medium text-book-text-main">
                  {inputDialogPrompt
                    ? `${t("video.chapter.frameNumber", { number: inputDialogPrompt.shot_number })} · ${t("video.chapter.imagePrompt")}`
                    : t("video.chapter.noFramePrompt")}
                </span>
                {inputDialogPrompt ? (
                  <textarea
                    aria-label={`${inputDialog.label}${t("video.chapter.imagePrompt")}`}
                    value={framePromptDraft}
                    onChange={(event) =>
                      setFramePromptDraft(event.target.value)
                    }
                    readOnly={frameImageLocked}
                    disabled={
                      acting != null ||
                      inputDialogPrompt.status === "running"
                    }
                    title={
                      frameImageLocked
                        ? t("video.chapter.deleteImageBefore", { action: t("common.edit") })
                        : undefined
                    }
                    className="book-control mt-2 min-h-[150px] w-full resize-y p-2 text-xs leading-relaxed"
                  />
                ) : (
                  <span className="mt-1 block text-small leading-relaxed text-book-text-muted">
                    {t("video.chapter.preparePromptHint", { label: inputDialog.label })}
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
        title={t("video.chapter.continuityConfigTitle")}
        subtitle={t("video.chapter.continuityConfigSubtitle")}
        icon={<PanelTop size={15} />}
        maxWidthClassName="max-w-2xl"
        zIndexClassName="z-[70]"
      >
        {inputDialog?.kind === "continuity" ? (
          <div className="space-y-3">
            <div className="flex aspect-video items-center justify-center overflow-hidden rounded-xl bg-book-bg-tertiary text-small text-book-text-muted">
              {selectedContinuityTailFrameSrc &&
              !failedFrameMediaSrcs.has(selectedContinuityTailFrameSrc) ? (
                <ZoomableImage
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
                      ? t("video.chapter.noSourceVideo")
                      : selectedContinuityTailFrame
                        ? t("video.chapter.imageLoadFailedChooseSource")
                        : t("video.chapter.sourceVideoNoTail", { id: selectedContinuitySourceVideoId })}
                  </span>
                </span>
              )}
            </div>
            <label className="block">
              <span className="mb-1 block text-xs text-book-text-muted">
                {t("video.chapter.sourceVideo")}
              </span>
              <ThemedSelect
                value={selectedContinuitySourceVideoId ?? ""}
                disabled={
                  acting != null ||
                  selectedSegmentNumber == null ||
                  savingRecipeSegments.has(selectedSegmentNumber)
                }
                onChange={(selectedValue) => {
                  if (selectedSegmentNumber == null) return;
                  void onContinuitySourceChange(
                    selectedSegmentNumber,
                    Number(selectedValue) || null,
                  );
                }}
                className="book-control h-10 w-full px-3 text-small"
              >
                <option value="">{t("video.chapter.chooseEarlierAdoptedVideo")}</option>
                {selectedContinuitySourceVideoId != null &&
                !selectedContinuitySourceAvailable ? (
                  <option value={selectedContinuitySourceVideoId}>
                    {t("video.chapter.invalidSourceVideo", { id: selectedContinuitySourceVideoId })}
                  </option>
                ) : null}
                {previousAdoptedVideos.map((video) => (
                  <option key={video.id} value={video.id}>
                    {t("video.chapter.segment", { number: String(
                      (video.metadata_json as Record<string, unknown> | null)?.shot_number ?? "?",
                    ) })}{" "}
                    · {t("video.chapter.videoNumber", { id: video.id })}
                  </option>
                ))}
              </ThemedSelect>
            </label>
            <div className="text-caption leading-relaxed text-book-text-muted">
              {t("video.chapter.selectSourceSavedHint")}
            </div>
          </div>
        ) : null}
      </Modal>
    </div>
  );
};
