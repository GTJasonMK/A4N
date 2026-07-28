import { useMemo } from "react";
import type { GeneratedImage } from "../../../api/image";
import type {
  ChapterVideoPrompt,
  GeneratedVideo,
  VideoGenConfig,
  PromptOutputLanguage,
} from "../../../api/video";
import type { ChapterRow } from "../shared";
import {
  type ChapterVideoWorkflowMode,
  extractPromptSourceHash,
  boundaryFramePromptsAreComplete,
  boundaryFrameStageMessage,
  expectedFrameCountFromSegmentPlanPrompt,
  firstNonEmptyText,
  isSelectableReferenceImage,
  isVideoLibraryVisibleImage,
  promptHasVideoWorkflowContract,
  finalVideoExecutionIdentityIsComplete,
  promptOutputLanguageOfPrompt,
  promptSourcePromptType,
  readSavedReferenceImageIds,
  trimmedText,
  type ChapterAnalysisSummary,
  type VideoDurationLimits,
  videoImageGenerationPromptId,
  videoImageGenerationSource,
  videoDurationLimitsFromConfig,
  videoWorkflowModeOfPrompt,
  videoExecutionIdentityMatches,
  videoExecutionIdentityOfPrompt,
  videoWorkflowModeOfVideo,
  VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
  VIDEO_CHAT_VIDEO_CONTRACT_VERSION,
  VIDEO_FINAL_PROMPT_TYPE,
  VIDEO_CHAT_FRAME_PROMPT_TYPE,
  VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION,
  VIDEO_FIRST_FRAME_PROMPT_TYPE,
  VIDEO_FRAME_PROMPT_TYPE,
  VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
  VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION,
  VIDEO_STORYBOARD_PROMPT_TYPE,
  VIDEO_STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
} from "./videoTabUtils";

interface UseVideoTabDerivedStateParams {
  prompts: ChapterVideoPrompt[];
  videos: GeneratedVideo[];
  images: GeneratedImage[];
  chapter: ChapterRow | null;
  selectedPromptId: number | null;
  selectedVideoId: number | null;
  pendingVideoId: number | null;
  promptReferenceIds: number[];
  videoLibraryImageIds: number[];
  workflowMode: ChapterVideoWorkflowMode;
  promptOutputLanguage: PromptOutputLanguage;
  currentContentHash: string | null;
  activeVideoConfig: VideoGenConfig | null;
}

export const useVideoTabDerivedState = ({
  prompts,
  videos,
  images,
  chapter,
  selectedPromptId,
  selectedVideoId,
  pendingVideoId,
  promptReferenceIds,
  videoLibraryImageIds,
  workflowMode,
  promptOutputLanguage,
  currentContentHash,
  activeVideoConfig,
}: UseVideoTabDerivedStateParams) => {
  const frameSegmentPlanType = VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE;
  const framePromptType =
    workflowMode === "chat_video"
      ? VIDEO_CHAT_FRAME_PROMPT_TYPE
      : workflowMode === "storyboard"
      ? VIDEO_STORYBOARD_PROMPT_TYPE
      : workflowMode === "first_frame_chain"
      ? VIDEO_FIRST_FRAME_PROMPT_TYPE
      : workflowMode === "boundary_frames"
        ? VIDEO_FRAME_PROMPT_TYPE
        : null;
  const frameSegmentPlanContractVersion = VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION;
  const framePromptContractVersion =
    workflowMode === "chat_video"
      ? VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION
      : workflowMode === "storyboard"
      ? VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION
      : workflowMode === "first_frame_chain"
      ? VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION
      : workflowMode === "boundary_frames"
        ? VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION
        : null;
  const finalVideoPromptContractVersion =
    workflowMode === "chat_video"
      ? VIDEO_CHAT_VIDEO_CONTRACT_VERSION
      : workflowMode === "storyboard"
      ? VIDEO_STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION
      : workflowMode === "first_frame_chain"
      ? VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION
      : workflowMode === "boundary_frames"
        ? VIDEO_FRAME_SEGMENT_PLAN_CONTRACT_VERSION
        : null;
  const frameSegmentPlanPrompt = useMemo(() => {
    let latest: ChapterVideoPrompt | null = null;
    for (const prompt of prompts) {
      if (
        prompt.prompt_type === frameSegmentPlanType &&
        prompt.workflow_mode === "shared_segment_plan" &&
        promptHasVideoWorkflowContract(prompt, frameSegmentPlanContractVersion) &&
        videoExecutionIdentityOfPrompt(prompt) != null &&
        (latest == null || prompt.id > latest.id)
      ) {
        latest = prompt;
      }
    }
    return latest;
  }, [prompts]);
  const videoPromptSourceType = VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE;
  const finalVideoPrompts = useMemo(
    () =>
      prompts
        .filter(
          (prompt) =>
            prompt.prompt_type === VIDEO_FINAL_PROMPT_TYPE &&
            finalVideoPromptContractVersion != null &&
            videoWorkflowModeOfPrompt(prompt) === workflowMode &&
            promptHasVideoWorkflowContract(prompt, finalVideoPromptContractVersion) &&
            promptSourcePromptType(prompt) === videoPromptSourceType &&
            videoExecutionIdentityMatches(prompt, frameSegmentPlanPrompt) &&
            finalVideoExecutionIdentityIsComplete(prompt),
        )
        .sort((a, b) => a.shot_number - b.shot_number || a.id - b.id),
    [
      finalVideoPromptContractVersion,
      prompts,
      videoPromptSourceType,
      workflowMode,
      frameSegmentPlanPrompt,
    ],
  );
  const currentLanguageFinalVideoPrompts = useMemo(
    () =>
      finalVideoPrompts.filter(
        (prompt) => promptOutputLanguageOfPrompt(prompt) === promptOutputLanguage,
      ),
    [finalVideoPrompts, promptOutputLanguage],
  );
  const selectedPrompt = useMemo(
    () => finalVideoPrompts.find((prompt) => prompt.id === selectedPromptId) ?? null,
    [finalVideoPrompts, selectedPromptId],
  );
  const framePrompts = useMemo(() => {
    return prompts
      .filter(
        (prompt) =>
          framePromptType != null &&
          framePromptContractVersion != null &&
          prompt.prompt_type === framePromptType &&
          videoWorkflowModeOfPrompt(prompt) === workflowMode &&
          (workflowMode !== "first_frame_chain" || prompt.shot_number === 1) &&
          promptHasVideoWorkflowContract(prompt, framePromptContractVersion) &&
          promptSourcePromptType(prompt) === frameSegmentPlanType &&
          videoExecutionIdentityMatches(prompt, frameSegmentPlanPrompt),
      )
      .sort((a, b) => a.shot_number - b.shot_number || a.id - b.id);
  }, [
    framePromptContractVersion,
    framePromptType,
    frameSegmentPlanType,
    prompts,
    workflowMode,
    frameSegmentPlanPrompt,
  ]);
  const currentLanguageFramePrompts = useMemo(
    () =>
      framePrompts.filter(
        (prompt) => promptOutputLanguageOfPrompt(prompt) === promptOutputLanguage,
      ),
    [framePrompts, promptOutputLanguage],
  );
  const expectedBoundaryFrameCount = useMemo(
    () =>
      framePromptType == null
        ? null
        : expectedFrameCountFromSegmentPlanPrompt(frameSegmentPlanPrompt, workflowMode),
    [framePromptType, frameSegmentPlanPrompt, workflowMode],
  );
  const framePromptStageReady = useMemo(
    () =>
      boundaryFramePromptsAreComplete(
        currentLanguageFramePrompts,
        expectedBoundaryFrameCount,
      ),
    [currentLanguageFramePrompts, expectedBoundaryFrameCount],
  );
  const framePromptStageMessage = useMemo(() => {
    if (framePromptStageReady) return null;
    return (
      boundaryFrameStageMessage(
        frameSegmentPlanPrompt,
        currentLanguageFramePrompts.length,
        expectedBoundaryFrameCount,
        workflowMode,
      ) ??
      (workflowMode === "first_frame_chain"
        ? "当前章节首帧提示词编号不连续，请重新生成完整首帧提示词"
        : workflowMode === "chat_video"
          ? "当前章节 Chat 首尾帧提示词编号不连续，请重新生成完整 Chat 首尾帧提示词"
        : workflowMode === "storyboard"
          ? "当前章节分镜提示词编号不连续，请重新生成完整分镜提示词"
          : "当前章节边界帧提示词编号不连续，请重新生成完整边界帧提示词")
    );
  }, [
    expectedBoundaryFrameCount,
    currentLanguageFramePrompts.length,
    framePromptStageReady,
    frameSegmentPlanPrompt,
    workflowMode,
  ]);
  const selectedPromptReferenceState = useMemo(
    () => readSavedReferenceImageIds(selectedPrompt?.reference_image_ids_json),
    [selectedPrompt?.reference_image_ids_json],
  );
  const selectedPromptReferenceIds = selectedPromptReferenceState.ids;
  const savedReferenceIdsError = selectedPromptReferenceState.error;
  const { workflowAssemblyVideos, workflowVideos, selectedVideo, pendingVideo } =
    useMemo(() => {
      const assemblyRows: GeneratedVideo[] = [];
      const workflowRows: GeneratedVideo[] = [];
      let selected: GeneratedVideo | null = null;
      let pending: GeneratedVideo | null = null;
      for (const video of videos) {
        if (videoWorkflowModeOfVideo(video) !== workflowMode) continue;
        assemblyRows.push(video);
        if (
          video.video_type === "assembly_insert" ||
          video.video_type === "assembly_push" ||
          video.video_type === "assembly_postprocess_edit"
        ) {
          continue;
        }
        if (workflowRows.length === 0) selected = video;
        workflowRows.push(video);
        if (video.id === selectedVideoId) selected = video;
        if (video.id === pendingVideoId) pending = video;
      }
      return {
        workflowAssemblyVideos: assemblyRows,
        workflowVideos: workflowRows,
        selectedVideo: selected,
        pendingVideo: pending,
      };
    }, [pendingVideoId, selectedVideoId, videos, workflowMode]);
  const selectedVideoPath = useMemo(
    () => trimmedText(selectedVideo?.file_path),
    [selectedVideo?.file_path],
  );
  const selectedPromptSceneSummary = trimmedText(selectedPrompt?.scene_summary);
  const chapterAnalysis = useMemo(
    () => (chapter?.analysis_data ?? null) as ChapterAnalysisSummary | null,
    [chapter?.analysis_data],
  );
  const chapterAnalysisSummary = firstNonEmptyText(
    chapterAnalysis?.summaries?.compressed,
    chapterAnalysis?.summaries?.one_line,
  );
  const promptSourceHash = useMemo(
    () => extractPromptSourceHash(selectedPrompt?.metadata_json),
    [selectedPrompt?.metadata_json],
  );
  const promptContentStale = Boolean(
    promptSourceHash && currentContentHash && promptSourceHash !== currentContentHash,
  );
  const selectedPromptStale = promptContentStale;
  const selectedPromptRunning = selectedPrompt?.status === "running";
  const framePromptIdSet = useMemo(
    () => new Set(framePrompts.map((prompt) => prompt.id)),
    [framePrompts],
  );
  const finalVideoPromptIdSet = useMemo(
    () => new Set(finalVideoPrompts.map((prompt) => prompt.id)),
    [finalVideoPrompts],
  );
  const { videoFrameImages, systemPromptReferenceIds } = useMemo(() => {
    const rows: GeneratedImage[] = [];
    const ids = new Set<number>();
    for (const image of images) {
      const source = videoImageGenerationSource(image);
      const promptId = videoImageGenerationPromptId(image);
      let matches = false;
      if (
        (source === "video_boundary_frame_prompt" ||
          source === "video_first_frame_prompt" ||
          source === "video_storyboard_prompt") &&
        promptId != null
      ) {
        matches = framePromptIdSet.has(promptId);
      } else if (source === "video_tail_frame_capture" && workflowMode !== "storyboard") {
        matches = promptId != null && finalVideoPromptIdSet.has(promptId);
      }
      if (!matches) continue;
      rows.push(image);
      ids.add(image.id);
    }
    return { videoFrameImages: rows, systemPromptReferenceIds: ids };
  }, [finalVideoPromptIdSet, framePromptIdSet, images, workflowMode]);
  const videoLibraryImageIdSet = useMemo(
    () => new Set(videoLibraryImageIds),
    [videoLibraryImageIds],
  );
  const {
    selectableReferenceImages,
    selectableReferenceIds,
    videoLibraryImages,
  } = useMemo(() => {
    const selectableRows: GeneratedImage[] = [];
    const selectableIds = new Set<number>();
    const libraryRows: GeneratedImage[] = [];
    for (const image of images) {
      if (isSelectableReferenceImage(image, chapter, currentContentHash)) {
        selectableRows.push(image);
        selectableIds.add(image.id);
      }
      if (isVideoLibraryVisibleImage(image, chapter, videoLibraryImageIdSet)) {
        libraryRows.push(image);
      }
    }
    return {
      selectableReferenceImages: selectableRows,
      selectableReferenceIds: selectableIds,
      videoLibraryImages: libraryRows,
    };
  }, [chapter, currentContentHash, images, videoLibraryImageIdSet]);
  const validPromptReferenceIds = useMemo(() => {
    const ids = new Set(selectableReferenceIds);
    for (const id of systemPromptReferenceIds) ids.add(id);
    return ids;
  }, [selectableReferenceIds, systemPromptReferenceIds]);
  const staleSavedReferenceIds = useMemo(
    () => selectedPromptReferenceIds.filter((id) => !validPromptReferenceIds.has(id)),
    [selectedPromptReferenceIds, validPromptReferenceIds],
  );
  const staleSavedReferenceCount = staleSavedReferenceIds.length;
  const imageById = useMemo(() => {
    const map = new Map<number, GeneratedImage>();
    for (const image of images) map.set(image.id, image);
    return map;
  }, [images]);
  const boundReferenceImages = useMemo(
    () =>
      promptReferenceIds
        .map((id) => imageById.get(id))
        .filter((image): image is GeneratedImage => image != null),
    [imageById, promptReferenceIds],
  );
  const durationLimits: VideoDurationLimits = useMemo(
    () => videoDurationLimitsFromConfig(activeVideoConfig),
    [activeVideoConfig],
  );

  return {
    finalVideoPrompts,
    currentLanguageFinalVideoPrompts,
    selectedPrompt,
    frameSegmentPlanPrompt,
    framePrompts,
    currentLanguageFramePrompts,
    expectedBoundaryFrameCount,
    framePromptStageReady,
    framePromptStageMessage,
    selectedVideo,
    workflowVideos,
    workflowAssemblyVideos,
    selectableReferenceImages,
    pendingVideo,
    selectedVideoPath,
    selectedPromptSceneSummary,
    chapterAnalysisSummary,
    promptSourceHash,
    promptContentStale,
    selectedPromptStale,
    selectedPromptRunning,
    selectableReferenceIds,
    systemPromptReferenceIds,
    validPromptReferenceIds,
    staleSavedReferenceCount,
    staleSavedReferenceIds,
    savedReferenceIdsError,
    boundReferenceImages,
    videoLibraryImages,
    videoFrameImages,
    durationLimits,
  };
};
