import { useMemo } from "react";
import {
  IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE,
  type GeneratedImage,
} from "../../../api/image";
import {
  VIDEO_TYPE_ASSEMBLY_INSERT,
  VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT,
  VIDEO_TYPE_ASSEMBLY_PUSH,
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type SegmentGuidanceMode,
  type VideoGenConfig,
  type PromptOutputLanguage,
  type VideoSegmentWorkflowMode,
} from "../../../api/video";
import type { ChapterRow } from "../shared";
import type { MessageTranslator } from "../../../i18n";
import {
  currentFinalVideoPromptMatchesSegment,
  extractSourceContentHash,
  isSelectableReferenceImage,
  isVideoFramePromptImageGenerationSource,
  promptHasVideoWorkflowContract,
  promptOutputLanguageOfPrompt,
  promptSourcePromptType,
  readSavedReferenceImageIds,
  trimmedText,
  type VideoDurationLimits,
  videoImageGenerationPromptId,
  videoImageGenerationSource,
  videoTailFrameSourceVideoId,
  videoDurationLimitsFromConfig,
  videoWorkflowModeOfPrompt,
  videoExecutionSegmentIdentityMatchesPrompt,
  videoExecutionSegmentIdentityMatchesVideo,
  videoExecutionSegmentIdentityIndex,
  videoWorkflowModeOfVideo,
  VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
  VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION,
  VIDEO_FIRST_FRAME_PROMPT_TYPE,
  VIDEO_FRAME_PROMPT_TYPE,
  VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION,
  VIDEO_STORYBOARD_PROMPT_TYPE,
} from "./videoTabUtils";

interface UseVideoTabDerivedStateParams {
  prompts: ChapterVideoPrompt[];
  videos: GeneratedVideo[];
  images: GeneratedImage[];
  chapter: ChapterRow | null;
  selectedPromptId: number | null;
  selectedVideoId: number | null;
  workflowMode: VideoSegmentWorkflowMode | null;
  promptOutputLanguage: PromptOutputLanguage;
  currentContentHash: string | null;
  frameSegmentPlanPrompt: ChapterVideoPrompt | null;
  activeVideoConfig: VideoGenConfig | null;
  selectedSegmentNumber: number | null;
  guidanceMode: SegmentGuidanceMode | null;
  recipeFingerprint: string | null;
  frameRecipeFingerprint: string | null;
  continuitySourceVideoId: number | null;
  t: MessageTranslator;
}

export const useVideoTabDerivedState = ({
  prompts,
  videos,
  images,
  chapter,
  selectedPromptId,
  selectedVideoId,
  workflowMode,
  promptOutputLanguage,
  currentContentHash,
  frameSegmentPlanPrompt,
  activeVideoConfig,
  selectedSegmentNumber,
  guidanceMode,
  recipeFingerprint,
  frameRecipeFingerprint,
  continuitySourceVideoId,
  t,
}: UseVideoTabDerivedStateParams) => {
  const framePromptType =
    workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
      ? VIDEO_STORYBOARD_PROMPT_TYPE
      : workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
        ? VIDEO_FIRST_FRAME_PROMPT_TYPE
        : workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
          ? VIDEO_FRAME_PROMPT_TYPE
          : null;
  const framePromptContractVersion =
    workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
      ? VIDEO_STORYBOARD_PROMPT_CONTRACT_VERSION
      : workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
        ? VIDEO_FIRST_FRAME_CHAIN_CONTRACT_VERSION
        : workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
          ? VIDEO_BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION
          : null;
  const frameSegmentIdentityIndex = useMemo(
    () => videoExecutionSegmentIdentityIndex(frameSegmentPlanPrompt),
    [frameSegmentPlanPrompt],
  );
  const finalVideoPrompts = useMemo(
    () =>
      prompts
        .filter(
          (prompt) =>
            selectedSegmentNumber != null &&
            currentFinalVideoPromptMatchesSegment(
              prompt,
              frameSegmentPlanPrompt,
              selectedSegmentNumber,
              recipeFingerprint,
              workflowMode,
              promptOutputLanguage,
              frameSegmentIdentityIndex,
            ),
        )
        .sort((a, b) => a.shot_number - b.shot_number || a.id - b.id),
    [
      prompts,
      workflowMode,
      frameSegmentPlanPrompt,
      promptOutputLanguage,
      recipeFingerprint,
      selectedSegmentNumber,
      frameSegmentIdentityIndex,
    ],
  );
  const selectedPrompt = useMemo(
    () =>
      finalVideoPrompts.find((prompt) => prompt.id === selectedPromptId) ??
      null,
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
          (workflowMode !== VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN ||
            prompt.shot_number === 1) &&
          promptHasVideoWorkflowContract(prompt, framePromptContractVersion) &&
          promptSourcePromptType(prompt) ===
            VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE &&
          selectedSegmentNumber != null &&
          videoExecutionSegmentIdentityMatchesPrompt(
            prompt,
            frameSegmentPlanPrompt,
            selectedSegmentNumber,
            frameRecipeFingerprint,
            frameSegmentIdentityIndex,
          ),
      )
      .sort((a, b) => a.shot_number - b.shot_number || a.id - b.id);
  }, [
    framePromptContractVersion,
    framePromptType,
    prompts,
    workflowMode,
    frameSegmentPlanPrompt,
    frameRecipeFingerprint,
    selectedSegmentNumber,
    frameSegmentIdentityIndex,
  ]);
  const currentLanguageFramePrompts = useMemo(
    () =>
      framePrompts.filter(
        (prompt) =>
          promptOutputLanguageOfPrompt(prompt) === promptOutputLanguage,
      ),
    [framePrompts, promptOutputLanguage],
  );
  const selectedPromptReferenceState = useMemo(
    () =>
      readSavedReferenceImageIds(
        selectedPrompt?.reference_image_ids_json,
        t,
      ),
    [selectedPrompt?.reference_image_ids_json, t],
  );
  const selectedPromptReferenceIds = selectedPromptReferenceState.ids;
  const savedReferenceIdsError = selectedPromptReferenceState.error;
  const { workflowVideos, selectedVideo } = useMemo(() => {
    const workflowRows: GeneratedVideo[] = [];
    let selectedById: GeneratedVideo | null = null;
    for (const video of videos) {
      if (videoWorkflowModeOfVideo(video) !== workflowMode) continue;
      if (
        selectedSegmentNumber == null ||
        !videoExecutionSegmentIdentityMatchesVideo(
          video,
          frameSegmentPlanPrompt,
          selectedSegmentNumber,
          recipeFingerprint,
          frameSegmentIdentityIndex,
        )
      )
        continue;
      if (
        video.video_type === VIDEO_TYPE_ASSEMBLY_INSERT ||
        video.video_type === VIDEO_TYPE_ASSEMBLY_PUSH ||
        video.video_type === VIDEO_TYPE_ASSEMBLY_POSTPROCESS_EDIT
      ) {
        continue;
      }
      workflowRows.push(video);
      if (video.id === selectedVideoId) selectedById = video;
    }
    return {
      workflowVideos: workflowRows,
      selectedVideo:
        selectedById ??
        workflowRows.find(
          (video) =>
            video.status === "completed" && Boolean(video.file_path?.trim()),
        ) ??
        workflowRows[0] ??
        null,
    };
  }, [
    frameSegmentPlanPrompt,
    recipeFingerprint,
    selectedSegmentNumber,
    selectedVideoId,
    videos,
    workflowMode,
    frameSegmentIdentityIndex,
  ]);
  const selectedVideoPath = useMemo(
    () => trimmedText(selectedVideo?.file_path),
    [selectedVideo?.file_path],
  );
  const promptSourceHash = useMemo(
    () => extractSourceContentHash(selectedPrompt?.metadata_json),
    [selectedPrompt?.metadata_json],
  );
  const selectedPromptStale = Boolean(
    promptSourceHash &&
    currentContentHash &&
    promptSourceHash !== currentContentHash,
  );
  const selectedPromptRunning = selectedPrompt?.status === "running";
  const framePromptIdSet = useMemo(
    () => new Set(framePrompts.map((prompt) => prompt.id)),
    [framePrompts],
  );
  const tailFrameSourceVideoIds = useMemo(
    () => {
      const ids = new Set(workflowVideos.map((video) => video.id));
      if (continuitySourceVideoId != null) ids.add(continuitySourceVideoId);
      return ids;
    },
    [continuitySourceVideoId, workflowVideos],
  );
  const { videoFrameImages, systemPromptReferenceIds } = useMemo(() => {
    const rows: GeneratedImage[] = [];
    const ids = new Set<number>();
    for (const image of images) {
      const source = videoImageGenerationSource(image);
      const promptId = videoImageGenerationPromptId(image);
      let matches = false;
      if (
        isVideoFramePromptImageGenerationSource(source) &&
        promptId != null
      ) {
        matches = framePromptIdSet.has(promptId);
      } else if (workflowMode !== VIDEO_WORKFLOW_MODE_STORYBOARD) {
        const sourceVideoId = videoTailFrameSourceVideoId(image);
        matches =
          sourceVideoId != null && tailFrameSourceVideoIds.has(sourceVideoId);
      }
      if (!matches) continue;
      rows.push(image);
      if (
        guidanceMode === "system_frames" ||
        (guidanceMode === "continuity" &&
          source === IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE)
      ) {
        ids.add(image.id);
      }
    }
    return { videoFrameImages: rows, systemPromptReferenceIds: ids };
  }, [
    framePromptIdSet,
    guidanceMode,
    images,
    tailFrameSourceVideoIds,
    workflowMode,
  ]);
  const { selectableReferenceImages, selectableReferenceIds } = useMemo(() => {
    const selectableRows: GeneratedImage[] = [];
    const selectableIds = new Set<number>();
    for (const image of images) {
      if (isSelectableReferenceImage(image, chapter, currentContentHash)) {
        selectableRows.push(image);
        selectableIds.add(image.id);
      }
    }
    return {
      selectableReferenceImages: selectableRows,
      selectableReferenceIds: selectableIds,
    };
  }, [chapter, currentContentHash, images]);
  const validPromptReferenceIds = useMemo(() => {
    const ids = new Set(selectableReferenceIds);
    for (const id of systemPromptReferenceIds) ids.add(id);
    return ids;
  }, [selectableReferenceIds, systemPromptReferenceIds]);
  const staleSavedReferenceCount = useMemo(
    () =>
      selectedPromptReferenceIds.filter(
        (id) => !validPromptReferenceIds.has(id),
      ).length,
    [selectedPromptReferenceIds, validPromptReferenceIds],
  );
  const durationLimits: VideoDurationLimits = useMemo(
    () => videoDurationLimitsFromConfig(activeVideoConfig),
    [activeVideoConfig],
  );

  return {
    finalVideoPrompts,
    selectedPrompt,
    currentLanguageFramePrompts,
    selectedVideo,
    workflowVideos,
    selectableReferenceImages,
    selectedVideoPath,
    selectedPromptStale,
    selectedPromptRunning,
    selectableReferenceIds,
    validPromptReferenceIds,
    staleSavedReferenceCount,
    savedReferenceIdsError,
    videoFrameImages,
    durationLimits,
  };
};
