import {
  useLayoutEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import type { GeneratedImage } from "../../../api/image";
import type { ToastType } from "../../../components/feedback/Toast";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../../components/feedback/PromptDialog";
import { extractErrorMessage } from "../../../api";
import {
  deleteChapterVideoPrompt,
  deleteVideo,
  optimizeChapterVideoPrompt,
  pollVideoJob,
  previewChapterVideoRequest,
  previewVideoProviderRequestByVideoId,
  startVideoGeneration,
  trimVideo,
  updateChapterVideoPrompt,
  updateVideo,
  uploadChapterVideo,
  verifyVideoGenerationResult,
  type ChapterVideoPrompt,
  type DownstreamConflictStrategy,
  type GeneratedVideo,
  type PreviewChapterVideoPromptOverrideInput,
  type VideoPromptDeletionEffects,
  type VideoRecordDeleteResult,
  type UpdateVideoFields,
} from "../../../api/video";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import type { RunProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { nonEmptyText } from "../../../utils/text";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import {
  pickVideoUploadFile,
  type VideoUploadFile,
} from "../../../utils/videoUpload";
import type { ChapterRow } from "../shared";
import { chooseDownstreamConflictStrategy } from "../dependencyConflict";
import type { MessageTranslator } from "../../../i18n";
import type { LoadVideoTabData } from "./useVideoTabLoading";
import { promptTitle } from "./videoGenerationDisplay";
import { extractSourceContentHash } from "./videoTabShared";
import {
  hasPendingProviderJob,
  isDurationAllowed,
  staleVideoPromptMessage,
} from "./videoOptionUtils";
import {
  isVideoFramePromptImageGenerationSource,
  readSavedReferenceImageIds,
  videoImageGenerationPromptId,
  videoImageGenerationSource,
  videoPromptSupportsProviderGeneration,
} from "./videoImageUtils";
import {
  isVideoFramePromptType,
  VIDEO_FINAL_PROMPT_TYPE,
  VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
  type PromptDraft,
  type VideoDurationLimits,
} from "./videoTabConstants";
import { videoExecutionPromptsShareGenerationChain } from "./videoExecutionIdentity";
import { videoProviderRequestPreviewBlockedReason } from "./videoAssemblyUtils";

type AddToast = (message: string, type: ToastType) => void;

const promptPreviewOverride = (
  draft: PromptDraft,
): PreviewChapterVideoPromptOverrideInput => ({
  visual_prompt: draft.visual_prompt,
  motion_prompt: nonEmptyText(draft.motion_prompt),
  negative_prompt: nonEmptyText(draft.negative_prompt),
  duration_secs: draft.duration_secs,
});

const videoPromptStage = (prompt: ChapterVideoPrompt): number | null => {
  if (prompt.prompt_type === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE) return 1;
  if (isVideoFramePromptType(prompt.prompt_type)) return 2;
  if (prompt.prompt_type === VIDEO_FINAL_PROMPT_TYPE) return 3;
  return null;
};

export const videoPromptDownstreamCount = (
  prompt: ChapterVideoPrompt,
  prompts: ChapterVideoPrompt[],
  frameImages: GeneratedImage[],
): number => {
  const stage = videoPromptStage(prompt);
  if (stage == null) return 0;
  const affectedPromptIds = new Set<number>();
  const affectedVideoIds = new Set<number>();
  if (stage === 3) affectedPromptIds.add(prompt.id);

  for (const row of prompts) {
    if (row.id === prompt.id) continue;
    const rowStage = videoPromptStage(row);
    if (rowStage == null || rowStage <= stage) continue;
    if (!videoExecutionPromptsShareGenerationChain(prompt, row)) continue;
    affectedPromptIds.add(row.id);
    if (row.generated_video_id != null)
      affectedVideoIds.add(row.generated_video_id);
  }

  const affectedImageIds = new Set<number>();
  for (const image of frameImages) {
    const imagePromptId = videoImageGenerationPromptId(image);
    if (
      isVideoFramePromptImageGenerationSource(
        videoImageGenerationSource(image),
      ) &&
      (affectedPromptIds.has(imagePromptId ?? -1) || imagePromptId === prompt.id)
    ) {
      affectedImageIds.add(image.id);
    }
  }
  if (stage === 3 && prompt.generated_video_id != null) {
    affectedVideoIds.add(prompt.generated_video_id);
  }

  const downstreamPromptCount =
    stage === 3
      ? Math.max(affectedPromptIds.size - 1, 0)
      : affectedPromptIds.size;
  return downstreamPromptCount + affectedImageIds.size + affectedVideoIds.size;
};

interface UseVideoRecordActionsParams {
  projectId: number;
  chapter: ChapterRow | null;
  selectedPrompt: ChapterVideoPrompt | null;
  selectedPromptId: number | null;
  frameStageLabel: string;
  selectedPromptStale: boolean;
  currentContentHash: string | null;
  selectedVideo: GeneratedVideo | null;
  previewScopeKey: string | null;
  actionScopeKey: string | null;
  prompts: ChapterVideoPrompt[];
  frameImages: GeneratedImage[];
  draft: PromptDraft;
  selectedPromptDirty: boolean;
  setDraft: Dispatch<SetStateAction<PromptDraft>>;
  validPromptReferenceIds: Set<number>;
  durationLimits: VideoDurationLimits;
  t: MessageTranslator;
  addToast: AddToast;
  runProgressTask: RunProgressTask;
  loadAll: LoadVideoTabData;
  ensureActionAvailable?: () => boolean;
  setPrompts: Dispatch<SetStateAction<ChapterVideoPrompt[]>>;
  setImages: Dispatch<SetStateAction<GeneratedImage[]>>;
  setVideos: Dispatch<SetStateAction<GeneratedVideo[]>>;
  setSelectedPromptId: Dispatch<SetStateAction<number | null>>;
  setSelectedVideoId: Dispatch<SetStateAction<number | null>>;
  setPendingVideoId: Dispatch<SetStateAction<number | null>>;
  onVideoRecordDeleted: (result: VideoRecordDeleteResult) => void;
  beginAction: (key: string) => boolean;
  isCurrentActionScope: (scope: string | null) => boolean;
  finishAction: (key: string) => void;
}

export function useVideoRecordActions({
  projectId,
  chapter,
  selectedPrompt,
  selectedPromptId,
  frameStageLabel,
  selectedPromptStale,
  currentContentHash,
  selectedVideo,
  previewScopeKey,
  actionScopeKey,
  prompts,
  frameImages,
  draft,
  selectedPromptDirty,
  setDraft,
  validPromptReferenceIds,
  durationLimits,
  t,
  addToast,
  runProgressTask,
  loadAll,
  ensureActionAvailable,
  setPrompts,
  setImages,
  setVideos,
  setSelectedPromptId,
  setSelectedVideoId,
  setPendingVideoId,
  onVideoRecordDeleted,
  beginAction,
  isCurrentActionScope,
  finishAction,
}: UseVideoRecordActionsParams) {
  const tRef = useRef(t);
  tRef.current = t;
  const currentT = () => tRef.current;
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const requestPreviewScopeRef = useRef(0);
  const requestPreviewBusyScopeRef = useRef<number | null>(null);
  const [trimStart, setTrimStart] = useState("0");
  const [trimEnd, setTrimEnd] = useState("");

  const upsertVideo = (video: GeneratedVideo) => {
    setVideos((current) => {
      const index = current.findIndex((item) => item.id === video.id);
      if (index < 0) return [...current, video];
      const next = [...current];
      next[index] = video;
      return next;
    });
  };

  const refreshVideoDataInBackground = (requestScope: string | null) => {
    void Promise.resolve().then(() => loadAll({ showLoading: false })).then((refreshed) => {
      if (isCurrentActionScope(requestScope) && !refreshed) {
        addToast(currentT()("video.chapter.refreshVideoDataFailed"), "error");
      }
    }).catch((error) => {
      if (isCurrentActionScope(requestScope)) {
        addToast(extractErrorMessage(error, currentT()("video.chapter.refreshVideoDataFailed")), "error");
      }
    });
  };

  const ensureCurrentVideoRead = () =>
    !ensureActionAvailable || ensureActionAvailable();

  const applyPromptMutation = (
    updatedPrompt: ChapterVideoPrompt | null,
    deletion: VideoPromptDeletionEffects,
  ) => {
    const deletedPromptIds = new Set(deletion.deleted_prompt_ids);
    const deletedImageIds = new Set(deletion.deleted_image_ids);
    const deletedVideoIds = new Set(deletion.deleted_video_ids);
    setPrompts((current) => {
      const remaining = current.filter((row) => !deletedPromptIds.has(row.id));
      if (!updatedPrompt) return remaining;
      const index = remaining.findIndex((row) => row.id === updatedPrompt.id);
      if (index < 0) return [...remaining, updatedPrompt];
      return remaining.map((row) => row.id === updatedPrompt.id ? updatedPrompt : row);
    });
    if (deletedImageIds.size > 0) {
      setImages((current) => current.filter((row) => !deletedImageIds.has(row.id)));
    }
    if (deletedVideoIds.size > 0) {
      setVideos((current) => current.filter((row) => !deletedVideoIds.has(row.id)));
      setSelectedVideoId((current) => current != null && deletedVideoIds.has(current) ? null : current);
      setPendingVideoId((current) => current != null && deletedVideoIds.has(current) ? null : current);
    }
    setSelectedPromptId((current) => current != null && deletedPromptIds.has(current) ? null : current);
    if (updatedPrompt && selectedPromptId === updatedPrompt.id) {
      setDraft({
        visual_prompt: updatedPrompt.visual_prompt,
        motion_prompt: updatedPrompt.motion_prompt ?? "",
        negative_prompt: updatedPrompt.negative_prompt ?? "",
        duration_secs: updatedPrompt.duration_secs,
      });
    }
  };

  const videoPromptGenerateBlockedReason = (
    prompt: ChapterVideoPrompt,
  ): string | null => {
    if (prompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE)
      return currentT()("video.actions.promptTypeRequired");
    if (!videoPromptSupportsProviderGeneration(prompt)) {
      return currentT()("video.actions.externalChatGeneration");
    }
    if (prompt.status === "running") return currentT()("video.actions.promptRunning", { id: prompt.id });
    const promptSourceHash = extractSourceContentHash(prompt.metadata_json);
    if (
      promptSourceHash &&
      currentContentHash &&
      promptSourceHash !== currentContentHash
    ) {
      return staleVideoPromptMessage(frameStageLabel, currentT());
    }
    const referenceState = readSavedReferenceImageIds(
      prompt.reference_image_ids_json,
      currentT(),
    );
    if (referenceState.error) {
      return currentT()("video.actions.savedReferenceInvalid", { id: prompt.id, error: referenceState.error });
    }
    const staleReferenceIds = referenceState.ids.filter(
      (id) => !validPromptReferenceIds.has(id),
    );
    if (staleReferenceIds.length > 0) {
      return currentT()("video.actions.staleReferences", { id: prompt.id, ids: staleReferenceIds.join(", ") });
    }
    if (selectedPromptDirty && prompt.id === selectedPrompt?.id) {
      return currentT()("video.actions.unsavedBefore", { operation: currentT()("video.chapter.generateVideo") });
    }
    if (!isDurationAllowed(prompt.duration_secs, durationLimits)) {
      return currentT()("video.actions.durationUnsupported", { id: prompt.id });
    }
    return null;
  };
  const choosePromptMutationStrategy = async (
    prompt: ChapterVideoPrompt,
    operation: string,
    downstreamCount = videoPromptDownstreamCount(prompt, prompts, frameImages),
  ): Promise<DownstreamConflictStrategy | null> => {
    const strategy = await chooseDownstreamConflictStrategy({
      title: currentT()("video.actions.downstreamTitle"),
      assetLabel: `${promptTitle(prompt, currentT())} #${prompt.id}`,
      downstreamLabel: currentT()("video.actions.downstreamLabel"),
      downstreamCount,
      operation,
      t: currentT(),
    });
    return strategy;
  };

  useLayoutEffect(() => {
    requestPreviewScopeRef.current += 1;
    requestPreviewBusyScopeRef.current = null;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setTrimStart("0");
    setTrimEnd("");
    return () => {
      requestPreviewScopeRef.current += 1;
      requestPreviewBusyScopeRef.current = null;
    };
  }, [previewScopeKey]);

  const wrapAct = async (
    key: string,
    fn: (isCurrent: () => boolean) => Promise<GeneratedVideo | void>,
    ok: string | (() => string),
    alreadyStarted = false,
  ) => {
    const requestScope = actionScopeKey;
    if (!alreadyStarted && !beginAction(key)) return;
    if (!ensureCurrentVideoRead()) {
      finishAction(key);
      return;
    }
    try {
      const result = await fn(() => isCurrentActionScope(requestScope));
      if (!isCurrentActionScope(requestScope)) return;
      if (result) upsertVideo(result);
      addToast(typeof ok === "function" ? ok() : ok, "success");
      refreshVideoDataInBackground(requestScope);
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, currentT()("video.actions.operationFailed"));
      if (message.includes("视频记录已被其他操作修改")) {
        refreshVideoDataInBackground(requestScope);
      }
      addToast(message, "error");
    } finally {
      finishAction(key);
    }
  };

  const handleOptimizePromptRow = async (prompt: ChapterVideoPrompt) => {
    if (prompt.status === "running") {
      addToast(currentT()("video.actions.promptRunningCannot", { id: prompt.id, operation: currentT()("video.chapter.optimizePrompt") }), "error");
      return null;
    }
    const requestScope = actionScopeKey;
    const actionKey = `optimize-prompt-${prompt.id}`;
    if (!beginAction(actionKey)) return null;
    try {
      const instruction = await promptDialog({
        title: currentT()("video.actions.optimizeTitle"),
        message: currentT()("video.actions.optimizeMessage"),
        placeholder: currentT()("video.actions.optimizePlaceholder"),
        multiline: true,
        required: true,
        confirmText: currentT()("video.actions.startOptimize"),
      });
      const trimmed = instruction?.trim();
      if (!trimmed) return null;
      if (!ensureCurrentVideoRead()) return null;
      const downstreamStrategy = await choosePromptMutationStrategy(
        prompt,
        currentT()("common.update"),
      );
      if (downstreamStrategy == null || !isCurrentActionScope(requestScope)) {
        return null;
      }
      if (!ensureCurrentVideoRead()) return null;
      const updated = await runProgressTask(
        {
          title: currentT()("video.chapter.optimizePrompt"),
          startMessage: currentT()("video.actions.preparing", { operation: currentT()("video.chapter.optimizePrompt") }),
          successMessage: currentT()("video.actions.optimizeSuccess"),
          errorMessage: currentT()("video.actions.optimizeFailed"),
          total: 3,
          runningHint: currentT()("video.actions.optimizeRunningHint"),
        },
        async ({ setProgress }) => {
          setProgress(currentT()("video.actions.optimizeProgressStart"), 1, 3);
          const result = await optimizeChapterVideoPrompt(prompt.id, {
            instruction: trimmed,
            downstream_strategy: downstreamStrategy,
            expected_updated_at: prompt.updated_at,
          });
          if (!isCurrentActionScope(requestScope)) return result.prompt;
          setProgress(currentT()("video.actions.optimizeProgressRefresh"), 2, 3);
          applyPromptMutation(result.prompt, result.deletion);
          return result.prompt;
        },
      );
      if (!isCurrentActionScope(requestScope)) return null;
      addToast(currentT()("video.actions.optimizeSuccess"), "success");
      refreshVideoDataInBackground(requestScope);
      return updated;
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return null;
      refreshVideoDataInBackground(requestScope);
      addToast(extractErrorMessage(err, currentT()("video.actions.optimizeFailed")), "error");
      return null;
    } finally {
      finishAction(actionKey);
    }
  };

  const handleDeletePromptRow = async (prompt: ChapterVideoPrompt) => {
    if (prompt.status === "running") {
      addToast(currentT()("video.actions.promptRunningCannot", { id: prompt.id, operation: currentT()("common.delete") }), "error");
      return;
    }
    const actionKey = `delete-prompt-${prompt.id}`;
    if (!beginAction(actionKey)) return;
    let downstreamStrategy: DownstreamConflictStrategy = "reject";
    const downstreamCount = videoPromptDownstreamCount(
      prompt,
      prompts,
      frameImages,
    );
    if (downstreamCount > 0) {
      const strategy = await choosePromptMutationStrategy(
        prompt,
        currentT()("common.delete"),
        downstreamCount,
      );
      if (strategy == null) {
        finishAction(actionKey);
        return;
      }
      downstreamStrategy = strategy;
    } else {
      const ok = await confirmDialog({
        title: currentT()("video.chapter.deletePrompt"),
        message: currentT()("common.deleteConfirm", { name: `${currentT()("video.chapter.promptLabel")} #${prompt.id}` }),
        confirmText: currentT()("common.delete"),
        dialogType: "danger",
      });
      if (!ok) {
        finishAction(actionKey);
        return;
      }
    }
    await wrapAct(
      actionKey,
      async (isCurrent) => {
        const result = await deleteChapterVideoPrompt(prompt.id, downstreamStrategy);
        if (!isCurrent()) return;
        applyPromptMutation(null, result.deletion);
      },
      () => currentT()("video.actions.promptDeleted"),
      true,
    );
  };

  const handleSavePromptRow = async (
    prompt: ChapterVideoPrompt,
    promptDraft: PromptDraft,
  ) => {
    if (prompt.status === "running") {
      addToast(currentT()("video.actions.promptRunningCannot", { id: prompt.id, operation: currentT()("common.save") }), "error");
      return null;
    }
    const visualPrompt = promptDraft.visual_prompt.trim();
    if (!visualPrompt) {
      addToast(currentT()("video.actions.visualPromptEmpty"), "error");
      return null;
    }
    if (!isDurationAllowed(promptDraft.duration_secs, durationLimits)) {
      addToast(currentT()("video.chapter.invalidDuration"), "error");
      return null;
    }
    const requestScope = actionScopeKey;
    const actionKey = `save-prompt-${prompt.id}`;
    if (!beginAction(actionKey)) return null;
    try {
      const downstreamStrategy = await choosePromptMutationStrategy(
        prompt,
        currentT()("common.save"),
      );
      if (downstreamStrategy == null || !isCurrentActionScope(requestScope)) {
        return null;
      }
      const result = await updateChapterVideoPrompt(prompt.id, {
        visual_prompt: visualPrompt,
        motion_prompt: nonEmptyText(promptDraft.motion_prompt),
        negative_prompt: nonEmptyText(promptDraft.negative_prompt),
        duration_secs: promptDraft.duration_secs,
        downstream_strategy: downstreamStrategy,
        expected_updated_at: prompt.updated_at,
      });
      if (!isCurrentActionScope(requestScope)) return null;
      applyPromptMutation(result.prompt, result.deletion);
      if (!isCurrentActionScope(requestScope)) return null;
      addToast(currentT()("video.actions.promptSaved"), "success");
      refreshVideoDataInBackground(requestScope);
      return result.prompt;
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return null;
      refreshVideoDataInBackground(requestScope);
      addToast(extractErrorMessage(err, currentT()("errors.saveFailed")), "error");
      return null;
    } finally {
      finishAction(actionKey);
    }
  };

  const handleSavePrompt = async () => {
    if (!selectedPrompt) return null;
    return handleSavePromptRow(selectedPrompt, draft);
  };

  const handleGenerateVideo = async () => {
    if (!chapter || !selectedPrompt) return;
    if (durationLimits.configError) {
      addToast(durationLimits.configError, "error");
      return;
    }
    const blockedReason = videoPromptGenerateBlockedReason(selectedPrompt);
    if (blockedReason) {
      addToast(blockedReason, "error");
      return;
    }
    const requestScope = actionScopeKey;
    const actionKey = `generate-video-${selectedPrompt.id}`;
    if (!beginAction(actionKey)) return;
    let generated = false;
    try {
      const handle = await startVideoGeneration({
        project_id: projectId,
        operation: {
          kind: "chapter",
          chapter_id: chapter.id,
          prompt_id: selectedPrompt.id,
          config_id: null,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: currentT()("video.chapter.generateVideo"),
          startMessage: currentT()("video.actions.preparing", { operation: currentT()("video.chapter.generateVideo") }),
          successMessage: currentT()("video.actions.videoGenerated"),
          errorMessage: currentT()("video.actions.failed", { operation: currentT()("video.chapter.generateVideo") }),
          cancelLabel: currentT()("workflow.stop"),
        },
        async (task) => {
          const [row] = await verifyVideoGenerationResult(
            task,
            projectId,
            chapter.id,
          );
          if (!isCurrentActionScope(requestScope)) return;
          setSelectedVideoId(row.id);
          setPendingVideoId(null);
          upsertVideo(row);
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      generated = true;
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, currentT()("video.actions.failed", { operation: currentT()("video.chapter.generateVideo") }));
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentActionScope(requestScope)) {
        if (generated) addToast(currentT()("video.actions.videoGenerated"), "success");
        refreshVideoDataInBackground(requestScope);
      }
      finishAction(actionKey);
    }
  };

  const handleImportVideo = async (): Promise<void> => {
    if (!chapter || !selectedPrompt) return;
    if (selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE) {
      addToast(currentT()("video.actions.promptTypeRequiredImport"), "error");
      return;
    }
    if (selectedPrompt.status === "running") {
      addToast(
        currentT()("video.actions.promptRunningCannot", {
          id: selectedPrompt.id,
          operation: currentT()("video.chapter.importVideo"),
        }),
        "error",
      );
      return;
    }
    if (selectedPromptStale) {
      addToast(staleVideoPromptMessage(frameStageLabel, currentT()), "error");
      return;
    }
    if (selectedPromptDirty) {
      addToast(currentT()("video.actions.unsavedBefore", { operation: currentT()("video.chapter.importVideo") }), "error");
      return;
    }

    const requestScope = actionScopeKey;
    const actionKey = `import-video-${selectedPrompt.id}`;
    if (!beginAction(actionKey)) return;
    let upload: VideoUploadFile | null = null;
    try {
      upload = await pickVideoUploadFile(currentT());
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, currentT()("video.chapter.filePickerFailed")), "error");
      return;
    } finally {
      if (!upload) finishAction(actionKey);
    }
    if (!upload) return;
    const { file, mimeType } = upload;
    if (!isCurrentActionScope(requestScope)) {
      finishAction(actionKey);
      return;
    }
      if (!ensureCurrentVideoRead()) {
        finishAction(actionKey);
        return;
      }

    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentActionScope(requestScope)) return;
      const row = await uploadChapterVideo(projectId, chapter.id, {
        file_name: file.name,
        mime_type: mimeType,
        data_base64: dataBase64,
        prompt_id: selectedPrompt.id,
      });
      if (!isCurrentActionScope(requestScope)) return;
      setSelectedVideoId(row.id);
      setPendingVideoId(null);
      upsertVideo(row);
      addToast(currentT()("video.actions.videoImported"), "success");
      refreshVideoDataInBackground(requestScope);
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, currentT()("video.actions.videoImportFailed")), "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const openVideoRequestPreviewData = async (
    loadPreview: () => Promise<ModelRequestPreview>,
    fallbackMessage: string,
  ) => {
    if (requestPreviewBusyScopeRef.current != null) return;
    const previewScope = ++requestPreviewScopeRef.current;
    requestPreviewBusyScopeRef.current = previewScope;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await loadPreview();
      if (requestPreviewScopeRef.current !== previewScope) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (requestPreviewScopeRef.current !== previewScope) return;
      addToast(extractErrorMessage(err, fallbackMessage), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (requestPreviewScopeRef.current === previewScope) {
        setRequestPreviewLoading(false);
      }
      if (requestPreviewBusyScopeRef.current === previewScope) {
        requestPreviewBusyScopeRef.current = null;
      }
    }
  };

  const closeRequestPreview = () => {
    requestPreviewScopeRef.current += 1;
    requestPreviewBusyScopeRef.current = null;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  };

  const openVideoRequestPreview = async (
    promptId: number,
    promptOverride?: PreviewChapterVideoPromptOverrideInput,
  ) => {
    if (!chapter) return;
    await openVideoRequestPreviewData(
      () =>
        previewChapterVideoRequest(
          projectId,
          chapter.id,
          promptId,
          null,
          promptOverride,
        ),
      currentT()("video.actions.previewRequestFailed"),
    );
  };

  const handlePreviewGenerateVideoRequest = async () => {
    if (!selectedPrompt) return;
    if (selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE) {
      addToast(currentT()("video.actions.promptTypeRequired"), "error");
      return;
    }
    const referenceState = readSavedReferenceImageIds(
      selectedPrompt.reference_image_ids_json,
      currentT(),
    );
    if (referenceState.error) {
      addToast(currentT()("video.actions.savedReferenceInvalid", {
        id: selectedPrompt.id,
        error: referenceState.error,
      }), "error");
      return;
    }
    const staleReferenceIds = referenceState.ids.filter(
      (id) => !validPromptReferenceIds.has(id),
    );
    if (staleReferenceIds.length > 0) {
      addToast(
        currentT()("video.actions.finalReferencesInvalid", { ids: staleReferenceIds.join(", ") }),
        "error",
      );
      return;
    }
    await openVideoRequestPreview(
      selectedPrompt.id,
      selectedPromptDirty ? promptPreviewOverride(draft) : undefined,
    );
  };

  const handlePreviewVideoRequestForVideo = async (video: GeneratedVideo) => {
    const blockedReason = videoProviderRequestPreviewBlockedReason(video, currentT());
    if (blockedReason) {
      addToast(blockedReason, "error");
      return;
    }
    if (!chapter) return;
    await openVideoRequestPreviewData(
      () =>
        previewVideoProviderRequestByVideoId(projectId, chapter.id, video.id),
      currentT()("video.actions.previewHistoryFailed"),
    );
  };

  const handlePollVideo = async (video: GeneratedVideo) => {
    await wrapAct(
      `poll-${video.id}`,
      async (isCurrent) => {
        const row = await pollVideoJob(video.id);
        if (!isCurrent()) return;
        setSelectedVideoId(row.id);
        setPendingVideoId(hasPendingProviderJob(row) ? row.id : null);
        return row;
      },
      () => currentT()("video.actions.videoStatusRefreshed"),
    );
  };

  const handleToggleVideoFavorite = async (video: GeneratedVideo) => {
    await wrapAct(
      `favorite-${video.id}`,
      () => updateVideo(video.id, {
        is_favorite: !video.is_favorite,
        expected_updated_at: video.updated_at,
      }),
      () => video.is_favorite
        ? currentT()("video.actions.videoFavoriteRemoved")
        : currentT()("video.actions.videoFavoriteAdded"),
    );
  };

  const handleUpdateVideo = async (
    video: GeneratedVideo,
    input: UpdateVideoFields,
  ) => {
    if (input.prompt != null && !input.prompt.trim()) {
      addToast(currentT()("video.actions.videoPromptEmpty"), "error");
      throw new Error(currentT()("video.actions.videoPromptEmpty"));
    }
    const requestScope = actionScopeKey;
    const actionKey = `update-video-${video.id}`;
    if (!beginAction(actionKey)) throw new Error(currentT()("video.actions.videoActionBusy"));
    try {
      const row = await updateVideo(video.id, {
        ...input,
        expected_updated_at: video.updated_at,
      });
      if (!isCurrentActionScope(requestScope)) return;
      setSelectedVideoId(row.id);
      upsertVideo(row);
      addToast(currentT()("video.actions.videoRecordSaved"), "success");
      refreshVideoDataInBackground(requestScope);
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, currentT()("video.actions.videoRecordSaveFailed"));
      if (message.includes("视频记录已被其他操作修改")) {
        refreshVideoDataInBackground(requestScope);
      }
      addToast(message, "error");
      throw err;
    } finally {
      finishAction(actionKey);
    }
  };

  const handleDeleteVideo = async (video: GeneratedVideo) => {
    const actionKey = `delete-video-${video.id}`;
    if (!beginAction(actionKey)) return;
    const ok = await confirmDialog({
      title: currentT()("video.actions.deleteVideoTitle"),
      message: currentT()("video.actions.deleteVideoMessage", { id: video.id }),
      confirmText: currentT()("common.delete"),
      dialogType: "danger",
    });
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    await wrapAct(
      actionKey,
      async (isCurrent) => {
        const result = await deleteVideo(video.id);
        if (!isCurrent()) return;
        onVideoRecordDeleted(result);
      },
      () => currentT()("video.actions.videoDeleted"),
      true,
    );
  };

  const handleTrim = async () => {
    if (!selectedVideo) return;
    const start = Number(trimStart);
    const end = Number(trimEnd);
    if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) {
      addToast(currentT()("video.actions.trimInvalid"), "error");
      return;
    }
    await wrapAct(
      `trim-${selectedVideo.id}`,
      async (isCurrent) => {
        const row = await trimVideo(selectedVideo.id, start, end);
        if (!isCurrent()) return;
        setSelectedVideoId(row.id);
        return row;
      },
      () => currentT()("video.actions.trimmedVideoCreated"),
    );
  };

  return {
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
    selectedPromptDirty,
    handlePollVideo,
    handleToggleVideoFavorite,
    handleUpdateVideo,
    handleDeleteVideo,
    handleTrim,
    requestPreviewOpen,
    requestPreviewLoading,
    requestPreviewData,
    closeRequestPreview,
  };
}
