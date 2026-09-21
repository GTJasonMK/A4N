import React, {
  lazy,
  Suspense,
  useCallback,
  useMemo,
} from "react";
import {
  Clapperboard,
  Film,
  ImagePlus,
  PanelTop,
  RotateCcw,
  Scissors,
} from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { previewBatchImagesRequest } from "../../api/image";
import {
  commitInsertedSegmentDraft,
  deleteInsertedSegment,
  moveInsertedSegment,
  previewReferenceMediaVideoRequest,
  selectSegmentVideo,
  startChapterVideoInsertedSegmentDraft,
  startVideoGeneration,
  updateChapterVideoSegmentPlan,
  uploadReferenceMediaVideo,
  verifyVideoGenerationResult,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type ReferenceMediaVideoInput,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentPlanV2,
  type VideoSegmentWorkflowMode,
  type InsertedSegmentGenerationOptions,
} from "../../api/video";
import { EmptyHint, type ChapterRow } from "./shared";
import { chooseDownstreamConflictStrategy } from "./dependencyConflict";
import { useProgressTask } from "../../hooks/useProgressTask";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import { copyTextToClipboard } from "../../utils/clipboard";
import { readFileAsBase64 } from "../../utils/fileUpload";
import { isCancelledOperation } from "../../utils/cancellation";
import {
  pickVideoUploadFile,
  type VideoUploadFile,
} from "../../utils/videoUpload";
import { VideoGenerationWorkspace } from "./video/VideoGenerationWorkspace";
import {
  InsertedSegmentTimelinePanel,
} from "./video/InsertedSegmentTimelinePanel";
import {
  normalizeReferenceMediaRows,
  ReferenceMediaWorkspace,
  referenceMediaDraftError,
  referenceMediaRowsByKindFromDraft,
  VideoReferenceInputSlots,
  type ReferenceMediaDraft,
} from "./video/referenceMediaWorkspace";
import {
  buildVideoFrameImageTraceMetadata,
  buildVideoFrameImageRequest,
  boundaryFramePromptChainTo,
  frameImageReferencesForPrompt,
  guidanceModeForWorkflowChange,
  isBoundaryFramePrompt,
  isDurationAllowed,
  latestVideoFrameImageForPrompt,
  MAX_CHAT_REFERENCE_IMAGES,
  MAX_REFERENCE_IMAGES,
  promptLengthError,
  referenceLabelOrderError,
  shiftReferenceLabels,
  VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
  videoFrameImageContractForPromptType,
  videoExecutionSegmentIdentityMatchesVideo,
  videoExecutionIdentityOfVideo,
  videoExecutionSegmentNumberInPlanByLineage,
  videoAspectRatiosForImageSizes,
  videoWorkflowModeLabel,
} from "./video/videoTabUtils";
import { useChapterContentHash } from "./video/useChapterContentHash";
import { useVideoImageActions } from "./video/useVideoImageActions";
import { useVideoPromptGenerationActions } from "./video/useVideoPromptGenerationActions";
import {
  useVideoRecordActions,
  videoPromptDownstreamCount,
} from "./video/useVideoRecordActions";
import { useVideoTabDerivedState } from "./video/useVideoTabDerivedState";
import { useI18n } from "../../i18n";
import { useVideoTabState } from "./video/useVideoTabState";
import { useVideoTabLoading } from "./video/useVideoTabLoading";
import { useVideoTabViewState } from "./video/useVideoTabViewState";
import { VideoWorkspaceHeader } from "./video/VideoWorkspaceHeader";

const VideoMaterialWorkspace = lazy(() =>
  import("./video/VideoMaterialWorkspace").then((module) => ({
    default: module.VideoMaterialWorkspace,
  })),
);
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
const ImageActionDialog = lazy(() =>
  import("./images/ImageActionDialog").then((module) => ({
    default: module.ImageActionDialog,
  })),
);
const RequestPreviewModal = lazy(() =>
  import("../../components/feedback/RequestPreviewModal").then((module) => ({
    default: module.RequestPreviewModal,
  })),
);

interface VideoTabProps {
  projectId: number;
  fixedChapter?: ChapterRow | null;
  imageProjectIds?: number[];
  materialProjectId?: number;
}

interface ReferenceMediaSegmentState {
  segmentPlanPromptId: number;
  segmentNumber: number;
  targetSegment: {
    source_shot_id: string;
    segment_index_in_shot: number;
  };
}

export const VideoTab: React.FC<VideoTabProps> = ({
  projectId,
  fixedChapter,
  imageProjectIds,
  materialProjectId,
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const {
    error: imageSizeError,
    loading: imageSizeLoading,
    sizes: imageSupportedSizes,
  } = useImageGenerationSize();
  const supportedVideoAspectRatios = useMemo(
    () =>
      imageSizeLoading || imageSizeError
        ? []
        : videoAspectRatiosForImageSizes(imageSupportedSizes),
    [imageSizeError, imageSizeLoading, imageSupportedSizes],
  );
  const workspaceTabs = useMemo(
    () => [
      { id: "materials" as const, label: t("video.chapter.tabMaterials"), icon: <ImagePlus size={13} /> },
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
    setTimeline,
    insertedSegmentPanelRef,
    segmentPlanPreview,
    setSegmentPlanPreview,
    segmentPlanDraft,
    videos,
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
    effectiveRecipeBySegmentRef,
    recipeFingerprintBySegment,
    frameRecipeFingerprintBySegment,
    selectedSegmentRecipe,
    workflowMode,
    segmentStorageKey,
    selectedPromptId,
    setSelectedPromptId,
    selectedVideoId,
    setSelectedVideoId,
    pendingVideoId,
    setPendingVideoId,
    options,
    videoLibraryImageIds,
    generationReferenceIds,
    updateSegmentRecipe,
    awaitSegmentRecipeSaved,
    draft,
    setDraft,
    referenceMediaDraftScopeForSegment,
    referenceMediaDraft,
    referenceMediaDraftRef,
    referenceMediaDraftDirty,
    referenceMediaDraftScopeRef,
    loading,
    videoDataError,
    activeVideoConfig,
    segmentPlanDraftLoading,
    segmentPlanDraftError,
    promptRequestPreviewOpen,
    setPromptRequestPreviewOpen,
    promptRequestPreviewLoading,
    setPromptRequestPreviewLoading,
    promptRequestPreviewData,
    setPromptRequestPreviewData,
    promptRequestPreviewScopeRef,
    imageRequestPreviewOpen,
    setImageRequestPreviewOpen,
    imageRequestPreviewLoading,
    setImageRequestPreviewLoading,
    imageRequestPreviewData,
    setImageRequestPreviewData,
    imageRequestPreviewScopeRef,
  } = videoTabState;
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
  const {
    imageBusyId,
    imageDetails,
    imageScoreInput,
    imageScoringId,
    setImageScoreInput,
    handleCloseImageDetails,
    handleDeleteImage,
    handleScoreImage,
    handleToggleImageFavorite,
    handleUpdateImage,
  } = useVideoImageActions({
    t,
    addToast,
    images,
    actionScopeKey: videoActionScopeKey,
    setImages,
    loadAll,
  });




  const viewState = useVideoTabViewState({
    ...videoTabState,
    ...derivedVideoTabState,
    chapter,
    currentContentHash,
    chapterContentHashError,
    addToast,
    t,
    loadAll,
  });
  const {
    selectedPromptDirty,
    frameSegmentIdentityIndex,
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
    videoOptionsStateError,
    videoPersistedStateError,
    effectiveDurationLimits,
    setReferenceMediaPrompt,
    setReferenceMediaImageRows,
    setReferenceMediaVideoRows,
    setReferenceMediaAudioRows,
    confirmSegmentRecipeChange,
    setGenerationReferenceIds,
    setVideoLibraryImageIds,
    finalPromptBlockedReasons,
    saveReferenceMediaDraft,
    handleSelectSegment,
    handleSelectPrompt,
    handleActivateSegment,
    handleCloseSegmentConfig,
  } = viewState;

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
    setRequestPreviewOpen: setVideoRequestPreviewOpen,
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
    pendingVideoId,
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
    setPrompts,
    setDraft,
    setSelectedPromptId,
    setSelectedVideoId,
    setPendingVideoId,
    beginAction: beginVideoAction,
    isCurrentActionScope: isCurrentVideoActionScope,
    finishAction: finishVideoAction,
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
    currentContentHash,
    actionScopeKey: videoActionScopeKey,
    locale,
    t,
    addToast,
    runProgressTask,
    loadAll,
    refreshPrompts,
    refreshSegmentPlanDraft,
    setSegmentPlanPreview,
    beginAction: beginVideoAction,
    isCurrentActionScope: isCurrentVideoActionScope,
    finishAction: finishVideoAction,
  });

  const handleSaveSegmentPlan = useCallback(
    async (segments: VideoSegmentPlanV2[]): Promise<boolean> => {
      const prompt = frameSegmentPlanPromptForCards;
      if (!prompt || prompt.status === "running" || acting != null) return false;
      const downstreamStrategy = await chooseDownstreamConflictStrategy({
        title: t("video.actions.downstreamTitle"),
        assetLabel: `${prompt.title ?? t("video.actions.segmentPlanLabel")} #${prompt.id}`,
        downstreamLabel: t("video.actions.downstreamLabel"),
        downstreamCount: videoPromptDownstreamCount(
          prompt,
          prompts,
          videoFrameImages,
        ),
        operation: t("video.chapter.savePlan"),
        t,
      });
      if (downstreamStrategy == null || !beginVideoAction(`edit-segment-plan-${prompt.id}`)) {
        return false;
      }
      const actionKey = `edit-segment-plan-${prompt.id}`;
      try {
        const updated = await updateChapterVideoSegmentPlan(prompt.id, {
          expected_updated_at: prompt.updated_at,
          segments,
          downstream_strategy: downstreamStrategy,
        });
        setPrompts((previous) =>
          previous.map((item) => (item.id === updated.id ? updated : item)),
        );
        await loadAll();
        addToast(t("video.chapter.planSaved"), "success");
        return true;
      } catch (error) {
        addToast(
          extractErrorMessage(error, t("video.chapter.planSaveFailed")),
          "error",
        );
        return false;
      } finally {
        finishVideoAction(actionKey);
      }
    },
    [
      acting,
      addToast,
      beginVideoAction,
      finishVideoAction,
      frameSegmentPlanPromptForCards,
      loadAll,
      prompts,
      t,
      videoFrameImages,
    ],
  );

  const recipeForSegmentAction = (
    segmentNumber: number,
  ): VideoSegmentExecutionRecipe | null => {
    const recipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber) ?? null;
    if (!recipe) {
      addToast(
        t("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        }),
        "error",
      );
    }
    return recipe;
  };

  const targetForSegmentAction = (segmentNumber: number) => {
    const card = segmentCardState.cards.find(
      (item) => item.segmentNumber === segmentNumber,
    );
    const sourceShotId = card?.raw.source_shot_id;
    const segmentIndexInShot = card?.raw.segment_index_in_shot;
    if (
      typeof sourceShotId !== "string" ||
      !sourceShotId.trim() ||
      typeof segmentIndexInShot !== "number" ||
      !Number.isInteger(segmentIndexInShot) ||
      segmentIndexInShot <= 0
    ) {
      addToast(
        t("video.chapter.segmentIdentityMissing", { number: segmentNumber }),
        "error",
      );
      return null;
    }
    return {
      source_shot_id: sourceShotId.trim(),
      segment_index_in_shot: segmentIndexInShot,
    };
  };

  const handleGenerateFramePrompts = async (segmentNumber: number) => {
    try {
      await awaitSegmentRecipeSaved(segmentNumber);
    } catch {
      return;
    }
    const recipe = recipeForSegmentAction(segmentNumber);
    const targetSegment = targetForSegmentAction(segmentNumber);
    if (!recipe || !targetSegment) return;
    await runGenerateFramePrompts(recipe, {
      targetSegment,
    });
  };

  const handleGenerateVideoPrompts = async (segmentNumber: number) => {
    try {
      await awaitSegmentRecipeSaved(segmentNumber);
    } catch {
      return;
    }
    const recipe = recipeForSegmentAction(segmentNumber);
    const targetSegment = targetForSegmentAction(segmentNumber);
    if (!recipe || !targetSegment) return;
    if (recipe.workflow_mode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA) {
      addToast(t("video.chapter.referenceMediaSelectionRequired"), "error");
      return;
    }
    const blockedReason = finalPromptBlockedReasons.get(segmentNumber) ?? null;
    if (blockedReason) {
      addToast(blockedReason, "error");
      return;
    }
    await runGenerateVideoPrompts(recipe, {
      targetSegment,
    });
  };

  const handleAdoptSegmentVideo = async (video: GeneratedVideo) => {
    const identity = videoExecutionIdentityOfVideo(video);
    const shotNumber = videoExecutionSegmentNumberInPlanByLineage(
      identity,
      frameSegmentPlanPromptForCards,
      frameSegmentIdentityIndex,
    );
    if (!chapter || !frameSegmentPlanPromptForCards || shotNumber == null) {
      addToast(t("video.chapter.selectedVideoMissingRecipe"), "error");
      return;
    }
    const targetSegment = targetForSegmentAction(shotNumber);
    if (!targetSegment) return;
    if (selectedSegmentNumber !== shotNumber) {
      addToast(
        t("video.chapter.videoBelongsToOtherSegment", {
          id: video.id,
          number: shotNumber,
        }),
        "error",
      );
      return;
    }
    const fingerprint = recipeFingerprintBySegment.get(shotNumber);
    if (
      !videoExecutionSegmentIdentityMatchesVideo(
        video,
        frameSegmentPlanPromptForCards,
        shotNumber,
        fingerprint,
        frameSegmentIdentityIndex,
      )
    ) {
      addToast(t("video.chapter.videoNotLatestRecipe"), "error");
      return;
    }
    const actionKey = `adopt-video-${video.id}`;
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    try {
      await selectSegmentVideo(
        projectId,
        chapter.id,
        video.id,
        frameSegmentPlanPromptForCards.id,
        targetSegment,
      );
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(t("video.chapter.adoptedSegment", { number: shotNumber }), "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, t("video.chapter.adoptVideoFailed")), "error");
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleSegmentWorkflowModeChange = async (
    mode: VideoSegmentWorkflowMode,
    segmentNumber: number,
  ): Promise<void> => {
    const currentRecipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber);
    if (!currentRecipe) {
      addToast(
        t("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        }),
        "error",
      );
      return;
    }
    const guidanceMode = guidanceModeForWorkflowChange(mode, segmentNumber);
    if (currentRecipe.workflow_mode === mode) {
      await handleSelectSegment(segmentNumber);
      return;
    }
    const savesReferenceMediaDraft =
      currentRecipe.workflow_mode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA &&
      mode !== VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA &&
      segmentNumber === selectedSegmentNumber &&
      referenceMediaDraftScopeRef.current ===
        referenceMediaDraftScopeForSegment(segmentNumber) &&
      referenceMediaDraftDirty;
    if (savesReferenceMediaDraft) {
      try {
        if (!(await saveReferenceMediaDraft(segmentNumber))) return;
      } catch {
        return;
      }
    }
    if (
      !(await confirmSegmentRecipeChange(
        segmentNumber,
        t("video.chapter.workflowMode"),
      ))
    ) {
      return;
    }
    if (!(await handleSelectSegment(segmentNumber))) return;
    try {
      await updateSegmentRecipe(segmentNumber, (recipe) => {
        return {
          ...recipe,
          workflow_mode: mode,
          guidance_mode: guidanceMode,
          guidance_reference_image_ids: [],
          continuity_source_video_id: null,
        };
      });
      setWorkspaceTab("video");
    } catch {
      // persistSegmentRecipe 已保留可见错误并恢复服务端状态。
    }
  };

  const handleFirstFrameSourceChange = async (
    segmentNumber: number,
    guidanceMode: Extract<
      VideoSegmentExecutionRecipe["guidance_mode"],
      "continuity" | "system_frames"
    >,
  ) => {
    const currentRecipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber);
    if (!currentRecipe) {
      addToast(
        t("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        }),
        "error",
      );
      return;
    }
    if (
      currentRecipe.workflow_mode !== VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN ||
      segmentNumber <= 1
    ) {
      addToast(t("video.chapter.firstFrameChainFollowupInputRequired"), "error");
      return;
    }
    if (currentRecipe.guidance_mode === guidanceMode) return;
    if (
      !(await confirmSegmentRecipeChange(
        segmentNumber,
        t("video.chapter.startFrameSource"),
      ))
    ) {
      return;
    }
    try {
      await updateSegmentRecipe(segmentNumber, (recipe) => ({
        ...recipe,
        guidance_mode: guidanceMode,
        guidance_reference_image_ids: [],
        continuity_source_video_id: null,
      }));
    } catch {
      // persistSegmentRecipe 已显示错误。
    }
  };

  const handleContinuitySourceChange = async (
    segmentNumber: number,
    continuitySourceVideoId: number | null,
  ) => {
    const currentRecipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber);
    if (!currentRecipe) {
      addToast(
        t("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        }),
        "error",
      );
      return;
    }
    const sourceChanged =
      currentRecipe.guidance_mode !== "continuity" ||
      (currentRecipe.continuity_source_video_id ?? null) !==
        continuitySourceVideoId;
    if (!sourceChanged) return;
    if (
      !(await confirmSegmentRecipeChange(
        segmentNumber,
        t("video.chapter.sourceVideo"),
      ))
    ) {
      return;
    }
    try {
      await updateSegmentRecipe(segmentNumber, (recipe) => ({
        ...recipe,
        guidance_mode: "continuity",
        guidance_reference_image_ids: [],
        continuity_source_video_id: continuitySourceVideoId,
      }));
    } catch {
      // persistSegmentRecipe 已显示错误。
    }
  };

  const handleGenerateInsertedSegment = async (
    input: InsertedSegmentGenerationOptions,
  ): Promise<number | null> => {
    const actionKey = "generate-inserted-segment";
    const requestScope = videoActionScopeKey;
    if (!chapter) {
      addToast(t("video.chapter.noChapter"), "error");
      return null;
    }
    if (!beginVideoAction(actionKey)) return null;
    try {
      const handle = await startChapterVideoInsertedSegmentDraft(
        projectId,
        chapter.id,
        input,
      );
      const workflowRunId = await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("video.chapter.insertedDescription"),
          startMessage: t("video.chapter.insertedDescriptionPreparing"),
          successMessage: t("video.chapter.insertedDescriptionGenerated"),
          errorMessage: t("video.chapter.insertedDescriptionFailed"),
          cancelLabel: t("video.chapter.stopGeneration"),
        },
        async (task) => {
          const result = task.resultRef;
          if (result?.kind !== "inserted_segment_draft") {
            throw new Error(t("video.chapter.insertedDraftMissing"));
          }
          const parsed = Number(result.id);
          if (!Number.isSafeInteger(parsed) || parsed <= 0) {
            throw new Error(t("video.chapter.invalidInsertedWorkflowId"));
          }
          return parsed;
        },
      );
      if (!isCurrentVideoActionScope(requestScope)) return null;
      addToast(t("video.chapter.insertedDescriptionConfirm"), "success");
      return workflowRunId;
    } catch (error) {
      if (isCurrentVideoActionScope(requestScope)) {
        const message = extractErrorMessage(
          error,
          t("video.chapter.insertedDescriptionFailed"),
        );
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
      return null;
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleCommitInsertedSegment = async (
    timelineId: number,
    expectedRevision: number,
    workflowRunId: number,
  ): Promise<boolean> => {
    const actionKey = "commit-inserted-segment";
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return false;
    try {
      const nextTimeline = await commitInsertedSegmentDraft({
        timeline_id: timelineId,
        expected_timeline_revision: expectedRevision,
        workflow_run_id: workflowRunId,
      });
      if (!isCurrentVideoActionScope(requestScope)) return false;
      setTimeline(nextTimeline);
      addToast(t("video.chapter.insertedSubmitted"), "success");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(
            refreshError,
            t("video.chapter.insertedSubmitRefreshFailed"),
          ),
          "error",
        );
      }
      return true;
    } catch (error) {
      if (!isCurrentVideoActionScope(requestScope)) return false;
      const message = extractErrorMessage(error, t("video.chapter.operationFailed"));
      addToast(
        message.includes("timeline_revision_conflict")
          ? t("video.chapter.timelineRevisionConflict", { error: message })
          : message,
        "error",
      );
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(refreshError, t("video.chapter.refreshTimelineFailed")),
          "error",
        );
      }
      return false;
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleMoveInsertedSegment = async (input: {
    timeline_id: number;
    expected_timeline_revision: number;
    timeline_item_id: number;
    previous_item_id: number | null;
    next_item_id: number | null;
  }): Promise<void> => {
    const actionKey = `move-inserted-segment-${input.timeline_item_id}`;
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    try {
      const nextTimeline = await moveInsertedSegment(input);
      if (!isCurrentVideoActionScope(requestScope)) return;
      setTimeline(nextTimeline);
      addToast(t("video.chapter.insertedPositionUpdated"), "success");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(
            refreshError,
            t("video.chapter.insertedMoveRefreshFailed"),
          ),
          "error",
        );
      }
    } catch (error) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(error, t("video.chapter.operationFailed")), "error");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(refreshError, t("video.chapter.refreshTimelineFailed")),
          "error",
        );
      }
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleDeleteInsertedSegment = async (input: {
    timeline_id: number;
    expected_timeline_revision: number;
    timeline_item_id: number;
  }): Promise<void> => {
    const actionKey = `delete-inserted-segment-${input.timeline_item_id}`;
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    try {
      const nextTimeline = await deleteInsertedSegment(input);
      if (!isCurrentVideoActionScope(requestScope)) return;
      setTimeline(nextTimeline);
      addToast(t("video.chapter.insertedRemoved"), "success");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(
            refreshError,
            t("video.chapter.insertedDeleteRefreshFailed"),
          ),
          "error",
        );
      }
    } catch (error) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(error, t("video.chapter.operationFailed")), "error");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(refreshError, t("video.chapter.refreshTimelineFailed")),
          "error",
        );
      }
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleGenerateInsertedVideo = async (
    prompt: ChapterVideoPrompt,
  ): Promise<void> => {
    const actionKey = `generate-inserted-video-${prompt.id}`;
    const requestScope = videoActionScopeKey;
    if (!chapter || !beginVideoAction(actionKey)) return;
    try {
      const handle = await startVideoGeneration({
        project_id: projectId,
        operation: {
          kind: "chapter",
          chapter_id: chapter.id,
          prompt_id: prompt.id,
          config_id: activeVideoConfig?.id ?? null,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("video.chapter.insertedVideoPreparing"),
          startMessage: t("video.chapter.insertedVideoPreparing"),
          successMessage: t("video.chapter.insertedVideoGenerated"),
          errorMessage: t("video.chapter.insertedVideoFailed"),
          cancelLabel: t("video.chapter.stopGeneration"),
        },
        async (task) => {
          await verifyVideoGenerationResult(task, projectId, chapter.id);
        },
      );
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(t("video.chapter.insertedVideoGenerated"), "success");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(
            refreshError,
            t("video.chapter.videoDataGeneratedRefreshFailed"),
          ),
          "error",
        );
      }
    } catch (error) {
      if (isCurrentVideoActionScope(requestScope)) {
        const message = extractErrorMessage(
          error,
          t("video.chapter.insertedVideoFailed"),
        );
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleUpdateInsertedPrompt = async (
    prompt: ChapterVideoPrompt,
    input: {
      visual_prompt: string;
      motion_prompt: string;
      negative_prompt: string;
    },
  ): Promise<void> => {
    await handleSavePromptRow(prompt, {
      ...input,
      duration_secs: prompt.duration_secs,
    });
  };

  const handleCopyChatVideoPrompt = async (prompt: ChapterVideoPrompt) => {
    const text = prompt.visual_prompt.trim();
    if (!text) {
      addToast(t("video.chapter.chatPromptEmpty"), "error");
      return;
    }
    try {
      await copyTextToClipboard(text);
      addToast(t("video.chapter.copiedChatPrompt"), "success");
    } catch (err) {
      addToast(
        extractErrorMessage(err, t("video.chapter.copyChatPromptFailed")),
        "error",
      );
    }
  };

  const buildFrameImageGenerationInput = (
    prompt: ChapterVideoPrompt,
  ): Record<string, unknown> => {
    const frameContract = videoFrameImageContractForPromptType(
      prompt.prompt_type,
      t,
    );
    const trimmedPrompt = prompt.visual_prompt.trim();
    const previewValidationError = promptLengthError(
      frameContract.promptLabel,
      trimmedPrompt,
      VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
      t,
    );
    if (previewValidationError) {
      throw new Error(previewValidationError);
    }
    const references = frameImageReferencesForPrompt(
      prompt,
      videoFrameImages,
      currentLanguageFramePrompts,
      t,
    );
    if (references.error) {
      throw new Error(references.error);
    }
    const effectiveVisualPrompt = shiftReferenceLabels(
      trimmedPrompt,
      references.promptReferenceOffset,
    );
    const referenceOrderError = referenceLabelOrderError(
      effectiveVisualPrompt,
      references.ids.length,
      t,
    );
    if (referenceOrderError) {
      throw new Error(referenceOrderError);
    }
    return buildVideoFrameImageRequest(
      frameContract.imageType,
      effectiveVisualPrompt,
      prompt.chapter_id || chapter?.id || 0,
      prompt.aspect_ratio,
      buildVideoFrameImageTraceMetadata(prompt, frameContract.source, t),
      references.ids,
      references.note,
      t,
    );
  };

  const buildFrameImagePreviewInputs = (
    prompt: ChapterVideoPrompt,
  ): {
    requests: Array<Record<string, unknown>>;
    note: string | null;
  } | null => {
    if (!isBoundaryFramePrompt(prompt)) {
      return {
        requests: [buildFrameImageGenerationInput(prompt)],
        note: null,
      };
    }
    const chain = boundaryFramePromptChainTo(
      prompt,
      currentLanguageFramePrompts,
      t("video.chapter.requestPreviewOperation"),
      t,
    );
    const missingPrompts = chain.filter(
      (item) => !latestVideoFrameImageForPrompt(item, videoFrameImages, t),
    );
    if (missingPrompts.length === 0) {
      addToast(
        t("video.chapter.boundaryImagesAlreadyExist", {
          number: prompt.shot_number,
        }),
        "info",
      );
      return null;
    }
    const firstMissing = missingPrompts[0];
    if (!firstMissing) return null;
    const dynamicTailCount = missingPrompts.length - 1;
    const extraPreviewNote =
      dynamicTailCount > 0
        ? t("video.chapter.boundaryPreviewNotice", {
            first: firstMissing.shot_number,
            count: dynamicTailCount,
          })
        : null;
    return {
      requests: [buildFrameImageGenerationInput(firstMissing)],
      note: extraPreviewNote,
    };
  };

  const previewImageRequests = async (
    requests: Array<Record<string, unknown>>,
    note: string | null = null,
  ) => {
    if (!chapter) return;
    if (requests.length === 0) {
      addToast(t("video.chapter.previewImageRequestsEmpty"), "error");
      return;
    }
    const previewScope = imageRequestPreviewScopeRef.current;
    setImageRequestPreviewOpen(true);
    setImageRequestPreviewLoading(true);
    setImageRequestPreviewData(null);
    try {
      const preview = await previewBatchImagesRequest(projectId, requests);
      if (imageRequestPreviewScopeRef.current !== previewScope) return;
      const previewNote = note?.trim();
      setImageRequestPreviewData(
        previewNote && preview.steps[0]
          ? {
              ...preview,
              steps: preview.steps.map((step, index) =>
                index === 0
                  ? {
                      ...step,
                      note: [step.note?.trim(), previewNote]
                        .filter(Boolean)
                        .join("\n"),
                    }
                  : step,
              ),
            }
          : preview,
      );
    } catch (err) {
      if (imageRequestPreviewScopeRef.current !== previewScope) return;
      addToast(extractErrorMessage(err, t("video.chapter.viewImageRequestFailed")), "error");
      setImageRequestPreviewOpen(false);
    } finally {
      if (imageRequestPreviewScopeRef.current === previewScope) {
        setImageRequestPreviewLoading(false);
      }
    }
  };

  const handlePreviewFrameImageRequest = async (prompt: ChapterVideoPrompt) => {
    let previewInput: ReturnType<typeof buildFrameImagePreviewInputs>;
    try {
      previewInput = buildFrameImagePreviewInputs(prompt);
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.chapter.previewImageRequestFailed")), "error");
      return;
    }
    if (!previewInput) return;
    await previewImageRequests(previewInput.requests, previewInput.note);
  };

  const buildReferenceMediaVideoInput = (
    segment: ReferenceMediaSegmentState,
    draft: ReferenceMediaDraft,
  ): ReferenceMediaVideoInput => {
    const imageRows = normalizeReferenceMediaRows(draft.imageRows);
    const videoRows = normalizeReferenceMediaRows(draft.videoRows);
    const audioRows = normalizeReferenceMediaRows(draft.audioRows);
    return {
      segment_plan_prompt_id: segment.segmentPlanPromptId,
      target_segment: segment.targetSegment,
      image_uploads: imageRows.flatMap((row) =>
        row.upload ? [row.upload] : [],
      ),
      video_uploads: videoRows.flatMap((row) =>
        row.upload ? [row.upload] : [],
      ),
      audio_uploads: audioRows.flatMap((row) =>
        row.upload ? [row.upload] : [],
      ),
      config_id: null,
    };
  };

  const referenceMediaSegmentFor = (
    requireProvider = true,
  ): ReferenceMediaSegmentState | null => {
    if (!chapter) return null;
    if (
      !frameSegmentPlanPromptForCards ||
      selectedSegmentNumber == null ||
      workflowMode !== VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA
    ) {
      addToast(t("video.chapter.referencePreviewOnly"), "error");
      return null;
    }
    if (videoOptionsStateError) {
      addToast(videoOptionsStateError, "error");
      return null;
    }
    const targetSegment = targetForSegmentAction(selectedSegmentNumber);
    if (!targetSegment) return null;
    const segment = {
      segmentPlanPromptId: frameSegmentPlanPromptForCards.id,
      segmentNumber: selectedSegmentNumber,
      targetSegment,
    };
    if (!requireProvider) return segment;
    if (effectiveDurationLimits.configError) {
      addToast(effectiveDurationLimits.configError, "error");
      return null;
    }
    if (!isDurationAllowed(options.durationSecs, effectiveDurationLimits)) {
      addToast(t("video.reference.durationUnsupported"), "error");
      return null;
    }
    return segment;
  };

  const prepareReferenceMediaVideoInput = async () => {
    const segment = referenceMediaSegmentFor();
    if (!segment) return null;
    const draft = referenceMediaDraftRef.current;
    const inputError = referenceMediaDraftError(
      draft.prompt,
      referenceMediaRowsByKindFromDraft(draft),
      t,
    );
    if (inputError) {
      addToast(inputError, "error");
      return null;
    }
    try {
      if (!(await saveReferenceMediaDraft(segment.segmentNumber))) return null;
    } catch {
      return null;
    }
    return buildReferenceMediaVideoInput(segment, draft);
  };

  const handlePreviewReferenceMediaVideoRequest = async () => {
    if (!chapter) return;
    const input = await prepareReferenceMediaVideoInput();
    if (!input) return;
    const previewScope = promptRequestPreviewScopeRef.current;
    setPromptRequestPreviewOpen(true);
    setPromptRequestPreviewLoading(true);
    setPromptRequestPreviewData(null);
    try {
      const preview = await previewReferenceMediaVideoRequest(
        projectId,
        chapter.id,
        input,
      );
      if (promptRequestPreviewScopeRef.current !== previewScope) return;
      setPromptRequestPreviewData(preview);
    } catch (err) {
      if (promptRequestPreviewScopeRef.current !== previewScope) return;
      addToast(
        extractErrorMessage(err, t("video.chapter.previewReferenceMediaFailed")),
        "error",
      );
      setPromptRequestPreviewOpen(false);
    } finally {
      if (promptRequestPreviewScopeRef.current === previewScope) {
        setPromptRequestPreviewLoading(false);
      }
    }
  };

  const handleSaveReferenceMediaDraft = async () => {
    if (selectedSegmentNumber == null) {
      addToast(t("video.chapter.selectReferenceMediaSegment"), "error");
      return;
    }
    try {
      if (!(await saveReferenceMediaDraft(selectedSegmentNumber))) return;
      addToast(t("video.chapter.referenceMediaDraftSaved"), "success");
    } catch {
      // persistSegmentRecipe 已显示错误。
    }
  };

  const handleGenerateReferenceMediaVideo = async () => {
    if (!chapter) return;
    const input = await prepareReferenceMediaVideoInput();
    if (!input) return;
    const actionKey = "reference-media-video";
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    try {
      const handle = await startVideoGeneration({
        project_id: projectId,
        operation: {
          kind: "reference_media",
          chapter_id: chapter.id,
          input,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("video.chapter.referenceMediaVideoTitle"),
          startMessage: t("video.chapter.referenceMediaVideoPreparing"),
          successMessage: t("video.chapter.referenceMediaVideoGenerated"),
          errorMessage: t("video.chapter.referenceMediaVideoFailed"),
          cancelLabel: t("video.chapter.stopGeneration"),
        },
        async (task) => {
          const [row] = await verifyVideoGenerationResult(
            task,
            projectId,
            chapter.id,
          );
          if (!isCurrentVideoActionScope(requestScope)) return;
          setSelectedVideoId(row.id);
          setPendingVideoId(null);
        },
      );
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(t("video.chapter.referenceMediaVideoGenerated"), "success");
    } catch (err) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      const message = extractErrorMessage(
        err,
        t("video.chapter.referenceMediaVideoFailed"),
      );
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentVideoActionScope(requestScope)) await loadAll();
      finishVideoAction(actionKey);
    }
  };

  const handleImportReferenceMediaVideo = async () => {
    if (!chapter) return;
    const segment = referenceMediaSegmentFor(false);
    if (!segment) return;
    if (!referenceMediaDraft.prompt.trim()) {
      addToast(t("video.chapter.mediaPromptEmpty"), "error");
      return;
    }
    try {
      if (!(await saveReferenceMediaDraft(segment.segmentNumber))) return;
    } catch {
      return;
    }

    const actionKey = "reference-media-import";
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    let upload: VideoUploadFile | null = null;
    try {
      upload = await pickVideoUploadFile(t, t("video.reference.video"));
    } catch (err) {
      if (isCurrentVideoActionScope(requestScope)) {
        addToast(
          extractErrorMessage(err, t("video.chapter.filePickerFailed")),
          "error",
        );
      }
      finishVideoAction(actionKey);
      return;
    }
    if (!upload) {
      finishVideoAction(actionKey);
      return;
    }
    const { file, mimeType } = upload;

    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentVideoActionScope(requestScope)) return;
      const row = await uploadReferenceMediaVideo(projectId, chapter.id, {
        file_name: file.name,
        mime_type: mimeType,
        data_base64: dataBase64,
        segment_plan_prompt_id: segment.segmentPlanPromptId,
        target_segment: segment.targetSegment,
      });
      if (!isCurrentVideoActionScope(requestScope)) return;
      setSelectedVideoId(row.id);
      setPendingVideoId(null);
      addToast(t("video.chapter.importedReferenceMediaVideo"), "success");
      await loadAll();
    } catch (err) {
      if (isCurrentVideoActionScope(requestScope)) {
        addToast(
          extractErrorMessage(err, t("video.chapter.importReferenceMediaVideoFailed")),
          "error",
        );
      }
    } finally {
      finishVideoAction(actionKey);
    }
  };

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
        <VideoWorkspaceHeader
          tabs={workspaceTabs}
          activeTab={workspaceTab}
          onTabChange={setWorkspaceTab}
        >
          {workflowMode == null ? (
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
        </VideoWorkspaceHeader>

        {workspaceTab === "materials" ? (
          <div className="min-h-0 flex-1">
            <Suspense fallback={<VideoWorkspaceFallback text={t("video.chapter.loadingMaterialsWorkspace")} />}>
              <VideoMaterialWorkspace
                projectId={profileMaterialProjectId}
                onToast={addToast}
                runProgressTask={runProgressTask}
                onRefreshVideoData={loadAll}
              />
            </Suspense>
          </div>
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
                supportedAspectRatios={supportedVideoAspectRatios}
                aspectRatioLoading={imageSizeLoading}
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
              videos={videos}
              timeline={timeline}
              adoptedVideoIds={mixedAdoptedVideoIds}
              unavailableSegmentNumbers={unavailableMixedSegmentNumbers}
              onToast={addToast}
              onRefresh={loadAll}
            />
          </Suspense>
        ) : workspaceTab === "postprocess" ? (
          <Suspense fallback={<VideoWorkspaceFallback text={t("video.chapter.loadingPostprocessWorkspace")} />}>
            <VideoPostProcessWorkspace
              projectId={projectId}
              chapter={chapter}
              videos={videos}
              onToast={addToast}
              onRefresh={loadAll}
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
              onDeleteFrameImage={(image) => handleDeleteImage(image)}
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
          supportedAspectRatios={supportedVideoAspectRatios}
          aspectRatioLoading={imageSizeLoading}
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
      {imageDetails ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <ImageActionDialog
            image={imageDetails}
            busyId={imageBusyId}
            scoringId={imageScoringId}
            scoreInput={imageScoreInput}
            onClose={handleCloseImageDetails}
            onToggleFavorite={(image) => void handleToggleImageFavorite(image)}
            onUpdate={(image, input) => handleUpdateImage(image, input)}
            onScoreInputChange={setImageScoreInput}
            onScore={(image) => void handleScoreImage(image)}
            onDelete={(image) => void handleDeleteImage(image)}
          />
        </Suspense>
      ) : null}
      {promptRequestPreviewOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <RequestPreviewModal
            open
            loading={promptRequestPreviewLoading}
            data={promptRequestPreviewData}
            fallbackTitle={t("video.chapter.modelRequestPreview")}
            onClose={() => setPromptRequestPreviewOpen(false)}
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
            onClose={() => setImageRequestPreviewOpen(false)}
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
            onClose={() => setVideoRequestPreviewOpen(false)}
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
