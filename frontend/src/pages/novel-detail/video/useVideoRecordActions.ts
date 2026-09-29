import {
  useEffect,
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
  type UpdateVideoInput,
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
  pendingVideoId: number | null;
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
  loadAll: () => Promise<boolean>;
  setPrompts: Dispatch<SetStateAction<ChapterVideoPrompt[]>>;
  setVideos: Dispatch<SetStateAction<GeneratedVideo[]>>;
  setSelectedPromptId: Dispatch<SetStateAction<number | null>>;
  setSelectedVideoId: Dispatch<SetStateAction<number | null>>;
  setPendingVideoId: Dispatch<SetStateAction<number | null>>;
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
  pendingVideoId,
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
  setPrompts,
  setVideos,
  setSelectedPromptId,
  setSelectedVideoId,
  setPendingVideoId,
  beginAction,
  isCurrentActionScope,
  finishAction,
}: UseVideoRecordActionsParams) {
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

  const videoPromptGenerateBlockedReason = (
    prompt: ChapterVideoPrompt,
  ): string | null => {
    if (prompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE)
      return t("video.actions.promptTypeRequired");
    if (!videoPromptSupportsProviderGeneration(prompt)) {
      return t("video.actions.externalChatGeneration");
    }
    if (prompt.status === "running") return t("video.actions.promptRunning", { id: prompt.id });
    const promptSourceHash = extractSourceContentHash(prompt.metadata_json);
    if (
      promptSourceHash &&
      currentContentHash &&
      promptSourceHash !== currentContentHash
    ) {
      return staleVideoPromptMessage(frameStageLabel, t);
    }
    const referenceState = readSavedReferenceImageIds(
      prompt.reference_image_ids_json,
      t,
    );
    if (referenceState.error) {
      return t("video.actions.savedReferenceInvalid", { id: prompt.id, error: referenceState.error });
    }
    const staleReferenceIds = referenceState.ids.filter(
      (id) => !validPromptReferenceIds.has(id),
    );
    if (staleReferenceIds.length > 0) {
      return t("video.actions.staleReferences", { id: prompt.id, ids: staleReferenceIds.join(", ") });
    }
    if (selectedPromptDirty && prompt.id === selectedPrompt?.id) {
      return t("video.actions.unsavedBefore", { operation: t("video.chapter.generateVideo") });
    }
    if (!isDurationAllowed(prompt.duration_secs, durationLimits)) {
      return t("video.actions.durationUnsupported", { id: prompt.id });
    }
    return null;
  };
  const choosePromptMutationStrategy = async (
    prompt: ChapterVideoPrompt,
    operation: string,
    downstreamCount = videoPromptDownstreamCount(prompt, prompts, frameImages),
  ): Promise<DownstreamConflictStrategy | null> => {
    const strategy = await chooseDownstreamConflictStrategy({
      title: t("video.actions.downstreamTitle"),
      assetLabel: `${promptTitle(prompt, t)} #${prompt.id}`,
      downstreamLabel: t("video.actions.downstreamLabel"),
      downstreamCount,
      operation,
      t,
    });
    return strategy;
  };

  useEffect(() => {
    requestPreviewScopeRef.current += 1;
    requestPreviewBusyScopeRef.current = null;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setTrimStart("0");
    setTrimEnd("");
  }, [previewScopeKey]);

  const wrapAct = async (
    key: string,
    fn: (isCurrent: () => boolean) => Promise<GeneratedVideo | void>,
    ok: string,
    alreadyStarted = false,
  ) => {
    const requestScope = actionScopeKey;
    if (!alreadyStarted && !beginAction(key)) return;
    try {
      const result = await fn(() => isCurrentActionScope(requestScope));
      if (!isCurrentActionScope(requestScope)) return;
      if (result) upsertVideo(result);
      await loadAll();
      if (!isCurrentActionScope(requestScope)) return;
      addToast(ok, "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, t("video.actions.operationFailed")), "error");
    } finally {
      finishAction(key);
    }
  };

  const handleOptimizePromptRow = async (prompt: ChapterVideoPrompt) => {
    if (prompt.status === "running") {
      addToast(t("video.actions.promptRunningCannot", { id: prompt.id, operation: t("video.chapter.optimizePrompt") }), "error");
      return null;
    }
    const requestScope = actionScopeKey;
    const actionKey = `optimize-prompt-${prompt.id}`;
    if (!beginAction(actionKey)) return null;
    try {
      const instruction = await promptDialog({
        title: t("video.actions.optimizeTitle"),
        message: t("video.actions.optimizeMessage"),
        placeholder: t("video.actions.optimizePlaceholder"),
        multiline: true,
        required: true,
        confirmText: t("video.actions.startOptimize"),
      });
      const trimmed = instruction?.trim();
      if (!trimmed) return null;
      const downstreamStrategy = await choosePromptMutationStrategy(
        prompt,
        t("common.update"),
      );
      if (downstreamStrategy == null || !isCurrentActionScope(requestScope)) {
        return null;
      }
      let refreshSucceeded = true;
      const updated = await runProgressTask(
        {
          title: t("video.chapter.optimizePrompt"),
          startMessage: t("video.actions.preparing", { operation: t("video.chapter.optimizePrompt") }),
          successMessage: t("video.actions.optimizeSuccess"),
          errorMessage: t("video.actions.optimizeFailed"),
          total: 3,
          runningHint: t("video.actions.optimizeRunningHint"),
        },
        async ({ setProgress }) => {
          setProgress(t("video.actions.optimizeProgressStart"), 1, 3);
          const result = await optimizeChapterVideoPrompt(prompt.id, {
            instruction: trimmed,
            downstream_strategy: downstreamStrategy,
          });
          if (!isCurrentActionScope(requestScope)) return result;
          setProgress(t("video.actions.optimizeProgressRefresh"), 2, 3);
          setPrompts((prev) =>
            prev.map((item) => (item.id === result.id ? result : item)),
          );
          if (selectedPromptId === result.id) {
            setDraft({
              visual_prompt: result.visual_prompt,
              motion_prompt: result.motion_prompt ?? "",
              negative_prompt: result.negative_prompt ?? "",
              duration_secs: result.duration_secs,
            });
          }
          refreshSucceeded = await loadAll();
          if (!refreshSucceeded) throw new Error(t("video.actions.optimizeFailed"));
          return result;
        },
      );
      if (!isCurrentActionScope(requestScope)) return null;
      if (!refreshSucceeded) return updated;
      addToast(t("video.actions.optimizeSuccess"), "success");
      return updated;
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return null;
      addToast(extractErrorMessage(err, t("video.actions.optimizeFailed")), "error");
      return null;
    } finally {
      finishAction(actionKey);
    }
  };

  const handleDeletePromptRow = async (prompt: ChapterVideoPrompt) => {
    if (prompt.status === "running") {
      addToast(t("video.actions.promptRunningCannot", { id: prompt.id, operation: t("common.delete") }), "error");
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
        t("common.delete"),
        downstreamCount,
      );
      if (strategy == null) {
        finishAction(actionKey);
        return;
      }
      downstreamStrategy = strategy;
    } else {
      const ok = await confirmDialog({
        title: t("video.chapter.deletePrompt"),
        message: t("common.deleteConfirm", { name: `${t("video.chapter.promptLabel")} #${prompt.id}` }),
        confirmText: t("common.delete"),
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
        await deleteChapterVideoPrompt(prompt.id, downstreamStrategy);
        if (!isCurrent()) return;
        if (selectedPromptId === prompt.id) setSelectedPromptId(null);
      },
      t("video.actions.promptDeleted"),
      true,
    );
  };

  const handleSavePromptRow = async (
    prompt: ChapterVideoPrompt,
    promptDraft: PromptDraft,
  ) => {
    if (prompt.status === "running") {
      addToast(t("video.actions.promptRunningCannot", { id: prompt.id, operation: t("common.save") }), "error");
      return null;
    }
    const visualPrompt = promptDraft.visual_prompt.trim();
    if (!visualPrompt) {
      addToast(t("video.actions.visualPromptEmpty"), "error");
      return null;
    }
    if (!isDurationAllowed(promptDraft.duration_secs, durationLimits)) {
      addToast(t("video.chapter.invalidDuration"), "error");
      return null;
    }
    const requestScope = actionScopeKey;
    const actionKey = `save-prompt-${prompt.id}`;
    if (!beginAction(actionKey)) return null;
    try {
      const downstreamStrategy = await choosePromptMutationStrategy(
        prompt,
        t("common.save"),
      );
      if (downstreamStrategy == null || !isCurrentActionScope(requestScope)) {
        return null;
      }
      const updated = await updateChapterVideoPrompt(prompt.id, {
        visual_prompt: visualPrompt,
        motion_prompt: nonEmptyText(promptDraft.motion_prompt),
        negative_prompt: nonEmptyText(promptDraft.negative_prompt),
        duration_secs: promptDraft.duration_secs,
        downstream_strategy: downstreamStrategy,
      });
      if (!isCurrentActionScope(requestScope)) return null;
      setPrompts((prev) =>
        prev.map((item) => (item.id === updated.id ? updated : item)),
      );
      if (selectedPromptId === updated.id) {
        setDraft({
          visual_prompt: updated.visual_prompt,
          motion_prompt: updated.motion_prompt ?? "",
          negative_prompt: updated.negative_prompt ?? "",
          duration_secs: updated.duration_secs,
        });
      }
      await loadAll();
      if (!isCurrentActionScope(requestScope)) return null;
      addToast(t("video.actions.promptSaved"), "success");
      return updated;
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return null;
      addToast(extractErrorMessage(err, t("errors.saveFailed")), "error");
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
          title: t("video.chapter.generateVideo"),
          startMessage: t("video.actions.preparing", { operation: t("video.chapter.generateVideo") }),
          successMessage: t("video.actions.videoGenerated"),
          errorMessage: t("video.actions.failed", { operation: t("video.chapter.generateVideo") }),
          cancelLabel: t("workflow.stop"),
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
      const message = extractErrorMessage(err, t("video.actions.failed", { operation: t("video.chapter.generateVideo") }));
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      try {
        const refreshed = await loadAll();
        if (generated && refreshed !== false && isCurrentActionScope(requestScope)) {
          addToast(t("video.actions.videoGenerated"), "success");
        }
      } finally {
        finishAction(actionKey);
      }
    }
  };

  const handleImportVideo = async (): Promise<void> => {
    if (!chapter || !selectedPrompt) return;
    if (selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE) {
      addToast(t("video.actions.promptTypeRequiredImport"), "error");
      return;
    }
    if (selectedPrompt.status === "running") {
      addToast(
        t("video.actions.promptRunningCannot", {
          id: selectedPrompt.id,
          operation: t("video.chapter.importVideo"),
        }),
        "error",
      );
      return;
    }
    if (selectedPromptStale) {
      addToast(staleVideoPromptMessage(frameStageLabel, t), "error");
      return;
    }
    if (selectedPromptDirty) {
      addToast(t("video.actions.unsavedBefore", { operation: t("video.chapter.importVideo") }), "error");
      return;
    }

    const requestScope = actionScopeKey;
    const actionKey = `import-video-${selectedPrompt.id}`;
    if (!beginAction(actionKey)) return;
    let upload: VideoUploadFile | null = null;
    try {
      upload = await pickVideoUploadFile(t);
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, t("video.chapter.filePickerFailed")), "error");
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
      await loadAll();
      if (!isCurrentActionScope(requestScope)) return;
      addToast(t("video.actions.videoImported"), "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, t("video.actions.videoImportFailed")), "error");
    } finally {
      finishAction(actionKey);
    }
  };

  const openVideoRequestPreviewData = async (
    loadPreview: () => Promise<ModelRequestPreview>,
    fallbackMessage: string,
  ) => {
    if (requestPreviewBusyScopeRef.current != null) return;
    const previewScope = requestPreviewScopeRef.current;
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
      t("video.actions.previewRequestFailed"),
    );
  };

  const handlePreviewGenerateVideoRequest = async () => {
    if (!selectedPrompt) return;
    if (selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE) {
      addToast(t("video.actions.promptTypeRequired"), "error");
      return;
    }
    const referenceState = readSavedReferenceImageIds(
      selectedPrompt.reference_image_ids_json,
      t,
    );
    if (referenceState.error) {
      addToast(t("video.actions.savedReferenceInvalid", {
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
        t("video.actions.finalReferencesInvalid", { ids: staleReferenceIds.join(", ") }),
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
    const blockedReason = videoProviderRequestPreviewBlockedReason(video, t);
    if (blockedReason) {
      addToast(blockedReason, "error");
      return;
    }
    if (!chapter) return;
    await openVideoRequestPreviewData(
      () =>
        previewVideoProviderRequestByVideoId(projectId, chapter.id, video.id),
      t("video.actions.previewHistoryFailed"),
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
      t("video.actions.videoStatusRefreshed"),
    );
  };

  const handleToggleVideoFavorite = async (video: GeneratedVideo) => {
    await wrapAct(
      `favorite-${video.id}`,
      () => updateVideo(video.id, { is_favorite: !video.is_favorite }),
      video.is_favorite ? t("video.actions.videoFavoriteRemoved") : t("video.actions.videoFavoriteAdded"),
    );
  };

  const handleUpdateVideo = async (
    video: GeneratedVideo,
    input: UpdateVideoInput,
  ) => {
    if (input.prompt != null && !input.prompt.trim()) {
      addToast(t("video.actions.videoPromptEmpty"), "error");
      throw new Error(t("video.actions.videoPromptEmpty"));
    }
    const requestScope = actionScopeKey;
    const actionKey = `update-video-${video.id}`;
    if (!beginAction(actionKey)) throw new Error(t("video.actions.videoActionBusy"));
    try {
      const row = await updateVideo(video.id, input);
      if (!isCurrentActionScope(requestScope)) return;
      setSelectedVideoId(row.id);
      upsertVideo(row);
      await loadAll();
      if (!isCurrentActionScope(requestScope)) return;
      addToast(t("video.actions.videoRecordSaved"), "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, t("video.actions.videoRecordSaveFailed")), "error");
      throw err;
    } finally {
      finishAction(actionKey);
    }
  };

  const handleDeleteVideo = async (video: GeneratedVideo) => {
    const actionKey = `delete-video-${video.id}`;
    if (!beginAction(actionKey)) return;
    const ok = await confirmDialog({
      title: t("video.actions.deleteVideoTitle"),
      message: t("video.actions.deleteVideoMessage", { id: video.id }),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!ok) {
      finishAction(actionKey);
      return;
    }
    await wrapAct(
      actionKey,
      async (isCurrent) => {
        await deleteVideo(video.id);
        if (!isCurrent()) return;
        if (pendingVideoId === video.id) setPendingVideoId(null);
      },
      t("video.actions.videoDeleted"),
      true,
    );
  };

  const handleTrim = async () => {
    if (!selectedVideo) return;
    const start = Number(trimStart);
    const end = Number(trimEnd);
    if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) {
      addToast(t("video.actions.trimInvalid"), "error");
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
      t("video.actions.trimmedVideoCreated"),
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
    setRequestPreviewOpen,
  };
}
