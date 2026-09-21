import React, {
  lazy,
  Suspense,
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
import {
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
} from "../../api/video";
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
  videoAspectRatiosForImageSizes,
  videoWorkflowModeLabel,
} from "./video/videoTabUtils";
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
import { VideoWorkspaceHeader } from "./video/VideoWorkspaceHeader";
import { useVideoTabActions } from "./video/useVideoTabActions";

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
    setPromptRequestPreviewOpen,
    promptRequestPreviewLoading,
    promptRequestPreviewData,
    imageRequestPreviewOpen,
    setImageRequestPreviewOpen,
    imageRequestPreviewLoading,
    imageRequestPreviewData,
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
    handleSaveReferenceMediaDraft,
    handleGenerateReferenceMediaVideo,
    handleImportReferenceMediaVideo,
  } = useVideoTabActions({
    ...videoTabState,
    ...derivedVideoTabState,
    ...viewState,
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
