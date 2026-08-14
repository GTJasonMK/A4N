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
  type ChapterVideoPrompt,
  type GeneratePromptOptions,
  type VideoSegmentExecutionRecipe,
} from "../../../api/video";
import {
  isSupportedImageMime,
  MAX_UPLOAD_IMAGE_BYTES,
  readFileAsBase64,
  supportedImageAccept,
} from "../../../utils/imageUpload";
import { pickLocalFile } from "../../../utils/fileUpload";
import type { RunProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import type { ChapterRow } from "../shared";
import {
  type ChapterVideoWorkflowMode,
  buildVideoFrameImageTraceMetadata,
  buildVideoFrameImageRequest,
  boundaryFramePromptChainTo,
  completedSegmentPlanPreviewItems,
  durationOptionsAllowed,
  frameImageReferencesForPrompt,
  isBoundaryFramePrompt,
  isSegmentPlanDraftCheckpointKey,
  isDurationAllowed,
  latestVideoFrameImageForPrompt,
  normalizeDurationOptions,
  promptHasVideoWorkflowContract,
  promptLengthError,
  promptOutputLanguageOfPrompt,
  promptSourcePromptType,
  readSavedReferenceImageIds,
  referenceLabelOrderError,
  shiftReferenceLabels,
  videoExecutionIdentityOfPrompt,
  videoExecutionSegmentNumberInPlan,
  videoExecutionSegmentNumberInPlanByLineage,
  videoWorkflowModeOfPrompt,
  videoExecutionIdentityMatches,
  videoFrameImageMetadata,
  videoImageTypeForFrameRole,
  type VideoFrameImageRole,
  type VideoDurationLimits,
  type VideoGenOptions,
  CHAT_VIDEO_DURATION_LIMITS,
  VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
  VIDEO_FIRST_FRAME_PROMPT_TYPE,
  VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION,
  VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
  VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
  VIDEO_FRAME_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION,
} from "./videoTabUtils";
import type { LoadedSegmentPlanDraft } from "./loadVideoTabData";

type AddToast = (message: unknown, type: ToastType) => void;
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
      throw new Error(
        `${extractErrorMessage(error, "生成失败")}；刷新已提交提示词失败：${extractErrorMessage(refreshError, "未知错误")}`,
      );
    }
    throw error;
  }
};

interface UseVideoPromptGenerationActionsParams {
  projectId: number;
  chapter: ChapterRow | null;
  options: VideoGenOptions;
  segmentPlanOptions: VideoGenOptions;
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
  addToast: AddToast;
  runProgressTask: RunProgressTask;
  loadAll: () => Promise<void>;
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
    workflowMode: ChapterVideoWorkflowMode | "shared_segment_plan";
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
          storageError: null,
        }
      : baseOptions;
    const operationDurationLimits = recipe?.workflow_mode === "chat_video"
      ? CHAT_VIDEO_DURATION_LIMITS
      : durationLimits;
    const style = effectiveOptions.style.trim();
    if (!style) {
      addToast(`请填写${operation}风格`, "error");
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
      addToast(`当前${operation}时长不在默认视频模型支持范围内`, "error");
      return null;
    }
    if (!durationOptionsAllowed(
      effectiveOptions.durationSecs,
      effectiveOptions.durationOptions,
      operationDurationLimits,
    )) {
      addToast(`当前${operation}候选时长包含默认视频模型不支持的秒数`, "error");
      return null;
    }
    return {
      ...effectiveOptions,
      style,
      workflowMode: recipe?.workflow_mode ?? "shared_segment_plan",
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
      addToast("请至少提供一个帧生成请求", "error");
      finishAction(key);
      return;
    }
    try {
      let generatedIds: number[] = [];
      const handle = await startImageGeneration({
        project_id: projectId,
        operation: {
          kind: "batch",
          requests: [requests[0], ...requests.slice(1)],
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title,
          startMessage: `正在准备${title}`,
          successMessage: `${title}已完成`,
          errorMessage: `${title}失败`,
          total: Math.max(1, requests.length),
          cancelLabel: "停止生成",
        },
        async (task) => {
          const rows = await verifyImageGenerationResult(task, projectId);
          generatedIds = rows.map((row, index) => {
            if (!Number.isInteger(row.id) || row.id <= 0) {
              throw new Error(`图片生成结果第 ${index + 1} 项缺少有效 ID`);
            }
            return row.id;
          });
          if (!isCurrentActionScope(requestScope)) return;
          await loadAll();
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(`已生成 ${generatedIds.length} 张${resultImageLabel}`, "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, `${title}失败`);
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      finishAction(key);
    }
  };

  const importFrameImage = async (
    prompt: ChapterVideoPrompt,
    role: VideoFrameImageRole,
    frameLabel: string,
    source:
      | "video_boundary_frame_prompt"
      | "video_first_frame_prompt"
      | "video_storyboard_prompt",
  ) => {
    if (!chapter) return;
    if (prompt.status === "running") {
      addToast(`${frameLabel}提示词 #${prompt.shot_number} 正在运行，不能导入图片`, "error");
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
      addToast(extractErrorMessage(err, "打开文件选择器失败"), "error");
      return;
    } finally {
      if (!file) finishAction(actionKey);
    }
    if (!file) return;
    if (!isSupportedImageMime(file.type)) {
      addToast("只支持 PNG/JPEG/WebP 图片", "error");
      finishAction(actionKey);
      return;
    }
    if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
      addToast(`上传图片不能超过 ${MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024}MB`, "error");
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
      `${frameLabel} #${prompt.shot_number} 手动导入图片`;
    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentActionScope(requestScope)) return;
      await uploadImage(projectId, {
        file_name: file.name,
        mime_type: file.type,
        data_base64: dataBase64,
        image_type: videoImageTypeForFrameRole(role),
        prompt: imagePrompt,
        chapter_id: chapter.id,
        metadata_json: videoFrameImageMetadata(
          role,
          buildVideoFrameImageTraceMetadata(prompt, source, {
            projectId,
            chapter,
            currentContentHash,
          }),
        ),
      });
      if (!isCurrentActionScope(requestScope)) return;
      addToast(`已导入${frameLabel}图片`, "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, `导入${frameLabel}图片失败`), "error");
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
    duration_secs_options: normalizeDurationOptions(
      promptOptions.durationSecs,
      promptOptions.durationOptions,
    ),
    aspect_ratio: promptOptions.aspectRatio,
    reference_image_ids: promptOptions.referenceIds,
    video_library_image_ids: promptOptions.videoLibraryImageIds,
    regenerate_existing: requestOptions.regenerateExisting ?? false,
  });

  const framePromptTypeForWorkflow = (mode: ChapterVideoWorkflowMode): string | null => {
    if (mode === "boundary_frames") return VIDEO_FRAME_PROMPT_TYPE;
    if (mode === "first_frame_chain") return VIDEO_FIRST_FRAME_PROMPT_TYPE;
    if (mode === "storyboard") return VIDEO_STORYBOARD_PROMPT_TYPE;
    return null;
  };

  const framePromptContractForWorkflow = (mode: ChapterVideoWorkflowMode): string | null => {
    if (mode === "boundary_frames") return VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION;
    if (mode === "first_frame_chain") return VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION;
    if (mode === "storyboard") return VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION;
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
      "generation_asset_fingerprint.reference_image_ids",
    );
    const savedVideoLibraryIds = readSavedReferenceImageIds(
      fingerprint.video_library_image_ids,
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
      "source.duration_secs_options",
    );
    if (savedDurationOptions.error) return false;
    return idsEqual(
      savedDurationOptions.ids,
      normalizeDurationOptions(promptOptions.durationSecs, promptOptions.durationOptions),
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
      targetWorkflowMode !== "boundary_frames" &&
      targetWorkflowMode !== "first_frame_chain" &&
      targetWorkflowMode !== "storyboard"
    ) {
      addToast("当前模式不能使用帧提示词链路", "error");
      return;
    }
    const segmentNumber = recipe.segment_number;
    const firstFrameMode = targetWorkflowMode === "first_frame_chain";
    const label =
      targetWorkflowMode === "storyboard"
          ? "分镜提示词"
        : firstFrameMode
          ? "首帧提示词"
          : "边界帧提示词";
    if (firstFrameMode && recipe.guidance_mode === "continuity") {
      addToast(
        "当前片段使用尾帧续接，无需生成首帧提示词；如需自己生成，请先把首帧来源切换为独立生成",
        "info",
      );
      return;
    }
    const regenerateExisting = requestOptions.regenerateExisting ?? false;
    const operationLabel = `${regenerateExisting ? "重新生成" : "生成"}${label}`;
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
        `${label} #${runningFramePrompt.shot_number} 正在运行，不能${operationLabel}`,
        "error",
      );
      return;
    }
    const promptOptions = validatedPromptOptions(label, recipe);
    if (!promptOptions) return;
    const expectedFrameCount =
      targetWorkflowMode === "boundary_frames"
        ? 2
        : targetWorkflowMode === "storyboard"
          ? 1
          : 1;
    const framePromptType = framePromptTypeForWorkflow(targetWorkflowMode);
    const framePromptContract = framePromptContractForWorkflow(targetWorkflowMode);
    if (!segmentPlanPrompt) {
      addToast("请先生成片段规划，再生成帧提示词", "error");
      return;
    }
    if (!framePromptType || !framePromptContract) {
      addToast("当前模式缺少帧提示词类型或合同版本，不能生成帧提示词", "error");
      return;
    }
    const targetFrameNumbers = targetWorkflowMode === "boundary_frames"
      ? [1, 2]
      : targetWorkflowMode === "first_frame_chain"
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
        `当前参数下${label}已全部生成，如需更新请使用重新生成。`,
        "info",
      );
      return;
    }
    const requestScope = actionScopeKey;
    const actionKey = "generate-frame-prompts";
    if (!beginAction(actionKey)) return;
    if (regenerateExisting && targetFramePrompts.length > 0) {
      const ok = await confirmDialog({
        title: `重新生成${label}`,
        message: `将重新生成本章已有${label}，并覆盖尚未成图的前置提示词。已生成图片的记录不能覆盖，请先删除对应图片。`,
        confirmText: "重新生成",
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
          startMessage: `正在准备${operationLabel}`,
          successMessage: `已完成${operationLabel}`,
          errorMessage: `${operationLabel}失败`,
          total: 100,
        },
        refreshPrompts,
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(`已完成${operationLabel}`, "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, `${operationLabel}失败`);
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const handleGenerateSegmentPlan = async (
    requestOptions: PromptGenerationRequestOptions = {},
  ) => {
    if (!chapter) return;
    const label = "视频片段规划";
    const regenerateExisting = requestOptions.regenerateExisting ?? false;
    const resumedDraft = segmentPlanDraft;
    const effectiveRegenerateExisting =
      resumedDraft?.resumeOptions.regenerate_existing ?? regenerateExisting;
    let generationOptions: GeneratePromptOptions | null = resumedDraft?.resumeOptions ?? null;
    if (!generationOptions) {
      const validatedOptions = validatedPromptOptions(label, undefined, segmentPlanOptions);
      if (!validatedOptions) return;
      generationOptions = {
        ...promptGenerationOptions({
          ...validatedOptions,
          referenceIds: [],
          videoLibraryImageIds: [],
        }, requestOptions),
        workflow_mode: "shared_segment_plan",
      };
    }
    const requestScope = actionScopeKey;
    const actionKey = "generate-segment-plan";
    if (!beginAction(actionKey)) return;
    if (effectiveRegenerateExisting && segmentPlanPrompt && !resumedDraft) {
      const ok = await confirmDialog({
        title: `重新生成${label}`,
        message: `将重新生成当前章节的${label}。如果后续已经派生首尾帧提示词、图片或视频，系统会阻止覆盖，请先处理下游产物。`,
        confirmText: "重新生成",
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
      ? `已加载持久化进度 ${Math.round(persistedProgress.percent)}%${persistedProgress.message.trim() ? ` · ${persistedProgress.message.trim()}` : ""}，正在继续生成`
      : resumedDraft && resumedDraft.completedShotCount > 0
        ? `已加载 ${resumedDraft.completedShotCount}/${resumedDraft.totalShotCount} 个已保存镜头，正在核对并继续生成`
        : `正在准备${effectiveRegenerateExisting ? "重新生成" : "生成"}${label}`;
    let previewFingerprint = "";
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
          title: `${effectiveRegenerateExisting ? "重新生成" : "生成"}${label}`,
          startMessage: resumeStartMessage,
          successMessage: `已${effectiveRegenerateExisting ? "重新生成" : "生成"}${label}`,
          errorMessage: `${effectiveRegenerateExisting ? "重新生成" : "生成"}${label}失败`,
          total: 100,
        },
        refreshPrompts,
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
      addToast(`已${effectiveRegenerateExisting ? "重新生成" : "生成"}${label}`, "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, `${effectiveRegenerateExisting ? "重新生成" : "生成"}${label}失败`);
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentActionScope(requestScope)) await loadAll();
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
      targetWorkflowMode !== "boundary_frames" &&
      targetWorkflowMode !== "first_frame_chain" &&
      targetWorkflowMode !== "storyboard" &&
      targetWorkflowMode !== "chat_video"
    ) {
      addToast("当前模式的视频提示词链路尚未接入", "error");
      return;
    }
    const segmentNumber = recipe.segment_number;
    const chatVideoMode = targetWorkflowMode === "chat_video";
    const promptLabel = chatVideoMode
      ? "Chat 视频提示词"
      : targetWorkflowMode === "storyboard"
        ? "分镜视频提示词"
        : "视频提示词";
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
          ? "首帧提示词"
          : runningFramePrompt.prompt_type === VIDEO_STORYBOARD_PROMPT_TYPE
            ? "分镜提示词"
          : "边界帧提示词";
      addToast(`${label} #${runningFramePrompt.shot_number} 正在运行，不能生成视频提示词`, "error");
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
      addToast(`视频提示词 #${runningPrompt.id} 正在运行，不能继续生成视频提示词`, "error");
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
      addToast("请先生成片段规划，才能生成视频提示词", "error");
      return;
    }
    if (regenerateExisting && generatedVideoPromptCount > 0) {
      addToast(
        `已有 ${generatedVideoPromptCount} 条视频提示词生成了视频，不能重新生成；请先删除对应视频`,
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
        title: `重新生成${promptLabel}`,
        message: `将重新生成本章已有${promptLabel}，覆盖尚未生成视频的提示词。已生成视频的提示词不能覆盖，请先删除对应视频。`,
        confirmText: "重新生成",
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
          title: `${regenerateExisting ? "重新生成" : "生成"}${promptLabel}`,
          startMessage: `正在准备${regenerateExisting ? "重新生成" : "生成"}${promptLabel}`,
          successMessage: `已${regenerateExisting ? "重新生成" : "生成"}${promptLabel}`,
          errorMessage: `${regenerateExisting ? "重新生成" : "生成"}${promptLabel}失败`,
          total: 100,
        },
        refreshPrompts,
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(`已${regenerateExisting ? "重新生成" : "生成"}${promptLabel}`, "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, `${regenerateExisting ? "重新生成" : "生成"}${promptLabel}失败`);
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const buildFrameImageGenerationRequest = (
    prompt: ChapterVideoPrompt,
    label: string,
    role: VideoFrameImageRole,
    source:
      | "video_boundary_frame_prompt"
      | "video_first_frame_prompt"
      | "video_storyboard_prompt",
    images: GeneratedImage[] = frameImages,
    prompts: ChapterVideoPrompt[] = framePrompts,
  ): Record<string, unknown> => {
    const framePrompt = prompt.visual_prompt.trim();
    const error = promptLengthError(
      label,
      framePrompt,
      VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
    );
    if (error) throw new Error(error);
    const references = frameImageReferencesForPrompt(prompt, images, prompts);
    if (references.error) {
      throw new Error(`${references.error}；请重新生成或手动修正${label}后再生成图片`);
    }
    const effectiveFramePrompt = shiftReferenceLabels(
      framePrompt,
      references.promptReferenceOffset,
    );
    const referenceOrderError = referenceLabelOrderError(
      effectiveFramePrompt,
      references.ids.length,
    );
    if (referenceOrderError) {
      throw new Error(`${referenceOrderError}；请重新生成或手动修正${label}后再生成图片`);
    }
    return buildVideoFrameImageRequest(
      role,
      effectiveFramePrompt,
      chapter?.id ?? prompt.chapter_id,
      buildVideoFrameImageTraceMetadata(prompt, source, {
        projectId,
        chapter,
        currentContentHash,
      }),
      references.ids,
      references.note,
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
      chain = boundaryFramePromptChainTo(targetPrompt, framePrompts, "生成");
    } catch (err) {
      addToast(extractErrorMessage(err, "边界帧提示词链不完整"), "error");
      finishAction(actionKey);
      return;
    }
    const missingPrompts: ChapterVideoPrompt[] = [];
    for (const item of chain) {
      if (item.status === "running") {
        addToast(`${label} #${item.shot_number} 正在运行，不能成图`, "error");
        finishAction(actionKey);
        return;
      }
      if (!latestVideoFrameImageForPrompt(item, frameImages)) {
        missingPrompts.push(item);
      }
      const error = promptLengthError(
        label,
        item.visual_prompt.trim(),
        VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
      );
      if (error) {
        addToast(error, "error");
        finishAction(actionKey);
        return;
      }
    }
    if (missingPrompts.length === 0) {
      addToast(`第 1-${targetPrompt.shot_number} 张${frameLabel}图片已存在，将直接复用`, "info");
      finishAction(actionKey);
      return;
    }
    const ok = await confirmDialog({
      title: `生成到第 ${targetPrompt.shot_number} 张${frameLabel}`,
      message: `将按顺序检查第 1-${targetPrompt.shot_number} 张${frameLabel}图片：已有图片直接复用，缺失的 ${missingPrompts.length} 张会从前到后依次生成；第 2 张开始都会使用上一张实际成图作为参考图 1。`,
      confirmText: "顺序生成",
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
      const chainImages = [...frameImages];
      let activeHandle: GenerationTaskHandle | null = null;
      const firstMissing = missingPrompts[0];
      if (!firstMissing) throw new Error("缺少可生成的边界帧提示词");
      const firstRequest = buildFrameImageGenerationRequest(
        firstMissing,
        label,
        "key_frame",
        "video_boundary_frame_prompt",
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
          title: `生成到第 ${targetPrompt.shot_number} 张${frameLabel}`,
          startMessage: `正在准备顺序生成${frameLabel}`,
          successMessage: `${frameLabel}顺序生成已完成`,
          errorMessage: `${frameLabel}顺序生成失败`,
          total: missingPrompts.length,
          cancelTaskId: firstHandle.task_id,
          cancelTask: async () => {
            if (activeHandle) await generationClient.cancel(activeHandle);
          },
          cancelLabel: "停止生成",
        },
        async ({ setProgress }) => {
          for (const [index, item] of missingPrompts.entries()) {
            if (index > 0) {
              const request = buildFrameImageGenerationRequest(
                item,
                label,
                "key_frame",
                "video_boundary_frame_prompt",
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
            if (!handle) throw new Error("图片生成任务未启动");
            const task = await generationClient.waitForTerminal(handle, (current) => {
              if (current?.progress) {
                setProgress(current.progress.message, index, missingPrompts.length);
              }
            });
            const rows = await verifyImageGenerationResult(task, projectId);
            const image = rows[0];
            if (!image || !Number.isInteger(image.id) || image.id <= 0) {
              throw new Error(`第 ${item.shot_number} 张${frameLabel}生成结果缺少有效图片 ID`);
            }
            generatedIds.push(image.id);
            chainImages.push(image);
            setProgress(
              `已生成第 ${item.shot_number} 张${frameLabel}`,
              index + 1,
              missingPrompts.length,
            );
          }
          if (!isCurrentActionScope(requestScope)) return;
          await loadAll();
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(
        `已生成 ${generatedIds.length} 张${frameLabel}，其余上游图片已复用`,
        "success",
      );
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, `${frameLabel}顺序生成失败`);
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const handleGenerateFrameImages = async (prompt: ChapterVideoPrompt) => {
    if (!chapter) return;
    const firstFramePrompt = prompt.prompt_type === VIDEO_FIRST_FRAME_PROMPT_TYPE;
    const storyboardPrompt = prompt.prompt_type === VIDEO_STORYBOARD_PROMPT_TYPE;
    const label = firstFramePrompt
      ? "首帧提示词"
      : storyboardPrompt
        ? "分镜提示词"
        : "边界帧提示词";
    const frameLabel = firstFramePrompt
      ? "首帧"
      : storyboardPrompt
        ? "分镜板"
        : "边界帧";
    const actionKey = `generate-frame-images-${prompt.id}`;
    if (isBoundaryFramePrompt(prompt)) {
      if (!beginAction(actionKey)) return;
      await runBoundaryFrameImageChainTask(actionKey, prompt, label, frameLabel);
      return;
    }
    if (prompt.status === "running") {
      addToast(`${label} #${prompt.shot_number} 正在运行，不能成图`, "error");
      return;
    }
    let request: Record<string, unknown>;
    try {
      request = buildFrameImageGenerationRequest(
        prompt,
        label,
        firstFramePrompt
          ? "start_frame"
          : storyboardPrompt
            ? "storyboard_frame"
            : "key_frame",
        firstFramePrompt
          ? "video_first_frame_prompt"
          : storyboardPrompt
            ? "video_storyboard_prompt"
            : "video_boundary_frame_prompt",
      );
    } catch (err) {
      addToast(extractErrorMessage(err, `生成${frameLabel}图片失败`), "error");
      return;
    }
    const requestScope = actionScopeKey;
    if (!beginAction(actionKey)) return;
    const ok = await confirmDialog({
      title: `生成${frameLabel}图片`,
      message: `将调用图片模型，为${label} #${prompt.shot_number} 生成 1 张${frameLabel}图片。生成后会作为后续视频生成的${frameLabel}输入；如需重生成，需要先删除对应图片。`,
      confirmText: "生成图片",
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
      firstFramePrompt
        ? "生成首帧"
        : storyboardPrompt
          ? "生成分镜板"
          : "生成边界帧",
      frameLabel,
      [request],
    );
  };

  const handleImportFrameImage = async (prompt: ChapterVideoPrompt) => {
    const firstFramePrompt = prompt.prompt_type === VIDEO_FIRST_FRAME_PROMPT_TYPE;
    const storyboardPrompt = prompt.prompt_type === VIDEO_STORYBOARD_PROMPT_TYPE;
    await importFrameImage(
      prompt,
      firstFramePrompt
        ? "start_frame"
        : storyboardPrompt
          ? "storyboard_frame"
          : "key_frame",
      firstFramePrompt
        ? "首帧"
        : storyboardPrompt
          ? "分镜板"
          : "边界帧",
      firstFramePrompt
        ? "video_first_frame_prompt"
        : storyboardPrompt
          ? "video_storyboard_prompt"
          : "video_boundary_frame_prompt",
    );
  };

  return {
    handleGenerateSegmentPlan,
    handleGenerateFramePrompts,
    handleGenerateVideoPrompts,
    handleGenerateFrameImages,
    handleImportFrameImage,
  };
}
