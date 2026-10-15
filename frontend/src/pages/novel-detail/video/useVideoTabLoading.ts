import { useCallback, useEffect, useLayoutEffect, useRef } from "react";
import { extractErrorMessage } from "../../../api";
import { generationClient } from "../../../api/generationTasks";
import {
  listChapterVideoPrompts,
  type ChapterVideoPrompt,
  type GeneratedVideo,
} from "../../../api/video";
import type { ChapterRow } from "../shared";
import { loadShootingScriptOrProgress } from "../loadShootingScriptData";
import {
  activeVideoSegmentPlanTask,
  loadVideoSegmentPlanDraft,
  loadVideoTabData,
} from "./loadVideoTabData";
import { completedSegmentPlanPreviewItems } from "./videoSegmentPlan";
import { hasPendingProviderJob } from "./videoOptionUtils";
import { isSegmentPlanDraftCheckpointKey } from "./videoFramePromptUtils";
import { isCancelledOperation } from "../../../utils/cancellation";
import {
  runVideoPromptGenerationTask,
} from "./useVideoPromptGenerationActions";
import { textInputValue } from "../../../utils/text";
import { DEFAULT_DRAFT } from "./videoTabConstants";
import type { MessageTranslator } from "../../../i18n";
import type { VideoTabState } from "./useVideoTabState";

export interface VideoTabLoadingOptions extends VideoTabState {
  projectId: number;
  chapter: ChapterRow | null;
  chapterId: number | null;
  videoDataKey: string | null;
  imageProjectIds?: number[];
  addToast: (message: string, type: "success" | "error" | "info") => void;
  runProgressTask: Parameters<typeof runVideoPromptGenerationTask>[0];
  t: MessageTranslator;
  finalVideoPrompts: ChapterVideoPrompt[];
  workflowVideos: GeneratedVideo[];
  selectedPrompt: ChapterVideoPrompt | null;
  selectedVideo: GeneratedVideo | null;
}

export interface LoadVideoTabDataOptions {
  showLoading?: boolean;
}

export type LoadVideoTabData = (options?: LoadVideoTabDataOptions) => Promise<boolean>;

export function useVideoTabLoading({
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
  ...state
}: VideoTabLoadingOptions) {
  const tRef = useRef(t);
  tRef.current = t;
  const {
    segmentPlanDraftLoadSeqRef,
    activeVideoDataKeyRef,
    promptLoadSeqRef,
    loadAllSeqRef,
    attachedSegmentPlanTaskRef,
    beginVideoAction,
    finishVideoAction,
    selectedVideoSelectionScopeRef,
    selectedSegmentNumber,
    workflowMode,
    setSegmentPlanDraftLoading,
    setSegmentPlanDraftError,
    setSegmentPlanDraft,
    setSegmentPlanPreview,
    setLoading,
    setVideoDataError,
    setActiveVideoConfig,
    setVideos,
    setImages,
    setTimeline,
    setShootingScenes,
    setLoadedDataKey,
    setPrompts,
    setDraft,
    loadedDataKey,
    videoDataError,
    selectedPromptState,
    setSelectedPromptId,
    pendingVideoState,
    setPendingVideoId,
    selectedVideoState,
    segmentStorageKey,
    selectedSegmentRecipe,
    setSelectedVideoId,
    promptRequestPreviewScopeRef,
    imageRequestPreviewScopeRef,
    setPromptRequestPreviewOpen,
    setPromptRequestPreviewLoading,
    setPromptRequestPreviewData,
    setImageRequestPreviewOpen,
    setImageRequestPreviewLoading,
    setImageRequestPreviewData,
  } = state;
  const refreshSegmentPlanDraft = useCallback(async () => {
    if (chapterId == null || !videoDataKey) return null;
    const requestSeq = segmentPlanDraftLoadSeqRef.current + 1;
    const requestDataKey = videoDataKey;
    segmentPlanDraftLoadSeqRef.current = requestSeq;
    setSegmentPlanDraftLoading(true);
    setSegmentPlanDraftError(null);
    try {
      const nextDraft = await loadVideoSegmentPlanDraft(projectId, chapterId, tRef.current);
      if (
        segmentPlanDraftLoadSeqRef.current !== requestSeq ||
        activeVideoDataKeyRef.current !== requestDataKey
      ) {
        return null;
      }
      setSegmentPlanDraft(nextDraft);
      setSegmentPlanPreview(nextDraft?.segments ?? []);
      if (nextDraft?.resumeTaskError !== undefined) {
        setSegmentPlanDraftError(
          extractErrorMessage(nextDraft.resumeTaskError, tRef.current("video.chapter.loadSavedPlanFailed")),
        );
      }
      return nextDraft;
    } catch (err) {
      if (
        segmentPlanDraftLoadSeqRef.current === requestSeq &&
        activeVideoDataKeyRef.current === requestDataKey
      ) {
        setSegmentPlanDraftError(
          extractErrorMessage(err, tRef.current("video.chapter.loadSavedPlanFailed")),
        );
      }
      throw err;
    } finally {
      if (
        segmentPlanDraftLoadSeqRef.current === requestSeq &&
        activeVideoDataKeyRef.current === requestDataKey
      ) {
        setSegmentPlanDraftLoading(false);
      }
    }
  }, [chapterId, projectId, videoDataKey]);

  async function loadAll({ showLoading = true }: LoadVideoTabDataOptions = {}): Promise<boolean> {
    if (!chapter || !videoDataKey) return false;
    void refreshSegmentPlanDraft().catch(() => undefined);
    const requestSeq = loadAllSeqRef.current + 1;
    const requestDataKey = videoDataKey;
    const requestImageProjectIds = imageProjectIds ?? [projectId];
    const promptRequestSeq = promptLoadSeqRef.current + 1;
    loadAllSeqRef.current = requestSeq;
    promptLoadSeqRef.current = promptRequestSeq;
    setLoading(showLoading);
    setVideoDataError(null);
    try {
      const [videoDataResult, shootingScriptResult] = await Promise.allSettled([
        loadVideoTabData(projectId, chapter, requestImageProjectIds),
        loadShootingScriptOrProgress(projectId, chapter.id),
      ]);
      if (
        loadAllSeqRef.current !== requestSeq ||
        activeVideoDataKeyRef.current !== requestDataKey
      ) {
        return false;
      }
      const failures: string[] = [];
      if (videoDataResult.status === "fulfilled") {
        const data = videoDataResult.value;
        if (promptLoadSeqRef.current === promptRequestSeq) {
          setPrompts(data.prompts ?? []);
        }
        setVideos(data.videos ?? []);
        setImages(data.images ?? []);
        setActiveVideoConfig(data.activeVideoConfig ?? null);
        setTimeline(data.timeline ?? null);
        data.errors.forEach(({ reason }) => failures.push(extractErrorMessage(reason, tRef.current("video.chapter.loadFailed"))));
      } else {
        failures.push(extractErrorMessage(videoDataResult.reason, tRef.current("video.chapter.loadFailed")));
      }
      if (shootingScriptResult.status === "fulfilled") {
        setShootingScenes(
          shootingScriptResult.value.unitProgress?.script.scenes ??
            shootingScriptResult.value.view?.script.scenes ??
            [],
        );
        if (shootingScriptResult.value.scriptError !== undefined) {
          failures.push(
            extractErrorMessage(
              shootingScriptResult.value.scriptError,
              tRef.current("video.chapter.loadFailed"),
            ),
          );
        }
      } else {
          failures.push(extractErrorMessage(shootingScriptResult.reason, tRef.current("video.chapter.loadFailed")));
      }
      setLoadedDataKey(requestDataKey);
      const message = failures.length > 0 ? failures.join("；") : null;
      setVideoDataError(message);
      return failures.length === 0;
    } finally {
      if (
        loadAllSeqRef.current === requestSeq &&
        activeVideoDataKeyRef.current === requestDataKey
      ) {
        setLoading(false);
      }
    }
  }

  const refreshPrompts = useCallback(async () => {
    if (chapterId == null || !videoDataKey) return;
    const requestDataKey = videoDataKey;
    const requestSeq = promptLoadSeqRef.current + 1;
    promptLoadSeqRef.current = requestSeq;
    const rows = await listChapterVideoPrompts(projectId, chapterId);
    if (
      promptLoadSeqRef.current !== requestSeq ||
      activeVideoDataKeyRef.current !== requestDataKey
    ) {
      return;
    }
    setPrompts(rows);
  }, [chapterId, projectId, videoDataKey]);

  useLayoutEffect(() => {
    activeVideoDataKeyRef.current = videoDataKey;
    promptRequestPreviewScopeRef.current += 1;
    imageRequestPreviewScopeRef.current += 1;
    setPromptRequestPreviewOpen(false);
    setPromptRequestPreviewLoading(false);
    setPromptRequestPreviewData(null);
    setImageRequestPreviewOpen(false);
    setImageRequestPreviewLoading(false);
    setImageRequestPreviewData(null);
    loadAllSeqRef.current += 1;
    promptLoadSeqRef.current += 1;
    segmentPlanDraftLoadSeqRef.current += 1;
    setPrompts([]);
    setSegmentPlanPreview([]);
    setSegmentPlanDraft(null);
    setSegmentPlanDraftLoading(false);
    setSegmentPlanDraftError(null);
    setVideos([]);
    setImages([]);
    setTimeline(null);
    setShootingScenes([]);
    setActiveVideoConfig(null);
    setLoadedDataKey(null);
    setVideoDataError(null);
    setDraft(DEFAULT_DRAFT);
    setLoading(false);
    attachedSegmentPlanTaskRef.current = null;
  }, [videoDataKey]);

  useEffect(() => {
    if (!videoDataKey || loadedDataKey === videoDataKey || videoDataError) {
      return;
    }
    void loadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loadedDataKey, videoDataError, videoDataKey]);

  useEffect(() => {
    if (!videoDataKey || loadedDataKey !== videoDataKey) return;
    const refreshWhenVisible = () => {
      if (document.visibilityState === "visible") void loadAll({ showLoading: false });
    };
    window.addEventListener("focus", refreshWhenVisible);
    document.addEventListener("visibilitychange", refreshWhenVisible);
    return () => {
      window.removeEventListener("focus", refreshWhenVisible);
      document.removeEventListener("visibilitychange", refreshWhenVisible);
    };
    // loadAll rejects responses from an obsolete chapter through its request sequence.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loadedDataKey, videoDataKey]);

  useEffect(() => {
    if (chapterId == null || !videoDataKey || loadedDataKey !== videoDataKey)
      return;
    const requestDataKey = videoDataKey;
    let disposed = false;
    void generationClient
      .listActive(projectId)
      .then(async (tasks) => {
        if (disposed || activeVideoDataKeyRef.current !== requestDataKey)
          return;
        const active = activeVideoSegmentPlanTask(tasks, projectId, chapterId);
        if (!active) return;
        const taskKey = `${active.handle.task_id}:${active.handle.attempt}`;
        if (attachedSegmentPlanTaskRef.current === taskKey) return;
        const actionKey = "generate-segment-plan";
        if (!beginVideoAction(actionKey)) return;
        attachedSegmentPlanTaskRef.current = taskKey;
        let previewFingerprint = "";
        try {
          await runVideoPromptGenerationTask(
            runProgressTask,
            active.handle,
            {
              title: tRef.current("video.actions.segmentPlanResumeTitle"),
              startMessage: tRef.current("video.actions.segmentPlanResumeStart"),
              successMessage: tRef.current("video.actions.segmentPlanCompleted"),
              errorMessage: tRef.current("video.actions.segmentPlanFailed"),
            },
            refreshPrompts,
            (streamText, checkpointKey) => {
              if (
                disposed ||
                activeVideoDataKeyRef.current !== requestDataKey ||
                !isSegmentPlanDraftCheckpointKey(checkpointKey)
              ) {
                return;
              }
              const preview = completedSegmentPlanPreviewItems(streamText);
              const fingerprint = JSON.stringify(preview);
              if (fingerprint === previewFingerprint) return;
              previewFingerprint = fingerprint;
              setSegmentPlanPreview(preview);
              void refreshSegmentPlanDraft().catch(() => undefined);
            },
            (error) => {
              if (!disposed && activeVideoDataKeyRef.current === requestDataKey) {
                addToast(extractErrorMessage(error, tRef.current("video.chapter.refreshVideoDataFailed")), "error");
              }
            },
          );
        } catch (error) {
          if (
            !disposed &&
            activeVideoDataKeyRef.current === requestDataKey &&
            !isCancelledOperation(extractErrorMessage(error, tRef.current("video.actions.segmentPlanFailed")))
          ) {
            addToast(extractErrorMessage(error, tRef.current("video.actions.segmentPlanFailed")), "error");
          }
        } finally {
          if (attachedSegmentPlanTaskRef.current === taskKey) {
            attachedSegmentPlanTaskRef.current = null;
          }
          if (!disposed && activeVideoDataKeyRef.current === requestDataKey) {
            void refreshSegmentPlanDraft().catch(() => undefined);
          }
          finishVideoAction(actionKey, requestDataKey);
        }
      })
      .catch((error) => {
        if (!disposed && activeVideoDataKeyRef.current === requestDataKey) {
          setSegmentPlanDraftError(
            extractErrorMessage(error, tRef.current("video.chapter.resumePlanFailed")),
          );
        }
      });
    return () => {
      disposed = true;
    };
  }, [
    addToast,
    beginVideoAction,
    chapterId,
    finishVideoAction,
    loadedDataKey,
    projectId,
    refreshSegmentPlanDraft,
    refreshPrompts,
    runProgressTask,
    videoDataKey,
  ]);

  useEffect(() => {
    if (selectedPromptState.storageError || loadedDataKey !== videoDataKey)
      return;
    setSelectedPromptId((current) => {
      if (
        current &&
        finalVideoPrompts.some((prompt) => prompt.id === current)
      ) {
        return current;
      }
      return finalVideoPrompts[finalVideoPrompts.length - 1]?.id ?? null;
    });
  }, [
    finalVideoPrompts,
    loadedDataKey,
    selectedPromptState.storageError,
    setSelectedPromptId,
    videoDataKey,
  ]);

  useEffect(() => {
    if (loadedDataKey !== videoDataKey) return;
    const pending = workflowVideos.find(hasPendingProviderJob) ?? null;
    if (!pendingVideoState.storageError) {
      setPendingVideoId((current) => {
        const currentPending = current
          ? workflowVideos.find(
              (video) => video.id === current && hasPendingProviderJob(video),
            )
          : null;
        return currentPending?.id ?? pending?.id ?? null;
      });
    }
    if (!selectedVideoState.storageError) {
      const selectionScopeChanged =
        selectedVideoSelectionScopeRef.current !== segmentStorageKey;
      const adoptedVideoId = selectedSegmentRecipe?.adopted_video_id ?? null;
      const availableAdoptedVideoId =
        adoptedVideoId != null &&
        workflowVideos.some((video) => video.id === adoptedVideoId)
          ? adoptedVideoId
          : null;
      if (
        selectionScopeChanged &&
        (adoptedVideoId == null ||
          availableAdoptedVideoId != null ||
          workflowVideos.length > 0)
      ) {
        selectedVideoSelectionScopeRef.current = segmentStorageKey;
      }
      setSelectedVideoId((current) => {
        if (selectionScopeChanged && availableAdoptedVideoId != null) {
          return availableAdoptedVideoId;
        }
        if (current && workflowVideos.some((video) => video.id === current)) {
          return current;
        }
        return selectedVideo?.id ?? null;
      });
    }
  }, [
    loadedDataKey,
    pendingVideoState.storageError,
    segmentStorageKey,
    selectedSegmentRecipe?.adopted_video_id,
    selectedVideoState.storageError,
    selectedVideo?.id,
    setPendingVideoId,
    setSelectedVideoId,
    videoDataKey,
    workflowVideos,
  ]);

  useLayoutEffect(() => {
    promptRequestPreviewScopeRef.current += 1;
    imageRequestPreviewScopeRef.current += 1;
    setPromptRequestPreviewOpen(false);
    setPromptRequestPreviewLoading(false);
    setPromptRequestPreviewData(null);
    setImageRequestPreviewOpen(false);
    setImageRequestPreviewLoading(false);
    setImageRequestPreviewData(null);
    return () => {
      promptRequestPreviewScopeRef.current += 1;
      imageRequestPreviewScopeRef.current += 1;
    };
  }, [selectedSegmentNumber, workflowMode]);

  useEffect(() => {
    if (!selectedPrompt) {
      setDraft(DEFAULT_DRAFT);
      return;
    }
    setDraft({
      visual_prompt: selectedPrompt.visual_prompt,
      motion_prompt: textInputValue(selectedPrompt.motion_prompt),
      negative_prompt: textInputValue(selectedPrompt.negative_prompt),
      duration_secs: selectedPrompt.duration_secs,
    });
    // 只在切换提示词时重置文字草稿，避免刷新素材/视频状态时覆盖未保存编辑。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedPrompt?.id]);


  return { loadAll, refreshPrompts, refreshSegmentPlanDraft };
}
