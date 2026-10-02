import { useCallback, useRef } from "react";
import { previewBatchImagesRequest } from "../../../api/image";
import { extractErrorMessage } from "../../../api";
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
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type ReferenceMediaVideoInput,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentPlanV2,
  type VideoSegmentWorkflowMode,
  type InsertedSegmentGenerationOptions,
} from "../../../api/video";
import type { ChapterRow } from "../shared";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import { copyTextToClipboard } from "../../../utils/clipboard";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import { isCancelledOperation } from "../../../utils/cancellation";
import { pickVideoUploadFile, type VideoUploadFile } from "../../../utils/videoUpload";
import type { MessageTranslator } from "../../../i18n";
import {
  buildVideoFrameImageTraceMetadata,
  buildVideoFrameImageRequest,
  boundaryFramePromptChainTo,
  frameImageReferencesForPrompt,
  isBoundaryFramePrompt,
  latestVideoFrameImageForPrompt,
  referenceLabelOrderError,
  shiftReferenceLabels,
} from "./videoFramePromptUtils";
import {
  guidanceModeForWorkflowChange,
  VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
  videoFrameImageContractForPromptType,
  videoGuidanceConfigurationError,
} from "./videoTabConstants";
import { isDurationAllowed, promptLengthError } from "./videoOptionUtils";
import {
  videoExecutionSegmentIdentityMatchesVideo,
  videoExecutionIdentityOfVideo,
  videoExecutionSegmentNumberInPlanByLineage,
} from "./videoExecutionIdentity";
import {
  normalizeReferenceMediaRows,
  referenceMediaDraftError,
  referenceMediaRowsByKindFromDraft,
  type ReferenceMediaDraft,
} from "./referenceMediaWorkspace";
import type { useVideoTabState } from "./useVideoTabState";
import type { useVideoTabDerivedState } from "./useVideoTabDerivedState";
import type { useVideoTabViewState } from "./useVideoTabViewState";
import type { RunProgressTask as ProgressTask } from "../../../hooks/useProgressTask";

type VideoTabStateValue = ReturnType<typeof useVideoTabState>;
type DerivedVideoTabState = ReturnType<typeof useVideoTabDerivedState>;
type VideoTabViewStateValue = ReturnType<typeof useVideoTabViewState>;

type VideoTabActionInput = VideoTabStateValue &
  DerivedVideoTabState &
  VideoTabViewStateValue & {
    projectId: number;
    chapter: ChapterRow | null;
    runProgressTask: ProgressTask;
    loadAll: () => Promise<boolean>;
    t: MessageTranslator;
    addToast: (message: string, type: "success" | "error" | "info") => void;
    runGenerateFramePrompts: (
      recipe: VideoSegmentExecutionRecipe,
      input: {
        targetSegment: {
          source_shot_id: string;
          segment_index_in_shot: number;
        };
      },
    ) => Promise<unknown>;
    runGenerateVideoPrompts: (
      recipe: VideoSegmentExecutionRecipe,
      input: {
        targetSegment: {
          source_shot_id: string;
          segment_index_in_shot: number;
        };
      },
    ) => Promise<unknown>;
    handleSavePromptRow: (
      prompt: ChapterVideoPrompt,
      input: {
        visual_prompt: string;
        motion_prompt: string;
        negative_prompt: string;
        duration_secs: number;
      },
    ) => Promise<unknown>;
  };

interface ReferenceMediaSegmentState {
  segmentPlanPromptId: number;
  segmentNumber: number;
  targetSegment: {
    source_shot_id: string;
    segment_index_in_shot: number;
  };
}

export function useVideoTabActions(context: VideoTabActionInput) {
  const {
    projectId,
    chapter,
    runProgressTask,
    loadAll,
    t,
    addToast,
    runGenerateFramePrompts,
    runGenerateVideoPrompts,
    handleSavePromptRow,
    acting,
    beginVideoAction,
    isCurrentVideoActionScope,
    finishVideoAction,
    setPrompts,
    setTimeline,
    frameSegmentPlanPromptForCards,
    segmentCardState,
    effectiveRecipeBySegmentRef,
    selectedSegmentNumber,
    setSelectedVideoId,
    setPendingVideoId,
    options,
    updateSegmentRecipe,
    awaitSegmentRecipeSaved,
    referenceMediaDraftScopeForSegment,
    referenceMediaDraft,
    setWorkspaceTab,
    referenceMediaDraftRef,
    referenceMediaDraftDirty,
    referenceMediaDraftScopeRef,
    activeVideoConfig,
    promptRequestPreviewScopeRef,
    setPromptRequestPreviewOpen,
    setPromptRequestPreviewLoading,
    setPromptRequestPreviewData,
    imageRequestPreviewScopeRef,
    setImageRequestPreviewOpen,
    setImageRequestPreviewLoading,
    setImageRequestPreviewData,
    effectiveDurationLimits,
    videoOptionsStateError,
    frameSegmentIdentityIndex,
    recipeFingerprintBySegment,
    finalPromptBlockedReasons,
    saveReferenceMediaDraft,
    confirmSegmentRecipeChange,
    handleSelectSegment,
    videoFrameImages,
    currentLanguageFramePrompts,
  } = context;
  const tRef = useRef(t);
  tRef.current = t;
  const currentT = () => tRef.current;
  const videoActionScopeKey = context.videoActionScopeKey;
  const workflowMode = context.workflowMode;

  const refreshVideoData = async (requestScope: string | null): Promise<boolean> => {
    if (!isCurrentVideoActionScope(requestScope)) return true;
    let refreshed = false;
    try {
      refreshed = await loadAll();
    } catch {
      refreshed = false;
    }
    return isCurrentVideoActionScope(requestScope) ? refreshed : true;
  };

  const handleSaveSegmentPlan = useCallback(
    async (segments: VideoSegmentPlanV2[]): Promise<boolean> => {
      const prompt = frameSegmentPlanPromptForCards;
      if (!prompt || prompt.status === "running" || acting != null) return false;
      if (!beginVideoAction(`edit-segment-plan-${prompt.id}`)) {
        return false;
      }
      const actionKey = `edit-segment-plan-${prompt.id}`;
      const requestScope = videoActionScopeKey;
      try {
        const updated = await updateChapterVideoSegmentPlan(prompt.id, {
          expected_updated_at: prompt.updated_at,
          segments,
        });
        if (!isCurrentVideoActionScope(requestScope)) return false;
        setPrompts((previous) =>
          previous.map((item) => (item.id === updated.id ? updated : item)),
        );
        const refreshed = await refreshVideoData(requestScope);
        if (!isCurrentVideoActionScope(requestScope)) return false;
        if (!refreshed) addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
        addToast(currentT()("video.chapter.planSaved"), "success");
        return true;
      } catch (error) {
        if (!isCurrentVideoActionScope(requestScope)) return false;
        addToast(
          extractErrorMessage(error, currentT()("video.chapter.planSaveFailed")),
          "error",
        );
        return false;
      } finally {
        finishVideoAction(actionKey, requestScope);
      }
    },
    [
      acting,
      addToast,
      beginVideoAction,
      finishVideoAction,
      frameSegmentPlanPromptForCards,
      isCurrentVideoActionScope,
      loadAll,
      t,
      videoActionScopeKey,
    ],
  );

  const recipeForSegmentAction = (
    segmentNumber: number,
  ): VideoSegmentExecutionRecipe | null => {
    const recipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber) ?? null;
    if (!recipe) {
      addToast(
        currentT()("video.chapter.segmentRecipeMissingForAction", {
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
        currentT()("video.chapter.segmentIdentityMissing", { number: segmentNumber }),
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
      addToast(currentT()("video.chapter.referenceMediaSelectionRequired"), "error");
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
      addToast(currentT()("video.chapter.selectedVideoMissingRecipe"), "error");
      return;
    }
    const targetSegment = targetForSegmentAction(shotNumber);
    if (!targetSegment) return;
    if (selectedSegmentNumber !== shotNumber) {
      addToast(
        currentT()("video.chapter.videoBelongsToOtherSegment", {
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
      addToast(currentT()("video.chapter.videoNotLatestRecipe"), "error");
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
      const refreshed = await refreshVideoData(requestScope);
      if (!isCurrentVideoActionScope(requestScope)) return;
      if (!refreshed) addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
      addToast(currentT()("video.chapter.adoptedSegment", { number: shotNumber }), "success");
    } catch (err) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, currentT()("video.chapter.adoptVideoFailed")), "error");
    } finally {
      finishVideoAction(actionKey, requestScope);
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
        currentT()("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        }),
        "error",
      );
      return;
    }
    const guidanceMode = guidanceModeForWorkflowChange(mode, segmentNumber);
    if (currentRecipe.workflow_mode === mode && !videoGuidanceConfigurationError(mode, segmentNumber, currentRecipe.guidance_mode, t)) {
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
        currentT()("video.chapter.workflowMode"),
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
        currentT()("video.chapter.segmentRecipeMissingForAction", {
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
      addToast(currentT()("video.chapter.firstFrameChainFollowupInputRequired"), "error");
      return;
    }
    if (currentRecipe.guidance_mode === guidanceMode) return;
    if (
      !(await confirmSegmentRecipeChange(
        segmentNumber,
        currentT()("video.chapter.startFrameSource"),
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
        currentT()("video.chapter.segmentRecipeMissingForAction", {
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
        currentT()("video.chapter.sourceVideo"),
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
      addToast(currentT()("video.chapter.noChapter"), "error");
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
          title: currentT()("video.chapter.insertedDescription"),
          startMessage: currentT()("video.chapter.insertedDescriptionPreparing"),
          successMessage: currentT()("video.chapter.insertedDescriptionGenerated"),
          errorMessage: currentT()("video.chapter.insertedDescriptionFailed"),
          cancelLabel: currentT()("video.chapter.stopGeneration"),
        },
        async (task) => {
          const result = task.resultRef;
          if (result?.kind !== "inserted_segment_draft") {
            throw new Error(currentT()("video.chapter.insertedDraftMissing"));
          }
          const parsed = Number(result.id);
          if (!Number.isSafeInteger(parsed) || parsed <= 0) {
            throw new Error(currentT()("video.chapter.invalidInsertedWorkflowId"));
          }
          return parsed;
        },
      );
      if (!isCurrentVideoActionScope(requestScope)) return null;
      addToast(currentT()("video.chapter.insertedDescriptionConfirm"), "success");
      return workflowRunId;
    } catch (error) {
      if (isCurrentVideoActionScope(requestScope)) {
        const message = extractErrorMessage(
          error,
          currentT()("video.chapter.insertedDescriptionFailed"),
        );
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
      return null;
    } finally {
      finishVideoAction(actionKey, requestScope);
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
      const refreshed = await refreshVideoData(requestScope);
      if (!isCurrentVideoActionScope(requestScope)) return false;
      if (!refreshed) addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
      addToast(currentT()("video.chapter.insertedSubmitted"), "success");
      return true;
    } catch (error) {
      if (!isCurrentVideoActionScope(requestScope)) return false;
      const message = extractErrorMessage(error, currentT()("video.chapter.operationFailed"));
      addToast(
        message.includes("timeline_revision_conflict")
          ? currentT()("video.chapter.timelineRevisionConflict", { error: message })
          : message,
        "error",
      );
      try {
        const refreshed = await refreshVideoData(requestScope);
        if (!refreshed) addToast(currentT()("video.chapter.refreshTimelineFailed"), "error");
      } catch (refreshError) {
        addToast(
          extractErrorMessage(refreshError, currentT()("video.chapter.refreshTimelineFailed")),
          "error",
        );
      }
      return false;
    } finally {
      finishVideoAction(actionKey, requestScope);
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
      const refreshed = await refreshVideoData(requestScope);
      if (!isCurrentVideoActionScope(requestScope)) return;
      if (!refreshed) addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
      addToast(currentT()("video.chapter.insertedPositionUpdated"), "success");
    } catch (error) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(error, currentT()("video.chapter.operationFailed")), "error");
      try {
        const refreshed = await refreshVideoData(requestScope);
        if (!refreshed) addToast(currentT()("video.chapter.refreshTimelineFailed"), "error");
      } catch (refreshError) {
        addToast(
          extractErrorMessage(refreshError, currentT()("video.chapter.refreshTimelineFailed")),
          "error",
        );
      }
    } finally {
      finishVideoAction(actionKey, requestScope);
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
      const refreshed = await refreshVideoData(requestScope);
      if (!isCurrentVideoActionScope(requestScope)) return;
      if (!refreshed) addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
      addToast(currentT()("video.chapter.insertedRemoved"), "success");
    } catch (error) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(error, currentT()("video.chapter.operationFailed")), "error");
      try {
        const refreshed = await refreshVideoData(requestScope);
        if (!refreshed) addToast(currentT()("video.chapter.refreshTimelineFailed"), "error");
      } catch (refreshError) {
        addToast(
          extractErrorMessage(refreshError, currentT()("video.chapter.refreshTimelineFailed")),
          "error",
        );
      }
    } finally {
      finishVideoAction(actionKey, requestScope);
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
          title: currentT()("video.chapter.insertedVideoPreparing"),
          startMessage: currentT()("video.chapter.insertedVideoPreparing"),
          successMessage: currentT()("video.chapter.insertedVideoGenerated"),
          errorMessage: currentT()("video.chapter.insertedVideoFailed"),
          cancelLabel: currentT()("video.chapter.stopGeneration"),
        },
        async (task) => {
          await verifyVideoGenerationResult(task, projectId, chapter.id);
        },
      );
      if (!isCurrentVideoActionScope(requestScope)) return;
      const refreshed = await refreshVideoData(requestScope);
      if (!isCurrentVideoActionScope(requestScope)) return;
      if (!refreshed) addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
      addToast(currentT()("video.chapter.insertedVideoGenerated"), "success");
    } catch (error) {
      if (isCurrentVideoActionScope(requestScope)) {
        const message = extractErrorMessage(
          error,
          currentT()("video.chapter.insertedVideoFailed"),
        );
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
    } finally {
      finishVideoAction(actionKey, requestScope);
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
      addToast(currentT()("video.chapter.chatPromptEmpty"), "error");
      return;
    }
    try {
      await copyTextToClipboard(text);
      addToast(currentT()("video.chapter.copiedChatPrompt"), "success");
    } catch (err) {
      addToast(
        extractErrorMessage(err, currentT()("video.chapter.copyChatPromptFailed")),
        "error",
      );
    }
  };

  const buildFrameImageGenerationInput = (
    prompt: ChapterVideoPrompt,
  ): Record<string, unknown> => {
    const frameContract = videoFrameImageContractForPromptType(
      prompt.prompt_type,
      currentT(),
    );
    const trimmedPrompt = prompt.visual_prompt.trim();
    const previewValidationError = promptLengthError(
      frameContract.promptLabel,
      trimmedPrompt,
      VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
      currentT(),
    );
    if (previewValidationError) {
      throw new Error(previewValidationError);
    }
    const references = frameImageReferencesForPrompt(
      prompt,
      videoFrameImages,
      currentLanguageFramePrompts,
      currentT(),
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
      currentT(),
    );
    if (referenceOrderError) {
      throw new Error(referenceOrderError);
    }
    return buildVideoFrameImageRequest(
      frameContract.imageType,
      effectiveVisualPrompt,
      prompt.chapter_id || chapter?.id || 0,
      prompt.aspect_ratio,
      buildVideoFrameImageTraceMetadata(prompt, frameContract.source, currentT()),
      references.ids,
      references.note,
      currentT(),
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
      currentT()("video.chapter.requestPreviewOperation"),
      currentT(),
    );
    const missingPrompts = chain.filter(
      (item) => !latestVideoFrameImageForPrompt(item, videoFrameImages, currentT()),
    );
    if (missingPrompts.length === 0) {
      addToast(
        currentT()("video.chapter.boundaryImagesAlreadyExist", {
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
        ? currentT()("video.chapter.boundaryPreviewNotice", {
            first: firstMissing.shot_number,
            count: dynamicTailCount,
          })
        : null;
    return {
      requests: [buildFrameImageGenerationInput(firstMissing)],
      note: extraPreviewNote,
    };
  };

  const closePromptRequestPreview = () => {
    promptRequestPreviewScopeRef.current += 1;
    setPromptRequestPreviewOpen(false);
    setPromptRequestPreviewLoading(false);
    setPromptRequestPreviewData(null);
  };

  const closeImageRequestPreview = () => {
    imageRequestPreviewScopeRef.current += 1;
    setImageRequestPreviewOpen(false);
    setImageRequestPreviewLoading(false);
    setImageRequestPreviewData(null);
  };

  const previewImageRequests = async (
    requests: Array<Record<string, unknown>>,
    note: string | null = null,
  ) => {
    if (!chapter) return;
    if (requests.length === 0) {
      addToast(currentT()("video.chapter.previewImageRequestsEmpty"), "error");
      return;
    }
    const previewScope = ++imageRequestPreviewScopeRef.current;
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
      addToast(extractErrorMessage(err, currentT()("video.chapter.viewImageRequestFailed")), "error");
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
      addToast(extractErrorMessage(err, currentT()("video.chapter.previewImageRequestFailed")), "error");
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
      addToast(currentT()("video.chapter.referencePreviewOnly"), "error");
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
      addToast(currentT()("video.reference.durationUnsupported"), "error");
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
      currentT(),
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
    const previewScope = ++promptRequestPreviewScopeRef.current;
    const input = await prepareReferenceMediaVideoInput();
    if (!input || promptRequestPreviewScopeRef.current !== previewScope) return;
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
        extractErrorMessage(err, currentT()("video.chapter.previewReferenceMediaFailed")),
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
      addToast(currentT()("video.chapter.selectReferenceMediaSegment"), "error");
      return;
    }
    try {
      if (!(await saveReferenceMediaDraft(selectedSegmentNumber))) return;
      addToast(currentT()("video.chapter.referenceMediaDraftSaved"), "success");
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
    let generated = false;
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
          title: currentT()("video.chapter.referenceMediaVideoTitle"),
          startMessage: currentT()("video.chapter.referenceMediaVideoPreparing"),
          successMessage: currentT()("video.chapter.referenceMediaVideoGenerated"),
          errorMessage: currentT()("video.chapter.referenceMediaVideoFailed"),
          cancelLabel: currentT()("video.chapter.stopGeneration"),
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
      generated = true;
    } catch (err) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      const message = extractErrorMessage(
        err,
        currentT()("video.chapter.referenceMediaVideoFailed"),
      );
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentVideoActionScope(requestScope)) {
        const refreshed = await loadAll();
        if (generated && refreshed !== false && isCurrentVideoActionScope(requestScope)) {
          addToast(currentT()("video.chapter.referenceMediaVideoGenerated"), "success");
        }
      }
      finishVideoAction(actionKey, requestScope);
    }
  };

  const handleImportReferenceMediaVideo = async () => {
    if (!chapter) return;
    const segment = referenceMediaSegmentFor(false);
    if (!segment) return;
    if (!referenceMediaDraft.prompt.trim()) {
      addToast(currentT()("video.chapter.mediaPromptEmpty"), "error");
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
      upload = await pickVideoUploadFile(t, currentT()("video.reference.video"));
    } catch (err) {
      if (isCurrentVideoActionScope(requestScope)) {
        addToast(
          extractErrorMessage(err, currentT()("video.chapter.filePickerFailed")),
          "error",
        );
      }
      finishVideoAction(actionKey, requestScope);
      return;
    }
    if (!upload) {
      finishVideoAction(actionKey, requestScope);
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
      const refreshed = await refreshVideoData(requestScope);
      if (!isCurrentVideoActionScope(requestScope)) return;
      if (!refreshed) addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
      addToast(currentT()("video.chapter.importedReferenceMediaVideo"), "success");
    } catch (err) {
      if (isCurrentVideoActionScope(requestScope)) {
        addToast(
          extractErrorMessage(err, currentT()("video.chapter.importReferenceMediaVideoFailed")),
          "error",
        );
      }
    } finally {
      finishVideoAction(actionKey, requestScope);
    }
  };



  return {
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
  };
}
