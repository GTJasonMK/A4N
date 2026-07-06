import { useEffect, useRef, type Dispatch, type SetStateAction } from "react";
import type { ToastType } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import { batchGenerateImages, uploadImage, type GeneratedImage } from "../../../api/image";
import {
  generateChapterVideoFramePrompts,
  generateChapterVideoPrompts,
  generateChapterVideoSegmentPlan,
  generateChapterVideoShotAnalysis,
  type ChapterVideoPrompt,
  type VideoWorkflowMode,
} from "../../../api/video";
import {
  isSupportedImageMime,
  MAX_UPLOAD_IMAGE_BYTES,
  readFileAsBase64,
  supportedImageAccept,
} from "../../../utils/imageUpload";
import { pickLocalFile } from "../../../utils/fileUpload";
import {
  createProgressTaskId,
  type RunProgressTask,
} from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import type { ChapterRow } from "../shared";
import { chapterGeneratingMessage } from "../shared";
import {
  buildVideoFrameImageTraceMetadata,
  buildVideoFrameImageRequest,
  boundaryFramePromptChainTo,
  durationOptionsAllowed,
  expectedFrameCountFromSegmentPlanPrompt,
  expectedVideoPromptCountFromSegmentPlanPrompt,
  frameImageReferencesForPrompt,
  isBoundaryFramePrompt,
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
  videoWorkflowModeOfPrompt,
  videoFrameImageMetadata,
  videoImageTypeForFrameRole,
  type VideoFrameImageRole,
  type VideoDurationLimits,
  type VideoGenOptions,
  MAX_VIDEO_PROMPT_BATCH_SIZE,
  VIDEO_CHAT_FRAME_PROMPT_TYPE,
  VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
  VIDEO_FIRST_FRAME_PROMPT_TYPE,
  VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION,
  VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
  VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
  VIDEO_FRAME_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION,
} from "./videoTabUtils";

type AddToast = (message: unknown, type: ToastType) => void;
type PromptGenerationRequestOptions = {
  regenerateExisting?: boolean;
  runUntilComplete?: boolean;
  promptBatchSize?: number;
};

interface UseVideoPromptGenerationActionsParams {
  projectId: number;
  chapter: ChapterRow | null;
  chapterGenerating: boolean;
  workflowMode: VideoWorkflowMode | null;
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  promptInputSelectionError: string | null;
  referenceIds: number[];
  videoLibraryImageIds: number[];
  frameImages: GeneratedImage[];
  shotAnalysisPrompt: ChapterVideoPrompt | null;
  segmentPlanPrompt: ChapterVideoPrompt | null;
  framePrompts: ChapterVideoPrompt[];
  finalVideoPrompts: ChapterVideoPrompt[];
  allFramePrompts: ChapterVideoPrompt[];
  allFinalVideoPrompts: ChapterVideoPrompt[];
  currentContentHash: string | null;
  actionScopeKey: string | null;
  addToast: AddToast;
  runProgressTask: RunProgressTask;
  loadAll: () => Promise<void>;
  setActing: Dispatch<SetStateAction<string | null>>;
  setVideoTaskId: Dispatch<SetStateAction<string | null>>;
}

export function useVideoPromptGenerationActions({
  projectId,
  chapter,
  chapterGenerating,
  workflowMode,
  options,
  durationLimits,
  promptInputSelectionError,
  referenceIds,
  videoLibraryImageIds,
  frameImages,
  shotAnalysisPrompt,
  segmentPlanPrompt,
  framePrompts,
  finalVideoPrompts,
  allFramePrompts,
  allFinalVideoPrompts,
  currentContentHash,
  actionScopeKey,
  addToast,
  runProgressTask,
  loadAll,
  setActing,
  setVideoTaskId,
}: UseVideoPromptGenerationActionsParams) {
  type ValidatedPromptOptions = VideoGenOptions & { workflowMode: VideoWorkflowMode };
  const actionScopeRef = useRef(actionScopeKey);
  const actionBusyRef = useRef<string | null>(null);

  useEffect(() => {
    actionScopeRef.current = actionScopeKey;
    actionBusyRef.current = null;
    setActing(null);
    setVideoTaskId(null);
  }, [actionScopeKey, setActing, setVideoTaskId]);

  const isCurrentActionScope = (scope: string | null) => actionScopeRef.current === scope;
  const beginAction = (key: string): boolean => {
    if (actionBusyRef.current) return false;
    actionBusyRef.current = key;
    setActing(key);
    return true;
  };
  const finishAction = (key: string, scope: string | null) => {
    if (!isCurrentActionScope(scope)) return;
    if (actionBusyRef.current === key) {
      actionBusyRef.current = null;
      setActing(null);
    }
  };

  const validatedPromptOptions = (operation: string): ValidatedPromptOptions | null => {
    if (!workflowMode) {
      addToast("请先选择视频生成模式", "error");
      return null;
    }
    const style = options.style.trim();
    if (!style) {
      addToast(`请填写${operation}风格`, "error");
      return null;
    }
    if (durationLimits.configError) {
      addToast(durationLimits.configError, "error");
      return null;
    }
    if (promptInputSelectionError) {
      addToast(promptInputSelectionError, "error");
      return null;
    }
    if (
      !Number.isInteger(options.promptBatchSize) ||
      options.promptBatchSize < 1 ||
      options.promptBatchSize > MAX_VIDEO_PROMPT_BATCH_SIZE
    ) {
      addToast("本次生成数量必须是正整数", "error");
      return null;
    }
    if (!isDurationAllowed(options.durationSecs, durationLimits)) {
      addToast(`当前${operation}时长不在默认视频模型支持范围内`, "error");
      return null;
    }
    if (!durationOptionsAllowed(options.durationSecs, options.durationOptions, durationLimits)) {
      addToast(`当前${operation}候选时长包含默认视频模型不支持的秒数`, "error");
      return null;
    }
    return { ...options, style, workflowMode };
  };

  const runImageAssetTask = async (
    key: string,
    title: string,
    resultImageLabel: string,
    requests: Array<Record<string, unknown>>,
  ) => {
    const requestScope = actionScopeKey;
    if (!chapter) {
      finishAction(key, requestScope);
      return;
    }
    if (chapterGenerating) {
      addToast(chapterGeneratingMessage(chapter, title), "error");
      finishAction(key, requestScope);
      return;
    }
    if (requests.length === 0) {
      addToast("请至少提供一个帧生成请求", "error");
      finishAction(key, requestScope);
      return;
    }
    const taskId = createProgressTaskId("video-assets", projectId);
    setVideoTaskId(taskId);
    try {
      let generatedIds: number[] = [];
      await runProgressTask(
        {
          title,
          startMessage: `正在准备${title}`,
          successMessage: `${title}已完成`,
          errorMessage: `${title}失败`,
          total: Math.max(1, requests.length),
          taskId,
        },
        async () => {
          const rows = await batchGenerateImages(projectId, requests, taskId);
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
      if (isCurrentActionScope(requestScope)) {
        setVideoTaskId(null);
      }
      finishAction(key, requestScope);
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
    if (chapterGenerating) {
      addToast(chapterGeneratingMessage(chapter, `导入${frameLabel}图片`), "error");
      return;
    }
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
      if (!file) finishAction(actionKey, requestScope);
    }
    if (!file) return;
    if (!isSupportedImageMime(file.type)) {
      addToast("只支持 PNG/JPEG/WebP 图片", "error");
      finishAction(actionKey, requestScope);
      return;
    }
    if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
      addToast(`上传图片不能超过 ${MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024}MB`, "error");
      finishAction(actionKey, requestScope);
      return;
    }
    if (!isCurrentActionScope(requestScope)) {
      finishAction(actionKey, requestScope);
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
      finishAction(actionKey, requestScope);
    }
  };

  const promptGenerationOptions = (
    promptOptions: ValidatedPromptOptions,
    requestOptions: PromptGenerationRequestOptions = {},
  ) => ({
    workflow_mode: promptOptions.workflowMode,
    style: promptOptions.style,
    prompt_output_language: promptOptions.promptOutputLanguage,
    prompt_batch_size: requestOptions.promptBatchSize ?? promptOptions.promptBatchSize,
    duration_secs: promptOptions.durationSecs,
    duration_secs_options: normalizeDurationOptions(
      promptOptions.durationSecs,
      promptOptions.durationOptions,
    ),
    aspect_ratio: promptOptions.aspectRatio,
    reference_image_ids: referenceIds,
    video_library_image_ids: videoLibraryImageIds,
    regenerate_existing: requestOptions.regenerateExisting ?? false,
  });

  const videoPromptGenerationOptions = (
    promptOptions: ValidatedPromptOptions,
    requestOptions: PromptGenerationRequestOptions = {},
  ) => ({
    workflow_mode: promptOptions.workflowMode,
    style: promptOptions.style,
    prompt_output_language: promptOptions.promptOutputLanguage,
    prompt_batch_size: requestOptions.promptBatchSize ?? promptOptions.promptBatchSize,
    duration_secs: promptOptions.durationSecs,
    duration_secs_options: normalizeDurationOptions(
      promptOptions.durationSecs,
      promptOptions.durationOptions,
    ),
    aspect_ratio: promptOptions.aspectRatio,
    reference_image_ids: referenceIds,
    video_library_image_ids: videoLibraryImageIds,
    regenerate_existing: requestOptions.regenerateExisting ?? false,
  });

  const framePromptTypeForWorkflow = (mode: VideoWorkflowMode): string | null => {
    if (mode === "boundary_frames") return VIDEO_FRAME_PROMPT_TYPE;
    if (mode === "first_frame_chain") return VIDEO_FIRST_FRAME_PROMPT_TYPE;
    if (mode === "storyboard") return VIDEO_STORYBOARD_PROMPT_TYPE;
    if (mode === "chat_video") return VIDEO_CHAT_FRAME_PROMPT_TYPE;
    return null;
  };

  const framePromptContractForWorkflow = (mode: VideoWorkflowMode): string | null => {
    if (mode === "boundary_frames") return VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION;
    if (mode === "first_frame_chain") return VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION;
    if (mode === "storyboard") return VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION;
    if (mode === "chat_video") return VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION;
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
      promptOutputLanguageOfPrompt(prompt) === promptOptions.promptOutputLanguage &&
      (prompt.style ?? "") === promptOptions.style &&
      prompt.aspect_ratio === promptOptions.aspectRatio &&
      promptSourceContextMatches(prompt, promptOptions) &&
      promptGenerationAssetIdsMatch(prompt, referenceIds, videoLibraryImageIds)
    );
  };

  const completedMatchingFramePromptPrefixCount = (
    rows: ChapterVideoPrompt[],
    promptType: string,
    contractVersion: string,
    promptOptions: ValidatedPromptOptions,
    expectedCount: number,
  ): number => {
    const shotNumbers = new Set(
      rows
        .filter((row) =>
          framePromptMatchesCurrentGenerationContext(
            row,
            promptType,
            contractVersion,
            promptOptions,
          ),
        )
        .map((row) => row.shot_number),
    );
    let count = 0;
    for (let shotNumber = 1; shotNumber <= expectedCount; shotNumber += 1) {
      if (!shotNumbers.has(shotNumber)) break;
      count += 1;
    }
    return count;
  };

  const visibleFramePromptPrefixCount = (
    rows: ChapterVideoPrompt[],
    promptType: string,
    contractVersion: string,
    workflowMode: VideoWorkflowMode,
    expectedCount: number,
  ): number => {
    const shotNumbers = new Set(
      rows
        .filter(
          (row) =>
            row.prompt_type === promptType &&
            videoWorkflowModeOfPrompt(row) === workflowMode &&
            promptHasVideoWorkflowContract(row, contractVersion) &&
            promptSourcePromptType(row) === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
        )
        .map((row) => row.shot_number),
    );
    let count = 0;
    for (let shotNumber = 1; shotNumber <= expectedCount; shotNumber += 1) {
      if (!shotNumbers.has(shotNumber)) break;
      count += 1;
    }
    return count;
  };

  const completedFramePromptPrefixCount = (
    rows: ChapterVideoPrompt[],
    promptType: string,
    expectedCount: number,
  ): number => {
    const shotNumbers = new Set(
      rows
        .filter((row) => row.prompt_type === promptType)
        .map((row) => row.shot_number),
    );
    let count = 0;
    for (let shotNumber = 1; shotNumber <= expectedCount; shotNumber += 1) {
      if (!shotNumbers.has(shotNumber)) break;
      count += 1;
    }
    return count;
  };

  const handleGenerateFramePrompts = async (
    requestOptions: PromptGenerationRequestOptions = {},
  ) => {
    if (!chapter) return;
    if (
      workflowMode !== "boundary_frames" &&
      workflowMode !== "first_frame_chain" &&
      workflowMode !== "storyboard" &&
      workflowMode !== "chat_video"
    ) {
      addToast("当前模式不能使用帧提示词链路", "error");
      return;
    }
    const firstFrameMode = workflowMode === "first_frame_chain";
    const label =
      workflowMode === "chat_video"
        ? "Chat 首尾帧提示词"
        : workflowMode === "storyboard"
          ? "分镜提示词"
        : firstFrameMode
          ? "首帧提示词"
          : "边界帧提示词";
    const regenerateExisting = requestOptions.regenerateExisting ?? false;
    const runUntilComplete = requestOptions.runUntilComplete ?? false;
    const operationLabel = runUntilComplete
      ? `一键续跑剩余${label}`
      : `${regenerateExisting ? "重新生成" : "生成"}${label}`;
    if (chapterGenerating) {
      addToast(chapterGeneratingMessage(chapter, operationLabel), "error");
      return;
    }
    const runningFramePrompt = allFramePrompts.find((prompt) => prompt.status === "running");
    if (runningFramePrompt) {
      addToast(
        `${label} #${runningFramePrompt.shot_number} 正在运行，不能${operationLabel}`,
        "error",
      );
      return;
    }
    const promptOptions = validatedPromptOptions(label);
    if (!promptOptions) return;
    const expectedFrameCount = expectedFrameCountFromSegmentPlanPrompt(
      segmentPlanPrompt,
      workflowMode,
    );
    const framePromptType = framePromptTypeForWorkflow(workflowMode);
    const framePromptContract = framePromptContractForWorkflow(workflowMode);
    if (!segmentPlanPrompt || expectedFrameCount == null || expectedFrameCount <= 0) {
      addToast("请先生成片段规划，再生成帧提示词", "error");
      return;
    }
    if (!framePromptType || !framePromptContract) {
      addToast("当前模式缺少帧提示词类型或合同版本，不能生成帧提示词", "error");
      return;
    }
    const completedMatchingCount = completedMatchingFramePromptPrefixCount(
      framePrompts,
      framePromptType,
      framePromptContract,
      promptOptions,
      expectedFrameCount,
    );
    const completedVisibleCount = visibleFramePromptPrefixCount(
      framePrompts,
      framePromptType,
      framePromptContract,
      workflowMode,
      expectedFrameCount,
    );
    if (!regenerateExisting && completedMatchingCount >= expectedFrameCount) {
      addToast(
        `当前参数下${label}已全部生成，如需更新请使用重新生成。`,
        "info",
      );
      return;
    }
    if (runUntilComplete) {
      if (firstFrameMode) {
        addToast("首帧模式只需要 1 条首帧提示词，请直接使用生成/续跑首帧提示词", "error");
        return;
      }
      if (workflowMode === "boundary_frames") {
        addToast("边界帧需要先生成上一帧图片，再续跑下一条边界帧提示词", "error");
        return;
      }
      if (!framePromptType) {
        addToast("请先生成片段规划，再一键续跑剩余帧提示词", "error");
        return;
      }
      const completedCount = completedFramePromptPrefixCount(
        framePrompts,
        framePromptType,
        expectedFrameCount,
      );
      if (completedCount >= expectedFrameCount) {
        addToast(`${label}已经全部生成，无需续跑`, "error");
        return;
      }
    }
    const requestScope = actionScopeKey;
    const actionKey = "generate-frame-prompts";
    if (!beginAction(actionKey)) return;
    if (regenerateExisting && framePrompts.length > 0) {
      const ok = await confirmDialog({
        title: `重新生成${label}`,
        message: `将重新生成本章已有${label}，并覆盖尚未成图的前置提示词。已生成图片的记录不能覆盖，请先删除对应图片。`,
        confirmText: "重新生成",
      });
      if (!ok) {
        finishAction(actionKey, requestScope);
        return;
      }
      if (!isCurrentActionScope(requestScope)) {
        finishAction(actionKey, requestScope);
        return;
      }
    }
    const taskId = createProgressTaskId("video-frame-prompts", projectId);
    setVideoTaskId(taskId);
    try {
      await runProgressTask(
        {
          title: operationLabel,
          startMessage: `正在准备${operationLabel}`,
          successMessage: `已完成${operationLabel}`,
          errorMessage: `${operationLabel}失败`,
          total: 100,
          taskId,
          showStream: false,
        },
        async () => {
          if (runUntilComplete && expectedFrameCount != null && framePromptType) {
            let completedCount = completedFramePromptPrefixCount(
              framePrompts,
              framePromptType,
              expectedFrameCount,
            );
            while (completedCount < expectedFrameCount) {
              const rows = await generateChapterVideoFramePrompts(
                projectId,
                chapter.id,
                promptGenerationOptions(promptOptions, {
                  ...requestOptions,
                  regenerateExisting: false,
                  promptBatchSize: 1,
                }),
                taskId,
              );
              if (!isCurrentActionScope(requestScope)) return;
              const nextCompletedCount = completedFramePromptPrefixCount(
                rows,
                framePromptType,
                expectedFrameCount,
              );
              if (nextCompletedCount <= completedCount) {
                throw new Error(`${label}续跑未产生新的连续帧提示词，请检查片段规划和已有提示词状态`);
              }
              completedCount = nextCompletedCount;
            }
            return;
          }
          const rows = await generateChapterVideoFramePrompts(
            projectId,
            chapter.id,
            promptGenerationOptions(promptOptions, requestOptions),
            taskId,
          );
          const returnedVisibleCount = visibleFramePromptPrefixCount(
            rows,
            framePromptType,
            framePromptContract,
            workflowMode,
            expectedFrameCount,
          );
          if (returnedVisibleCount <= 0) {
            throw new Error(
              `${label}生成完成但未返回当前工作流下的可见记录，请刷新后检查当前工作流模式或删除旧记录后重新生成`,
            );
          }
          if (!regenerateExisting && returnedVisibleCount <= completedVisibleCount) {
            throw new Error(`${label}生成完成但未产生新的连续帧提示词，请检查片段规划和已有提示词状态`);
          }
          if (firstFrameMode && returnedVisibleCount < expectedFrameCount) {
            throw new Error(
              `${label}生成完成但未返回当前工作流下的可见首帧记录，请刷新后检查当前工作流模式或删除旧记录后重新生成`,
            );
          }
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(`已完成${operationLabel}`, "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, `${operationLabel}失败`);
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentActionScope(requestScope)) {
        setVideoTaskId(null);
      }
      finishAction(actionKey, requestScope);
    }
  };

  const handleGenerateSegmentPlan = async (
    requestOptions: PromptGenerationRequestOptions = {},
  ) => {
    if (!chapter) return;
    if (
      workflowMode !== "boundary_frames" &&
      workflowMode !== "first_frame_chain" &&
      workflowMode !== "storyboard" &&
      workflowMode !== "chat_video"
    ) {
      addToast("当前模式不能生成片段规划", "error");
      return;
    }
    const label =
      workflowMode === "chat_video"
        ? "Chat 视频片段规划"
        : workflowMode === "storyboard"
          ? "分镜视频片段规划"
        : workflowMode === "first_frame_chain"
          ? "首帧串联片段规划"
          : "视频片段规划";
    const downstreamLabel =
      workflowMode === "chat_video"
        ? "Chat 首尾帧提示词"
        : workflowMode === "storyboard"
          ? "分镜提示词"
        : workflowMode === "first_frame_chain"
          ? "首帧提示词"
          : "边界帧提示词";
    const regenerateExisting = requestOptions.regenerateExisting ?? false;
    if (chapterGenerating) {
      addToast(chapterGeneratingMessage(chapter, `${regenerateExisting ? "重新生成" : "生成"}${label}`), "error");
      return;
    }
    const runningFramePrompt = allFramePrompts.find((prompt) => prompt.status === "running");
    if (runningFramePrompt) {
      addToast(
        `${downstreamLabel} #${runningFramePrompt.shot_number} 正在运行，不能${regenerateExisting ? "重新生成" : "生成"}片段规划`,
        "error",
      );
      return;
    }
    if (allFramePrompts.length > 0) {
      addToast(`已存在后续${downstreamLabel}，请先在片段 tab 顶部处理后再重新生成${label}`, "error");
      return;
    }
    if (allFinalVideoPrompts.length > 0) {
      addToast("已存在后续视频提示词，请先处理后续视频提示词再重新生成片段规划", "error");
      return;
    }
    const promptOptions = validatedPromptOptions(label);
    if (!promptOptions) return;
    const requestScope = actionScopeKey;
    const actionKey = "generate-segment-plan";
    if (!beginAction(actionKey)) return;
    if (regenerateExisting && segmentPlanPrompt) {
      const ok = await confirmDialog({
        title: `重新生成${label}`,
        message: `将重新生成当前章节的${label}。如果后续已经派生首尾帧提示词、图片或视频，系统会阻止覆盖，请先处理下游产物。`,
        confirmText: "重新生成",
      });
      if (!ok) {
        finishAction(actionKey, requestScope);
        return;
      }
      if (!isCurrentActionScope(requestScope)) {
        finishAction(actionKey, requestScope);
        return;
      }
    }
    const taskId = createProgressTaskId("video-segment-plan", projectId);
    setVideoTaskId(taskId);
    try {
      await runProgressTask(
        {
          title: `${regenerateExisting ? "重新生成" : "生成"}${label}`,
          startMessage: `正在准备${regenerateExisting ? "重新生成" : "生成"}${label}`,
          successMessage: `已${regenerateExisting ? "重新生成" : "生成"}${label}`,
          errorMessage: `${regenerateExisting ? "重新生成" : "生成"}${label}失败`,
          total: 100,
          taskId,
          showStream: false,
        },
        async () => {
          await generateChapterVideoSegmentPlan(
            projectId,
            chapter.id,
            promptGenerationOptions(promptOptions, requestOptions),
            taskId,
          );
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(`已${regenerateExisting ? "重新生成" : "生成"}${label}`, "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, `${regenerateExisting ? "重新生成" : "生成"}${label}失败`);
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentActionScope(requestScope)) {
        setVideoTaskId(null);
      }
      finishAction(actionKey, requestScope);
    }
  };

  const handleGenerateShotAnalysis = async (
    requestOptions: PromptGenerationRequestOptions = {},
  ) => {
    if (!chapter) return;
    if (
      workflowMode !== "boundary_frames" &&
      workflowMode !== "first_frame_chain" &&
      workflowMode !== "storyboard" &&
      workflowMode !== "chat_video"
    ) {
      addToast("当前模式不能生成镜头分析", "error");
      return;
    }
    const regenerateExisting = requestOptions.regenerateExisting ?? false;
    if (chapterGenerating) {
      addToast(chapterGeneratingMessage(chapter, `${regenerateExisting ? "重新生成" : "生成"}镜头分析`), "error");
      return;
    }
    if (segmentPlanPrompt && regenerateExisting) {
      addToast("已存在后续片段规划，不能重新生成镜头分析；请先删除片段规划", "error");
      return;
    }
    const promptOptions = validatedPromptOptions("镜头分析");
    if (!promptOptions) return;
    const requestScope = actionScopeKey;
    const actionKey = "generate-shot-analysis";
    if (!beginAction(actionKey)) return;
    if (regenerateExisting && shotAnalysisPrompt) {
      const ok = await confirmDialog({
        title: "重新生成镜头分析",
        message: "将重新生成当前章节镜头语言分析。若已存在片段规划，请先删除或处理后续片段规划。",
        confirmText: "重新生成",
      });
      if (!ok) {
        finishAction(actionKey, requestScope);
        return;
      }
      if (!isCurrentActionScope(requestScope)) {
        finishAction(actionKey, requestScope);
        return;
      }
    }
    const taskId = createProgressTaskId("video-shot-analysis", projectId);
    setVideoTaskId(taskId);
    try {
      await runProgressTask(
        {
          title: `${regenerateExisting ? "重新生成" : "生成"}镜头分析`,
          startMessage: `正在准备${regenerateExisting ? "重新生成" : "生成"}镜头分析`,
          successMessage: `已${regenerateExisting ? "重新生成" : "生成"}镜头分析`,
          errorMessage: `${regenerateExisting ? "重新生成" : "生成"}镜头分析失败`,
          total: 100,
          taskId,
          showStream: false,
        },
        async () => {
          await generateChapterVideoShotAnalysis(
            projectId,
            chapter.id,
            promptGenerationOptions(promptOptions, requestOptions),
            taskId,
          );
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(`已${regenerateExisting ? "重新生成" : "生成"}镜头分析`, "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, `${regenerateExisting ? "重新生成" : "生成"}镜头分析失败`);
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentActionScope(requestScope)) {
        setVideoTaskId(null);
      }
      finishAction(actionKey, requestScope);
    }
  };

  const handleGenerateVideoPrompts = async (
    requestOptions: PromptGenerationRequestOptions = {},
  ) => {
    if (!chapter) return;
    if (
      workflowMode !== "boundary_frames" &&
      workflowMode !== "first_frame_chain" &&
      workflowMode !== "storyboard" &&
      workflowMode !== "chat_video"
    ) {
      addToast("当前模式的视频提示词链路尚未接入", "error");
      return;
    }
    const chatVideoMode = workflowMode === "chat_video";
    const promptLabel = chatVideoMode
      ? "Chat 视频提示词"
      : workflowMode === "storyboard"
        ? "分镜视频提示词"
        : "视频提示词";
    const regenerateExisting = requestOptions.regenerateExisting ?? false;
    if (chapterGenerating) {
      addToast(chapterGeneratingMessage(chapter, `${regenerateExisting ? "重新生成" : "生成"}${promptLabel}`), "error");
      return;
    }
    const runningFramePrompt = allFramePrompts.find((prompt) => prompt.status === "running");
    if (runningFramePrompt) {
      const label =
        runningFramePrompt.prompt_type === VIDEO_FIRST_FRAME_PROMPT_TYPE
          ? "首帧提示词"
          : runningFramePrompt.prompt_type === VIDEO_STORYBOARD_PROMPT_TYPE
            ? "分镜提示词"
          : runningFramePrompt.prompt_type === VIDEO_CHAT_FRAME_PROMPT_TYPE
            ? "Chat 首尾帧提示词"
            : "边界帧提示词";
      addToast(`${label} #${runningFramePrompt.shot_number} 正在运行，不能生成视频提示词`, "error");
      return;
    }
    const runningPrompt = allFinalVideoPrompts.find((prompt) => prompt.status === "running");
    if (runningPrompt) {
      addToast(`视频提示词 #${runningPrompt.id} 正在运行，不能继续生成视频提示词`, "error");
      return;
    }
    const generatedVideoPromptCount = finalVideoPrompts.filter(
      (prompt) => prompt.generated_video_id != null,
    ).length;
    const expectedVideoPromptCount =
      expectedVideoPromptCountFromSegmentPlanPrompt(segmentPlanPrompt) ?? 0;
    if (expectedVideoPromptCount === 0) {
      addToast("请先生成片段规划，才能生成视频提示词", "error");
      return;
    }
    const remainingVideoPromptCount = Math.max(
      0,
      expectedVideoPromptCount - finalVideoPrompts.length,
    );
    if (!regenerateExisting && remainingVideoPromptCount === 0) {
      addToast(`${promptLabel}已全部生成，无需继续生成`, "error");
      return;
    }
    if (regenerateExisting && generatedVideoPromptCount > 0) {
      addToast(
        `已有 ${generatedVideoPromptCount} 条视频提示词生成了视频，不能重新生成；请先删除对应视频`,
        "error",
      );
      return;
    }
    const promptOptions = validatedPromptOptions(promptLabel);
    if (!promptOptions) return;
    const effectivePromptBatchSize =
      requestOptions.promptBatchSize ?? promptOptions.promptBatchSize;
    if (
      !Number.isInteger(effectivePromptBatchSize) ||
      effectivePromptBatchSize < 1 ||
      effectivePromptBatchSize > MAX_VIDEO_PROMPT_BATCH_SIZE
    ) {
      addToast("本次生成数量必须是正整数", "error");
      return;
    }
    if (!regenerateExisting && effectivePromptBatchSize > remainingVideoPromptCount) {
      addToast(`本次生成数量不能超过当前剩余 ${remainingVideoPromptCount} 条`, "error");
      return;
    }
    const requestScope = actionScopeKey;
    const actionKey = "generate-video-prompts";
    if (!beginAction(actionKey)) return;
    if (regenerateExisting && finalVideoPrompts.length > 0) {
      const ok = await confirmDialog({
        title: `重新生成${promptLabel}`,
        message: `将重新生成本章已有${promptLabel}，覆盖尚未生成视频的提示词。已生成视频的提示词不能覆盖，请先删除对应视频。`,
        confirmText: "重新生成",
      });
      if (!ok) {
        finishAction(actionKey, requestScope);
        return;
      }
      if (!isCurrentActionScope(requestScope)) {
        finishAction(actionKey, requestScope);
        return;
      }
    }
    const taskId = createProgressTaskId("video-prompts", projectId);
    setVideoTaskId(taskId);
    try {
      await runProgressTask(
        {
          title: `${regenerateExisting ? "重新生成" : "生成下一批"}${promptLabel}`,
          startMessage: `正在准备${regenerateExisting ? "重新生成" : "生成"}${promptLabel}`,
          successMessage: `已${regenerateExisting ? "重新生成" : "生成本批"}${promptLabel}`,
          errorMessage: `${regenerateExisting ? "重新生成" : "生成"}${promptLabel}失败`,
          total: 100,
          taskId,
          showStream: false,
        },
        async () => {
          await generateChapterVideoPrompts(
            projectId,
            chapter.id,
            videoPromptGenerationOptions(promptOptions, requestOptions),
            taskId,
          );
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast(`已${regenerateExisting ? "重新生成" : "生成本批"}${promptLabel}`, "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, `${regenerateExisting ? "重新生成" : "生成"}${promptLabel}失败`);
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentActionScope(requestScope)) {
        setVideoTaskId(null);
      }
      finishAction(actionKey, requestScope);
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
      finishAction(actionKey, requestScope);
      return;
    }
    if (chapterGenerating) {
      addToast(chapterGeneratingMessage(chapter, `生成${frameLabel}`), "error");
      finishAction(actionKey, requestScope);
      return;
    }
    let chain: ChapterVideoPrompt[];
    try {
      chain = boundaryFramePromptChainTo(targetPrompt, framePrompts, "生成");
    } catch (err) {
      addToast(extractErrorMessage(err, "边界帧提示词链不完整"), "error");
      finishAction(actionKey, requestScope);
      return;
    }
    const missingPrompts: ChapterVideoPrompt[] = [];
    for (const item of chain) {
      if (item.status === "running") {
        addToast(`${label} #${item.shot_number} 正在运行，不能成图`, "error");
        finishAction(actionKey, requestScope);
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
        finishAction(actionKey, requestScope);
        return;
      }
    }
    if (missingPrompts.length === 0) {
      addToast(`第 1-${targetPrompt.shot_number} 张${frameLabel}图片已存在，将直接复用`, "info");
      finishAction(actionKey, requestScope);
      return;
    }
    const ok = await confirmDialog({
      title: `生成到第 ${targetPrompt.shot_number} 张${frameLabel}`,
      message: `将按顺序检查第 1-${targetPrompt.shot_number} 张${frameLabel}图片：已有图片直接复用，缺失的 ${missingPrompts.length} 张会从前到后依次生成；第 2 张开始都会使用上一张实际成图作为参考图 1。`,
      confirmText: "顺序生成",
    });
    if (!ok) {
      finishAction(actionKey, requestScope);
      return;
    }
    if (!isCurrentActionScope(requestScope)) {
      finishAction(actionKey, requestScope);
      return;
    }
    const taskId = createProgressTaskId("video-assets", projectId);
    setVideoTaskId(taskId);
    try {
      const generatedIds: number[] = [];
      const chainImages = [...frameImages];
      await runProgressTask(
        {
          title: `生成到第 ${targetPrompt.shot_number} 张${frameLabel}`,
          startMessage: `正在准备顺序生成${frameLabel}`,
          successMessage: `${frameLabel}顺序生成已完成`,
          errorMessage: `${frameLabel}顺序生成失败`,
          total: missingPrompts.length,
          taskId,
        },
        async () => {
          for (const item of missingPrompts) {
            const request = buildFrameImageGenerationRequest(
              item,
              label,
              "key_frame",
              "video_boundary_frame_prompt",
              chainImages,
              chain,
            );
            const rows = await batchGenerateImages(projectId, [request], taskId);
            const image = rows[0];
            if (!image || !Number.isInteger(image.id) || image.id <= 0) {
              throw new Error(`第 ${item.shot_number} 张${frameLabel}生成结果缺少有效图片 ID`);
            }
            generatedIds.push(image.id);
            chainImages.push(image);
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
      if (isCurrentActionScope(requestScope)) {
        setVideoTaskId(null);
      }
      finishAction(actionKey, requestScope);
    }
  };

  const handleGenerateFrameImages = async (prompt: ChapterVideoPrompt) => {
    if (!chapter) return;
    const firstFramePrompt = prompt.prompt_type === VIDEO_FIRST_FRAME_PROMPT_TYPE;
    const chatFramePrompt = prompt.prompt_type === VIDEO_CHAT_FRAME_PROMPT_TYPE;
    const storyboardPrompt = prompt.prompt_type === VIDEO_STORYBOARD_PROMPT_TYPE;
    const label = firstFramePrompt
      ? "首帧提示词"
      : storyboardPrompt
        ? "分镜提示词"
      : chatFramePrompt
        ? "Chat 首尾帧提示词"
        : "边界帧提示词";
    const frameLabel = firstFramePrompt
      ? "首帧"
      : storyboardPrompt
        ? "分镜板"
        : chatFramePrompt
          ? "Chat 首尾帧"
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
      finishAction(actionKey, requestScope);
      return;
    }
    if (!isCurrentActionScope(requestScope)) {
      finishAction(actionKey, requestScope);
      return;
    }
    await runImageAssetTask(
      actionKey,
      firstFramePrompt
        ? "生成首帧"
        : storyboardPrompt
          ? "生成分镜板"
          : chatFramePrompt
            ? "生成 Chat 首尾帧"
            : "生成边界帧",
      frameLabel,
      [request],
    );
  };

  const handleImportFrameImage = async (prompt: ChapterVideoPrompt) => {
    const firstFramePrompt = prompt.prompt_type === VIDEO_FIRST_FRAME_PROMPT_TYPE;
    const chatFramePrompt = prompt.prompt_type === VIDEO_CHAT_FRAME_PROMPT_TYPE;
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
          : chatFramePrompt
            ? "Chat 首尾帧"
            : "边界帧",
      firstFramePrompt
        ? "video_first_frame_prompt"
        : storyboardPrompt
          ? "video_storyboard_prompt"
          : "video_boundary_frame_prompt",
    );
  };

  return {
    handleGenerateShotAnalysis,
    handleGenerateSegmentPlan,
    handleGenerateFramePrompts,
    handleGenerateVideoPrompts,
    handleGenerateFrameImages,
    handleImportFrameImage,
  };
}
