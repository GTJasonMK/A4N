import React, {
  lazy,
  Suspense,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Clapperboard,
  FileText,
  Film,
  ImagePlus,
  PanelTop,
  RotateCcw,
  Scissors,
} from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import {
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_SEGMENT_WORKFLOW_MODES,
  type GeneratedVideo,
  type VideoRecordDeleteResult,
} from "../../api/video";
import type { GeneratedImage, ImageRecordDeleteResult } from "../../api/image";
import { EmptyHint, type ChapterRow } from "./shared";
import { useProgressTask } from "../../hooks/useProgressTask";
import { VideoGenerationWorkspace } from "./video/VideoGenerationWorkspace";
import {
  InsertedSegmentTimelinePanel,
} from "./video/InsertedSegmentTimelinePanel";
import {
  ReferenceMediaWorkspace,
  VideoReferenceInputSlots,
} from "./video/referenceMediaWorkspace";
import {
  MAX_CHAT_REFERENCE_IMAGES,
  MAX_REFERENCE_IMAGES,
  videoWorkflowModeLabel,
} from "./video/videoTabConstants";
import { useChapterContentHash } from "./video/useChapterContentHash";
import { useVideoImageActions } from "./video/useVideoImageActions";
import { useVideoPromptGenerationActions } from "./video/useVideoPromptGenerationActions";
import {
  useVideoRecordActions,
} from "./video/useVideoRecordActions";
import { useVideoTabDerivedState } from "./video/useVideoTabDerivedState";
import { useI18n } from "../../i18n";
import { useVideoTabState } from "./video/useVideoTabState";
import { useVideoTabLoading } from "./video/useVideoTabLoading";
import { useVideoTabViewState } from "./video/useVideoTabViewState";
import { MediaStageHeader } from "../../components/shell/MediaStageHeader";
import { confirmDiscardMediaChanges } from "../../components/feedback/UnsavedChangesGuard";
import { useVideoTabActions } from "./video/useVideoTabActions";
import {
  useNovelDetailDirty,
  useNovelDetailDirtyRegistry,
} from "./novelDetailDirty";
import { useDirtyRegistryVersion } from "../../components/navigation/dirtyRegistry";
import { BaseMaterialsWorkspace } from "../derived/BaseMaterialsWorkspace";

const ChapterVisualPlanTab = lazy(() => import("./ChapterVisualPlanTab").then((module) => ({ default: module.ChapterVisualPlanTab })));
const ShootingScriptTab = lazy(() => import("./ShootingScriptTab").then((module) => ({ default: module.ShootingScriptTab })));

const VideoPlanningWorkspace = lazy(() =>
  import("./video/VideoPlanningWorkspace").then((module) => ({
    default: module.VideoPlanningWorkspace,
  })),
);
const VideoAssemblyWorkspace = lazy(() =>
  import("./video/VideoAssemblyWorkspace").then((module) => ({
    default: module.VideoAssemblyWorkspace,
  })),
);
const VideoPostProcessWorkspace = lazy(() =>
  import("./video/VideoPostProcessWorkspace").then((module) => ({
    default: module.VideoPostProcessWorkspace,
  })),
);
const RequestPreviewModal = lazy(() =>
  import("../../components/feedback/RequestPreviewModal").then((module) => ({
    default: module.RequestPreviewModal,
  })),
);

interface VideoTabProps {
  onDirtyChange?: (dirty: boolean) => void;
  projectId: number;
  canEdit?: boolean;
  fixedChapter?: ChapterRow | null;
  imageProjectIds?: number[];
  materialProjectId?: number;
}

export const VideoTab: React.FC<VideoTabProps> = ({
  onDirtyChange,
  projectId,
  canEdit = true,
  fixedChapter,
  imageProjectIds,
  materialProjectId,
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const translatorRef = useRef(t);
  translatorRef.current = t;
  const localeRef = useRef(locale);
  localeRef.current = locale;
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  useDirtyRegistryVersion();
  const [authoringDirty, setAuthoringDirty] = useState(false);
  const [authoringBusy, setAuthoringBusy] = useState(false);
  const workspaceTabs = useMemo(
    () => [
      { id: "materials" as const, label: t("common.mediaAuthoring.materials"), icon: <ImagePlus size={13} /> },
      { id: "director" as const, label: t("writing.visualPlan"), icon: <Clapperboard size={13} /> },
      { id: "screenplay" as const, label: t("shootingScript.title"), icon: <FileText size={13} /> },
      { id: "frames" as const, label: t("video.chapter.tabPlanning"), icon: <PanelTop size={13} /> },
      { id: "video" as const, label: t("video.chapter.tabGeneration"), icon: <Film size={13} /> },
      { id: "assembly" as const, label: t("video.chapter.tabAssembly"), icon: <Clapperboard size={13} /> },
      { id: "postprocess" as const, label: t("video.chapter.tabPostprocess"), icon: <Scissors size={13} /> },
    ],
    [t],
  );
  const { runProgressTask } = useProgressTask();
  const chapter = fixedChapter ?? null;
  const chapterId = chapter?.id ?? null;
  const profileMaterialProjectId = materialProjectId ?? projectId;
  const authoringChapter = chapter ? { id: chapter.id, project_id: profileMaterialProjectId, chapter_number: chapter.chapter_number, title: chapter.title ?? null } : null;
  const stageId = `video-stages-${projectId}`;
  const chapterDataKey =
    chapter && Number.isFinite(chapter.id)
      ? `${projectId}:${chapter.id}`
      : null;
  const imageProjectIdsKey = (imageProjectIds ?? [projectId])
    .filter((id) => Number.isFinite(id) && id > 0)
    .join(",");
  const videoDataKey = chapterDataKey
    ? `${chapterDataKey}:${imageProjectIdsKey}`
    : null;
  const chapterStorageKey =
    chapter && Number.isFinite(chapter.id)
      ? `a4n:video:${projectId}:${chapter.id}`
      : null;
  const { currentContentHash, chapterContentReady, chapterContentHashError } =
    useChapterContentHash(chapter);

  const videoTabState = useVideoTabState({
    projectId,
    chapterId,
    chapterStorageKey,
    videoDataKey,
    currentContentHash,
    workspaceTabs,
    t,
    addToast,
  });
  const { recipeOverridesDirty, discardUnsavedRecipeOverrides } = videoTabState;
  const [segmentPlanDirty, setSegmentPlanDirty] = React.useState(false);
  const {
    planningOptionsState,
    planningOptions,
    setPlanningOptions,
    resetPlanningOptions,
    acting,
    videoActionScopeKey,
    beginVideoAction,
    isCurrentVideoActionScope,
    finishVideoAction,
    workspaceTab,
    setWorkspaceTab,
    prompts,
    setPrompts,
    timeline,
    insertedSegmentPanelRef,
    segmentPlanPreview,
    setSegmentPlanPreview,
    segmentPlanDraft,
    videos,
    setVideos,
    images,
    setImages,
    shootingScenes,
    frameSegmentPlanPromptForCards,
    segmentPlanDraftVisible,
    segmentCardState,
    selectedSegmentNumber,
    segmentConfigOpen,
    savingRecipeSegments,
    effectiveRecipeBySegment,
    recipeFingerprintBySegment,
    frameRecipeFingerprintBySegment,
    selectedSegmentRecipe,
    workflowMode,
    segmentStorageKey,
    selectedPromptId,
    setSelectedPromptId,
    selectedVideoId,
    setSelectedVideoId,
    setPendingVideoId,
    options,
    videoLibraryImageIds,
    generationReferenceIds,
    draft,
    setDraft,
    referenceMediaDraft,
    referenceMediaDraftDirty,
    loading,
    videoDataError,
    activeVideoConfig,
    segmentPlanDraftLoading,
    segmentPlanDraftError,
    promptRequestPreviewOpen,
    promptRequestPreviewLoading,
    promptRequestPreviewData,
    imageRequestPreviewOpen,
    imageRequestPreviewLoading,
    imageRequestPreviewData,
  } = videoTabState;
  const videoDataReadUnavailable = loading || videoDataError != null;
  const actionAvailabilityRef = useRef({ canEdit, videoDataReadUnavailable, videoDataError });
  actionAvailabilityRef.current = { canEdit, videoDataReadUnavailable, videoDataError };
  const ensureVideoActionAvailable = () => {
    const current = actionAvailabilityRef.current;
    if (!current.canEdit) {
      addToast(t("video.noEditPermission"), "error");
      return false;
    }
    if (current.videoDataReadUnavailable) {
      addToast(current.videoDataError ?? t("common.operationInProgress"), "error");
      return false;
    }
    return true;
  };
  const beginVideoActionGuarded = (key: string): boolean => {
    if (!ensureVideoActionAvailable()) return false;
    return beginVideoAction(key);
  };
  const derivedVideoTabState = useVideoTabDerivedState({
    prompts,
    videos,
    images,
    chapter,
    selectedPromptId,
    selectedVideoId,
    workflowMode,
    promptOutputLanguage: options.promptOutputLanguage,
    currentContentHash,
    frameSegmentPlanPrompt: frameSegmentPlanPromptForCards,
    activeVideoConfig,
    selectedSegmentNumber,
    guidanceMode: selectedSegmentRecipe?.guidance_mode ?? null,
    recipeFingerprint:
      selectedSegmentNumber == null
        ? null
        : (recipeFingerprintBySegment.get(selectedSegmentNumber) ?? null),
    frameRecipeFingerprint:
      selectedSegmentNumber == null
        ? null
        : (frameRecipeFingerprintBySegment.get(selectedSegmentNumber) ?? null),
    continuitySourceVideoId:
      selectedSegmentRecipe?.continuity_source_video_id ?? null,
    t,
  });
  const {
    finalVideoPrompts,
    selectedPrompt,
    currentLanguageFramePrompts,
    selectedVideo,
    selectedVideoPath,
    selectedPromptStale,
    selectedPromptRunning,
    workflowVideos,
    selectableReferenceImages,
    selectableReferenceIds,
    validPromptReferenceIds,
    staleSavedReferenceCount,
    savedReferenceIdsError,
    videoFrameImages,
    durationLimits,
    } = derivedVideoTabState;

  const { loadAll, refreshPrompts, refreshSegmentPlanDraft } = useVideoTabLoading({
    ...videoTabState,
    projectId,
    chapter,
    chapterId,
    videoDataKey,
    imageProjectIds,
    addToast,
    runProgressTask,
    t,
    finalVideoPrompts,
    workflowVideos,
    selectedPrompt,
    selectedVideo,
  });
  const refreshVideoDataInBackground = () => loadAll({ showLoading: false });
  const applyVideoRecordDeletion = (result: VideoRecordDeleteResult) => {
    const deletedPromptIds = new Set(result.deleted_prompt_ids);
    const deletedVideoIds = new Set(result.deleted_video_ids);
    const updatedPrompts = new Map(result.updated_prompts.map((prompt) => [prompt.id, prompt]));
    const updatedVideos = new Map(result.updated_videos.map((video) => [video.id, {
      ...video,
      workflow_mode: VIDEO_SEGMENT_WORKFLOW_MODES.includes(
        video.workflow_mode as (typeof VIDEO_SEGMENT_WORKFLOW_MODES)[number],
      )
        ? video.workflow_mode as GeneratedVideo["workflow_mode"]
        : null,
    }]));
    setPrompts((current) => current
      .filter((prompt) => !deletedPromptIds.has(prompt.id))
      .map((prompt) => updatedPrompts.get(prompt.id) ?? prompt));
    setVideos((current) => current
      .filter((video) => !deletedVideoIds.has(video.id))
      .map((video) => updatedVideos.get(video.id) ?? video));
    setSelectedPromptId((current) => current != null && deletedPromptIds.has(current) ? null : current);
    setSelectedVideoId((current) => current != null && deletedVideoIds.has(current) ? null : current);
    setPendingVideoId((current) => current != null && deletedVideoIds.has(current) ? null : current);
  };
  const applyImageRecordDeletion = (result: ImageRecordDeleteResult) => {
    const updatedPrompts = new Map(result.updated_prompts.map((prompt) => [prompt.id, prompt]));
    const updatedVideos = new Map(result.updated_videos.map((video) => [video.id, {
      ...video,
      workflow_mode: VIDEO_SEGMENT_WORKFLOW_MODES.includes(
        video.workflow_mode as (typeof VIDEO_SEGMENT_WORKFLOW_MODES)[number],
      ) ? video.workflow_mode as GeneratedVideo["workflow_mode"] : null,
    }]));
    setImages((current) => current.filter((image) => image.id !== result.deleted_image_id));
    setPrompts((current) => current.map((prompt) => updatedPrompts.get(prompt.id) ?? prompt));
    setVideos((current) => current.map((video) => updatedVideos.get(video.id) ?? video));
  };
  const {
    imageBusyId,
    handleDeleteImage,
  } = useVideoImageActions({
    t,
    addToast,
    actionScopeKey: videoActionScopeKey,
    loadAll,
    ensureActionAvailable: ensureVideoActionAvailable,
    onImageDeleted: applyImageRecordDeletion,
  });
  const handleDeleteImageGuarded = (image: GeneratedImage) => {
    if (!ensureVideoActionAvailable()) return;
    return handleDeleteImage(image);
  };




  const viewState = useVideoTabViewState({
    ...videoTabState,
    ...derivedVideoTabState,
    chapter,
    currentContentHash,
    chapterContentHashError,
    addToast,
    ensureActionAvailable: ensureVideoActionAvailable,
    t,
    loadAll,
  });
  const {
    selectedPromptDirty,
    previousAdoptedVideos,
    mixedAdoptedVideoIds,
    unavailableMixedSegmentNumbers,
    allCurrentFramePrompts,
    allSegmentPrompts,
    selectionValidationReady,
    videoDataLoading,
    invalidGenerationReferenceIds,
    invalidVideoLibraryImageIds,
    promptInputSelectionError,
    videoPersistedStateError,
    effectiveDurationLimits,
    setReferenceMediaPrompt,
    setReferenceMediaImageRows,
    setReferenceMediaVideoRows,
    setReferenceMediaAudioRows,
    setGenerationReferenceIds,
    setVideoLibraryImageIds,
    finalPromptBlockedReasons,
    handleSelectPrompt,
    handleActivateSegment,
    handleCloseSegmentConfig,
  } = viewState;
  const selectedSegmentNumberRef = useRef(selectedSegmentNumber);
  selectedSegmentNumberRef.current = selectedSegmentNumber;

  const dirty = authoringDirty || selectedPromptDirty || referenceMediaDraftDirty || segmentPlanDirty || recipeOverridesDirty;
  const discardStageDrafts = () => {
    if (!discardUnsavedRecipeOverrides()) return false;
    setAuthoringDirty(false);
    setSegmentPlanDirty(false);
    viewState.discardSelectedPromptDraft();
    videoTabState.setReferenceMediaDraftDirty(false);
  };
  useEffect(() => { onDirtyChange?.(dirty); }, [dirty, onDirtyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  useNovelDetailDirty(
    `video:${projectId}:${chapterId ?? "none"}`,
    dirty,
    discardStageDrafts,
    () => !authoringBusy && acting == null && savingRecipeSegments.size === 0,
  );
  const handleWorkspaceTabChange = async (nextTab: typeof workspaceTab) => {
    if (nextTab === workspaceTab) return true;
    if (authoringBusy || acting != null || savingRecipeSegments.size > 0) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    if (!await confirmWorkspaceDraftDiscard()) return false;
    if (workspaceTab === "materials") void refreshVideoDataInBackground();
    setWorkspaceTab(nextTab);
    return true;
  };

  const confirmWorkspaceDraftDiscard = async () => {
    const hasDirtyState = dirty || dirtyRegistry.hasDirty();
    if (!hasDirtyState) return true;
    if (!dirtyRegistry.canDiscardDirty()) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    if (!await confirmDiscardMediaChanges(true, t)) return false;
    const videoOwnerDirty = dirtyRegistry.hasDirty(`video:${projectId}:${chapterId ?? "none"}`);
    if (!await dirtyRegistry.discardDirty()) return false;
    if (dirty && !videoOwnerDirty) discardStageDrafts();
    return true;
  };

  const clearInvalidGenerationReferences = async () => {
    if (invalidGenerationReferenceIds.length === 0) return;
    const next = generationReferenceIds.filter((id) =>
      selectableReferenceIds.has(id),
    );
    if (await setGenerationReferenceIds(next)) {
      addToast(t("video.reference.clearInvalid"), "success");
    }
  };
  const clearInvalidVideoLibraryImages = async () => {
    if (invalidVideoLibraryImageIds.length === 0) return;
    const next = videoLibraryImageIds.filter((id) =>
      selectableReferenceIds.has(id),
    );
    if (await setVideoLibraryImageIds(next)) {
      addToast(t("video.reference.clearInvalid"), "success");
    }
  };
  const {
    trimStart,
    trimEnd,
    setTrimStart,
    setTrimEnd,
    handleOptimizePromptRow,
    handleDeletePromptRow,
    handleSavePrompt,
    handleSavePromptRow,
    handleGenerateVideo,
    handleImportVideo,
    handlePreviewGenerateVideoRequest,
    handlePreviewVideoRequestForVideo,
    handlePollVideo,
    handleToggleVideoFavorite,
    handleUpdateVideo,
    handleDeleteVideo,
    handleTrim,
    requestPreviewOpen: videoRequestPreviewOpen,
    requestPreviewLoading: videoRequestPreviewLoading,
    requestPreviewData: videoRequestPreviewData,
    closeRequestPreview: closeVideoRequestPreview,
  } = useVideoRecordActions({
    projectId,
    chapter,
    selectedPrompt,
    selectedPromptId,
    frameStageLabel:
      workflowMode == null
        ? t("video.chapter.tabGeneration")
        : videoWorkflowModeLabel(workflowMode, t),
    selectedPromptStale,
    currentContentHash,
    selectedVideo,
    previewScopeKey: segmentStorageKey,
    actionScopeKey: videoActionScopeKey,
    prompts,
    frameImages: videoFrameImages,
    draft,
    selectedPromptDirty,
    validPromptReferenceIds,
    durationLimits: effectiveDurationLimits,
    t,
    addToast,
    runProgressTask,
    loadAll,
    ensureActionAvailable: ensureVideoActionAvailable,
    setPrompts,
    setImages,
    setVideos,
    setDraft,
    setSelectedPromptId,
    setSelectedVideoId,
    setPendingVideoId,
    onVideoRecordDeleted: applyVideoRecordDeletion,
    beginAction: beginVideoActionGuarded,
    isCurrentActionScope: isCurrentVideoActionScope,
    finishAction: (key) => finishVideoAction(key, videoActionScopeKey),
  });

  const {
    handleGenerateSegmentPlan,
    handleGenerateFramePrompts: runGenerateFramePrompts,
    handleGenerateVideoPrompts: runGenerateVideoPrompts,
    handleGenerateFrameImages,
    handleImportFrameImage,
  } = useVideoPromptGenerationActions({
    projectId,
    chapter,
    options,
    segmentPlanOptions: planningOptions,
    durationLimits,
    promptInputSelectionError,
    referenceIds: generationReferenceIds,
    videoLibraryImageIds,
    frameImages: videoFrameImages,
    segmentPlanPrompt: frameSegmentPlanPromptForCards,
    segmentPlanDraft,
    framePrompts: currentLanguageFramePrompts,
    allFramePrompts: allCurrentFramePrompts,
    allFinalVideoPrompts: allSegmentPrompts,
    setImages,
    currentContentHash,
    actionScopeKey: videoActionScopeKey,
    locale,
    t,
    getTranslator: () => translatorRef.current,
    getLocale: () => localeRef.current,
    addToast,
    runProgressTask,
    loadAll,
    refreshPrompts,
    refreshSegmentPlanDraft,
    setSegmentPlanPreview,
    beginAction: beginVideoActionGuarded,
    isCurrentActionScope: isCurrentVideoActionScope,
    finishAction: (key) => finishVideoAction(key, videoActionScopeKey),
  });

  const {
    handleSaveSegmentPlan,
    handleGenerateFramePrompts,
    handleGenerateVideoPrompts,
    handleAdoptSegmentVideo,
    handleSegmentWorkflowModeChange,
    handleFirstFrameSourceChange,
    handleContinuitySourceChange,
    handleGenerateInsertedSegment,
    handleCommitInsertedSegment,
    handleMoveInsertedSegment,
    handleDeleteInsertedSegment,
    handleGenerateInsertedVideo,
    handleUpdateInsertedPrompt,
    handleCopyChatVideoPrompt,
    handlePreviewFrameImageRequest,
    handlePreviewReferenceMediaVideoRequest,
    closePromptRequestPreview,
    closeImageRequestPreview,
    handleSaveReferenceMediaDraft,
    handleGenerateReferenceMediaVideo,
    handleImportReferenceMediaVideo,
  } = useVideoTabActions({
    ...videoTabState,
    ...derivedVideoTabState,
    ...viewState,
    ensureVideoActionAvailable,
    beginVideoAction: beginVideoActionGuarded,
    selectedSegmentNumberRef,
    projectId,
    chapter,
    runProgressTask,
    loadAll,
    t,
    addToast,
    runGenerateFramePrompts,
    runGenerateVideoPrompts,
    handleSavePromptRow,
  });

  if (!chapter) {
    return <EmptyHint text={t("video.chapter.noChapterSelectedForVideo")} />;
  }

  const referenceMediaWorkspace =
    workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA ? (
      <ReferenceMediaWorkspace
        prompt={referenceMediaDraft.prompt}
        imageRows={referenceMediaDraft.imageRows}
        videoRows={referenceMediaDraft.videoRows}
        audioRows={referenceMediaDraft.audioRows}
        durationSecs={options.durationSecs}
        aspectRatio={options.aspectRatio}
        durationLimits={effectiveDurationLimits}
        activeVideoConfig={activeVideoConfig}
        acting={acting}
        previewing={promptRequestPreviewLoading}
        draftDirty={referenceMediaDraftDirty}
        savingDraft={
          selectedSegmentNumber != null &&
          savingRecipeSegments.has(selectedSegmentNumber)
        }
        onPromptChange={setReferenceMediaPrompt}
        onImageRowsChange={setReferenceMediaImageRows}
        onVideoRowsChange={setReferenceMediaVideoRows}
        onAudioRowsChange={setReferenceMediaAudioRows}
        onSaveDraft={handleSaveReferenceMediaDraft}
        onPreview={handlePreviewReferenceMediaVideoRequest}
        onGenerate={handleGenerateReferenceMediaVideo}
        onImport={handleImportReferenceMediaVideo}
      />
    ) : null;
  const referenceInputs =
    workflowMode == null || workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA ? undefined : (
      <VideoReferenceInputSlots
        images={selectableReferenceImages}
        chapter={chapter}
        workflowMode={workflowMode}
        currentContentHash={currentContentHash}
        generationReferenceIds={generationReferenceIds}
        videoLibraryImageIds={videoLibraryImageIds}
        invalidGenerationReferenceIds={invalidGenerationReferenceIds}
        invalidVideoLibraryImageIds={invalidVideoLibraryImageIds}
        disabled={
          acting != null ||
          selectedSegmentNumber == null ||
          savingRecipeSegments.has(selectedSegmentNumber)
        }
        onGenerationReferenceIdsChange={setGenerationReferenceIds}
        onVideoLibraryImageIdsChange={setVideoLibraryImageIds}
        onReferenceLimit={() =>
          addToast(
            t("video.chapter.referenceLimit", {
              label:
                workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO
                  ? t("video.reference.chatUpload")
                  : t("video.reference.generationReference"),
              count:
                workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO
                  ? MAX_CHAT_REFERENCE_IMAGES
                  : MAX_REFERENCE_IMAGES,
            }),
            "error",
          )
        }
        onSelectionError={(message) => addToast(message, "error")}
        onClearInvalidGenerationReferences={clearInvalidGenerationReferences}
        onClearInvalidVideoLibraryImages={clearInvalidVideoLibraryImages}
      />
    );

  return (
    <>
      <div className="flex h-full min-h-0 flex-col gap-3">
        {videoPersistedStateError && (
          <div
            role="alert"
            className="shrink-0 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-xs leading-relaxed text-error"
          >
            {videoPersistedStateError}
            {t("video.chapter.invalidStateContinue")}
          </div>
        )}
        {videoDataError && (
          <div
            role="alert"
            className="flex shrink-0 items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-xs leading-relaxed text-error"
          >
            <span className="min-w-0 flex-1">
              {t("video.chapter.loadDataFailed", { error: videoDataError })}
            </span>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void loadAll()}
              disabled={videoDataLoading}
            >
              <RotateCcw size={12} />
              {t("video.chapter.retryLoad")}
            </BookButton>
          </div>
        )}
        <MediaStageHeader
          id={stageId}
          label={t("video.workspace")}
          tabs={workspaceTabs}
          activeTab={workspaceTab}
          onTabChange={handleWorkspaceTabChange}
        >
          {workspaceTab !== "video" ? null : workflowMode == null ? (
            <span>{t("video.chapter.noExecutionRecipeSelected")}</span>
          ) : workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA ? (
            <span>{t("video.chapter.currentReferenceMediaSegment")}</span>
          ) : (
            <span>
              {t("video.chapter.currentWorkflowSegment", {
                workflow: videoWorkflowModeLabel(workflowMode, t),
              })}
            </span>
          )}
        </MediaStageHeader>

        <section
          id={`${stageId}-panel`}
          role="tabpanel"
          aria-labelledby={`${stageId}-tab-${workspaceTab}`}
          className={`flex min-h-0 flex-1 flex-col ${
            workspaceTab === "assembly"
              ? "a4n-video-assembly-tabpanel custom-scrollbar"
              : ""
          }`}
        >
        {workspaceTab === "director" ? (
          <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar">
            <Suspense fallback={<VideoWorkspaceFallback text={t("writing.loadingVisualPlan")} />}>
              <ChapterVisualPlanTab projectId={projectId} canEdit={canEdit} chapter={authoringChapter} onDirtyChange={setAuthoringDirty} onBusyChange={setAuthoringBusy} />
            </Suspense>
          </div>
        ) : workspaceTab === "screenplay" ? (
          <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar">
            <Suspense fallback={<VideoWorkspaceFallback text={t("writing.loadingShootingScript")} />}>
              <ShootingScriptTab projectId={projectId} canEdit={canEdit} chapter={authoringChapter} onDirtyChange={setAuthoringDirty} onBusyChange={setAuthoringBusy} />
            </Suspense>
          </div>
        ) : workspaceTab === "materials" ? (
          <BaseMaterialsWorkspace projectId={profileMaterialProjectId} canEdit={canEdit}
            onChanged={refreshVideoDataInBackground} onBeforeKindChange={confirmWorkspaceDraftDiscard} onNext={async () => { if (await loadAll()) await handleWorkspaceTabChange("director"); }} />
        ) : workspaceTab === "frames" ? (
          planningOptionsState.options ? (
            <Suspense fallback={<VideoWorkspaceFallback text={t("video.chapter.loadingPlanningWorkspace")} />}>
              <VideoPlanningWorkspace
                prompt={frameSegmentPlanPromptForCards}
                previewSegments={segmentPlanPreview}
                draft={segmentPlanDraft}
                options={planningOptionsState.options}
                durationLimits={durationLimits}
                loading={segmentPlanDraftLoading}
                loadError={segmentPlanDraftError}
                dataReady={selectionValidationReady}
                acting={acting}
                shootingScenes={shootingScenes}
                onGenerate={(targetNarrativeUnitIndex, regenerateExisting) =>
                  handleGenerateSegmentPlan({
                    targetNarrativeUnitIndex,
                    regenerateExisting,
                  })
                }
                onOptionsChange={setPlanningOptions}
                onRetryLoad={() => {
                  void refreshSegmentPlanDraft().catch(() => undefined);
                }}
                onDelete={(prompt) => void handleDeletePromptRow(prompt)}
                onSavePlan={handleSaveSegmentPlan}
                onDirtyChange={setSegmentPlanDirty}
              />
            </Suspense>
          ) : (
            <div
              role="alert"
              className="flex min-h-0 flex-1 items-center justify-center"
            >
              <div className="flex max-w-xl flex-col gap-3 rounded-xl border border-error/40 bg-error-bg px-4 py-3 text-sm text-error">
                <span>{planningOptionsState.storageError}</span>
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={resetPlanningOptions}
                >
                  <RotateCcw size={12} />
                  {t("video.chapter.resetPlanningConfig")}
                </BookButton>
              </div>
            </div>
          )
        ) : workspaceTab === "assembly" ? (
          <Suspense fallback={<VideoWorkspaceFallback text={t("video.chapter.loadingAssemblyWorkspace")} />}>
            <VideoAssemblyWorkspace
              projectId={projectId}
              chapter={chapter}
              canEdit={canEdit}
              readUnavailable={videoDataReadUnavailable}
              readUnavailableReason={videoDataError}
              videos={videos}
              timeline={timeline}
              adoptedVideoIds={mixedAdoptedVideoIds}
              unavailableSegmentNumbers={unavailableMixedSegmentNumbers}
              onToast={addToast}
              onRefresh={refreshVideoDataInBackground}
              onVideoRecordUpdated={(video) =>
                setVideos((current) => {
                  const index = current.findIndex((item) => item.id === video.id);
                  if (index < 0) return [...current, video];
                  const next = [...current];
                  next[index] = video;
                  return next;
                })
              }
              onVideoRecordDeleted={applyVideoRecordDeletion}
            />
          </Suspense>
        ) : workspaceTab === "postprocess" ? (
          <Suspense fallback={<VideoWorkspaceFallback text={t("video.chapter.loadingPostprocessWorkspace")} />}>
            <VideoPostProcessWorkspace
              projectId={projectId}
              chapter={chapter}
              canEdit={canEdit}
              readUnavailable={videoDataReadUnavailable}
              readUnavailableReason={videoDataError}
              videos={videos}
              onToast={addToast}
              onRefresh={refreshVideoDataInBackground}
              onVideoRecordUpdated={(video) =>
                setVideos((current) => {
                  const index = current.findIndex((item) => item.id === video.id);
                  if (index < 0) return [...current, video];
                  const next = [...current];
                  next[index] = video;
                  return next;
                })
              }
            />
          </Suspense>
        ) : (
          <div className="min-h-0 flex-1">
            <VideoGenerationWorkspace
              prompts={allSegmentPrompts}
              segmentCards={segmentCardState.cards}
              segmentRecipes={effectiveRecipeBySegment}
              selectedSegmentNumber={selectedSegmentNumber}
              planningDraftVisible={segmentPlanDraftVisible}
              finalPromptBlockedReasons={finalPromptBlockedReasons}
              timeline={timeline}
              selectedPrompt={selectedPrompt}
              selectedPromptId={selectedPromptId}
              selectedPromptStale={selectedPromptStale}
              durationLimits={effectiveDurationLimits}
              loading={loading}
              acting={acting}
              chapterContentReady={chapterContentReady}
              savingRecipeSegments={savingRecipeSegments}
              draft={draft}
              selectedPromptRunning={selectedPromptRunning}
              selectedPromptDirty={selectedPromptDirty}
              staleSavedReferenceCount={staleSavedReferenceCount}
              savedReferenceIdsError={savedReferenceIdsError}
              framePrompts={currentLanguageFramePrompts}
              frameImages={videoFrameImages}
              imageBusyId={imageBusyId}
              videos={workflowVideos}
              previousAdoptedVideos={previousAdoptedVideos}
              referenceInputs={referenceInputs}
              selectedVideo={selectedVideo}
              selectedVideoPath={selectedVideoPath}
              trimStart={trimStart}
              trimEnd={trimEnd}
              onLoad={loadAll}
              onGeneratePromptRequests={handleGenerateVideoPrompts}
              onSelectPrompt={handleSelectPrompt}
              segmentConfigOpen={segmentConfigOpen}
              onActivateSegment={handleActivateSegment}
              onCloseSegmentConfig={handleCloseSegmentConfig}
              onDeletePrompt={(prompt) => void handleDeletePromptRow(prompt)}
              onInsertSegment={(gap) =>
                insertedSegmentPanelRef.current?.openCreate(gap)
              }
              onOpenInsertedSegment={(timelineItemId) =>
                insertedSegmentPanelRef.current?.openEdit(timelineItemId)
              }
              onWorkflowModeChange={handleSegmentWorkflowModeChange}
              onFirstFrameSourceChange={handleFirstFrameSourceChange}
              onContinuitySourceChange={handleContinuitySourceChange}
              onGenerateFramePrompts={handleGenerateFramePrompts}
              onPreviewFrameImage={handlePreviewFrameImageRequest}
              onGenerateFrameImage={(prompt) =>
                handleGenerateFrameImages(prompt)
              }
              onImportFrameImage={(prompt) => handleImportFrameImage(prompt)}
              onDeleteFrameImage={(image) => handleDeleteImageGuarded(image)}
              onDraftChange={setDraft}
              onSavePrompt={() => void handleSavePrompt()}
              onSaveFramePrompt={(prompt, visualPrompt) =>
                handleSavePromptRow(prompt, {
                  visual_prompt: visualPrompt,
                  motion_prompt: prompt.motion_prompt ?? "",
                  negative_prompt: prompt.negative_prompt ?? "",
                  duration_secs: prompt.duration_secs,
                })
              }
              onOptimizePrompt={handleOptimizePromptRow}
              onPreviewGenerateVideo={handlePreviewGenerateVideoRequest}
              onCopyPrompt={handleCopyChatVideoPrompt}
              onPreviewVideoRequest={handlePreviewVideoRequestForVideo}
              onGenerateVideo={handleGenerateVideo}
              onImportVideo={handleImportVideo}
              onAdoptVideo={handleAdoptSegmentVideo}
              onPollVideo={(video) => void handlePollVideo(video)}
              onToggleVideoFavorite={(video) =>
                void handleToggleVideoFavorite(video)
              }
              onUpdateVideo={handleUpdateVideo}
              onDeleteVideo={(video) => void handleDeleteVideo(video)}
              onTrimStartChange={setTrimStart}
              onTrimEndChange={setTrimEnd}
              onTrim={handleTrim}
              referenceMediaWorkspace={referenceMediaWorkspace}
            />
          </div>
        )}
        </section>
      </div>
      {workspaceTab === "video" ? (
        <InsertedSegmentTimelinePanel
          ref={insertedSegmentPanelRef}
          timeline={timeline}
          prompts={prompts}
          videos={videos}
          referenceImages={selectableReferenceImages}
          defaultStyle={options.style}
          defaultPromptOutputLanguage={options.promptOutputLanguage}
          defaultDurationSecs={options.durationSecs}
          defaultDurationOptions={options.durationOptions}
          defaultAspectRatio={options.aspectRatio}
          loading={loading}
          acting={acting}
          onGenerate={handleGenerateInsertedSegment}
          onCommit={handleCommitInsertedSegment}
          onMove={handleMoveInsertedSegment}
          onDelete={handleDeleteInsertedSegment}
          onGenerateVideo={handleGenerateInsertedVideo}
          onUpdatePrompt={handleUpdateInsertedPrompt}
          onPreviewVideo={handlePreviewVideoRequestForVideo}
          onDeleteVideo={handleDeleteVideo}
        />
      ) : null}
      {promptRequestPreviewOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <RequestPreviewModal
            open
            loading={promptRequestPreviewLoading}
            data={promptRequestPreviewData}
            fallbackTitle={t("video.chapter.modelRequestPreview")}
            onClose={closePromptRequestPreview}
          />
        </Suspense>
      ) : null}
      {imageRequestPreviewOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <RequestPreviewModal
            open
            loading={imageRequestPreviewLoading}
            data={imageRequestPreviewData}
            fallbackTitle={t("video.chapter.imageRequestPreview")}
            onClose={closeImageRequestPreview}
          />
        </Suspense>
      ) : null}
      {videoRequestPreviewOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <RequestPreviewModal
            open
            loading={videoRequestPreviewLoading}
            data={videoRequestPreviewData}
            fallbackTitle={t("video.chapter.videoRequestPreview")}
            onClose={closeVideoRequestPreview}
          />
        </Suspense>
      ) : null}
    </>
  );
};

const VideoWorkspaceFallback: React.FC<{ text: string }> = ({ text }) => (
  <div className="flex h-full min-h-48 items-center justify-center text-xs text-book-text-muted">
    <LoadingSpinner size="sm" className="mr-2" />
    {text}
  </div>
);
