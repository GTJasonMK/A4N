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
import { promptTitle } from "./videoGenerationDisplay";
import {
  extractPromptSourceHash,
  hasPendingProviderJob,
  isDurationAllowed,
  isVideoFramePromptType,
  readSavedReferenceImageIds,
  staleVideoPromptMessage,
  videoExecutionPromptsShareGenerationChain,
  videoImageGenerationPromptId,
  videoImageGenerationSource,
  videoPromptSupportsProviderGeneration,
  videoProviderRequestPreviewBlockedReason,
  type PromptDraft,
  type VideoDurationLimits,
} from "./videoTabUtils";

type AddToast = (message: unknown, type: ToastType) => void;

const promptPreviewOverride = (
  draft: PromptDraft,
): PreviewChapterVideoPromptOverrideInput => ({
  visual_prompt: draft.visual_prompt,
  motion_prompt: nonEmptyText(draft.motion_prompt),
  negative_prompt: nonEmptyText(draft.negative_prompt),
  duration_secs: draft.duration_secs,
});

const videoPromptStage = (prompt: ChapterVideoPrompt): number | null => {
  if (prompt.prompt_type === "frame_segment_plan") return 1;
  if (isVideoFramePromptType(prompt.prompt_type)) return 2;
  if (prompt.prompt_type === "video_prompt") return 3;
  return null;
};

const frameImageBelongsToPrompt = (
  image: GeneratedImage,
  promptId: number,
): boolean => {
  const source = videoImageGenerationSource(image);
  return (
    (source === "video_boundary_frame_prompt" ||
      source === "video_first_frame_prompt" ||
      source === "video_storyboard_prompt") &&
    videoImageGenerationPromptId(image) === promptId
  );
};

const videoPromptDownstreamCount = (
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
    for (const promptId of affectedPromptIds) {
      if (frameImageBelongsToPrompt(image, promptId)) {
        affectedImageIds.add(image.id);
        break;
      }
    }
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
  addToast: AddToast;
  runProgressTask: RunProgressTask;
  loadAll: () => Promise<void>;
  setPrompts: Dispatch<SetStateAction<ChapterVideoPrompt[]>>;
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
  addToast,
  runProgressTask,
  loadAll,
  setPrompts,
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
  const [trimEnd, setTrimEnd] = useState("3");

  const videoPromptGenerateBlockedReason = (
    prompt: ChapterVideoPrompt,
  ): string | null => {
    if (prompt.prompt_type !== "video_prompt")
      return "请选择 video_prompt 类型的提示词";
    if (!videoPromptSupportsProviderGeneration(prompt)) {
      return "Chat 生视频模式不使用应用内 provider 生成，请复制提示词到外部 Chat 后导入视频";
    }
    if (prompt.status === "running") return `视频提示词 #${prompt.id} 正在运行`;
    const promptSourceHash = extractPromptSourceHash(prompt.metadata_json);
    if (
      promptSourceHash &&
      currentContentHash &&
      promptSourceHash !== currentContentHash
    ) {
      return staleVideoPromptMessage(frameStageLabel);
    }
    const referenceState = readSavedReferenceImageIds(
      prompt.reference_image_ids_json,
    );
    if (referenceState.error) {
      return `视频提示词 #${prompt.id} 保存的参考图数据异常：${referenceState.error}`;
    }
    const staleReferenceIds = referenceState.ids.filter(
      (id) => !validPromptReferenceIds.has(id),
    );
    if (staleReferenceIds.length > 0) {
      return `视频提示词 #${prompt.id} 的冻结参考图已失效：${staleReferenceIds.join(", ")}；请修改当前片段执行方案并重新生成最终提示词`;
    }
    if (selectedPromptDirty && prompt.id === selectedPrompt?.id) {
      return "当前视频提示词有未保存编辑，请先保存后再生成视频";
    }
    if (!isDurationAllowed(prompt.duration_secs, durationLimits)) {
      return `视频提示词 #${prompt.id} 时长不在默认视频模型支持范围内`;
    }
    return null;
  };
  const choosePromptMutationStrategy = async (
    prompt: ChapterVideoPrompt,
    operation: string,
  ): Promise<DownstreamConflictStrategy | null> => {
    const downstreamCount = videoPromptDownstreamCount(
      prompt,
      prompts,
      frameImages,
    );
    const strategy = await chooseDownstreamConflictStrategy({
      title: "处理下游派生产物",
      assetLabel: `${promptTitle(prompt)} #${prompt.id}`,
      downstreamLabel: "下游派生产物",
      downstreamCount,
      operation,
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
    setTrimEnd("3");
  }, [previewScopeKey]);

  const wrapAct = async (
    key: string,
    fn: (isCurrent: () => boolean) => Promise<unknown>,
    ok: string,
    alreadyStarted = false,
  ) => {
    const requestScope = actionScopeKey;
    if (!alreadyStarted && !beginAction(key)) return;
    try {
      await fn(() => isCurrentActionScope(requestScope));
      if (!isCurrentActionScope(requestScope)) return;
      addToast(ok, "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, "操作失败"), "error");
    } finally {
      finishAction(key);
    }
  };

  const handleOptimizePromptRow = async (prompt: ChapterVideoPrompt) => {
    if (prompt.status === "running") {
      addToast(`提示词 #${prompt.id} 正在运行，不能优化`, "error");
      return null;
    }
    const requestScope = actionScopeKey;
    const actionKey = `optimize-prompt-${prompt.id}`;
    if (!beginAction(actionKey)) return null;
    try {
      const instruction = await promptDialog({
        title: "优化文字资产",
        message:
          "输入你希望如何优化这条提示词。有下游派生产物时，下一步会让你选择删除下游或保留下游继续。",
        placeholder:
          "例如：保留剧情和参考图约束，增强空间关系描述，压缩重复形容词…",
        multiline: true,
        required: true,
        confirmText: "开始优化",
      });
      const trimmed = instruction?.trim();
      if (!trimmed) return null;
      const downstreamStrategy = await choosePromptMutationStrategy(
        prompt,
        "优化",
      );
      if (downstreamStrategy == null || !isCurrentActionScope(requestScope)) {
        return null;
      }
      const updated = await runProgressTask(
        {
          title: "优化提示词",
          startMessage: "正在准备提示词优化",
          successMessage: "提示词优化完成",
          errorMessage: "优化提示词失败",
          total: 3,
          runningHint: "模型正在重写提示词，完成后会自动更新当前内容",
        },
        async ({ setProgress }) => {
          setProgress("正在调用模型并校验优化结果", 1, 3);
          const result = await optimizeChapterVideoPrompt(prompt.id, {
            instruction: trimmed,
            downstream_strategy: downstreamStrategy,
          });
          if (!isCurrentActionScope(requestScope)) return result;
          setProgress("正在刷新优化后的提示词", 2, 3);
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
          await loadAll();
          return result;
        },
      );
      if (!isCurrentActionScope(requestScope)) return null;
      addToast("提示词已优化", "success");
      return updated;
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return null;
      addToast(extractErrorMessage(err, "优化提示词失败"), "error");
      return null;
    } finally {
      finishAction(actionKey);
    }
  };

  const handleDeletePromptRow = async (prompt: ChapterVideoPrompt) => {
    if (prompt.status === "running") {
      addToast(`提示词 #${prompt.id} 正在运行，不能删除`, "error");
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
      const strategy = await choosePromptMutationStrategy(prompt, "删除");
      if (strategy == null) {
        finishAction(actionKey);
        return;
      }
      downstreamStrategy = strategy;
    } else {
      const ok = await confirmDialog({
        title: "删除视频提示词",
        message: `删除提示词 #${prompt.id}？`,
        confirmText: "删除",
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
      "提示词已删除",
      true,
    );
  };

  const handleSavePromptRow = async (
    prompt: ChapterVideoPrompt,
    promptDraft: PromptDraft,
  ) => {
    if (prompt.status === "running") {
      addToast(`提示词 #${prompt.id} 正在运行，不能保存`, "error");
      return null;
    }
    const visualPrompt = promptDraft.visual_prompt.trim();
    if (!visualPrompt) {
      addToast("Visual Prompt 不能为空", "error");
      return null;
    }
    if (!isDurationAllowed(promptDraft.duration_secs, durationLimits)) {
      addToast("视频时长不在当前视频模型支持范围内", "error");
      return null;
    }
    const requestScope = actionScopeKey;
    const actionKey = `save-prompt-${prompt.id}`;
    if (!beginAction(actionKey)) return null;
    try {
      const downstreamStrategy = await choosePromptMutationStrategy(
        prompt,
        "保存",
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
      addToast("提示词已保存", "success");
      await loadAll();
      return updated;
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return null;
      addToast(extractErrorMessage(err, "保存提示词失败"), "error");
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
          title: "生成视频",
          startMessage: "正在准备视频生成",
          successMessage: "视频生成完成",
          errorMessage: "视频生成失败",
          total: 7,
          cancelLabel: "停止生成",
        },
        async (task) => {
          const [row] = await verifyVideoGenerationResult(
            task,
            projectId,
            chapter.id,
          );
          if (!row) throw new Error("视频生成已结束，但没有正式视频结果");
          if (!isCurrentActionScope(requestScope)) return;
          setSelectedVideoId(row.id);
          setPendingVideoId(null);
        },
      );
      if (!isCurrentActionScope(requestScope)) return;
      addToast("视频生成完成", "success");
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      const message = extractErrorMessage(err, "视频生成失败");
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      try {
        await loadAll();
      } finally {
        finishAction(actionKey);
      }
    }
  };

  const handleImportVideo = async (): Promise<void> => {
    if (!chapter || !selectedPrompt) return;
    if (selectedPrompt.prompt_type !== "video_prompt") {
      addToast("请选择 video_prompt 类型的提示词导入视频", "error");
      return;
    }
    if (selectedPrompt.status === "running") {
      addToast(
        `视频提示词 #${selectedPrompt.id} 正在运行，不能导入视频`,
        "error",
      );
      return;
    }
    if (selectedPromptStale) {
      addToast(staleVideoPromptMessage(frameStageLabel), "error");
      return;
    }
    if (selectedPromptDirty) {
      addToast("当前视频提示词有未保存编辑，请先保存后再导入视频", "error");
      return;
    }

    const requestScope = actionScopeKey;
    const actionKey = `import-video-${selectedPrompt.id}`;
    if (!beginAction(actionKey)) return;
    let upload: VideoUploadFile | null = null;
    try {
      upload = await pickVideoUploadFile();
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, "打开文件选择器失败"), "error");
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
      addToast("视频已导入并关联当前提示词", "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, "导入视频失败"), "error");
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
      "预览视频请求失败",
    );
  };

  const handlePreviewGenerateVideoRequest = async () => {
    if (!selectedPrompt) return;
    if (selectedPrompt.prompt_type !== "video_prompt") {
      addToast("请选择 video_prompt 类型的提示词", "error");
      return;
    }
    const referenceState = readSavedReferenceImageIds(
      selectedPrompt.reference_image_ids_json,
    );
    if (referenceState.error) {
      addToast(`保存的参考图数据异常：${referenceState.error}`, "error");
      return;
    }
    const staleReferenceIds = referenceState.ids.filter(
      (id) => !validPromptReferenceIds.has(id),
    );
    if (staleReferenceIds.length > 0) {
      addToast(
        `最终提示词的冻结参考图已失效：${staleReferenceIds.join(", ")}；请修改片段执行方案并重新生成最终提示词`,
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
    const blockedReason = videoProviderRequestPreviewBlockedReason(video);
    if (blockedReason) {
      addToast(blockedReason, "error");
      return;
    }
    if (!chapter) return;
    await openVideoRequestPreviewData(
      () =>
        previewVideoProviderRequestByVideoId(projectId, chapter.id, video.id),
      "预览历史视频请求失败",
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
      },
      "视频状态已刷新",
    );
  };

  const handleToggleVideoFavorite = async (video: GeneratedVideo) => {
    await wrapAct(
      `favorite-${video.id}`,
      () => updateVideo(video.id, { is_favorite: !video.is_favorite }),
      video.is_favorite ? "已取消收藏" : "已收藏",
    );
  };

  const handleUpdateVideo = async (
    video: GeneratedVideo,
    input: UpdateVideoInput,
  ) => {
    if (input.prompt != null && !input.prompt.trim()) {
      addToast("视频 Prompt 不能为空", "error");
      throw new Error("视频 Prompt 不能为空");
    }
    const requestScope = actionScopeKey;
    const actionKey = `update-video-${video.id}`;
    if (!beginAction(actionKey)) throw new Error("已有视频操作正在进行");
    try {
      const row = await updateVideo(video.id, input);
      if (!isCurrentActionScope(requestScope)) return;
      setSelectedVideoId(row.id);
      addToast("视频记录已保存", "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, "保存视频记录失败"), "error");
      throw err;
    } finally {
      finishAction(actionKey);
    }
  };

  const handleDeleteVideo = async (video: GeneratedVideo) => {
    const actionKey = `delete-video-${video.id}`;
    if (!beginAction(actionKey)) return;
    const ok = await confirmDialog({
      title: "删除视频",
      message: `删除视频 #${video.id}？`,
      confirmText: "删除",
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
      "已删除",
      true,
    );
  };

  const handleTrim = async () => {
    if (!selectedVideo) return;
    const start = Number(trimStart);
    const end = Number(trimEnd);
    if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) {
      addToast("裁剪时间范围无效", "error");
      return;
    }
    await wrapAct(
      `trim-${selectedVideo.id}`,
      async (isCurrent) => {
        const row = await trimVideo(selectedVideo.id, start, end);
        if (!isCurrent()) return;
        setSelectedVideoId(row.id);
      },
      "已生成裁剪片段",
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
