import type { Dispatch, SetStateAction } from "react";
import type { ToastType } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  startImageGeneration,
  uploadImage,
  verifyImageGenerationResult,
  type GeneratedImage,
} from "../../../api/image";
import { generationClient } from "../../../api/generationTasks";
import {
  runGenerationProgressTaskThen,
  type GenerationProgressUiOptions,
} from "../../../hooks/runGenerationProgressTask";
import type { GenerationTaskHandle } from "../../../api/generated/generationContracts";
import {
  startChapterVideoFramePrompts,
  startChapterVideoPrompts,
  startChapterVideoSegmentPlan,
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type GeneratePromptOptions,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentWorkflowMode,
} from "../../../api/video";
import { readFileAsBase64, pickLocalFile } from "../../../utils/fileUpload";
import {
  isSupportedImageMime,
  MAX_UPLOAD_IMAGE_BYTES,
  supportedImageAccept,
} from "../../../utils/imageUpload";
import type { RunProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import type { ChapterRow } from "../shared";
import {
  buildVideoFrameImageTraceMetadata,
  buildVideoFrameImageRequest,
  boundaryFramePromptChainTo,
  frameImageReferencesForPrompt,
  isBoundaryFramePrompt,
  isSegmentPlanDraftCheckpointKey,
  latestVideoFrameImageForPrompt,
  referenceLabelOrderError,
  shiftReferenceLabels,
} from "./videoFramePromptUtils";
import { completedSegmentPlanPreviewItems } from "./videoSegmentPlan";
import {
  durationOptionsAllowed,
  isDurationAllowed,
  promptLengthError,
} from "./videoOptionUtils";
import {
  promptHasVideoWorkflowContract,
  promptOutputLanguageOfPrompt,
  promptSourcePromptType,
  readSavedReferenceImageIds,
  videoWorkflowModeOfPrompt,
  CHAT_VIDEO_DURATION_LIMITS,
} from "./videoImageUtils";
import {
  videoExecutionIdentityOfPrompt,
  videoExecutionSegmentNumberInPlan,
  videoExecutionSegmentNumberInPlanByLineage,
  videoExecutionIdentityMatches,
} from "./videoExecutionIdentity";
import {
  videoFrameImageContractForPromptType,
  type VideoDurationLimits,
  type VideoGenOptions,
  VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
  VIDEO_FIRST_FRAME_PROMPT_TYPE,
  VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION,
  VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
  VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
  VIDEO_FRAME_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION,
} from "./videoTabConstants";
import type { LoadedSegmentPlanDraft } from "./loadVideoTabData";
import { formatNumber, type Locale, type MessageTranslator } from "../../../i18n";

type AddToast = (message: string, type: ToastType) => void;
type PromptGenerationRequestOptions = {
  regenerateExisting?: boolean;
  targetSegment?: {
    source_shot_id: string;
    segment_index_in_shot: number;
  };
  targetNarrativeUnitIndex?: number;
};

export const runVideoPromptGenerationTask = async (
  runProgressTask: RunProgressTask,
  handle: GenerationTaskHandle,
  options: GenerationProgressUiOptions,
  refreshPrompts: () => Promise<void>,
  t: MessageTranslator,
  onDraftCheckpoint?: (text: string, checkpointKey: string) => void,
): Promise<void> => {
  let lastCommittedKey: string | null = null;
  let lastDraftCheckpointKey: string | null = null;
  let refreshQueue = Promise.resolve();
  let generationCompleted = false;
  const enqueueRefresh = () => {
    refreshQueue = refreshQueue.then(refreshPrompts).catch((error) => {
      console.warn("视频提示词增量刷新失败，将在任务结束后重新同步", error);
    });
  };
  const flushRefresh = async () => {
    await refreshQueue;
    await refreshPrompts();
  };
  try {
    await runGenerationProgressTaskThen(
      runProgressTask,
      handle,
      options,
      async () => {
        generationCompleted = true;
        await flushRefresh();
      },
      (task) => {
        const checkpointKey = task?.latestCheckpoint?.checkpoint_key;
        if (
          onDraftCheckpoint
          && checkpointKey
          && checkpointKey !== lastDraftCheckpointKey
        ) {
          lastDraftCheckpointKey = checkpointKey;
          onDraftCheckpoint(
            task?.streams.segment_plan_draft?.text ?? "",
            checkpointKey,
          );
        }
        const progress = task?.progress;
        if (
          progress?.stage_code !== "video_prompt_item_committed"
          || progress.phase_kind !== "save"
          || progress.item_key == null
        ) {
          return;
        }
        const committedKey = `${progress.item_key}:${progress.item_index ?? ""}`;
        if (committedKey === lastCommittedKey) return;
        lastCommittedKey = committedKey;
        enqueueRefresh();
      },
    );
  } catch (error) {
    if (generationCompleted) throw error;
    try {
      await flushRefresh();
    } catch (refreshError) {
      throw new Error(t("video.actions.refreshSubmittedFailed", {
        error: extractErrorMessage(refreshError, t("common.unknown")),
      }));
    }
    throw error;
  }
};

interface UseVideoPromptGenerationActionsParams {
  projectId: number;
  chapter: ChapterRow | null;
  options: VideoGenOptions;
  segmentPlanOptions: VideoGenOptions | null;
  durationLimits: VideoDurationLimits;
  promptInputSelectionError: string | null;
  referenceIds: number[];
  videoLibraryImageIds: number[];
  frameImages: GeneratedImage[];
  segmentPlanPrompt: ChapterVideoPrompt | null;
  segmentPlanDraft: LoadedSegmentPlanDraft | null;
  framePrompts: ChapterVideoPrompt[];
  allFramePrompts: ChapterVideoPrompt[];
  allFinalVideoPrompts: ChapterVideoPrompt[];
  currentContentHash: string | null;
  actionScopeKey: string | null;
  locale: Locale;
  t: MessageTranslator;
  addToast: AddToast;
  runProgressTask: RunProgressTask;
  loadAll: () => Promise<boolean>;
  refreshPrompts: () => Promise<void>;
  refreshSegmentPlanDraft: () => Promise<LoadedSegmentPlanDraft | null>;
  setSegmentPlanPreview: Dispatch<SetStateAction<Record<string, unknown>[]>>;
  beginAction: (key: string) => boolean;
  isCurrentActionScope: (scope: string | null) => boolean;
  finishAction: (key: string) => void;
}

export function useVideoPromptGenerationActions({
  projectId,
  chapter,
  options,
  segmentPlanOptions,
  durationLimits,
  promptInputSelectionError,
  referenceIds,
  videoLibraryImageIds,
  frameImages,
  segmentPlanPrompt,
  segmentPlanDraft,
  framePrompts,
  allFramePrompts,
  allFinalVideoPrompts,
  currentContentHash,
  actionScopeKey,
  locale,
  t,
  addToast,
  runProgressTask,
  loadAll,
  refreshPrompts,
  refreshSegmentPlanDraft,
  setSegmentPlanPreview,
  beginAction,
  isCurrentActionScope,
  finishAction,
}: UseVideoPromptGenerationActionsParams) {
  type ValidatedPromptOptions = VideoGenOptions & {
    workflowMode:
      | VideoSegmentWorkflowMode
      | typeof VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN;
    referenceIds: number[];
    videoLibraryImageIds: number[];
  };
  const validatedPromptOptions = (
    operation: string,
    recipe?: VideoSegmentExecutionRecipe,
    baseOptions: VideoGenOptions = options,
  ): ValidatedPromptOptions | null => {
    const effectiveOptions: VideoGenOptions = recipe
      ? {
          style: recipe.style,
          promptOutputLanguage: recipe.prompt_output_language,
          durationSecs: recipe.duration_secs,
          durationOptions: recipe.duration_secs_options,
          aspectRatio: recipe.aspect_ratio as VideoGenOptions["aspectRatio"],
        }
      : baseOptions;
    const operationDurationLimits =
      recipe?.workflow_mode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO
        ? CHAT_VIDEO_DURATION_LIMITS
        : durationLimits;
    const style = effectiveOptions.style.trim();
    if (!style) {
      addToast(t("video.actions.styleRequired", { operation }), "error");
      return null;
    }
    if (operationDurationLimits.configError) {
      addToast(operationDurationLimits.configError, "error");
      return null;
    }
    if (!recipe && promptInputSelectionError) {
      addToast(promptInputSelectionError, "error");
      return null;
    }
    if (!isDurationAllowed(effectiveOptions.durationSecs, operationDurationLimits)) {
      addToast(t("video.actions.durationUnsupportedForOperation", { operation }), "error");
      return null;
    }
    if (!durationOptionsAllowed(
      effectiveOptions.durationSecs,
      effectiveOptions.durationOptions,
      operationDurationLimits,
      t,
    )) {
      addToast(t("video.actions.durationOptionsUnsupported", { operation }), "error");
      return null;
    }
    return {
      ...effectiveOptions,
      style,
      workflowMode:
        recipe?.workflow_mode ?? VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
      referenceIds: recipe?.reference_image_ids ?? referenceIds,
      videoLibraryImageIds: recipe?.video_library_image_ids ?? videoLibraryImageIds,
    };
  };

  const runImageAssetTask = async (
    key: string,
    title: string,
    resultImageLabel: string,
    requests: Array<Record<string, unknown>>,
  ) => {
    const requestScope = actionScopeKey;
    if (!chapter) {
      finishAction(key);
      return;
    }
    if (requests.length === 0) {
      addToast(t("video.actions.atLeastOneFrameRequest"), "error");
      finishAction(key);
      return;
    }
    let refreshSucceeded = true;
    try {
      const handle = await startImageGeneration({
        project_id: projectId,
        operation: {
          kind: "batch",
          requests: [requests[0], ...requests.slice(1)],
        },
      });
      const generatedIds = await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title,
          startMessage: t("video.actions.preparing", { operation: title }),
          successMessage: t("video.actions.completed", { operation: title }),
          errorMessage: t("video.actions.failed", { operation: title }),
          cancelLabel: t("workflow.stop"),
        },
        async (task) => {
          const rows = await verifyImageGenerationResult(task, projectId);
          if (isCurrentActionScope(requestScope)) {
            refreshSucceeded = await loadAll();
          }
          return rows.map((row) => row.id);
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      if (!refreshSucceeded) return;
      addToast(t("video.actions.generatedFramesReused", { count: formatNumber(generatedIds.length, locale), label: resultImageLabel }), "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.actions.failed", { operation: title }));
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      finishAction(key);
    }
  };

  const importFrameImage = async (prompt: ChapterVideoPrompt) => {
    if (!chapter) return;
    let frameContract: ReturnType<typeof videoFrameImageContractForPromptType>;
    try {
      frameContract = videoFrameImageContractForPromptType(prompt.prompt_type, t);
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.actions.framePromptImportFailed")), "error");
      return;
    }
    const { frameLabel, imageType, source } = frameContract;
    if (prompt.status === "running") {
      addToast(t("video.actions.framePromptRunningCannot", {
        label: frameLabel,
        number: formatNumber(prompt.shot_number, locale),
        operation: t("common.import"),
      }), "error");
      return;
    }

    const requestScope = actionScopeKey;
    const actionKey = `import-frame-image-${prompt.id}`;
    if (!beginAction(actionKey)) return;
    let file: File | null = null;
    try {
      file = await pickLocalFile(supportedImageAccept());
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, t("video.chapter.filePickerFailed")), "error");
      return;
    } finally {
      if (!file) finishAction(actionKey);
    }
    if (!file) return;
    if (!isSupportedImageMime(file.type)) {
      addToast(t("video.actions.unsupportedImageFormat"), "error");
      finishAction(actionKey);
      return;
    }
    if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
      addToast(t("video.actions.imageTooLarge", { size: formatNumber(MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024, locale, { maximumFractionDigits: 1 }) }), "error");
      finishAction(actionKey);
      return;
    }
    if (!isCurrentActionScope(requestScope)) {
      finishAction(actionKey);
      return;
    }

    const imagePrompt =
      prompt.visual_prompt.trim() ||
      prompt.scene_summary?.trim() ||
      prompt.title?.trim() ||
      t("video.actions.imagePromptManualImport", {
        label: frameLabel,
        number: formatNumber(prompt.shot_number, locale),
      });
    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentActionScope(requestScope)) return;
      await uploadImage(projectId, {
        file_name: file.name,
        mime_type: file.type,
        data_base64: dataBase64,
        image_type: imageType,
        prompt: imagePrompt,
        chapter_id: chapter.id,
        metadata_json: buildVideoFrameImageTraceMetadata(prompt, source, t),
      });
      if (!isCurrentActionScope(requestScope)) return;
      const refreshed = await loadAll();
      if (refreshed === false) return;
      addToast(t("video.actions.frameImageImported", { label: frameLabel }), "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, t("video.actions.frameImageImportFailed", { label: frameLabel })), "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const promptGenerationOptions = (
    promptOptions: ValidatedPromptOptions,
    requestOptions: PromptGenerationRequestOptions = {},
  ) => ({
    workflow_mode: promptOptions.workflowMode,
    segment_plan_prompt_id:
      requestOptions.targetSegment == null ? undefined : segmentPlanPrompt?.id,
    style: promptOptions.style,
    prompt_output_language: promptOptions.promptOutputLanguage,
    prompt_batch_size: 1,
    target_segment: requestOptions.targetSegment,
    target_narrative_unit_index: requestOptions.targetNarrativeUnitIndex,
    duration_secs: promptOptions.durationSecs,
    duration_secs_options: promptOptions.durationOptions,
    aspect_ratio: promptOptions.aspectRatio,
    reference_image_ids: promptOptions.referenceIds,
    video_library_image_ids: promptOptions.videoLibraryImageIds,
    regenerate_existing: requestOptions.regenerateExisting ?? false,
  });

  const framePromptTypeForWorkflow = (
    mode: VideoSegmentWorkflowMode,
  ): string | null => {
    if (mode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES) return VIDEO_FRAME_PROMPT_TYPE;
    if (mode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN) return VIDEO_FIRST_FRAME_PROMPT_TYPE;
    if (mode === VIDEO_WORKFLOW_MODE_STORYBOARD) return VIDEO_STORYBOARD_PROMPT_TYPE;
    return null;
  };

  const framePromptContractForWorkflow = (
    mode: VideoSegmentWorkflowMode,
  ): string | null => {
    if (mode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES)
      return VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION;
    if (mode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN)
      return VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION;
    if (mode === VIDEO_WORKFLOW_MODE_STORYBOARD)
      return VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION;
    return null;
  };

  const recordFromUnknown = (value: unknown): Record<string, unknown> | null =>
    value && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : null;

  const idsEqual = (actual: number[], expected: number[]): boolean =>
    actual.length === expected.length && actual.every((id, index) => id === expected[index]);

  const promptGenerationAssetIdsMatch = (
    prompt: ChapterVideoPrompt,
    expectedReferenceIds: number[],
    expectedVideoLibraryIds: number[],
  ): boolean => {
    const metadata = recordFromUnknown(prompt.metadata_json);
    const fingerprint = recordFromUnknown(metadata?.generation_asset_fingerprint);
    if (!fingerprint) return false;
    const savedReferenceIds = readSavedReferenceImageIds(
      fingerprint.reference_image_ids,
      t,
      "generation_asset_fingerprint.reference_image_ids",
    );
    const savedVideoLibraryIds = readSavedReferenceImageIds(
      fingerprint.video_library_image_ids,
      t,
      "generation_asset_fingerprint.video_library_image_ids",
    );
    if (savedReferenceIds.error || savedVideoLibraryIds.error) return false;
    return (
      idsEqual(savedReferenceIds.ids, expectedReferenceIds) &&
      idsEqual(savedVideoLibraryIds.ids, expectedVideoLibraryIds)
    );
  };

  const promptSourceContextMatches = (
    prompt: ChapterVideoPrompt,
    promptOptions: ValidatedPromptOptions,
  ): boolean => {
    const metadata = recordFromUnknown(prompt.metadata_json);
    const source = recordFromUnknown(metadata?.source);
    if (!source) return false;
    if (currentContentHash && source.content_sha256 !== currentContentHash) {
      return false;
    }
    const savedDurationOptions = readSavedReferenceImageIds(
      source.duration_secs_options,
      t,
      "source.duration_secs_options",
    );
    if (savedDurationOptions.error) return false;
    return idsEqual(
      savedDurationOptions.ids,
      promptOptions.durationOptions,
    );
  };

  const framePromptMatchesCurrentGenerationContext = (
    prompt: ChapterVideoPrompt,
    promptType: string,
    contractVersion: string,
    promptOptions: ValidatedPromptOptions,
  ): boolean => {
    return (
      prompt.prompt_type === promptType &&
      videoWorkflowModeOfPrompt(prompt) === promptOptions.workflowMode &&
      promptHasVideoWorkflowContract(prompt, contractVersion) &&
      promptSourcePromptType(prompt) === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE &&
      videoExecutionIdentityMatches(prompt, segmentPlanPrompt) &&
      promptOutputLanguageOfPrompt(prompt) === promptOptions.promptOutputLanguage &&
      (prompt.style ?? "") === promptOptions.style &&
      prompt.aspect_ratio === promptOptions.aspectRatio &&
      promptSourceContextMatches(prompt, promptOptions) &&
      promptGenerationAssetIdsMatch(
        prompt,
        promptOptions.referenceIds,
        promptOptions.videoLibraryImageIds,
      )
    );
  };

  const handleGenerateFramePrompts = async (
    recipe: VideoSegmentExecutionRecipe,
    requestOptions: PromptGenerationRequestOptions = {},
  ) => {
    if (!chapter) return;
    const targetWorkflowMode = recipe.workflow_mode;
    if (
      targetWorkflowMode !== VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES &&
      targetWorkflowMode !== VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN &&
      targetWorkflowMode !== VIDEO_WORKFLOW_MODE_STORYBOARD
    ) {
      addToast(t("video.actions.frameChainUnsupported"), "error");
      return;
    }
    const segmentNumber = recipe.segment_number;
    const firstFrameMode =
      targetWorkflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN;
    const label =
      targetWorkflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
          ? t("video.actions.storyboardPromptLabel")
        : firstFrameMode
          ? t("video.actions.firstFramePromptLabel")
          : t("video.actions.boundaryPromptLabel");
    if (firstFrameMode && recipe.guidance_mode === "continuity") {
      addToast(
        t("video.actions.continuityFrameInfo"),
        "info",
      );
      return;
    }
    const regenerateExisting = requestOptions.regenerateExisting ?? false;
    const operationLabel = regenerateExisting
      ? t("video.actions.regenerateTitle", { label })
      : t("video.actions.generateTitle", { label });
    const runningFramePrompt = allFramePrompts.find(
      (prompt) =>
        prompt.status === "running" &&
        videoExecutionSegmentNumberInPlan(
          videoExecutionIdentityOfPrompt(prompt),
          segmentPlanPrompt,
        ) === segmentNumber,
    );
    if (runningFramePrompt) {
      addToast(
        t("video.actions.framePromptRunningCannot", {
          label,
          number: formatNumber(runningFramePrompt.shot_number, locale),
          operation: t("video.actions.generate"),
        }),
        "error",
      );
      return;
    }
    const promptOptions = validatedPromptOptions(label, recipe);
    if (!promptOptions) return;
    const expectedFrameCount =
      targetWorkflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES ? 2 : 1;
    const framePromptType = framePromptTypeForWorkflow(targetWorkflowMode);
    const framePromptContract = framePromptContractForWorkflow(targetWorkflowMode);
    if (!segmentPlanPrompt) {
      addToast(t("video.actions.segmentPlanRequired"), "error");
      return;
    }
    if (!framePromptType || !framePromptContract) {
      addToast(t("video.actions.frameContractMissing"), "error");
      return;
    }
    const targetFrameNumbers =
      targetWorkflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
        ? [1, 2]
        : targetWorkflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
          ? [1]
          : [segmentNumber];
    const targetFramePrompts = allFramePrompts.filter(
      (prompt) =>
        videoExecutionSegmentNumberInPlan(
          videoExecutionIdentityOfPrompt(prompt),
          segmentPlanPrompt,
        ) === segmentNumber &&
        framePromptMatchesCurrentGenerationContext(
          prompt,
          framePromptType,
          framePromptContract,
          promptOptions,
        ),
    );
    const completedFrameNumbers = new Set(targetFramePrompts.map((prompt) => prompt.shot_number));
    const completedMatchingCount = targetFrameNumbers.filter(
      (frameNumber) => completedFrameNumbers.has(frameNumber),
    ).length;
    if (!regenerateExisting && completedMatchingCount >= expectedFrameCount) {
      addToast(
        t("video.actions.allFramesGenerated", { label }),
        "info",
      );
      return;
    }
    const requestScope = actionScopeKey;
    const actionKey = "generate-frame-prompts";
    if (!beginAction(actionKey)) return;
    if (regenerateExisting && targetFramePrompts.length > 0) {
      const ok = await confirmDialog({
        title: t("video.actions.regenerateTitle", { label }),
        message: t("video.actions.regenerateFrameMessage", { label }),
        confirmText: t("video.actions.regenerateConfirm"),
      });
      if (!ok) {
        finishAction(actionKey);
        return;
      }
      if (!isCurrentActionScope(requestScope)) {
        finishAction(actionKey);
        return;
      }
    }
    try {
      const handle = await startChapterVideoFramePrompts(
        projectId,
        chapter.id,
        {
          ...promptGenerationOptions(promptOptions, requestOptions),
          prompt_batch_size: expectedFrameCount,
        },
      );
      await runVideoPromptGenerationTask(
        runProgressTask,
        handle,
        {
          title: operationLabel,
          startMessage: t("video.actions.preparing", { operation: operationLabel }),
          successMessage: t("video.actions.completed", { operation: operationLabel }),
          errorMessage: t("video.actions.failed", { operation: operationLabel }),
        },
        refreshPrompts,
        t,
        undefined,
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(t("video.actions.completed", { operation: operationLabel }), "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.actions.failed", { operation: operationLabel }));
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const handleGenerateSegmentPlan = async (
    requestOptions: PromptGenerationRequestOptions = {},
  ) => {
    if (!chapter) return;
    const label = t("video.actions.segmentPlanLabel");
    const regenerateExisting = requestOptions.regenerateExisting ?? false;
    const resumedDraft = segmentPlanDraft;
    const effectiveRegenerateExisting =
      resumedDraft?.resumeOptions.regenerate_existing ?? regenerateExisting;
    let generationOptions: GeneratePromptOptions | null =
      resumedDraft?.resumeOptions ?? null;
    if (!generationOptions) {
      if (!segmentPlanOptions) {
        addToast(t("video.actions.segmentPlanConfigInvalid"), "error");
        return;
      }
      const validatedOptions = validatedPromptOptions(
        label,
        undefined,
        segmentPlanOptions,
      );
      if (!validatedOptions) return;
      generationOptions = {
        ...promptGenerationOptions(
          {
            ...validatedOptions,
            referenceIds: [],
            videoLibraryImageIds: [],
          },
          requestOptions,
        ),
        workflow_mode: VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
      };
    }
    const requestScope = actionScopeKey;
    const actionKey = "generate-segment-plan";
    if (!beginAction(actionKey)) return;
    if (effectiveRegenerateExisting && segmentPlanPrompt && !resumedDraft) {
      const ok = await confirmDialog({
        title: t("video.actions.regenerateTitle", { label }),
        message: t("video.actions.regenerateSegmentPlanMessage", { label }),
        confirmText: t("video.actions.regenerateConfirm"),
      });
      if (!ok) {
        finishAction(actionKey);
        return;
      }
      if (!isCurrentActionScope(requestScope)) {
        finishAction(actionKey);
        return;
      }
    }
    if (effectiveRegenerateExisting && !resumedDraft) setSegmentPlanPreview([]);
    const persistedProgress = resumedDraft?.workflowProgress;
    const resumeStartMessage = persistedProgress
      ? t("video.actions.resumePersisted", { percent: formatNumber(Math.round(persistedProgress.percent), locale), detail: persistedProgress.message.trim() ? ` · ${persistedProgress.message.trim()}` : "" })
      : resumedDraft && resumedDraft.completedShotCount > 0
        ? t("video.actions.resumeShots", { completed: formatNumber(resumedDraft.completedShotCount, locale), total: formatNumber(resumedDraft.totalShotCount, locale) })
        : t("video.actions.preparing", { operation: effectiveRegenerateExisting ? t("video.actions.regenerateTitle", { label }) : t("video.actions.generateTitle", { label }) });
    let previewFingerprint = "";
    let generated = false;
    try {
      const handle = await startChapterVideoSegmentPlan(
        projectId,
        chapter.id,
        generationOptions,
        resumedDraft?.resumeTaskId,
      );
      await runVideoPromptGenerationTask(
        runProgressTask,
        handle,
        {
          title: effectiveRegenerateExisting ? t("video.actions.regenerateTitle", { label }) : t("video.actions.generateTitle", { label }),
          startMessage: resumeStartMessage,
          successMessage: t("video.actions.completed", { operation: effectiveRegenerateExisting ? t("video.actions.regenerateTitle", { label }) : t("video.actions.generateTitle", { label }) }),
          errorMessage: t("video.actions.failed", { operation: effectiveRegenerateExisting ? t("video.actions.regenerateTitle", { label }) : t("video.actions.generateTitle", { label }) }),
        },
        refreshPrompts,
        t,
        (streamText, checkpointKey) => {
          if (!isCurrentActionScope(requestScope)) return;
          if (!isSegmentPlanDraftCheckpointKey(checkpointKey)) return;
          const preview = completedSegmentPlanPreviewItems(streamText);
          const fingerprint = JSON.stringify(preview);
          if (fingerprint === previewFingerprint) return;
          previewFingerprint = fingerprint;
          setSegmentPlanPreview(preview);
          void refreshSegmentPlanDraft().catch(() => undefined);
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      generated = true;
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.actions.failed", { operation: effectiveRegenerateExisting ? t("video.actions.regenerateTitle", { label }) : t("video.actions.generateTitle", { label }) }));
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentActionScope(requestScope)) {
        const refreshed = await loadAll();
        if (generated && refreshed !== false && isCurrentActionScope(requestScope)) {
          addToast(t("video.actions.completed", { operation: effectiveRegenerateExisting ? t("video.actions.regenerateTitle", { label }) : t("video.actions.generateTitle", { label }) }), "success");
        }
      }
      finishAction(actionKey);
    }
  };

  const handleGenerateVideoPrompts = async (
    recipe: VideoSegmentExecutionRecipe,
    requestOptions: PromptGenerationRequestOptions = {},
  ) => {
    if (!chapter) return;
    const targetWorkflowMode = recipe.workflow_mode;
    if (
      targetWorkflowMode !== VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES &&
      targetWorkflowMode !== VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN &&
      targetWorkflowMode !== VIDEO_WORKFLOW_MODE_STORYBOARD &&
      targetWorkflowMode !== VIDEO_WORKFLOW_MODE_CHAT_VIDEO
    ) {
      addToast(t("video.actions.generateModeUnavailable"), "error");
      return;
    }
    const segmentNumber = recipe.segment_number;
    const chatVideoMode = targetWorkflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO;
    const promptLabel = chatVideoMode
      ? t("video.actions.chatVideoPromptLabel")
      : targetWorkflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
        ? t("video.actions.storyboardVideoPromptLabel")
        : t("video.actions.videoPromptLabel");
    const regenerateExisting = requestOptions.regenerateExisting ?? false;
    const runningFramePrompt = allFramePrompts.find(
      (prompt) =>
        prompt.status === "running" &&
        videoExecutionSegmentNumberInPlan(
          videoExecutionIdentityOfPrompt(prompt),
          segmentPlanPrompt,
        ) === segmentNumber,
    );
    if (runningFramePrompt) {
      const label =
        runningFramePrompt.prompt_type === VIDEO_FIRST_FRAME_PROMPT_TYPE
          ? t("video.actions.firstFramePromptLabel")
          : runningFramePrompt.prompt_type === VIDEO_STORYBOARD_PROMPT_TYPE
            ? t("video.actions.storyboardPromptLabel")
          : t("video.actions.boundaryPromptLabel");
      addToast(t("video.actions.framePromptRunningCannot", {
        label,
        number: formatNumber(runningFramePrompt.shot_number, locale),
        operation: t("video.actions.generate"),
      }), "error");
      return;
    }
    const runningPrompt = allFinalVideoPrompts.find(
      (prompt) =>
        prompt.status === "running" &&
        videoExecutionSegmentNumberInPlanByLineage(
          videoExecutionIdentityOfPrompt(prompt),
          segmentPlanPrompt,
        ) === segmentNumber,
    );
    if (runningPrompt) {
      addToast(t("video.actions.finalPromptRunningCannot", { id: formatNumber(runningPrompt.id, locale) }), "error");
      return;
    }
    const targetFinalVideoPrompts = allFinalVideoPrompts.filter(
      (prompt) =>
        videoExecutionSegmentNumberInPlanByLineage(
          videoExecutionIdentityOfPrompt(prompt),
          segmentPlanPrompt,
        ) === segmentNumber &&
        videoWorkflowModeOfPrompt(prompt) === targetWorkflowMode &&
        promptOutputLanguageOfPrompt(prompt) === recipe.prompt_output_language,
    );
    const generatedVideoPromptCount = targetFinalVideoPrompts.filter(
      (prompt) => prompt.generated_video_id != null,
    ).length;
    if (!segmentPlanPrompt) {
      addToast(t("video.actions.videoPromptsRequirePlan"), "error");
      return;
    }
    if (regenerateExisting && generatedVideoPromptCount > 0) {
      addToast(
        t("video.actions.generatedVideosBlockRegenerate", { count: formatNumber(generatedVideoPromptCount, locale) }),
        "error",
      );
      return;
    }
    const promptOptions = validatedPromptOptions(promptLabel, recipe);
    if (!promptOptions) return;
    const requestScope = actionScopeKey;
    const actionKey = "generate-video-prompts";
    if (!beginAction(actionKey)) return;
    if (regenerateExisting && targetFinalVideoPrompts.length > 0) {
      const ok = await confirmDialog({
        title: t("video.actions.regenerateTitle", { label: promptLabel }),
        message: t("video.actions.regenerateVideoMessage", { label: promptLabel }),
        confirmText: t("video.actions.regenerateConfirm"),
      });
      if (!ok) {
        finishAction(actionKey);
        return;
      }
      if (!isCurrentActionScope(requestScope)) {
        finishAction(actionKey);
        return;
      }
    }
    try {
      const handle = await startChapterVideoPrompts(
        projectId,
        chapter.id,
        promptGenerationOptions(promptOptions, requestOptions),
      );
      await runVideoPromptGenerationTask(
        runProgressTask,
        handle,
        {
          title: regenerateExisting ? t("video.actions.regenerateTitle", { label: promptLabel }) : t("video.actions.generateTitle", { label: promptLabel }),
          startMessage: t("video.actions.preparing", { operation: regenerateExisting ? t("video.actions.regenerateTitle", { label: promptLabel }) : t("video.actions.generateTitle", { label: promptLabel }) }),
          successMessage: t("video.actions.completed", { operation: regenerateExisting ? t("video.actions.regenerateTitle", { label: promptLabel }) : t("video.actions.generateTitle", { label: promptLabel }) }),
          errorMessage: t("video.actions.failed", { operation: regenerateExisting ? t("video.actions.regenerateTitle", { label: promptLabel }) : t("video.actions.generateTitle", { label: promptLabel }) }),
        },
        refreshPrompts,
        t,
        undefined,
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(t("video.actions.completed", { operation: regenerateExisting ? t("video.actions.regenerateTitle", { label: promptLabel }) : t("video.actions.generateTitle", { label: promptLabel }) }), "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.actions.failed", { operation: regenerateExisting ? t("video.actions.regenerateTitle", { label: promptLabel }) : t("video.actions.generateTitle", { label: promptLabel }) }));
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const buildFrameImageGenerationRequest = (
    prompt: ChapterVideoPrompt,
    images: GeneratedImage[] = frameImages,
    prompts: ChapterVideoPrompt[] = framePrompts,
  ): Record<string, unknown> => {
    const { imageType, source, promptLabel: label } =
      videoFrameImageContractForPromptType(prompt.prompt_type, t);
    const framePrompt = prompt.visual_prompt.trim();
    const error = promptLengthError(
      label,
      framePrompt,
      VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
      t,
    );
    if (error) throw new Error(error);
    const references = frameImageReferencesForPrompt(prompt, images, prompts, t);
    if (references.error) {
      throw new Error(
        `${references.error}\n${t("video.chapter.frameImageMustPrepare", { label })}`,
      );
    }
    const effectiveFramePrompt = shiftReferenceLabels(
      framePrompt,
      references.promptReferenceOffset,
    );
    const referenceOrderError = referenceLabelOrderError(
      effectiveFramePrompt,
      references.ids.length,
      t,
    );
    if (referenceOrderError) {
      throw new Error(
        `${referenceOrderError}\n${t("video.chapter.frameImageMustPrepare", { label })}`,
      );
    }
    return buildVideoFrameImageRequest(
      imageType,
      effectiveFramePrompt,
      chapter?.id ?? prompt.chapter_id,
      prompt.aspect_ratio,
      buildVideoFrameImageTraceMetadata(prompt, source, t),
      references.ids,
      references.note,
      t,
    );
  };

  const runBoundaryFrameImageChainTask = async (
    actionKey: string,
    targetPrompt: ChapterVideoPrompt,
    label: string,
    frameLabel: string,
  ) => {
    const requestScope = actionScopeKey;
    if (!chapter) {
      finishAction(actionKey);
      return;
    }
    let chain: ChapterVideoPrompt[];
    try {
      chain = boundaryFramePromptChainTo(
        targetPrompt,
        framePrompts,
        t("video.actions.generate"),
        t,
      );
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.actions.frameChainIncomplete")), "error");
      finishAction(actionKey);
      return;
    }
    const missingPrompts: ChapterVideoPrompt[] = [];
    for (const item of chain) {
      if (item.status === "running") {
        addToast(t("video.actions.frameImageRunningCannot", { label, number: formatNumber(item.shot_number, locale) }), "error");
        finishAction(actionKey);
        return;
      }
      if (!latestVideoFrameImageForPrompt(item, frameImages, t)) {
        missingPrompts.push(item);
      }
      const error = promptLengthError(
        label,
        item.visual_prompt.trim(),
        VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
        t,
      );
      if (error) {
        addToast(error, "error");
        finishAction(actionKey);
        return;
      }
    }
    if (missingPrompts.length === 0) {
      addToast(t("video.actions.sequentialReuseNotice", { number: formatNumber(targetPrompt.shot_number, locale), label: frameLabel }), "info");
      finishAction(actionKey);
      return;
    }
    const ok = await confirmDialog({
      title: t("video.actions.sequentialTitle", { number: formatNumber(targetPrompt.shot_number, locale), label: frameLabel }),
      message: t("video.actions.sequentialMessage", { number: formatNumber(targetPrompt.shot_number, locale), label: frameLabel, missing: formatNumber(missingPrompts.length, locale) }),
      confirmText: t("video.actions.sequentialConfirm"),
    });
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    if (!isCurrentActionScope(requestScope)) {
      finishAction(actionKey);
      return;
    }
    try {
      const generatedIds: number[] = [];
      let refreshSucceeded = true;
      const chainImages = [...frameImages];
      let activeHandle: GenerationTaskHandle | null = null;
      const firstMissing = missingPrompts[0];
      if (!firstMissing) throw new Error(t("video.actions.missingFramePrompt"));
      const firstRequest = buildFrameImageGenerationRequest(
        firstMissing,
        chainImages,
        chain,
      );
      const firstHandle = await startImageGeneration({
        project_id: projectId,
        operation: { kind: "batch", requests: [firstRequest] },
      });
      activeHandle = firstHandle;
      await runProgressTask(
        {
          title: t("video.actions.sequentialTitle", { number: formatNumber(targetPrompt.shot_number, locale), label: frameLabel }),
          startMessage: t("video.actions.preparing", { operation: t("video.actions.sequentialTitle", { number: formatNumber(targetPrompt.shot_number, locale), label: frameLabel }) }),
          successMessage: t("video.actions.completed", { operation: t("video.actions.sequentialTitle", { number: formatNumber(targetPrompt.shot_number, locale), label: frameLabel }) }),
          errorMessage: t("video.actions.failed", { operation: t("video.actions.sequentialTitle", { number: formatNumber(targetPrompt.shot_number, locale), label: frameLabel }) }),
          total: missingPrompts.length,
          cancelTaskId: firstHandle.task_id,
          cancelTask: async () => {
            if (activeHandle) await generationClient.cancel(activeHandle);
          },
          cancelLabel: t("workflow.stop"),
        },
        async ({ setProgress }) => {
          for (const [index, item] of missingPrompts.entries()) {
            if (index > 0) {
              const request = buildFrameImageGenerationRequest(
                item,
                chainImages,
                chain,
              );
              const nextHandle = await startImageGeneration({
                project_id: projectId,
                operation: { kind: "batch", requests: [request] },
              });
              activeHandle = nextHandle;
            }
            const handle = activeHandle;
            if (!handle) throw new Error(t("video.actions.imageTaskNotStarted"));
            const task = await generationClient.waitForTerminal(handle, (current) => {
              if (current?.progress) {
                setProgress(current.progress.message, index, missingPrompts.length);
              }
            });
            const [image] = await verifyImageGenerationResult(task, projectId);
            generatedIds.push(image.id);
            chainImages.push(image);
            setProgress(
              t("video.actions.generatedFrameProgress", { number: formatNumber(item.shot_number, locale), label: frameLabel }),
              index + 1,
              missingPrompts.length,
            );
          }
          if (!isCurrentActionScope(requestScope)) return;
          refreshSucceeded = await loadAll();
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      if (!refreshSucceeded) return;
      addToast(
        t("video.actions.generatedFramesReused", { count: formatNumber(generatedIds.length, locale), label: frameLabel }),
        "success",
      );
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, t("video.actions.failed", { operation: t("video.actions.sequentialTitle", { number: formatNumber(targetPrompt.shot_number, locale), label: frameLabel }) }));
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const handleGenerateFrameImages = async (prompt: ChapterVideoPrompt) => {
    if (!chapter) return;
    let frameContract: ReturnType<typeof videoFrameImageContractForPromptType>;
    try {
      frameContract = videoFrameImageContractForPromptType(prompt.prompt_type, t);
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.actions.frameImageGenerationFailed")), "error");
      return;
    }
    const { promptLabel: label, frameLabel } = frameContract;
    const actionKey = `generate-frame-images-${prompt.id}`;
    if (isBoundaryFramePrompt(prompt)) {
      if (!beginAction(actionKey)) return;
      await runBoundaryFrameImageChainTask(actionKey, prompt, label, frameLabel);
      return;
    }
    if (prompt.status === "running") {
      addToast(t("video.actions.frameImageRunningCannot", { label, number: formatNumber(prompt.shot_number, locale) }), "error");
      return;
    }
    let request: Record<string, unknown>;
    try {
      request = buildFrameImageGenerationRequest(prompt);
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.actions.failed", { operation: t("video.actions.generateFrameImageTitle", { label: frameLabel }) })), "error");
      return;
    }
    const requestScope = actionScopeKey;
    if (!beginAction(actionKey)) return;
    const ok = await confirmDialog({
      title: t("video.actions.generateFrameImageTitle", { label: frameLabel }),
      message: t("video.actions.generateFrameImageMessage", { promptLabel: label, number: formatNumber(prompt.shot_number, locale), label: frameLabel }),
      confirmText: t("video.actions.generateFrameImageConfirm"),
    });
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    if (!isCurrentActionScope(requestScope)) {
      finishAction(actionKey);
      return;
    }
    await runImageAssetTask(
      actionKey,
      t("video.actions.generateFrameImageTitle", { label: frameLabel }),
      frameLabel,
      [request],
    );
  };

  const handleImportFrameImage = async (prompt: ChapterVideoPrompt) => {
    await importFrameImage(prompt);
  };

  return {
    handleGenerateSegmentPlan,
    handleGenerateFramePrompts,
    handleGenerateVideoPrompts,
    handleGenerateFrameImages,
    handleImportFrameImage,
  };
}
