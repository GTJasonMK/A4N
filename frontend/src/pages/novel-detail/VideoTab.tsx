import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  Clapperboard,
  Film,
  ImagePlus,
  PanelTop,
  Scissors,
} from "lucide-react";
import { useToast } from "../../components/feedback/Toast";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../../api";
import { previewBatchImagesRequest, type GeneratedImage } from "../../api/image";
import type { ModelRequestPreview } from "../../api/requestPreview";
import {
  previewChapterVideoPromptRequests,
  previewReferenceMediaVideoRequest,
  selectSegmentVideo,
  startVideoGeneration,
  updateChapterVideoPrompt,
  uploadReferenceMediaVideo,
  verifyVideoGenerationResult,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type ReferenceMediaVideoInput,
  type VideoGenConfig,
  type SegmentGuidanceMode,
} from "../../api/video";
import { usePersistedState } from "../../hooks/usePersistedState";
import {
  chapterGeneratingMessage,
  EmptyHint,
  isChapterGenerating,
  type ChapterRow,
} from "./shared";
import { useProgressTask } from "../../hooks/useProgressTask";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import { textInputValue } from "../../utils/text";
import { copyTextToClipboard } from "../../utils/clipboard";
import { pickLocalFile, readFileAsBase64 } from "../../utils/fileUpload";
import {
  MAX_UPLOAD_VIDEO_BYTES,
  supportedVideoAccept,
  videoMimeForUploadFile,
} from "../../utils/videoUpload";
import { ImageActionDialog } from "./images/ImageActionDialog";
import { VideoAssemblyWorkspace } from "./video/VideoAssemblyWorkspace";
import { VideoPlanningWorkspace } from "./video/VideoPlanningWorkspace";
import { VideoGenerationWorkspace } from "./video/VideoGenerationWorkspace";
import { VideoMaterialWorkspace } from "./video/VideoMaterialWorkspace";
import { VideoPostProcessWorkspace } from "./video/VideoPostProcessWorkspace";
import {
  buildReferenceMediaPrompt,
  clearReferenceMediaDraftStorageError,
  createReferenceMediaDraft,
  normalizeReferenceMediaRows,
  normalizeReferenceMediaRowsByKind,
  parseReferenceMediaDraft,
  REFERENCE_MEDIA_KIND_META,
  REFERENCE_MEDIA_LIMITS,
  REFERENCE_MEDIA_PROMPT_MIN_CHARS,
  ReferenceMediaWorkspace,
  referenceMediaGenerateDraftError,
  referenceMediaRowsByKindFromDraft,
  serializeReferenceMediaDraft,
  validReferenceMediaUrl,
  VideoPromptReferenceInputDialog,
  type ReferenceMediaBindingRow,
  type ReferenceMediaDraft,
  type ReferenceMediaInputPurpose,
  type ReferenceMediaKind,
  type ReferenceMediaRowsByKind,
} from "./video/referenceMediaWorkspace";
import {
  buildVideoFrameImageTraceMetadata,
  buildVideoFrameImageRequest,
  boundaryFramePromptChainTo,
  CHAT_VIDEO_DURATION_LIMITS,
  DEFAULT_DRAFT,
  DEFAULT_VIDEO_OPTIONS,
  DEFAULT_VIDEO_WORKFLOW_MODE,
  EMPTY_VIDEO_IMAGE_ID_SELECTION,
  frameImageReferencesForPrompt,
  hasPendingProviderJob,
  isBoundaryFramePrompt,
  isDurationAllowed,
  isFinalVideoPrompt,
  latestVideoFrameImageForPrompt,
  MAX_CHAT_REFERENCE_IMAGES,
  MAX_REFERENCE_IMAGES,
  normalizeDurationOptions,
  parseGenerationReferenceImageSelection,
  parseNullableNumber,
  parseVideoLibraryImageSelection,
  parseVideoOptions,
  parseVideoWorkflowMode,
  promptLengthError,
  promptSourcePromptType,
  readSavedReferenceImageIds,
  referenceLabelOrderError,
  referenceImageScopeLabel,
  serializeGenerationReferenceImageSelection,
  serializeVideoLibraryImageSelection,
  serializeNullableNumber,
  serializeVideoOptions,
  serializeVideoWorkflowMode,
  shiftReferenceLabels,
  VIDEO_CHAT_FRAME_PROMPT_TYPE,
  VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
  VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
  VIDEO_WORKFLOW_MODE_LABELS,
  VIDEO_STORYBOARD_PROMPT_TYPE,
  expectedVideoPromptCountFromSegmentPlanPrompt,
  videoOptionsValidationError,
  videoWorkflowModeOfPrompt,
  videoWorkflowModeOfVideo,
  type ChapterVideoWorkflowMode,
  type PromptDraft,
  type VideoGenOptions,
} from "./video/videoTabUtils";
import { loadVideoTabData } from "./video/loadVideoTabData";
import { useChapterContentHash } from "./video/useChapterContentHash";
import { useVideoImageActions } from "./video/useVideoImageActions";
import { useVideoPromptGenerationActions } from "./video/useVideoPromptGenerationActions";
import { useVideoRecordActions } from "./video/useVideoRecordActions";
import { useVideoTabDerivedState } from "./video/useVideoTabDerivedState";

interface VideoTabProps {
  projectId: number;
  fixedChapter?: ChapterRow | null;
  imageProjectIds?: number[];
  materialProjectId?: number;
}

type VideoWorkspaceTab =
  | "materials"
  | "frames"
  | "video"
  | "assembly"
  | "postprocess";

const VIDEO_WORKSPACE_TABS: Array<{
  id: VideoWorkspaceTab;
  label: string;
  icon: React.ReactNode;
}> = [
  { id: "materials", label: "素材生成", icon: <ImagePlus size={13} /> },
  { id: "frames", label: "片段规划", icon: <PanelTop size={13} /> },
  { id: "video", label: "片段生成", icon: <Film size={13} /> },
  { id: "assembly", label: "成片合成", icon: <Clapperboard size={13} /> },
  { id: "postprocess", label: "成片后处理", icon: <Scissors size={13} /> },
];

interface PersistedWorkflowModeState {
  mode: ChapterVideoWorkflowMode;
  storageError: string | null;
}

interface PersistedWorkspaceTabState {
  tab: VideoWorkspaceTab;
  storageError: string | null;
}

interface PersistedNullableIdState {
  id: number | null;
  storageError: string | null;
}

interface ReferenceMediaSegmentState {
  promptId: number;
  shotNumber: number;
}

const parseReferenceMediaSegmentState = (raw: string): ReferenceMediaSegmentState | null => {
  const value = JSON.parse(raw) as Partial<ReferenceMediaSegmentState> | null;
  return value && Number.isInteger(value.promptId) && Number.isInteger(value.shotNumber)
    ? { promptId: value.promptId as number, shotNumber: value.shotNumber as number }
    : null;
};

const createPersistedWorkflowModeState = (
  mode: ChapterVideoWorkflowMode,
  storageError: string | null = null,
): PersistedWorkflowModeState => ({ mode, storageError });

const parsePersistedWorkflowModeState = (raw: string): PersistedWorkflowModeState => {
  try {
    return createPersistedWorkflowModeState(parseVideoWorkflowMode(raw));
  } catch (err) {
    return createPersistedWorkflowModeState(
      DEFAULT_VIDEO_WORKFLOW_MODE,
      `视频生成模式本地存储损坏：${extractErrorMessage(err, "")}`,
    );
  }
};

const serializePersistedWorkflowModeState = (state: PersistedWorkflowModeState): string =>
  serializeVideoWorkflowMode(state.mode);

const createPersistedWorkspaceTabState = (
  tab: VideoWorkspaceTab,
  storageError: string | null = null,
): PersistedWorkspaceTabState => ({ tab, storageError });

const parsePersistedWorkspaceTabState = (
  raw: string,
  defaultTab: VideoWorkspaceTab,
  allowedTabs: readonly VideoWorkspaceTab[],
): PersistedWorkspaceTabState => {
  if (allowedTabs.includes(raw as VideoWorkspaceTab)) {
    return createPersistedWorkspaceTabState(raw as VideoWorkspaceTab);
  }
  return createPersistedWorkspaceTabState(
    defaultTab,
    `视频工作区 tab 本地存储损坏：${raw || "<empty>"}`,
  );
};

const serializePersistedWorkspaceTabState = (state: PersistedWorkspaceTabState): string =>
  state.tab;

const createPersistedNullableIdState = (
  id: number | null,
  storageError: string | null = null,
): PersistedNullableIdState => ({ id, storageError });

const parsePersistedNullableIdState = (
  raw: string,
  label: string,
): PersistedNullableIdState => {
  try {
    return createPersistedNullableIdState(parseNullableNumber(raw));
  } catch (err) {
    return createPersistedNullableIdState(
      null,
      `${label}本地存储损坏：${extractErrorMessage(err, "")}`,
    );
  }
};

const serializePersistedNullableIdState = (state: PersistedNullableIdState): string =>
  serializeNullableNumber(state.id);

const VideoWorkspaceHeader: React.FC<{
  tabs: Array<{ id: VideoWorkspaceTab; label: string; icon: React.ReactNode }>;
  activeTab: VideoWorkspaceTab;
  onTabChange: (tab: VideoWorkspaceTab) => void;
  children: React.ReactNode;
}> = ({ tabs, activeTab, onTabChange, children }) => {
  const activeTabRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      activeTabRef.current?.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [activeTab, tabs]);

  return (
    <div className="grid min-w-0 shrink-0 gap-2 border-b border-book-border/60 pb-2 sm:flex sm:items-center">
      <div
        role="tablist"
        aria-label="视频工作区阶段"
        className="tab-bar a4n-mobile-scroll-rail min-w-0 w-full flex-1 scroll-px-3"
      >
        {tabs.map((tab) => {
          const active = activeTab === tab.id;
          return (
            <button
              ref={active ? activeTabRef : undefined}
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => onTabChange(tab.id)}
              data-active={active ? "1" : undefined}
              className="pyqt-tab inline-flex shrink-0 items-center gap-1.5 !px-2"
            >
              {tab.icon}
              {tab.label}
            </button>
          );
        })}
      </div>
      <div className="flex min-w-0 items-center justify-end gap-2 text-[11px] text-book-text-muted sm:shrink-0">
        {children}
      </div>
    </div>
  );
};

export const VideoTab: React.FC<VideoTabProps> = ({
  projectId,
  fixedChapter,
  imageProjectIds,
  materialProjectId,
}) => {
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const chapter = fixedChapter ?? null;
  const profileMaterialProjectId = materialProjectId ?? projectId;
  const chapterDataKey =
    chapter && Number.isFinite(chapter.id) ? `${projectId}:${chapter.id}` : null;
  const imageProjectIdsKey = (imageProjectIds ?? [projectId])
    .filter((id) => Number.isFinite(id) && id > 0)
    .join(",");
  const videoDataKey = chapterDataKey ? `${chapterDataKey}:${imageProjectIdsKey}` : null;
  const chapterStorageKey =
    chapter && Number.isFinite(chapter.id) ? `a4n:video:${projectId}:${chapter.id}` : null;
  const workflowModeInitialValue = useMemo(
    () => createPersistedWorkflowModeState(DEFAULT_VIDEO_WORKFLOW_MODE),
    [chapterStorageKey],
  );
  const [workflowModeState, setWorkflowModeState] = usePersistedState<PersistedWorkflowModeState>(
    chapterStorageKey ? `${chapterStorageKey}:workflow-mode` : null,
    workflowModeInitialValue,
    {
      parse: parsePersistedWorkflowModeState,
      serialize: serializePersistedWorkflowModeState,
    },
  );
  const workflowMode = workflowModeState.mode;
  const setWorkflowMode = useCallback(
    (mode: ChapterVideoWorkflowMode) =>
      setWorkflowModeState(createPersistedWorkflowModeState(mode)),
    [setWorkflowModeState],
  );
  const modeStorageKey = chapterStorageKey ? `${chapterStorageKey}:${workflowMode}` : null;
  const projectModeStorageKey =
    Number.isFinite(projectId) && projectId > 0
      ? `a4n:video:${projectId}:${workflowMode}`
      : null;
  const videoActionScopeKey = videoDataKey
    ? `${videoDataKey}:${workflowMode}`
    : null;
  const workspaceTabIds = VIDEO_WORKSPACE_TABS.map((tab) => tab.id);
  const defaultWorkspaceTab: VideoWorkspaceTab = "video";
  const workspaceTabInitialValue = useMemo(
    () => createPersistedWorkspaceTabState(defaultWorkspaceTab),
    [defaultWorkspaceTab, modeStorageKey],
  );
  const [workspaceTabState, setWorkspaceTabState] =
    usePersistedState<PersistedWorkspaceTabState>(
      modeStorageKey ? `${modeStorageKey}:workspace-tab` : null,
      workspaceTabInitialValue,
      {
        parse: (raw) =>
          parsePersistedWorkspaceTabState(raw, defaultWorkspaceTab, workspaceTabIds),
        serialize: serializePersistedWorkspaceTabState,
      },
    );
  const workspaceTab = workspaceTabState.tab;
  const setWorkspaceTab = useCallback(
    (tab: VideoWorkspaceTab) => setWorkspaceTabState(createPersistedWorkspaceTabState(tab)),
    [setWorkspaceTabState],
  );
  const [prompts, setPrompts] = useState<ChapterVideoPrompt[]>([]);
  const workflowTargetShotRef = useRef<number | null>(null);
  const [referenceMediaSegment, setReferenceMediaSegment] = usePersistedState<ReferenceMediaSegmentState | null>(
    chapterStorageKey ? `${chapterStorageKey}:reference-media-segment` : null,
    null,
    { parse: parseReferenceMediaSegmentState, serialize: JSON.stringify },
  );
  const [videos, setVideos] = useState<GeneratedVideo[]>([]);
  const [images, setImages] = useState<GeneratedImage[]>([]);
  const persistedNullIdInitialValue = useMemo(
    () => createPersistedNullableIdState(null),
    [modeStorageKey],
  );
  const [selectedPromptState, setSelectedPromptState] =
    usePersistedState<PersistedNullableIdState>(
      modeStorageKey ? `${modeStorageKey}:selected-prompt` : null,
      persistedNullIdInitialValue,
      {
        parse: (raw) => parsePersistedNullableIdState(raw, "选中视频提示词"),
        serialize: serializePersistedNullableIdState,
      },
    );
  const [selectedVideoState, setSelectedVideoState] =
    usePersistedState<PersistedNullableIdState>(
      modeStorageKey ? `${modeStorageKey}:selected-video` : null,
      persistedNullIdInitialValue,
      {
        parse: (raw) => parsePersistedNullableIdState(raw, "选中视频"),
        serialize: serializePersistedNullableIdState,
      },
    );
  const [pendingVideoState, setPendingVideoState] =
    usePersistedState<PersistedNullableIdState>(
      modeStorageKey ? `${modeStorageKey}:pending-video` : null,
      persistedNullIdInitialValue,
      {
        parse: (raw) => parsePersistedNullableIdState(raw, "待恢复视频任务"),
        serialize: serializePersistedNullableIdState,
      },
    );
  const selectedPromptId = selectedPromptState.id;
  const selectedVideoId = selectedVideoState.id;
  const pendingVideoId = pendingVideoState.id;
  const setSelectedPromptId = useCallback<React.Dispatch<React.SetStateAction<number | null>>>(
    (action) => {
      setSelectedPromptState((prev) =>
        createPersistedNullableIdState(
          typeof action === "function" ? action(prev.id) : action,
        ),
      );
    },
    [setSelectedPromptState],
  );
  const setSelectedVideoId = useCallback<React.Dispatch<React.SetStateAction<number | null>>>(
    (action) => {
      setSelectedVideoState((prev) =>
        createPersistedNullableIdState(
          typeof action === "function" ? action(prev.id) : action,
        ),
      );
    },
    [setSelectedVideoState],
  );
  const setPendingVideoId = useCallback<React.Dispatch<React.SetStateAction<number | null>>>(
    (action) => {
      setPendingVideoState((prev) =>
        createPersistedNullableIdState(
          typeof action === "function" ? action(prev.id) : action,
        ),
      );
    },
    [setPendingVideoState],
  );
  const [videoLibraryImageSelection, setVideoLibraryImageSelection] = usePersistedState(
    modeStorageKey ? `${modeStorageKey}:video-library-image-ids` : null,
    EMPTY_VIDEO_IMAGE_ID_SELECTION,
    { parse: parseVideoLibraryImageSelection, serialize: serializeVideoLibraryImageSelection },
  );
  const [generationReferenceSelection, setGenerationReferenceSelection] = usePersistedState(
    modeStorageKey ? `${modeStorageKey}:generation-reference-image-ids` : null,
    EMPTY_VIDEO_IMAGE_ID_SELECTION,
    {
      parse: parseGenerationReferenceImageSelection,
      serialize: serializeGenerationReferenceImageSelection,
    },
  );
  const [options, setOptions] = usePersistedState<VideoGenOptions>(
    projectModeStorageKey ? `${projectModeStorageKey}:options` : null,
    DEFAULT_VIDEO_OPTIONS,
    { parse: parseVideoOptions, serialize: serializeVideoOptions },
  );
  const videoLibraryImageIds = videoLibraryImageSelection.ids;
  const generationReferenceIds = generationReferenceSelection.ids;
  const setVideoLibraryImageIds = useCallback<React.Dispatch<React.SetStateAction<number[]>>>(
    (action) => {
      setVideoLibraryImageSelection((prev) => ({
        ids: typeof action === "function" ? action(prev.ids) : action,
        storageError: null,
      }));
    },
    [setVideoLibraryImageSelection],
  );
  const setGenerationReferenceIds = useCallback<React.Dispatch<React.SetStateAction<number[]>>>(
    (action) => {
      setGenerationReferenceSelection((prev) => ({
        ids: typeof action === "function" ? action(prev.ids) : action,
        storageError: null,
      }));
    },
    [setGenerationReferenceSelection],
  );
  const setVideoOptions = useCallback<React.Dispatch<React.SetStateAction<VideoGenOptions>>>(
    (action) => {
      setOptions((prev) => {
        const next = typeof action === "function" ? action(prev) : action;
        return { ...next, storageError: null };
      });
    },
    [setOptions],
  );
  const [draft, setDraft] = useState<PromptDraft>(DEFAULT_DRAFT);
  const referenceMediaDraftInitialValue = useMemo(
    () => createReferenceMediaDraft(),
    [modeStorageKey],
  );
  const [referenceMediaDraft, setReferenceMediaDraft] = usePersistedState<ReferenceMediaDraft>(
    modeStorageKey && workflowMode === "reference_media" ? `${modeStorageKey}:draft` : null,
    referenceMediaDraftInitialValue,
    { parse: parseReferenceMediaDraft, serialize: serializeReferenceMediaDraft },
  );
  const [promptReferenceIds, setPromptReferenceIds] = useState<number[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadedDataKey, setLoadedDataKey] = useState<string | null>(null);
  const [activeVideoConfig, setActiveVideoConfig] = useState<VideoGenConfig | null>(null);
  const [promptRequestPreviewOpen, setPromptRequestPreviewOpen] = useState(false);
  const [promptRequestPreviewLoading, setPromptRequestPreviewLoading] = useState(false);
  const [promptRequestPreviewData, setPromptRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const promptRequestPreviewScopeRef = useRef(0);
  const loadAllSeqRef = useRef(0);
  const activeVideoDataKeyRef = useRef(videoDataKey);
  const [imageRequestPreviewOpen, setImageRequestPreviewOpen] = useState(false);
  const [imageRequestPreviewLoading, setImageRequestPreviewLoading] = useState(false);
  const [imageRequestPreviewData, setImageRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const imageRequestPreviewScopeRef = useRef(0);
  const setAllReferenceIds: React.Dispatch<React.SetStateAction<number[]>> = (updater) => {
    setGenerationReferenceIds(updater);
    setPromptReferenceIds(updater);
  };
  const { currentContentHash, chapterContentReady } = useChapterContentHash(chapter);
  const {
    imageBusyId,
    imageDetails,
    imageScoreInput,
    imageScoringId,
    setImageScoreInput,
    handleCloseImageDetails,
    handleDeleteImage,
    handleScoreImage,
    handleToggleImageFavorite,
    handleUpdateImage,
  } = useVideoImageActions({
    addToast,
    images,
    actionScopeKey: videoActionScopeKey,
    setImages,
    setReferenceIds: setAllReferenceIds,
    setVideoLibraryImageIds,
  });

  const {
    finalVideoPrompts,
    currentLanguageFinalVideoPrompts,
    selectedPrompt,
    frameSegmentPlanPrompt,
    framePrompts,
    currentLanguageFramePrompts,
    expectedBoundaryFrameCount,
    selectedVideo,
    pendingVideo,
    selectedVideoPath,
    selectedPromptSceneSummary,
    selectedPromptStale,
    selectedPromptRunning,
    workflowVideos,
    selectableReferenceImages,
    selectableReferenceIds,
    systemPromptReferenceIds,
    validPromptReferenceIds,
    staleSavedReferenceCount,
    savedReferenceIdsError,
    boundReferenceImages,
    videoFrameImages,
    durationLimits,
  } = useVideoTabDerivedState({
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
    promptOutputLanguage: options.promptOutputLanguage,
    currentContentHash,
    activeVideoConfig,
  });

  const referenceMediaSegmentVideos = useMemo(() => {
    if (!referenceMediaSegment) return workflowVideos;
    return workflowVideos.filter((video) => {
      const metadata =
        video.metadata_json && typeof video.metadata_json === "object" && !Array.isArray(video.metadata_json)
          ? (video.metadata_json as Record<string, unknown>)
          : null;
      return metadata?.shot_number === referenceMediaSegment.shotNumber;
    });
  }, [referenceMediaSegment, workflowVideos]);
  const allSegmentPrompts = useMemo(
    () => prompts
      .filter((prompt) => prompt.prompt_type === "video_prompt")
      .sort((a, b) => a.shot_number - b.shot_number || a.id - b.id),
    [prompts],
  );
  const expectedVideoPromptCount = useMemo(
    () => expectedVideoPromptCountFromSegmentPlanPrompt(frameSegmentPlanPrompt),
    [frameSegmentPlanPrompt],
  );
  const chapterGenerating = isChapterGenerating(chapter);
  const visibleWorkspaceTabs = VIDEO_WORKSPACE_TABS;
  const activeWorkspaceTab =
    visibleWorkspaceTabs.some((tab) => tab.id === workspaceTab) ? workspaceTab : defaultWorkspaceTab;
  const {
    chapterVideoAssetCount,
    chapterVideoChainPromptCount,
  } = useMemo(() => {
    if (!chapter) {
      return {
        chapterVideoAssetCount: 0,
        chapterVideoChainPromptCount: 0,
      };
    }

    let chapterVideoAssetCount = 0;
    for (const video of videos) {
      if (video.chapter_id === chapter.id) chapterVideoAssetCount += 1;
    }

    let chapterVideoChainPromptCount = 0;
    for (const prompt of prompts) {
      if (prompt.chapter_id !== chapter.id) continue;
      chapterVideoChainPromptCount += 1;
    }

    return {
      chapterVideoAssetCount,
      chapterVideoChainPromptCount,
    };
  }, [chapter, prompts, videos]);
  const selectionValidationReady = loadedDataKey === videoDataKey;
  const invalidGenerationReferenceIds = useMemo(
    () =>
      selectionValidationReady
        ? generationReferenceIds.filter((id) => !selectableReferenceIds.has(id))
        : [],
    [generationReferenceIds, selectableReferenceIds, selectionValidationReady],
  );
  const invalidVideoLibraryImageIds = useMemo(
    () =>
      selectionValidationReady
        ? videoLibraryImageIds.filter((id) => !selectableReferenceIds.has(id))
        : [],
    [selectableReferenceIds, selectionValidationReady, videoLibraryImageIds],
  );
  const invalidPromptReferenceIds = useMemo(
    () =>
      selectionValidationReady
        ? promptReferenceIds.filter((id) => !validPromptReferenceIds.has(id))
        : [],
    [promptReferenceIds, selectionValidationReady, validPromptReferenceIds],
  );
  const promptInputSelectionError =
    generationReferenceSelection.storageError ??
    videoLibraryImageSelection.storageError ??
    (invalidGenerationReferenceIds.length > 0
      ? `生成参考图中有 ${invalidGenerationReferenceIds.length} 张已不可用，请清除后重选`
      : null) ??
    (invalidVideoLibraryImageIds.length > 0
      ? `视频素材库中有 ${invalidVideoLibraryImageIds.length} 张图片已不可用，请清除后重选`
      : null);
  const videoOptionsStateError =
    options.storageError ??
    videoOptionsValidationError(options);
  const videoPersistedStateError =
    workflowModeState.storageError ??
    workspaceTabState.storageError ??
    selectedPromptState.storageError ??
    selectedVideoState.storageError ??
    pendingVideoState.storageError ??
    null;
  const effectiveDurationLimits = useMemo(
    () => {
      if (workflowMode === "chat_video") return CHAT_VIDEO_DURATION_LIMITS;
      return videoOptionsStateError
        ? {
            ...durationLimits,
            configError: videoOptionsStateError,
          }
        : durationLimits;
    },
    [durationLimits, videoOptionsStateError, workflowMode],
  );

  const setReferenceMediaPrompt = (prompt: string) =>
    setReferenceMediaDraft((prev) => ({ ...clearReferenceMediaDraftStorageError(prev), prompt }));
  const setReferenceMediaImageRows = (imageRows: ReferenceMediaBindingRow[]) =>
    setReferenceMediaDraft((prev) => ({ ...clearReferenceMediaDraftStorageError(prev), imageRows }));
  const setReferenceMediaVideoRows = (videoRows: ReferenceMediaBindingRow[]) =>
    setReferenceMediaDraft((prev) => ({ ...clearReferenceMediaDraftStorageError(prev), videoRows }));
  const setReferenceMediaAudioRows = (audioRows: ReferenceMediaBindingRow[]) =>
    setReferenceMediaDraft((prev) => ({ ...clearReferenceMediaDraftStorageError(prev), audioRows }));
  const loadAll = async () => {
    if (!chapter || !videoDataKey) return;
    const requestSeq = loadAllSeqRef.current + 1;
    const requestDataKey = videoDataKey;
    const requestImageProjectIds = imageProjectIds ?? [projectId];
    loadAllSeqRef.current = requestSeq;
    setLoading(true);
    try {
      const {
        prompts: promptRows,
        videos: videoRows,
        images: imageRows,
        activeVideoConfig: nextActiveVideoConfig,
      } = await loadVideoTabData(projectId, chapter, requestImageProjectIds);
      if (
        loadAllSeqRef.current !== requestSeq ||
        activeVideoDataKeyRef.current !== requestDataKey
      ) {
        return;
      }
      setActiveVideoConfig(nextActiveVideoConfig);
      setPrompts(promptRows);
      setVideos(videoRows);
      setImages(imageRows);
      setLoadedDataKey(requestDataKey);
      if (!selectedPromptState.storageError) {
        setSelectedPromptId((current) => {
          const isSelectableVideoPrompt = (id: number) =>
            promptRows.some(
              (p) =>
                p.id === id &&
                isFinalVideoPrompt(p.prompt_type) &&
                videoWorkflowModeOfPrompt(p) === workflowMode &&
                promptSourcePromptType(p) === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
            );
          return current && isSelectableVideoPrompt(current) ? current : null;
        });
      }
      if (!selectedVideoState.storageError) {
        setSelectedVideoId((current) => {
          return current && videoRows.some(
              (v) =>
                v.id === current &&
                videoWorkflowModeOfVideo(v) === workflowMode,
            )
            ? current
            : null;
        });
      }
      if (!pendingVideoState.storageError) {
        setPendingVideoId((current) => {
          const pending = current
              ? videoRows.find(
                  (v) =>
                    v.id === current &&
                    videoWorkflowModeOfVideo(v) === workflowMode,
                )
              : null;
          return pending && hasPendingProviderJob(pending) ? pending.id : null;
        });
      }
    } catch (err) {
      if (
        loadAllSeqRef.current !== requestSeq ||
        activeVideoDataKeyRef.current !== requestDataKey
      ) {
        return;
      }
      addToast(extractErrorMessage(err, "加载视频数据失败"), "error");
    } finally {
      if (
        loadAllSeqRef.current === requestSeq &&
        activeVideoDataKeyRef.current === requestDataKey
      ) {
        setLoading(false);
      }
    }
  };

  useLayoutEffect(() => {
    activeVideoDataKeyRef.current = videoDataKey;
    loadAllSeqRef.current += 1;
    setPrompts([]);
    setVideos([]);
    setImages([]);
    setActiveVideoConfig(null);
    setLoadedDataKey(null);
    setDraft(DEFAULT_DRAFT);
    setPromptReferenceIds([]);
    setLoading(false);
  }, [videoDataKey]);

  useEffect(() => {
    void loadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [videoDataKey]);

  useEffect(() => {
    if (selectedPromptState.storageError) return;
    setSelectedPromptId((current) => {
      if (current && finalVideoPrompts.some((prompt) => prompt.id === current)) {
        return current;
      }
      return finalVideoPrompts[0]?.id ?? null;
    });
  }, [finalVideoPrompts, selectedPromptState.storageError, setSelectedPromptId, workflowMode]);

  useEffect(() => {
    const pending = workflowVideos.find(hasPendingProviderJob) ?? null;
    if (!pendingVideoState.storageError) {
      setPendingVideoId((current) => {
        const currentPending = current
          ? workflowVideos.find((video) => video.id === current && hasPendingProviderJob(video))
          : null;
        return currentPending?.id ?? pending?.id ?? null;
      });
    }
    if (!selectedVideoState.storageError) {
      setSelectedVideoId((current) => {
        if (current && workflowVideos.some((video) => video.id === current)) {
          return current;
        }
        return pending?.id ?? workflowVideos[0]?.id ?? null;
      });
    }
  }, [
    pendingVideoState.storageError,
    selectedVideoState.storageError,
    setPendingVideoId,
    setSelectedVideoId,
    workflowMode,
    workflowVideos,
  ]);

  useEffect(() => {
    promptRequestPreviewScopeRef.current += 1;
    imageRequestPreviewScopeRef.current += 1;
    setDraft(DEFAULT_DRAFT);
    setPromptReferenceIds([]);
    setPromptRequestPreviewOpen(false);
    setPromptRequestPreviewLoading(false);
    setPromptRequestPreviewData(null);
    setImageRequestPreviewOpen(false);
    setImageRequestPreviewLoading(false);
    setImageRequestPreviewData(null);
  }, [workflowMode]);

  useEffect(() => {
    if (!selectedPrompt) {
      setDraft(DEFAULT_DRAFT);
      setPromptReferenceIds([]);
      return;
    }
    setDraft({
      visual_prompt: selectedPrompt.visual_prompt,
      motion_prompt: textInputValue(selectedPrompt.motion_prompt),
      negative_prompt: textInputValue(selectedPrompt.negative_prompt),
      duration_secs: selectedPrompt.duration_secs,
    });
    setPromptReferenceIds(readSavedReferenceImageIds(selectedPrompt.reference_image_ids_json).ids);
    // 只在切换提示词时重置草稿和素材绑定，避免刷新素材/视频状态时覆盖未保存编辑。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedPrompt?.id]);

  const clearInvalidGenerationReferences = () => {
    if (invalidGenerationReferenceIds.length === 0) return;
    setGenerationReferenceIds((prev) => prev.filter((id) => selectableReferenceIds.has(id)));
    addToast("已清除不可用生成参考图片选择", "success");
  };
  const clearInvalidVideoLibraryImages = () => {
    if (invalidVideoLibraryImageIds.length === 0) return;
    setVideoLibraryImageIds((prev) => prev.filter((id) => selectableReferenceIds.has(id)));
    addToast("已清除不可用素材库图片选择", "success");
  };
  const clearInvalidPromptReferences = () => {
    if (invalidPromptReferenceIds.length === 0) return;
    setPromptReferenceIds((prev) => prev.filter((id) => validPromptReferenceIds.has(id)));
    addToast("已从当前草稿清除不可用参考图，请保存提示词后生效", "success");
  };

  const {
    acting,
    setActing,
    setVideoTaskId,
    trimStart,
    trimEnd,
    setTrimStart,
    setTrimEnd,
    handleOptimizePromptRow,
    handleDeletePromptRow,
    handleSavePrompt,
    handleGenerateVideo,
    handleGenerateAllVideos,
    handleImportVideo,
    handlePreviewGenerateVideoRequest,
    handlePreviewVideoRequestForVideo,
    selectedPromptDirty,
    handlePollVideo,
    handleToggleVideoFavorite,
    handleUpdateVideo,
    handleDeleteVideo,
    handleDeleteAllVideoAssets,
    handleDeleteAllVideoPrompts,
    handleTrim,
    requestPreviewOpen: videoRequestPreviewOpen,
    requestPreviewLoading: videoRequestPreviewLoading,
    requestPreviewData: videoRequestPreviewData,
    setRequestPreviewOpen: setVideoRequestPreviewOpen,
  } = useVideoRecordActions({
    projectId,
    chapter,
    chapterGenerating,
    selectedPrompt,
    selectedPromptId,
    frameStageLabel:
      workflowMode === "chat_video"
        ? "Chat 生视频"
        : workflowMode === "storyboard"
          ? "分镜生成"
        : workflowMode === "first_frame_chain"
          ? "首帧生成"
          : "首尾帧生成",
    selectedPromptStale,
    currentContentHash,
    selectedVideo,
    pendingVideoId,
    previewScopeKey: modeStorageKey,
    actionScopeKey: videoActionScopeKey,
    prompts,
    frameImages: videoFrameImages,
    draft,
    referenceIds: promptReferenceIds,
    invalidReferenceIds: invalidPromptReferenceIds,
    systemReferenceIds: systemPromptReferenceIds,
    validPromptReferenceIds,
    durationLimits: effectiveDurationLimits,
    providerBackend: activeVideoConfig?.backend ?? null,
    providerOptionsJson: activeVideoConfig?.provider_options_json ?? null,
    addToast,
    runProgressTask,
    loadAll,
    setPrompts,
    setDraft,
    setSelectedPromptId,
    setSelectedVideoId,
    setPendingVideoId,
  });

  const {
    handleGenerateSegmentPlan,
    handleGenerateFramePrompts,
    handleGenerateVideoPrompts,
    handleGenerateFrameImages,
    handleImportFrameImage,
  } = useVideoPromptGenerationActions({
    projectId,
    chapter,
    chapterGenerating,
    workflowMode,
    options,
    durationLimits: effectiveDurationLimits,
    promptInputSelectionError,
    referenceIds: generationReferenceIds,
    videoLibraryImageIds,
    frameImages: videoFrameImages,
    segmentPlanPrompt: frameSegmentPlanPrompt,
    framePrompts: currentLanguageFramePrompts,
    finalVideoPrompts: currentLanguageFinalVideoPrompts,
    allFramePrompts: framePrompts,
    allFinalVideoPrompts: finalVideoPrompts,
    currentContentHash,
    actionScopeKey: videoActionScopeKey,
    addToast,
    runProgressTask,
    loadAll,
    setActing,
    setVideoTaskId,
  });

  const handleAdoptSegmentVideo = async (video: GeneratedVideo) => {
    const videoMetadata =
      video.metadata_json && typeof video.metadata_json === "object" && !Array.isArray(video.metadata_json)
        ? (video.metadata_json as Record<string, unknown>)
        : null;
    const shotNumber =
      selectedPrompt?.shot_number ??
      (typeof videoMetadata?.shot_number === "number" ? videoMetadata.shot_number : null);
    if (!chapter || shotNumber == null) {
      addToast("请先选择片段提示词", "error");
      return;
    }
    setActing(`adopt-video-${video.id}`);
    try {
      await selectSegmentVideo(
        projectId,
        chapter.id,
        video.id,
        shotNumber,
      );
      addToast(`已采用片段 ${shotNumber} 的当前视频`, "success");
      await loadAll();
    } catch (err) {
      addToast(extractErrorMessage(err, "采用片段视频失败"), "error");
    } finally {
      setActing(null);
    }
  };

  const handleSegmentWorkflowModeChange = (
    mode: ChapterVideoWorkflowMode,
    shotNumber?: number,
  ) => {
    const targetShotNumber = shotNumber ?? selectedPrompt?.shot_number ?? null;
    workflowTargetShotRef.current = targetShotNumber;
    if (mode === "reference_media") {
      const targetPrompt =
        (selectedPrompt?.shot_number === targetShotNumber ? selectedPrompt : null) ??
        allSegmentPrompts.find(
          (prompt) =>
            prompt.shot_number === targetShotNumber &&
            videoWorkflowModeOfPrompt(prompt) === workflowMode,
        ) ??
        allSegmentPrompts.find((prompt) => prompt.shot_number === targetShotNumber);
      if (!targetPrompt) {
        addToast("当前片段还没有可关联的视频提示词", "error");
        return;
      }
      setReferenceMediaSegment({
        promptId: targetPrompt.id,
        shotNumber: targetPrompt.shot_number,
      });
    }
    setWorkflowMode(mode);
    setWorkspaceTab("video");
  };

  const handleUpdateSegmentGuidance = async (
    prompt: ChapterVideoPrompt,
    guidanceMode: SegmentGuidanceMode,
    referenceImageIds: number[],
    continuitySourceVideoId: number | null,
  ) => {
    setActing(`guidance-${prompt.id}`);
    try {
      await updateChapterVideoPrompt(prompt.id, {
        reference_image_ids_json: referenceImageIds,
        guidance_mode: guidanceMode,
        continuity_source_video_id: continuitySourceVideoId,
      });
      addToast("片段生成方式已保存", "success");
      await loadAll();
    } catch (err) {
      addToast(extractErrorMessage(err, "保存片段生成方式失败"), "error");
    } finally {
      setActing(null);
    }
  };

  useEffect(() => {
    const shotNumber = workflowTargetShotRef.current;
    if (shotNumber == null || finalVideoPrompts.length === 0) return;
    const target = finalVideoPrompts.find((prompt) => prompt.shot_number === shotNumber);
    if (!target) return;
    workflowTargetShotRef.current = null;
    setSelectedPromptId(target.id);
  }, [finalVideoPrompts, setSelectedPromptId]);

  const videoPromptRequestOptions = (
    workflowModeOverride: ChapterVideoWorkflowMode = workflowMode,
  ) => {
    if (videoOptionsStateError) throw new Error(videoOptionsStateError);
    if (promptInputSelectionError) throw new Error(promptInputSelectionError);
    return {
      workflow_mode: workflowModeOverride,
      style: options.style.trim(),
      prompt_batch_size: options.promptBatchSize,
      duration_secs: options.durationSecs,
      duration_secs_options: normalizeDurationOptions(options.durationSecs, options.durationOptions),
      aspect_ratio: options.aspectRatio,
      reference_image_ids: generationReferenceIds,
      video_library_image_ids: videoLibraryImageIds,
    };
  };

  const validatePromptRequestPreview = (): boolean => {
    if (!chapter) return false;
    if (videoOptionsStateError) {
      addToast(videoOptionsStateError, "error");
      return false;
    }
    if (promptInputSelectionError) {
      addToast(promptInputSelectionError, "error");
      return false;
    }
    return true;
  };

  const openPromptRequestPreview = async (
    loadPreview: () => Promise<ModelRequestPreview>,
    errorMessage: string,
  ) => {
    if (!chapter) return;
    const previewScope = promptRequestPreviewScopeRef.current;
    setPromptRequestPreviewOpen(true);
    setPromptRequestPreviewLoading(true);
    setPromptRequestPreviewData(null);
    try {
      const preview = await loadPreview();
      if (promptRequestPreviewScopeRef.current !== previewScope) return;
      setPromptRequestPreviewData(preview);
    } catch (err) {
      if (promptRequestPreviewScopeRef.current !== previewScope) return;
      addToast(extractErrorMessage(err, errorMessage), "error");
      setPromptRequestPreviewOpen(false);
    } finally {
      if (promptRequestPreviewScopeRef.current === previewScope) {
        setPromptRequestPreviewLoading(false);
      }
    }
  };

  const handlePreviewVideoPromptRequests = async () => {
    if (!chapter || !validatePromptRequestPreview()) return;
    if (
      workflowMode !== "boundary_frames" &&
      workflowMode !== "first_frame_chain" &&
      workflowMode !== "storyboard" &&
      workflowMode !== "chat_video"
    ) {
      addToast("当前模式的视频提示词链路尚未接入", "error");
      return;
    }
    await openPromptRequestPreview(
      () =>
        previewChapterVideoPromptRequests(
          projectId,
          chapter.id,
          videoPromptRequestOptions(),
        ),
      "预览视频提示词请求失败",
    );
  };

  const handleCopyChatVideoPrompt = async (prompt: ChapterVideoPrompt) => {
    const text = prompt.visual_prompt.trim();
    if (!text) {
      addToast("当前 Chat 视频提示词为空，无法复制", "error");
      return;
    }
    try {
      await copyTextToClipboard(text);
      addToast("已复制 Chat 视频提示词", "success");
    } catch (err) {
      addToast(extractErrorMessage(err, "复制 Chat 视频提示词失败"), "error");
    }
  };

  const buildFrameImageGenerationInput = (
    prompt: ChapterVideoPrompt,
    extraPreviewNote: string | null = null,
  ): Record<string, unknown> => {
    const firstFramePrompt = prompt.prompt_type === "first_frame_prompt";
    const chatFramePrompt = prompt.prompt_type === VIDEO_CHAT_FRAME_PROMPT_TYPE;
    const storyboardPrompt = prompt.prompt_type === VIDEO_STORYBOARD_PROMPT_TYPE;
    const label = firstFramePrompt
      ? "首帧提示词"
      : storyboardPrompt
        ? "分镜提示词"
      : chatFramePrompt
        ? "Chat 首尾帧提示词"
        : "边界帧提示词";
    const trimmedPrompt = prompt.visual_prompt.trim();
    const previewValidationError = promptLengthError(
      label,
      trimmedPrompt,
      VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
    );
    if (previewValidationError) {
      throw new Error(previewValidationError);
    }
    const references = frameImageReferencesForPrompt(
      prompt,
      videoFrameImages,
      currentLanguageFramePrompts,
    );
    if (references.error) {
      throw new Error(references.error);
    }
    const effectiveVisualPrompt = shiftReferenceLabels(
      trimmedPrompt,
      references.promptReferenceOffset,
    );
    const referenceOrderError = referenceLabelOrderError(effectiveVisualPrompt, references.ids.length);
    if (referenceOrderError) {
      throw new Error(referenceOrderError);
    }
    const requestPreviewNote = [
      extraPreviewNote,
    ].filter(Boolean).join(" ");
    return buildVideoFrameImageRequest(
      firstFramePrompt
        ? "start_frame"
        : storyboardPrompt
          ? "storyboard_frame"
          : "key_frame",
      effectiveVisualPrompt,
      prompt.chapter_id || chapter?.id || 0,
      buildVideoFrameImageTraceMetadata(
        prompt,
        firstFramePrompt
          ? "video_first_frame_prompt"
          : storyboardPrompt
            ? "video_storyboard_prompt"
            : "video_boundary_frame_prompt",
        {
          projectId,
          chapter,
          currentContentHash,
          requestPreviewNote: requestPreviewNote || null,
        },
      ),
      references.ids,
      references.note,
    );
  };

  const buildFrameImagePreviewInputs = (
    prompt: ChapterVideoPrompt,
  ): Array<Record<string, unknown>> | null => {
    if (!isBoundaryFramePrompt(prompt)) {
      return [buildFrameImageGenerationInput(prompt)];
    }
    const chain = boundaryFramePromptChainTo(
      prompt,
      currentLanguageFramePrompts,
      "预览",
    );
    const missingPrompts = chain.filter(
      (item) => !latestVideoFrameImageForPrompt(item, videoFrameImages),
    );
    if (missingPrompts.length === 0) {
      addToast(`第 1-${prompt.shot_number} 张边界帧图片已存在，无需新增成图请求`, "info");
      return null;
    }
    const firstMissing = missingPrompts[0];
    if (!firstMissing) return null;
    const dynamicTailCount = missingPrompts.length - 1;
    const extraPreviewNote =
      dynamicTailCount > 0
        ? `边界帧图片会顺序生成；当前只预览第 ${firstMissing.shot_number} 张缺失图片请求。后续 ${dynamicTailCount} 张请求依赖本次新生成的图片，暂时无法完整预览。`
        : null;
    return [buildFrameImageGenerationInput(firstMissing, extraPreviewNote)];
  };

  const previewImageRequests = async (requests: Array<Record<string, unknown>>) => {
    if (!chapter) return;
    if (requests.length === 0) {
      addToast("没有可预览的成图请求", "error");
      return;
    }
    const previewScope = imageRequestPreviewScopeRef.current;
    setImageRequestPreviewOpen(true);
    setImageRequestPreviewLoading(true);
    setImageRequestPreviewData(null);
    try {
      const preview = await previewBatchImagesRequest(projectId, requests);
      if (imageRequestPreviewScopeRef.current !== previewScope) return;
      setImageRequestPreviewData(preview);
    } catch (err) {
      if (imageRequestPreviewScopeRef.current !== previewScope) return;
      addToast(extractErrorMessage(err, "查看成图请求失败"), "error");
      setImageRequestPreviewOpen(false);
    } finally {
      if (imageRequestPreviewScopeRef.current === previewScope) {
        setImageRequestPreviewLoading(false);
      }
    }
  };

  const handlePreviewFrameImageRequest = async (prompt: ChapterVideoPrompt) => {
    let requests: Array<Record<string, unknown>> | null;
    try {
      requests = buildFrameImagePreviewInputs(prompt);
    } catch (err) {
      addToast(extractErrorMessage(err, "预览成图请求失败"), "error");
      return;
    }
    if (!requests) return;
    await previewImageRequests(requests);
  };

  const collectReferenceMediaRows = (
    kind: ReferenceMediaKind,
    rows: ReferenceMediaBindingRow[],
    purpose: ReferenceMediaInputPurpose,
  ): ReferenceMediaBindingRow[] | null => {
    const activeRows = normalizeReferenceMediaRows(rows);
    const meta = REFERENCE_MEDIA_KIND_META[kind];
    if (purpose === "generate" && activeRows.length > REFERENCE_MEDIA_LIMITS[kind]) {
      addToast(`${meta.label}最多允许 ${REFERENCE_MEDIA_LIMITS[kind]} 个输入`, "error");
      return null;
    }
    if (purpose === "preview") {
      return activeRows;
    }
    for (const [index, row] of activeRows.entries()) {
      const token = `@${meta.tokenPrefix}_${index + 1}`;
      if (row.url && row.upload) {
        addToast(`${token} 不能同时使用 URL 和本地文件`, "error");
        return null;
      }
      if (!row.url && !row.upload) {
        addToast(`${token} 缺少 URL 或本地文件`, "error");
        return null;
      }
      if (row.url && !validReferenceMediaUrl(row.url)) {
        addToast(`${token} URL 必须以小写 http:// 或 https:// 开头`, "error");
        return null;
      }
      if (!row.usage) {
        addToast(`${token} 缺少用途描述`, "error");
        return null;
      }
    }
    return activeRows;
  };

  const buildReferenceMediaVideoInput = (
    segment: ReferenceMediaSegmentState,
    purpose: ReferenceMediaInputPurpose = "generate",
  ): ReferenceMediaVideoInput | null => {
    if (referenceMediaDraft.storageError) {
      addToast(
        `${referenceMediaDraft.storageError}；请重新编辑后再${
          purpose === "preview" ? "查看模型请求" : "生成视频"
        }`,
        "error",
      );
      return null;
    }
    const imageRows = collectReferenceMediaRows("image", referenceMediaDraft.imageRows, purpose);
    if (!imageRows) return null;
    const videoRows = collectReferenceMediaRows("video", referenceMediaDraft.videoRows, purpose);
    if (!videoRows) return null;
    const audioRows = collectReferenceMediaRows("audio", referenceMediaDraft.audioRows, purpose);
    if (!audioRows) return null;
    if (imageRows.length === 0 && videoRows.length === 0 && audioRows.length === 0) {
      if (purpose !== "preview") {
        addToast("参考媒体模式至少需要一个图片、视频或音频输入", "error");
        return null;
      }
    }
    const rowsByKind: ReferenceMediaRowsByKind = {
      image: imageRows,
      video: videoRows,
      audio: audioRows,
    };
    const prompt = buildReferenceMediaPrompt(referenceMediaDraft.prompt, rowsByKind);
    if ([...prompt].length < REFERENCE_MEDIA_PROMPT_MIN_CHARS) {
      if (purpose !== "preview") {
        addToast(
          `参考媒体 prompt 至少需要 ${REFERENCE_MEDIA_PROMPT_MIN_CHARS} 个字符，请补全画面、动作、风格和媒体用途`,
          "error",
        );
        return null;
      }
    }
    const input: ReferenceMediaVideoInput = {
      segment_prompt_id: segment.promptId,
      shot_number: segment.shotNumber,
      prompt,
      image_files: imageRows.flatMap((row) => row.url ? [row.url] : []),
      image_uploads: imageRows.flatMap((row) => row.upload ? [row.upload] : []),
      video_files: videoRows.flatMap((row) => row.url ? [row.url] : []),
      video_uploads: videoRows.flatMap((row) => row.upload ? [row.upload] : []),
      audio_files: audioRows.flatMap((row) => row.url ? [row.url] : []),
      audio_uploads: audioRows.flatMap((row) => row.upload ? [row.upload] : []),
      duration_secs: options.durationSecs,
      aspect_ratio: options.aspectRatio,
      config_id: null,
    };
    return input;
  };

  const referenceMediaSegmentFor = (
    operation: string,
    requireWritable = true,
  ): ReferenceMediaSegmentState | null => {
    if (!chapter) return null;
    if (!referenceMediaSegment) {
      addToast("请从片段提示词卡片选择参考媒体方案", "error");
      return null;
    }
    if (requireWritable && chapterGenerating) {
      addToast(chapterGeneratingMessage(chapter, operation), "error");
      return null;
    }
    if (videoOptionsStateError) {
      addToast(videoOptionsStateError, "error");
      return null;
    }
    if (!requireWritable) return referenceMediaSegment;
    if (activeVideoConfig?.capabilities?.supports_reference_media !== true) {
      addToast("当前视频服务不支持参考媒体生成", "error");
      return null;
    }
    if (effectiveDurationLimits.configError) {
      addToast(effectiveDurationLimits.configError, "error");
      return null;
    }
    if (!isDurationAllowed(options.durationSecs, effectiveDurationLimits)) {
      addToast("视频时长不在默认视频模型支持范围内", "error");
      return null;
    }
    return referenceMediaSegment;
  };

  const handlePreviewReferenceMediaVideoRequest = async () => {
    if (!chapter) return;
    const segment = referenceMediaSegmentFor("预览参考媒体视频请求", false);
    if (!segment) return;
    const input = buildReferenceMediaVideoInput(segment, "preview");
    if (!input) return;
    const previewScope = promptRequestPreviewScopeRef.current;
    setPromptRequestPreviewOpen(true);
    setPromptRequestPreviewLoading(true);
    setPromptRequestPreviewData(null);
    try {
      const preview = await previewReferenceMediaVideoRequest(projectId, chapter.id, input);
      if (promptRequestPreviewScopeRef.current !== previewScope) return;
      setPromptRequestPreviewData(preview);
    } catch (err) {
      if (promptRequestPreviewScopeRef.current !== previewScope) return;
      addToast(extractErrorMessage(err, "预览参考媒体视频请求失败"), "error");
      setPromptRequestPreviewOpen(false);
    } finally {
      if (promptRequestPreviewScopeRef.current === previewScope) {
        setPromptRequestPreviewLoading(false);
      }
    }
  };

  const handleGenerateReferenceMediaVideo = async () => {
    if (!chapter) return;
    const segment = referenceMediaSegmentFor("生成参考媒体视频");
    if (!segment) return;
    const generateInputError = referenceMediaGenerateDraftError(
      referenceMediaDraft.prompt,
      referenceMediaRowsByKindFromDraft(referenceMediaDraft),
      referenceMediaDraft.storageError,
    );
    if (generateInputError) {
      addToast(generateInputError, "error");
      return;
    }
    const input = buildReferenceMediaVideoInput(segment);
    if (!input) return;
    setActing("reference-media-video");
    try {
      const handle = await startVideoGeneration({
        project_id: projectId,
        operation: {
          kind: "reference_media",
          chapter_id: chapter.id,
          input,
        },
      });
      setVideoTaskId(handle.task_id);
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: "参考媒体视频生成",
          startMessage: "正在准备参考媒体视频生成",
          successMessage: "参考媒体视频生成完成",
          errorMessage: "参考媒体视频生成失败",
          total: 7,
          cancelLabel: "停止生成",
        },
        async (task) => {
          const [row] = await verifyVideoGenerationResult(
            task,
            projectId,
            chapter.id,
          );
          if (!row) throw new Error("参考媒体视频生成已结束，但没有正式视频结果");
          setSelectedVideoId(row.id);
          setPendingVideoId(null);
        },
      );
      addToast("参考媒体视频生成完成", "success");
    } catch (err) {
      const message = extractErrorMessage(err, "参考媒体视频生成失败");
      addToast(message, "error");
    } finally {
      await loadAll();
      setActing(null);
      setVideoTaskId(null);
    }
  };

  const handleImportReferenceMediaVideo = async () => {
    if (!chapter) return;
    const segment = referenceMediaSegmentFor("导入参考媒体视频");
    if (!segment) return;
    if (referenceMediaDraft.storageError) {
      addToast(`${referenceMediaDraft.storageError}；请重新编辑后再导入`, "error");
      return;
    }
    const rowsByKind = normalizeReferenceMediaRowsByKind(
      referenceMediaRowsByKindFromDraft(referenceMediaDraft),
    );
    const prompt = buildReferenceMediaPrompt(referenceMediaDraft.prompt, rowsByKind);
    if (!prompt.trim()) {
      addToast("参考媒体视频 prompt 不能为空", "error");
      return;
    }
    if (!Number.isInteger(options.durationSecs) || options.durationSecs <= 0) {
      addToast("参考媒体视频时长无效", "error");
      return;
    }

    let file: File | null = null;
    try {
      file = await pickLocalFile(supportedVideoAccept());
    } catch (err) {
      addToast(extractErrorMessage(err, "打开文件选择器失败"), "error");
      return;
    }
    if (!file) return;
    if (file.size > MAX_UPLOAD_VIDEO_BYTES) {
      addToast(`参考媒体视频不能超过 ${MAX_UPLOAD_VIDEO_BYTES / 1024 / 1024}MB`, "error");
      return;
    }
    const mimeType = videoMimeForUploadFile(file);
    if (!mimeType) {
      addToast("参考媒体视频必须是 mp4、webm、mov 或 m4v 文件", "error");
      return;
    }

    setActing("reference-media-import");
    try {
      const dataBase64 = await readFileAsBase64(file);
      const row = await uploadReferenceMediaVideo(projectId, chapter.id, {
        file_name: file.name,
        mime_type: mimeType,
        data_base64: dataBase64,
        prompt,
        duration_secs: options.durationSecs,
        aspect_ratio: options.aspectRatio,
        segment_prompt_id: segment.promptId,
        shot_number: segment.shotNumber,
      });
      setSelectedVideoId(row.id);
      setPendingVideoId(null);
      addToast("参考媒体视频已导入", "success");
      await loadAll();
    } catch (err) {
      addToast(extractErrorMessage(err, "导入参考媒体视频失败"), "error");
    } finally {
      setActing(null);
    }
  };

  if (!chapter) {
    return <EmptyHint text="请先在写作桌选择已生成的章节。" />;
  }

  const referenceMediaWorkspace = workflowMode === "reference_media" ? (
    <ReferenceMediaWorkspace
      prompt={referenceMediaDraft.prompt}
      imageRows={referenceMediaDraft.imageRows}
      videoRows={referenceMediaDraft.videoRows}
      audioRows={referenceMediaDraft.audioRows}
      draftStorageError={referenceMediaDraft.storageError}
      durationSecs={options.durationSecs}
      aspectRatio={options.aspectRatio}
      durationLimits={effectiveDurationLimits}
      activeVideoConfig={activeVideoConfig}
      videos={referenceMediaSegmentVideos}
      totalVideoAssetCount={chapterVideoAssetCount}
      selectedVideo={selectedVideo}
      selectedVideoId={selectedVideo?.id ?? selectedVideoId}
      selectedVideoPath={selectedVideoPath}
      trimStart={trimStart}
      trimEnd={trimEnd}
      acting={acting}
      previewing={promptRequestPreviewLoading}
      onPromptChange={setReferenceMediaPrompt}
      onImageRowsChange={setReferenceMediaImageRows}
      onVideoRowsChange={setReferenceMediaVideoRows}
      onAudioRowsChange={setReferenceMediaAudioRows}
      onDurationSecsChange={(durationSecs) =>
        setVideoOptions((prev) => ({ ...prev, durationSecs }))
      }
      onAspectRatioChange={(aspectRatio) =>
        setVideoOptions((prev) => ({ ...prev, aspectRatio }))
      }
      onPreview={handlePreviewReferenceMediaVideoRequest}
      onGenerate={handleGenerateReferenceMediaVideo}
      onImport={handleImportReferenceMediaVideo}
      onSelectVideo={setSelectedVideoId}
      onAdoptVideo={handleAdoptSegmentVideo}
      onPollVideo={(video) => void handlePollVideo(video)}
      onPreviewVideoRequest={handlePreviewVideoRequestForVideo}
      onToggleVideoFavorite={(video) => void handleToggleVideoFavorite(video)}
      onUpdateVideo={handleUpdateVideo}
      onDeleteVideo={(video) => void handleDeleteVideo(video)}
      onDeleteAllVideoAssets={handleDeleteAllVideoAssets}
      onTrimStartChange={setTrimStart}
      onTrimEndChange={setTrimEnd}
      onTrim={handleTrim}
    />
  ) : undefined;

  return (
    <>
      <div className="flex h-full min-h-0 flex-col gap-3">
        {videoPersistedStateError && (
          <div role="alert" className="shrink-0 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-xs leading-relaxed text-error">
            {videoPersistedStateError}；请重新选择当前视频生成模式、工作区或条目后继续。
          </div>
        )}
        <VideoWorkspaceHeader
          tabs={visibleWorkspaceTabs}
          activeTab={activeWorkspaceTab}
          onTabChange={setWorkspaceTab}
        >
          {workflowMode === "reference_media" ? (
            <span>参考媒体 · 当前片段</span>
          ) : (
            <>
              <VideoPromptReferenceInputDialog
                images={selectableReferenceImages}
                chapter={chapter}
                workflowMode={workflowMode}
                currentContentHash={currentContentHash}
                generationReferenceIds={generationReferenceIds}
                videoLibraryImageIds={videoLibraryImageIds}
                invalidGenerationReferenceIds={invalidGenerationReferenceIds}
                invalidVideoLibraryImageIds={invalidVideoLibraryImageIds}
                onGenerationReferenceIdsChange={setGenerationReferenceIds}
                onVideoLibraryImageIdsChange={setVideoLibraryImageIds}
                onReferenceLimit={() =>
                  addToast(
                    `${workflowMode === "chat_video" ? "Chat上传" : "生成参考"}最多只能选择 ${
                      workflowMode === "chat_video"
                        ? MAX_CHAT_REFERENCE_IMAGES
                        : MAX_REFERENCE_IMAGES
                    } 张`,
                    "error",
                  )
                }
                onSelectionError={(message) => addToast(message, "error")}
                onClearInvalidGenerationReferences={clearInvalidGenerationReferences}
                onClearInvalidVideoLibraryImages={clearInvalidVideoLibraryImages}
              />
              <span>{VIDEO_WORKFLOW_MODE_LABELS[workflowMode]} · 当前片段</span>
            </>
          )}
        </VideoWorkspaceHeader>

        {activeWorkspaceTab === "materials" ? (
          <div className="min-h-0 flex-1">
            <VideoMaterialWorkspace
              projectId={profileMaterialProjectId}
              onToast={addToast}
              runProgressTask={runProgressTask}
              onRefreshVideoData={loadAll}
            />
          </div>
        ) : activeWorkspaceTab === "frames" ? (
          <VideoPlanningWorkspace
            prompt={frameSegmentPlanPrompt}
            acting={acting}
            chapterGenerating={chapterGenerating}
            onGenerate={(regenerateExisting) =>
              handleGenerateSegmentPlan({ regenerateExisting })
            }
            onDelete={(prompt) => void handleDeletePromptRow(prompt)}
          />
        ) : activeWorkspaceTab === "assembly" ? (
          <VideoAssemblyWorkspace
            key="assembly:mixed"
            projectId={projectId}
            chapter={chapter}
            workflowMode="mixed"
            videos={videos}
            onToast={addToast}
            onRefresh={loadAll}
          />
        ) : activeWorkspaceTab === "postprocess" ? (
          <VideoPostProcessWorkspace
            key="postprocess:mixed"
            projectId={projectId}
            chapter={chapter}
            workflowMode="mixed"
            videos={videos}
            onToast={addToast}
            onRefresh={loadAll}
          />
        ) : (
          <VideoGenerationWorkspace
                chapter={chapter}
                prompts={allSegmentPrompts}
                generationPrompts={currentLanguageFinalVideoPrompts}
                selectedPrompt={selectedPrompt}
                selectedPromptId={selectedPromptId}
                workflowMode={workflowMode}
                pendingVideo={pendingVideo}
                selectedPromptStale={selectedPromptStale}
                expectedBoundaryFrameCount={expectedBoundaryFrameCount}
                expectedVideoPromptCount={expectedVideoPromptCount}
                options={options}
                durationLimits={effectiveDurationLimits}
                providerBackend={activeVideoConfig?.backend ?? null}
                providerOptionsJson={activeVideoConfig?.provider_options_json ?? null}
                providerCapabilities={activeVideoConfig?.capabilities}
                promptInputSelectionError={promptInputSelectionError}
                loading={loading}
                acting={acting}
                chapterGenerating={chapterGenerating}
                chapterContentReady={chapterContentReady}
                previewingPromptRequests={promptRequestPreviewLoading}
                draft={draft}
                selectedPromptSceneSummary={selectedPromptSceneSummary}
                selectedPromptRunning={selectedPromptRunning}
                selectedPromptDirty={selectedPromptDirty}
                staleSavedReferenceCount={staleSavedReferenceCount}
                savedReferenceIdsError={savedReferenceIdsError}
                invalidPromptReferenceIds={invalidPromptReferenceIds}
                systemReferenceIds={systemPromptReferenceIds}
                validPromptReferenceIds={validPromptReferenceIds}
                boundReferenceImages={boundReferenceImages}
                framePrompts={currentLanguageFramePrompts}
                frameImages={videoFrameImages}
                videos={videos}
                selectableReferenceImages={selectableReferenceImages}
                totalVideoAssetCount={chapterVideoAssetCount}
                totalVideoChainPromptCount={chapterVideoChainPromptCount}
                selectedVideo={selectedVideo}
                selectedVideoId={selectedVideo?.id ?? selectedVideoId}
                selectedVideoPath={selectedVideoPath}
                trimStart={trimStart}
                trimEnd={trimEnd}
                referenceImageScopeLabel={(img) =>
                  referenceImageScopeLabel(img, chapter, currentContentHash)
                }
                onLoad={loadAll}
                onOptionsChange={setVideoOptions}
                onGeneratePromptRequests={(requestOptions) =>
                  handleGenerateVideoPrompts({
                    ...requestOptions,
                    targetShotNumber:
                      requestOptions?.targetShotNumber ?? workflowTargetShotRef.current ?? undefined,
                    promptBatchSize:
                      workflowTargetShotRef.current != null
                        ? 1
                        : requestOptions?.promptBatchSize,
                  })
                }
                onPreviewPromptRequests={handlePreviewVideoPromptRequests}
                onSelectPrompt={setSelectedPromptId}
                onDeletePrompt={(prompt) => void handleDeletePromptRow(prompt)}
                onDeleteAllVideoPrompts={handleDeleteAllVideoPrompts}
                onPollPendingVideo={handlePollVideo}
                onWorkflowModeChange={handleSegmentWorkflowModeChange}
                onUpdateGuidance={handleUpdateSegmentGuidance}
                onGenerateFramePrompts={() => handleGenerateFramePrompts({ runUntilComplete: true })}
                onPreviewFrameImage={handlePreviewFrameImageRequest}
                onGenerateFrameImage={(prompt) => handleGenerateFrameImages(prompt)}
                onImportFrameImage={(prompt) => handleImportFrameImage(prompt)}
                onDraftChange={setDraft}
                onClearInvalidPromptReferences={clearInvalidPromptReferences}
                onSavePrompt={handleSavePrompt}
                onOptimizePrompt={(prompt) => void handleOptimizePromptRow(prompt)}
                onPreviewGenerateVideo={handlePreviewGenerateVideoRequest}
                onCopyPrompt={handleCopyChatVideoPrompt}
                onPreviewVideoRequest={handlePreviewVideoRequestForVideo}
                onGenerateVideo={handleGenerateVideo}
                onGenerateAllVideos={handleGenerateAllVideos}
                onImportVideo={handleImportVideo}
                onSelectVideo={setSelectedVideoId}
                onAdoptVideo={handleAdoptSegmentVideo}
                onPollVideo={(video) => void handlePollVideo(video)}
                onToggleVideoFavorite={(video) => void handleToggleVideoFavorite(video)}
                onUpdateVideo={handleUpdateVideo}
                onDeleteVideo={(video) => void handleDeleteVideo(video)}
                onDeleteAllVideoAssets={handleDeleteAllVideoAssets}
                onTrimStartChange={setTrimStart}
                onTrimEndChange={setTrimEnd}
                onTrim={handleTrim}
                workspaceOverride={referenceMediaWorkspace}
          />
        )}
      </div>
      <ImageActionDialog
        image={imageDetails}
        busyId={imageBusyId}
        scoringId={imageScoringId}
        scoreInput={imageScoreInput}
        onClose={handleCloseImageDetails}
        onToggleFavorite={(image) => void handleToggleImageFavorite(image)}
        onUpdate={(image, input) => handleUpdateImage(image, input)}
        onScoreInputChange={setImageScoreInput}
        onScore={(image) => void handleScoreImage(image)}
        onDelete={(image) => void handleDeleteImage(image)}
      />
      <RequestPreviewModal
        open={promptRequestPreviewOpen}
        loading={promptRequestPreviewLoading}
        data={promptRequestPreviewData}
        fallbackTitle="完整提示词与模型请求"
        onClose={() => setPromptRequestPreviewOpen(false)}
      />
      <RequestPreviewModal
        open={imageRequestPreviewOpen}
        loading={imageRequestPreviewLoading}
        data={imageRequestPreviewData}
        fallbackTitle="成图模型请求"
        onClose={() => setImageRequestPreviewOpen(false)}
      />
      <RequestPreviewModal
        open={videoRequestPreviewOpen}
        loading={videoRequestPreviewLoading}
        data={videoRequestPreviewData}
        fallbackTitle="视频模型请求"
        onClose={() => setVideoRequestPreviewOpen(false)}
      />
    </>
  );
};
