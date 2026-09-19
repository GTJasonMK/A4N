import React, {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Clapperboard,
  Film,
  ImagePlus,
  PanelTop,
  RotateCcw,
  Scissors,
} from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import { generationClient } from "../../api/generationTasks";
import {
  previewBatchImagesRequest,
  type GeneratedImage,
} from "../../api/image";
import type { ShootingSceneV1 } from "../../api/shootingScript";
import type { ModelRequestPreview } from "../../api/requestPreview";
import {
  listChapterVideoPrompts,
  commitInsertedSegmentDraft,
  deleteInsertedSegment,
  moveInsertedSegment,
  previewReferenceMediaVideoRequest,
  selectSegmentVideo,
  startChapterVideoInsertedSegmentDraft,
  startVideoGeneration,
  updateChapterVideoSegmentPlan,
  updateChapterVideoSegmentRecipe,
  updateVideoSegmentPlanDraftRecipe,
  uploadReferenceMediaVideo,
  verifyVideoGenerationResult,
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type ReferenceMediaVideoInput,
  type VideoGenConfig,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentPlanV2,
  type VideoSegmentWorkflowMode,
  type InsertedSegmentGenerationOptions,
  type VideoTimelineSummary,
} from "../../api/video";
import { usePersistedState } from "../../hooks/usePersistedState";
import { EmptyHint, type ChapterRow } from "./shared";
import { chooseDownstreamConflictStrategy } from "./dependencyConflict";
import { useProgressTask } from "../../hooks/useProgressTask";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import { textInputValue } from "../../utils/text";
import { asJsonRecord } from "../../utils/jsonRecord";
import { copyTextToClipboard } from "../../utils/clipboard";
import { sha256Hex } from "../../utils/contentHash";
import { readFileAsBase64 } from "../../utils/fileUpload";
import { isCancelledOperation } from "../../utils/cancellation";
import {
  pickVideoUploadFile,
  type VideoUploadFile,
} from "../../utils/videoUpload";
import { VideoGenerationWorkspace } from "./video/VideoGenerationWorkspace";
import { loadShootingScriptOrProgress } from "./loadShootingScriptData";
import {
  InsertedSegmentTimelinePanel,
  type InsertedSegmentTimelinePanelHandle,
} from "./video/InsertedSegmentTimelinePanel";
import {
  createReferenceMediaDraft,
  normalizeReferenceMediaRows,
  ReferenceMediaWorkspace,
  referenceMediaDraftScopeKey,
  referenceMediaDraftError,
  referenceMediaDraftFromRecipe,
  referenceMediaRecipeFromDraft,
  referenceMediaRowsByKindFromDraft,
  VideoReferenceInputSlots,
  type ReferenceMediaBindingRow,
  type ReferenceMediaDraft,
} from "./video/referenceMediaWorkspace";
import {
  buildVideoFrameImageTraceMetadata,
  buildVideoFrameImageRequest,
  boundaryFramePromptChainTo,
  CHAT_VIDEO_DURATION_LIMITS,
  completedSegmentPlanPreviewItems,
  DEFAULT_DRAFT,
  DEFAULT_VIDEO_OPTIONS,
  frameImageReferencesForPrompt,
  guidanceModeForWorkflowChange,
  hasPendingProviderJob,
  isBoundaryFramePrompt,
  isSegmentPlanDraftCheckpointKey,
  isDurationAllowed,
  latestVideoFrameImageForPrompt,
  MAX_CHAT_REFERENCE_IMAGES,
  MAX_REFERENCE_IMAGES,
  parseNullableNumber,
  parseVideoOptions,
  promptLengthError,
  referenceLabelOrderError,
  serializeNullableNumber,
  serializeVideoOptions,
  shiftReferenceLabels,
  VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
  VIDEO_FINAL_PROMPT_TYPE,
  VIDEO_FRAME_PROMPT_TYPE,
  VIDEO_FIRST_FRAME_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_TYPE,
  latestVideoSegmentPlanPrompt,
  readVideoSegmentRecipes,
  videoOptionsFromSegmentRecipe,
  videoFrameImageContractForPromptType,
  videoSegmentCardsFromPlanPrompt,
  videoSegmentCardsFromSegments,
  videoSegmentRecipeFrameFingerprintJson,
  videoSegmentRecipeGenerationFingerprintJson,
  videoExecutionSegmentIdentityMatchesPrompt,
  videoExecutionSegmentIdentityMatchesVideo,
  videoExecutionSegmentIdentityIndex,
  videoExecutionIdentityOfPrompt,
  videoExecutionIdentityOfVideo,
  videoExecutionSegmentNumberInPlanByLineage,
  videoGuidanceConfigurationError,
  videoTailFrameSourceVideoId,
  finalVideoExecutionIdentityIsComplete,
  videoOptionsValidationError,
  videoAspectRatiosForImageSizes,
  videoWorkflowModeLabel,
  type PromptDraft,
  type VideoSegmentCardPlan,
  type VideoGenOptions,
} from "./video/videoTabUtils";
import {
  activeVideoSegmentPlanTask,
  loadedSegmentPlanDraftIsActive,
  loadVideoSegmentPlanDraft,
  loadVideoTabData,
  type LoadedSegmentPlanDraft,
} from "./video/loadVideoTabData";
import { useChapterContentHash } from "./video/useChapterContentHash";
import { useVideoImageActions } from "./video/useVideoImageActions";
import {
  runVideoPromptGenerationTask,
  useVideoPromptGenerationActions,
} from "./video/useVideoPromptGenerationActions";
import {
  useVideoRecordActions,
  videoPromptDownstreamCount,
} from "./video/useVideoRecordActions";
import { useVideoTabDerivedState } from "./video/useVideoTabDerivedState";
import { useI18n, type MessageTranslator } from "../../i18n";
import {
  VideoWorkspaceHeader,
  type VideoWorkspaceTab,
} from "./video/VideoWorkspaceHeader";

const VideoMaterialWorkspace = lazy(() =>
  import("./video/VideoMaterialWorkspace").then((module) => ({
    default: module.VideoMaterialWorkspace,
  })),
);
const VideoPlanningWorkspace = lazy(() =>
  import("./video/VideoPlanningWorkspace").then((module) => ({
    default: module.VideoPlanningWorkspace,
  })),
);
const VideoAssemblyWorkspace = lazy(() =>
  import("./video/VideoAssemblyWorkspace").then((module) => ({
    default: module.VideoAssemblyWorkspace,
  })),
);
const VideoPostProcessWorkspace = lazy(() =>
  import("./video/VideoPostProcessWorkspace").then((module) => ({
    default: module.VideoPostProcessWorkspace,
  })),
);
const ImageActionDialog = lazy(() =>
  import("./images/ImageActionDialog").then((module) => ({
    default: module.ImageActionDialog,
  })),
);
const RequestPreviewModal = lazy(() =>
  import("../../components/feedback/RequestPreviewModal").then((module) => ({
    default: module.RequestPreviewModal,
  })),
);

interface VideoTabProps {
  projectId: number;
  fixedChapter?: ChapterRow | null;
  imageProjectIds?: number[];
  materialProjectId?: number;
}

interface PersistedWorkspaceTabState {
  tab: VideoWorkspaceTab;
  storageError: string | null;
}

interface PersistedNullableIdState {
  id: number | null;
  storageError: string | null;
}

type PersistedVideoOptionsState =
  | { options: VideoGenOptions }
  | { options: null; storageError: string; raw: string };

interface ReferenceMediaSegmentState {
  segmentPlanPromptId: number;
  segmentNumber: number;
  targetSegment: {
    source_shot_id: string;
    segment_index_in_shot: number;
  };
}

const createPersistedWorkspaceTabState = (
  tab: VideoWorkspaceTab,
  storageError: string | null = null,
): PersistedWorkspaceTabState => ({ tab, storageError });

const parsePersistedWorkspaceTabState = (
  raw: string,
  defaultTab: VideoWorkspaceTab,
  allowedTabs: readonly VideoWorkspaceTab[],
  t: MessageTranslator,
): PersistedWorkspaceTabState => {
  if (allowedTabs.includes(raw as VideoWorkspaceTab)) {
    return createPersistedWorkspaceTabState(raw as VideoWorkspaceTab);
  }
  return createPersistedWorkspaceTabState(
    defaultTab,
    t("video.chapter.invalidPersistedState", { raw: raw || "<empty>" }),
  );
};

const serializePersistedWorkspaceTabState = (
  state: PersistedWorkspaceTabState,
): string => state.tab;

const createPersistedNullableIdState = (
  id: number | null,
  storageError: string | null = null,
): PersistedNullableIdState => ({ id, storageError });

const parsePersistedNullableIdState = (
  raw: string,
  label: string,
  t: MessageTranslator,
): PersistedNullableIdState => {
  try {
    return createPersistedNullableIdState(parseNullableNumber(raw, t));
  } catch (err) {
    return createPersistedNullableIdState(
      null,
      t("video.chapter.invalidSelectionStorage", {
        label,
        error: extractErrorMessage(err, t("common.unknown")),
      }),
    );
  }
};

const serializePersistedNullableIdState = (
  state: PersistedNullableIdState,
): string => serializeNullableNumber(state.id);

const validPersistedVideoOptionsState = (
  options: VideoGenOptions,
): PersistedVideoOptionsState => ({ options });

const parsePersistedVideoOptionsState = (
  raw: string,
  t: MessageTranslator,
): PersistedVideoOptionsState => {
  try {
    return validPersistedVideoOptionsState(parseVideoOptions(raw, t));
  } catch (err) {
    return {
      options: null,
      storageError: t("video.chapter.invalidPlanningStorage", {
        error: extractErrorMessage(err, t("common.unknown")),
      }),
      raw,
    };
  }
};

const serializePersistedVideoOptionsState = (
  state: PersistedVideoOptionsState,
  t: MessageTranslator,
): string => {
  if (state.options) return serializeVideoOptions(state.options, t);
  return state.raw;
};

export const VideoTab: React.FC<VideoTabProps> = ({
  projectId,
  fixedChapter,
  imageProjectIds,
  materialProjectId,
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const {
    error: imageSizeError,
    loading: imageSizeLoading,
    sizes: imageSupportedSizes,
  } = useImageGenerationSize();
  const supportedVideoAspectRatios = useMemo(
    () =>
      imageSizeLoading || imageSizeError
        ? []
        : videoAspectRatiosForImageSizes(imageSupportedSizes),
    [imageSizeError, imageSizeLoading, imageSupportedSizes],
  );
  const workspaceTabs = useMemo(
    () => [
      { id: "materials" as const, label: t("video.chapter.tabMaterials"), icon: <ImagePlus size={13} /> },
      { id: "frames" as const, label: t("video.chapter.tabPlanning"), icon: <PanelTop size={13} /> },
      { id: "video" as const, label: t("video.chapter.tabGeneration"), icon: <Film size={13} /> },
      { id: "assembly" as const, label: t("video.chapter.tabAssembly"), icon: <Clapperboard size={13} /> },
      { id: "postprocess" as const, label: t("video.chapter.tabPostprocess"), icon: <Scissors size={13} /> },
    ],
    [t],
  );
  const { runProgressTask } = useProgressTask();
  const chapter = fixedChapter ?? null;
  const chapterId = chapter?.id ?? null;
  const profileMaterialProjectId = materialProjectId ?? projectId;
  const chapterDataKey =
    chapter && Number.isFinite(chapter.id)
      ? `${projectId}:${chapter.id}`
      : null;
  const imageProjectIdsKey = (imageProjectIds ?? [projectId])
    .filter((id) => Number.isFinite(id) && id > 0)
    .join(",");
  const videoDataKey = chapterDataKey
    ? `${chapterDataKey}:${imageProjectIdsKey}`
    : null;
  const chapterStorageKey =
    chapter && Number.isFinite(chapter.id)
      ? `a4n:video:${projectId}:${chapter.id}`
      : null;
  const { currentContentHash, chapterContentReady, chapterContentHashError } =
    useChapterContentHash(chapter);
  const planningOptionsInitialValue = useMemo<PersistedVideoOptionsState>(
    () =>
      validPersistedVideoOptionsState({
        ...DEFAULT_VIDEO_OPTIONS,
        durationOptions: [...DEFAULT_VIDEO_OPTIONS.durationOptions],
      }),
    [chapterStorageKey],
  );
  const [planningOptionsState, setPlanningOptionsState] =
    usePersistedState<PersistedVideoOptionsState>(
      chapterStorageKey ? `${chapterStorageKey}:segment-plan-options` : null,
      planningOptionsInitialValue,
      {
        parse: (raw) => parsePersistedVideoOptionsState(raw, t),
        serialize: (state) => serializePersistedVideoOptionsState(state, t),
      },
    );
  const planningOptions = planningOptionsState.options;
  const setPlanningOptions = useCallback<
    React.Dispatch<React.SetStateAction<VideoGenOptions>>
  >(
    (action) =>
      setPlanningOptionsState((previous) => {
        if (!previous.options) {
          throw new Error(t("video.chapter.invalidPlanningOptions"));
        }
        const options =
          typeof action === "function" ? action(previous.options) : action;
        return validPersistedVideoOptionsState(options);
      }),
    [setPlanningOptionsState, t],
  );
  const resetPlanningOptions = useCallback(
    () =>
      setPlanningOptionsState(
        validPersistedVideoOptionsState({
          ...DEFAULT_VIDEO_OPTIONS,
          durationOptions: [...DEFAULT_VIDEO_OPTIONS.durationOptions],
        }),
      ),
    [setPlanningOptionsState],
  );
  const videoActionScopeKey = videoDataKey;
  const [acting, setActing] = useState<string | null>(null);
  const actionScopeRef = useRef(videoActionScopeKey);
  const actionBusyRef = useRef<string | null>(null);
  useEffect(() => {
    actionScopeRef.current = videoActionScopeKey;
  }, [videoActionScopeKey]);
  const beginVideoAction = useCallback((key: string): boolean => {
    if (actionBusyRef.current) return false;
    actionBusyRef.current = key;
    setActing(key);
    return true;
  }, []);
  const isCurrentVideoActionScope = useCallback(
    (scope: string | null): boolean => actionScopeRef.current === scope,
    [],
  );
  const finishVideoAction = useCallback((key: string): void => {
    if (actionBusyRef.current !== key) return;
    actionBusyRef.current = null;
    setActing((current) => (current === key ? null : current));
  }, []);
  const workspaceTabIds = workspaceTabs.map((tab) => tab.id);
  const defaultWorkspaceTab: VideoWorkspaceTab = "video";
  const workspaceTabInitialValue = useMemo(
    () => createPersistedWorkspaceTabState(defaultWorkspaceTab),
    [defaultWorkspaceTab, chapterStorageKey],
  );
  const [workspaceTabState, setWorkspaceTabState] =
    usePersistedState<PersistedWorkspaceTabState>(
      chapterStorageKey ? `${chapterStorageKey}:workspace-tab` : null,
      workspaceTabInitialValue,
      {
        parse: (raw) =>
          parsePersistedWorkspaceTabState(
            raw,
            defaultWorkspaceTab,
            workspaceTabIds,
            t,
          ),
        serialize: serializePersistedWorkspaceTabState,
      },
    );
  const workspaceTab = workspaceTabState.tab;
  const setWorkspaceTab = useCallback(
    (tab: VideoWorkspaceTab) =>
      setWorkspaceTabState(createPersistedWorkspaceTabState(tab)),
    [setWorkspaceTabState],
  );
  const [prompts, setPrompts] = useState<ChapterVideoPrompt[]>([]);
  const [timeline, setTimeline] = useState<VideoTimelineSummary | null>(null);
  const insertedSegmentPanelRef =
    useRef<InsertedSegmentTimelinePanelHandle>(null);
  const [segmentPlanPreview, setSegmentPlanPreview] = useState<
    Record<string, unknown>[]
  >([]);
  const [segmentPlanDraft, setSegmentPlanDraft] =
    useState<LoadedSegmentPlanDraft | null>(null);
  const [videos, setVideos] = useState<GeneratedVideo[]>([]);
  const [images, setImages] = useState<GeneratedImage[]>([]);
  const [shootingScenes, setShootingScenes] = useState<ShootingSceneV1[]>([]);
  const frameSegmentPlanPromptForCards = useMemo(
    () => latestVideoSegmentPlanPrompt(prompts, currentContentHash),
    [currentContentHash, prompts],
  );
  const segmentPlanDraftVisible =
    segmentPlanPreview.length > 0 &&
    loadedSegmentPlanDraftIsActive(
      segmentPlanDraft,
      acting === "generate-segment-plan",
    );
  const segmentPlanIdentity = segmentPlanDraftVisible
    ? segmentPlanDraft?.workflowRunId == null
      ? null
      : `workflow-run:${segmentPlanDraft.workflowRunId}`
    : frameSegmentPlanPromptForCards == null
      ? null
      : `prompt:${frameSegmentPlanPromptForCards.id}`;
  const segmentCardState = useMemo(() => {
    try {
      return {
        cards: segmentPlanDraftVisible
          ? videoSegmentCardsFromSegments(segmentPlanPreview, t)
          : videoSegmentCardsFromPlanPrompt(frameSegmentPlanPromptForCards, t),
        error: null as string | null,
      };
    } catch (error) {
      return {
        cards: [] as VideoSegmentCardPlan[],
        error: extractErrorMessage(
          error,
          t("video.chapter.planningLoadFailed", { error: t("common.unknown") }),
        ),
      };
    }
  }, [
    frameSegmentPlanPromptForCards,
    segmentPlanDraftVisible,
    segmentPlanPreview,
    t,
  ]);
  const segmentRecipeState = useMemo(
    () =>
      segmentPlanDraftVisible
        ? {
            recipes:
              segmentPlanDraft?.recipes ??
              new Map<number, VideoSegmentExecutionRecipe>(),
            error: null,
          }
        : readVideoSegmentRecipes(frameSegmentPlanPromptForCards, t),
    [
      frameSegmentPlanPromptForCards,
      segmentPlanDraft?.recipes,
      segmentPlanDraftVisible,
    ],
  );
  const persistedRecipeBySegmentRef = useRef(segmentRecipeState.recipes);
  useLayoutEffect(() => {
    persistedRecipeBySegmentRef.current = segmentRecipeState.recipes;
  }, [segmentPlanIdentity, segmentRecipeState.recipes]);
  const persistedNullIdInitialValue = useMemo(
    () => createPersistedNullableIdState(null),
    [chapterStorageKey],
  );
  const [selectedSegmentState, setSelectedSegmentState] =
    usePersistedState<PersistedNullableIdState>(
      chapterStorageKey && segmentPlanIdentity
        ? `${chapterStorageKey}:${segmentPlanIdentity}:selected-segment`
        : null,
      persistedNullIdInitialValue,
      {
        parse: (raw) =>
          parsePersistedNullableIdState(raw, t("video.chapter.segmentTab"), t),
        serialize: serializePersistedNullableIdState,
      },
    );
  const selectedSegmentNumber = selectedSegmentState.id;
  const setSelectedSegmentNumber = useCallback(
    (segmentNumber: number | null) => {
      setSelectedSegmentState(createPersistedNullableIdState(segmentNumber));
    },
    [setSelectedSegmentState],
  );
  const [segmentConfigOpen, setSegmentConfigOpen] = useState(false);
  useLayoutEffect(() => {
    setSegmentConfigOpen(false);
  }, [segmentPlanIdentity, videoDataKey]);
  const [recipeOverrides, setRecipeOverrides] = useState<
    Map<number, VideoSegmentExecutionRecipe>
  >(new Map());
  const recipeSaveBySegmentRef = useRef(new Map<number, Promise<void>>());
  const recipeSaveErrorRef = useRef(new Map<number, string>());
  const recipeSaveScopeRef = useRef(segmentPlanIdentity);
  const [savingRecipeSegments, setSavingRecipeSegments] = useState<Set<number>>(
    new Set(),
  );
  const [recipeSaveErrors, setRecipeSaveErrors] = useState<Map<number, string>>(
    new Map(),
  );
  useLayoutEffect(() => {
    recipeSaveScopeRef.current = segmentPlanIdentity;
    setRecipeOverrides(new Map());
    recipeSaveBySegmentRef.current.clear();
    recipeSaveErrorRef.current.clear();
    setSavingRecipeSegments(new Set());
    setRecipeSaveErrors(new Map());
  }, [segmentPlanIdentity]);
  const effectiveRecipeBySegment = useMemo(() => {
    const recipes = new Map(segmentRecipeState.recipes);
    for (const [segmentNumber, recipe] of recipeOverrides) {
      recipes.set(segmentNumber, recipe);
    }
    return recipes;
  }, [recipeOverrides, segmentRecipeState.recipes]);
  const effectiveRecipeBySegmentRef = useRef(effectiveRecipeBySegment);
  useLayoutEffect(() => {
    effectiveRecipeBySegmentRef.current = effectiveRecipeBySegment;
  }, [effectiveRecipeBySegment]);
  const [recipeFingerprintBySegment, setRecipeFingerprintBySegment] = useState<
    Map<number, string>
  >(new Map());
  const [frameRecipeFingerprintBySegment, setFrameRecipeFingerprintBySegment] =
    useState<Map<number, string | null>>(new Map());
  const [
    recipeFingerprintErrorsBySegment,
    setRecipeFingerprintErrorsBySegment,
  ] = useState<Map<number, string>>(new Map());
  const recipeFingerprintInputBySegmentRef = useRef(new Map<number, string>());
  useEffect(() => {
    let cancelled = false;
    const fingerprintInputs = new Map(
      [...effectiveRecipeBySegment.entries()].map(([segmentNumber, recipe]) => [
        segmentNumber,
        {
          generation: videoSegmentRecipeGenerationFingerprintJson(recipe),
          frame: videoSegmentRecipeFrameFingerprintJson(recipe),
        },
      ]),
    );
    const previousInputs = recipeFingerprintInputBySegmentRef.current;
    setRecipeFingerprintBySegment(
      (previous) =>
        new Map(
          [...previous].filter(
            ([segmentNumber]) =>
              previousInputs.get(segmentNumber) ===
              JSON.stringify(fingerprintInputs.get(segmentNumber)),
          ),
        ),
    );
    setFrameRecipeFingerprintBySegment(
      (previous) =>
        new Map(
          [...previous].filter(
            ([segmentNumber]) =>
              previousInputs.get(segmentNumber) ===
              JSON.stringify(fingerprintInputs.get(segmentNumber)),
          ),
        ),
    );
    setRecipeFingerprintErrorsBySegment(
      (previous) =>
        new Map(
          [...previous].filter(
            ([segmentNumber]) =>
              previousInputs.get(segmentNumber) ===
              JSON.stringify(fingerprintInputs.get(segmentNumber)),
          ),
        ),
    );
    recipeFingerprintInputBySegmentRef.current = new Map(
      [...fingerprintInputs].map(([segmentNumber, input]) => [
        segmentNumber,
        JSON.stringify(input),
      ]),
    );
    void Promise.all(
      [...fingerprintInputs.entries()].map(async ([segmentNumber, input]) => {
        const [generation, frame] = await Promise.all([
          sha256Hex(input.generation),
          sha256Hex(input.frame),
        ]);
        return [segmentNumber, generation, frame] as const;
      }),
    )
      .then((entries) => {
        if (cancelled) return;
        setRecipeFingerprintBySegment(
          new Map(
            entries
              .filter(
                (entry): entry is readonly [number, string, string | null] =>
                  entry[1] != null,
              )
              .map(([segmentNumber, generation]) => [
                segmentNumber,
                generation,
              ]),
          ),
        );
        setFrameRecipeFingerprintBySegment(
          new Map(
            entries
              .filter(
                (entry): entry is readonly [number, string | null, string] =>
                  entry[2] != null,
              )
              .map(([segmentNumber, , frame]) => [segmentNumber, frame]),
          ),
        );
        setRecipeFingerprintErrorsBySegment(
          new Map(
            entries.flatMap(([segmentNumber, generation, frame]) =>
              generation == null || frame == null
                ? [[segmentNumber, t("video.chapter.executionIdentityUnavailable")] as const]
                : [],
            ),
          ),
        );
      })
      .catch(() => {
        if (cancelled) return;
        setRecipeFingerprintBySegment(new Map());
        setFrameRecipeFingerprintBySegment(new Map());
        setRecipeFingerprintErrorsBySegment(
          new Map(
            [...fingerprintInputs.keys()].map((segmentNumber) => [
              segmentNumber,
              t("video.chapter.executionIdentityCheckFailed"),
            ]),
          ),
        );
      });
    return () => {
      cancelled = true;
    };
  }, [effectiveRecipeBySegment]);
  const selectedSegmentRecipe =
    selectedSegmentNumber == null
      ? null
      : (effectiveRecipeBySegment.get(selectedSegmentNumber) ?? null);
  const workflowMode = selectedSegmentRecipe?.workflow_mode ?? null;
  const segmentStorageKey =
    chapterStorageKey && segmentPlanIdentity && selectedSegmentNumber != null
      ? `${chapterStorageKey}:${segmentPlanIdentity}:segment:${selectedSegmentNumber}`
      : null;
  const [selectedPromptState, setSelectedPromptState] =
    usePersistedState<PersistedNullableIdState>(
      segmentStorageKey ? `${segmentStorageKey}:selected-prompt` : null,
      persistedNullIdInitialValue,
      {
        parse: (raw) =>
          parsePersistedNullableIdState(raw, t("video.chapter.promptRecord"), t),
        serialize: serializePersistedNullableIdState,
      },
    );
  const [selectedVideoState, setSelectedVideoState] =
    usePersistedState<PersistedNullableIdState>(
      segmentStorageKey ? `${segmentStorageKey}:selected-video` : null,
      persistedNullIdInitialValue,
      {
        parse: (raw) =>
          parsePersistedNullableIdState(raw, t("video.chapter.videoTab"), t),
        serialize: serializePersistedNullableIdState,
      },
    );
  const [pendingVideoState, setPendingVideoState] =
    usePersistedState<PersistedNullableIdState>(
      segmentStorageKey ? `${segmentStorageKey}:pending-video` : null,
      persistedNullIdInitialValue,
      {
        parse: (raw) =>
          parsePersistedNullableIdState(raw, t("workflow.taskVideoGeneration"), t),
        serialize: serializePersistedNullableIdState,
      },
    );
  const selectedVideoSelectionScopeRef = useRef<string | null | undefined>(
    undefined,
  );
  const selectedPromptId = selectedPromptState.id;
  const selectedVideoId = selectedVideoState.id;
  const pendingVideoId = pendingVideoState.id;
  const setSelectedPromptId = useCallback<
    React.Dispatch<React.SetStateAction<number | null>>
  >(
    (action) => {
      setSelectedPromptState((prev) =>
        createPersistedNullableIdState(
          typeof action === "function" ? action(prev.id) : action,
        ),
      );
    },
    [setSelectedPromptState],
  );
  const setSelectedVideoId = useCallback<
    React.Dispatch<React.SetStateAction<number | null>>
  >(
    (action) => {
      setSelectedVideoState((prev) =>
        createPersistedNullableIdState(
          typeof action === "function" ? action(prev.id) : action,
        ),
      );
    },
    [setSelectedVideoState],
  );
  const setPendingVideoId = useCallback<
    React.Dispatch<React.SetStateAction<number | null>>
  >(
    (action) => {
      setPendingVideoState((prev) =>
        createPersistedNullableIdState(
          typeof action === "function" ? action(prev.id) : action,
        ),
      );
    },
    [setPendingVideoState],
  );
  const options = videoOptionsFromSegmentRecipe(selectedSegmentRecipe);
  const videoLibraryImageIds =
    selectedSegmentRecipe?.video_library_image_ids ?? [];
  const generationReferenceIds =
    selectedSegmentRecipe?.reference_image_ids ?? [];
  const persistSegmentRecipe = useCallback(
    (recipe: VideoSegmentExecutionRecipe): Promise<void> => {
      const saveScope = segmentPlanIdentity;
      const planPromptId = segmentPlanDraftVisible
        ? null
        : frameSegmentPlanPromptForCards?.id;
      const draftWorkflowRunId = segmentPlanDraftVisible
        ? (segmentPlanDraft?.workflowRunId ?? null)
        : null;
      if (!draftWorkflowRunId && !planPromptId) {
        const message = segmentPlanDraftVisible
          ? t("video.chapter.segmentPlanDraftMissingWorkflow")
          : t("video.chapter.noSegmentPlanToSave");
        addToast(message, "error");
        return Promise.reject(new Error(message));
      }
      // 配方写入优先于已在途的旧读取，避免慢刷新覆盖刚保存的选择。
      promptLoadSeqRef.current += 1;
      segmentPlanDraftLoadSeqRef.current += 1;
      effectiveRecipeBySegmentRef.current = new Map(
        effectiveRecipeBySegmentRef.current,
      ).set(recipe.segment_number, recipe);
      setRecipeOverrides((previous) =>
        new Map(previous).set(recipe.segment_number, recipe),
      );
      recipeSaveErrorRef.current.delete(recipe.segment_number);
      setRecipeSaveErrors((previous) => {
        if (!previous.has(recipe.segment_number)) return previous;
        const next = new Map(previous);
        next.delete(recipe.segment_number);
        return next;
      });
      setSavingRecipeSegments((previous) =>
        new Set(previous).add(recipe.segment_number),
      );
      const save = (
        recipeSaveBySegmentRef.current.get(recipe.segment_number) ??
        Promise.resolve()
      )
        .catch(() => undefined)
        .then(async () => {
          try {
            let updatedRecipes: VideoSegmentExecutionRecipe[] | null = null;
            if (draftWorkflowRunId) {
              if (chapterId == null)
                throw new Error(t("video.chapter.invalidChapterForRecipe"));
              const card = segmentCardState.cards.find(
                (item) => item.segmentNumber === recipe.segment_number,
              );
              const sourceShotId = card?.raw.source_shot_id;
              const segmentIndexInShot = card?.raw.segment_index_in_shot;
              if (
                typeof sourceShotId !== "string" ||
                !sourceShotId.trim() ||
                typeof segmentIndexInShot !== "number" ||
                !Number.isInteger(segmentIndexInShot) ||
                segmentIndexInShot <= 0
              ) {
                throw new Error(
                  t("video.chapter.draftIdentityMissing", {
                    number: recipe.segment_number,
                  }),
                );
              }
              await updateVideoSegmentPlanDraftRecipe({
                project_id: projectId,
                chapter_id: chapterId,
                workflow_run_id: draftWorkflowRunId,
                source_shot_id: sourceShotId.trim(),
                segment_index_in_shot: segmentIndexInShot,
                recipe,
              });
            } else {
              updatedRecipes = await updateChapterVideoSegmentRecipe(
                planPromptId!,
                recipe,
              );
            }
            if (recipeSaveScopeRef.current !== saveScope) return;
            promptLoadSeqRef.current += 1;
            segmentPlanDraftLoadSeqRef.current += 1;
            const persistedRecipe =
              updatedRecipes == null
                ? recipe
                : updatedRecipes.find(
                    (saved) => saved.segment_number === recipe.segment_number,
                  );
            if (!persistedRecipe) {
              throw new Error(
                t("video.chapter.recipeSaveResponseMissing", {
                  number: recipe.segment_number,
                }),
              );
            }
            const mergedRecipes = new Map(
              persistedRecipeBySegmentRef.current,
            ).set(recipe.segment_number, persistedRecipe);
            persistedRecipeBySegmentRef.current = mergedRecipes;
            if (draftWorkflowRunId) {
              setSegmentPlanDraft((previous) => {
                if (!previous || previous.workflowRunId !== draftWorkflowRunId)
                  return previous;
                return {
                  ...previous,
                  recipes: new Map(previous.recipes).set(
                    recipe.segment_number,
                    persistedRecipe,
                  ),
                };
              });
            } else if (updatedRecipes) {
              const metadata = asJsonRecord(
                frameSegmentPlanPromptForCards?.metadata_json,
              );
              if (!metadata) throw new Error(t("video.chapter.invalidPlanMetadata"));
              const updatedMetadata = {
                ...metadata,
                segment_recipes: [...mergedRecipes.values()],
              };
              setPrompts((previous) =>
                previous.map((prompt) =>
                  prompt.id === planPromptId
                    ? { ...prompt, metadata_json: updatedMetadata }
                    : prompt,
                ),
              );
            }
            setRecipeOverrides((previous) => {
              if (previous.get(recipe.segment_number) !== recipe)
                return previous;
              const next = new Map(previous);
              next.delete(recipe.segment_number);
              return next;
            });
          } catch (error) {
            if (recipeSaveScopeRef.current !== saveScope) throw error;
            const message = extractErrorMessage(error, t("video.chapter.recipeSaveFailed"));
            if (
              effectiveRecipeBySegmentRef.current.get(recipe.segment_number) !==
              recipe
            ) {
              throw error;
            }
            setRecipeOverrides((previous) => {
              if (previous.get(recipe.segment_number) !== recipe)
                return previous;
              const next = new Map(previous);
              next.delete(recipe.segment_number);
              const persisted = persistedRecipeBySegmentRef.current.get(
                recipe.segment_number,
              );
              const effective = new Map(effectiveRecipeBySegmentRef.current);
              if (persisted) effective.set(recipe.segment_number, persisted);
              else effective.delete(recipe.segment_number);
              effectiveRecipeBySegmentRef.current = effective;
              return next;
            });
            recipeSaveErrorRef.current.set(recipe.segment_number, message);
            setRecipeSaveErrors((previous) =>
              new Map(previous).set(recipe.segment_number, message),
            );
            addToast(message, "error");
            throw error;
          }
        });
      recipeSaveBySegmentRef.current.set(recipe.segment_number, save);
      const finishSave = () => {
        if (recipeSaveBySegmentRef.current.get(recipe.segment_number) !== save)
          return;
        recipeSaveBySegmentRef.current.delete(recipe.segment_number);
        setSavingRecipeSegments((previous) => {
          const next = new Set(previous);
          next.delete(recipe.segment_number);
          return next;
        });
      };
      void save.then(finishSave, finishSave);
      return save;
    },
    [
      addToast,
      chapterId,
      frameSegmentPlanPromptForCards,
      projectId,
      segmentCardState.cards,
      segmentPlanIdentity,
      segmentPlanDraft?.workflowRunId,
      segmentPlanDraftVisible,
      t,
    ],
  );
  const updateSegmentRecipe = useCallback(
    (
      segmentNumber: number,
      updater: (
        recipe: VideoSegmentExecutionRecipe,
      ) => VideoSegmentExecutionRecipe,
    ): Promise<void> => {
      const current = effectiveRecipeBySegmentRef.current.get(segmentNumber);
      if (!current) {
        const message = t("video.chapter.editableRecipeMissing", {
          number: segmentNumber,
        });
        addToast(message, "error");
        return Promise.reject(new Error(message));
      }
      return persistSegmentRecipe(updater(current));
    },
    [addToast, persistSegmentRecipe, t],
  );
  const awaitSegmentRecipeSaved = useCallback(async (segmentNumber: number) => {
    const pending = recipeSaveBySegmentRef.current.get(segmentNumber);
    if (pending) await pending;
    const error = recipeSaveErrorRef.current.get(segmentNumber);
    if (error) throw new Error(error);
  }, []);
  const [draft, setDraft] = useState<PromptDraft>(DEFAULT_DRAFT);
  const referenceMediaDraftScopeForSegment = useCallback(
    (segmentNumber: number) => {
      const card = segmentCardState.cards.find(
        (item) => item.segmentNumber === segmentNumber,
      );
      return referenceMediaDraftScopeKey(
        segmentPlanIdentity,
        segmentNumber,
        card?.raw.source_shot_id,
        card?.raw.segment_index_in_shot,
      );
    },
    [segmentCardState.cards, segmentPlanIdentity],
  );
  const selectedReferenceMediaDraftScope =
    selectedSegmentNumber == null
      ? null
      : referenceMediaDraftScopeForSegment(selectedSegmentNumber);
  const [referenceMediaDraft, setReferenceMediaDraft] =
    useState<ReferenceMediaDraft>(() => createReferenceMediaDraft());
  const referenceMediaDraftRef = useRef(referenceMediaDraft);
  const [referenceMediaDraftDirty, setReferenceMediaDraftDirty] =
    useState(false);
  const referenceMediaDraftScopeRef = useRef<string | null>(null);
  useEffect(() => {
    const sameScope =
      referenceMediaDraftScopeRef.current === selectedReferenceMediaDraftScope;
    if (sameScope) {
      if (referenceMediaDraftDirty) return;
      const persistedRecipe = selectedSegmentRecipe?.reference_media ?? {
        prompt: "",
        image_rows: [],
        video_rows: [],
        audio_rows: [],
      };
      if (
        JSON.stringify(
          referenceMediaRecipeFromDraft(referenceMediaDraftRef.current),
        ) === JSON.stringify(persistedRecipe)
      ) {
        return;
      }
    }
    referenceMediaDraftScopeRef.current = selectedReferenceMediaDraftScope;
    const next = referenceMediaDraftFromRecipe(
      selectedSegmentRecipe?.reference_media,
    );
    referenceMediaDraftRef.current = next;
    setReferenceMediaDraft(next);
    setReferenceMediaDraftDirty(false);
  }, [
    referenceMediaDraftDirty,
    selectedReferenceMediaDraftScope,
    selectedSegmentRecipe?.reference_media,
  ]);
  const [loading, setLoading] = useState(false);
  const [videoDataError, setVideoDataError] = useState<string | null>(null);
  const [segmentPlanDraftLoading, setSegmentPlanDraftLoading] = useState(false);
  const [segmentPlanDraftError, setSegmentPlanDraftError] = useState<
    string | null
  >(null);
  const [loadedDataKey, setLoadedDataKey] = useState<string | null>(null);
  const [activeVideoConfig, setActiveVideoConfig] =
    useState<VideoGenConfig | null>(null);
  useEffect(() => {
    if (loadedDataKey !== videoDataKey) return;
    const selectedExists = segmentCardState.cards.some(
      (card) => card.segmentNumber === selectedSegmentNumber,
    );
    if (selectedExists) return;
    setSegmentConfigOpen(false);
    setSelectedSegmentNumber(segmentCardState.cards[0]?.segmentNumber ?? null);
  }, [
    loadedDataKey,
    segmentCardState.cards,
    selectedSegmentNumber,
    setSelectedSegmentNumber,
    videoDataKey,
  ]);
  const [promptRequestPreviewOpen, setPromptRequestPreviewOpen] =
    useState(false);
  const [promptRequestPreviewLoading, setPromptRequestPreviewLoading] =
    useState(false);
  const [promptRequestPreviewData, setPromptRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const promptRequestPreviewScopeRef = useRef(0);
  const loadAllSeqRef = useRef(0);
  const promptLoadSeqRef = useRef(0);
  const segmentPlanDraftLoadSeqRef = useRef(0);
  const attachedSegmentPlanTaskRef = useRef<string | null>(null);
  const activeVideoDataKeyRef = useRef(videoDataKey);
  const [imageRequestPreviewOpen, setImageRequestPreviewOpen] = useState(false);
  const [imageRequestPreviewLoading, setImageRequestPreviewLoading] =
    useState(false);
  const [imageRequestPreviewData, setImageRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const imageRequestPreviewScopeRef = useRef(0);
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
    t,
    addToast,
    images,
    actionScopeKey: videoActionScopeKey,
    setImages,
    loadAll,
  });

  const {
    finalVideoPrompts,
    selectedPrompt,
    currentLanguageFramePrompts,
    selectedVideo,
    selectedVideoPath,
    selectedPromptStale,
    selectedPromptRunning,
    workflowVideos,
    selectableReferenceImages,
    selectableReferenceIds,
    validPromptReferenceIds,
    staleSavedReferenceCount,
    savedReferenceIdsError,
    videoFrameImages,
    durationLimits,
  } = useVideoTabDerivedState({
    prompts,
    videos,
    images,
    chapter,
    selectedPromptId,
    selectedVideoId,
    workflowMode,
    promptOutputLanguage: options.promptOutputLanguage,
    currentContentHash,
    frameSegmentPlanPrompt: frameSegmentPlanPromptForCards,
    activeVideoConfig,
    selectedSegmentNumber,
    guidanceMode: selectedSegmentRecipe?.guidance_mode ?? null,
    recipeFingerprint:
      selectedSegmentNumber == null
        ? null
        : (recipeFingerprintBySegment.get(selectedSegmentNumber) ?? null),
    frameRecipeFingerprint:
      selectedSegmentNumber == null
        ? null
        : (frameRecipeFingerprintBySegment.get(selectedSegmentNumber) ?? null),
    continuitySourceVideoId:
      selectedSegmentRecipe?.continuity_source_video_id ?? null,
    t,
  });
  const selectedPromptDirty = Boolean(
    selectedPrompt &&
    (draft.visual_prompt.trim() !== selectedPrompt.visual_prompt.trim() ||
      draft.motion_prompt.trim() !==
        (selectedPrompt.motion_prompt ?? "").trim() ||
      draft.negative_prompt.trim() !==
        (selectedPrompt.negative_prompt ?? "").trim() ||
      draft.duration_secs !== selectedPrompt.duration_secs),
  );
  const videosById = useMemo(() => {
    const index = new Map<number, GeneratedVideo>();
    for (const video of videos) {
      if (!index.has(video.id)) index.set(video.id, video);
    }
    return index;
  }, [videos]);
  const frameSegmentIdentityIndex = useMemo(
    () => videoExecutionSegmentIdentityIndex(frameSegmentPlanPromptForCards),
    [frameSegmentPlanPromptForCards],
  );

  const previousAdoptedVideos = useMemo(() => {
    if (selectedSegmentNumber == null) return [];
    const rows: GeneratedVideo[] = [];
    for (const [segmentNumber, recipe] of effectiveRecipeBySegment) {
      if (
        segmentNumber >= selectedSegmentNumber ||
        recipe.adopted_video_id == null
      )
        continue;
      const video = videosById.get(recipe.adopted_video_id);
      if (
        video &&
        video.status === "completed" &&
        Boolean(video.file_path?.trim()) &&
        videoExecutionSegmentIdentityMatchesVideo(
          video,
          frameSegmentPlanPromptForCards,
          segmentNumber,
          recipeFingerprintBySegment.get(segmentNumber),
          frameSegmentIdentityIndex,
        )
      ) {
        rows.push(video);
      }
    }
    return rows.sort((left, right) => {
      const leftShot = (left.metadata_json as Record<string, unknown> | null)
        ?.shot_number;
      const rightShot = (right.metadata_json as Record<string, unknown> | null)
        ?.shot_number;
      return Number(leftShot ?? 0) - Number(rightShot ?? 0);
    });
  }, [
    effectiveRecipeBySegment,
    frameSegmentPlanPromptForCards,
    recipeFingerprintBySegment,
    selectedSegmentNumber,
    videosById,
    frameSegmentIdentityIndex,
  ]);
  const { mixedAdoptedVideoIds, unavailableMixedSegmentNumbers } = useMemo(() => {
    const adoptedVideoIds = new Set<number>();
    const unavailableSegmentNumbers: number[] = [];
    for (const card of segmentCardState.cards) {
      const recipe = effectiveRecipeBySegment.get(card.segmentNumber);
      const fingerprint = recipeFingerprintBySegment.get(card.segmentNumber);
      const video =
        recipe?.adopted_video_id == null
          ? null
          : videosById.get(recipe.adopted_video_id);
      const valid =
        video &&
        video.status === "completed" &&
        Boolean(video.file_path?.trim()) &&
        videoExecutionSegmentIdentityMatchesVideo(
          video,
          frameSegmentPlanPromptForCards,
          card.segmentNumber,
          fingerprint,
          frameSegmentIdentityIndex,
        );
      if (valid) adoptedVideoIds.add(video.id);
      else unavailableSegmentNumbers.push(card.segmentNumber);
    }
    for (const item of timeline?.items ?? []) {
      if (
        item.kind !== "inserted" ||
        item.status !== "active" ||
        item.generated_video_id == null
      ) {
        continue;
      }
      const video = videosById.get(item.generated_video_id);
      if (
        video?.status === "completed" &&
        Boolean(video.file_path?.trim()) &&
        video.timeline_item_id === item.id
      ) {
        adoptedVideoIds.add(video.id);
      }
    }
    return {
      mixedAdoptedVideoIds: adoptedVideoIds,
      unavailableMixedSegmentNumbers: unavailableSegmentNumbers,
    };
  }, [
    effectiveRecipeBySegment,
    frameSegmentPlanPromptForCards,
    recipeFingerprintBySegment,
    segmentCardState.cards,
    timeline,
    videosById,
    frameSegmentIdentityIndex,
  ]);
  const allCurrentFramePrompts = useMemo(
    () =>
      prompts.filter((prompt) => {
        if (
          prompt.prompt_type !== VIDEO_FRAME_PROMPT_TYPE &&
          prompt.prompt_type !== VIDEO_FIRST_FRAME_PROMPT_TYPE &&
          prompt.prompt_type !== VIDEO_STORYBOARD_PROMPT_TYPE
        ) {
          return false;
        }
        const segmentNumber = videoExecutionSegmentNumberInPlanByLineage(
          videoExecutionIdentityOfPrompt(prompt),
          frameSegmentPlanPromptForCards,
          frameSegmentIdentityIndex,
        );
        return (
          typeof segmentNumber === "number" &&
          Number.isInteger(segmentNumber) &&
          videoExecutionSegmentIdentityMatchesPrompt(
            prompt,
            frameSegmentPlanPromptForCards,
            segmentNumber,
            frameRecipeFingerprintBySegment.get(segmentNumber),
            frameSegmentIdentityIndex,
          )
        );
      }),
    [
      frameRecipeFingerprintBySegment,
      frameSegmentIdentityIndex,
      frameSegmentPlanPromptForCards,
      prompts,
    ],
  );
  const allSegmentPrompts = useMemo(
    () =>
      prompts
        .filter((prompt) => {
          const segmentNumber = videoExecutionSegmentNumberInPlanByLineage(
            videoExecutionIdentityOfPrompt(prompt),
            frameSegmentPlanPromptForCards,
            frameSegmentIdentityIndex,
          );
          return (
            segmentNumber != null &&
            prompt.prompt_type === VIDEO_FINAL_PROMPT_TYPE &&
            videoExecutionSegmentIdentityMatchesPrompt(
              prompt,
              frameSegmentPlanPromptForCards,
              segmentNumber,
              recipeFingerprintBySegment.get(segmentNumber),
              frameSegmentIdentityIndex,
            ) &&
            finalVideoExecutionIdentityIsComplete(prompt)
          );
        })
        .sort((a, b) => a.shot_number - b.shot_number || a.id - b.id),
    [
      frameSegmentIdentityIndex,
      frameSegmentPlanPromptForCards,
      prompts,
      recipeFingerprintBySegment,
    ],
  );
  const latestFramePromptBySegment = useMemo(() => {
    const index = new Map<number, Map<string, ChapterVideoPrompt>>();
    for (const prompt of prompts) {
      if (
        prompt.prompt_type !== VIDEO_FRAME_PROMPT_TYPE &&
        prompt.prompt_type !== VIDEO_FIRST_FRAME_PROMPT_TYPE &&
        prompt.prompt_type !== VIDEO_STORYBOARD_PROMPT_TYPE
      ) {
        continue;
      }
      const segmentNumber = videoExecutionSegmentNumberInPlanByLineage(
        videoExecutionIdentityOfPrompt(prompt),
        frameSegmentPlanPromptForCards,
        frameSegmentIdentityIndex,
      );
      if (segmentNumber == null) continue;
      if (
        !videoExecutionSegmentIdentityMatchesPrompt(
          prompt,
          frameSegmentPlanPromptForCards,
          segmentNumber,
          frameRecipeFingerprintBySegment.get(segmentNumber),
          frameSegmentIdentityIndex,
        )
      ) {
        continue;
      }
      const byPrompt = index.get(segmentNumber) ?? new Map();
      const key = `${prompt.prompt_type}:${prompt.shot_number}`;
      const current = byPrompt.get(key);
      if (!current || prompt.id > current.id) byPrompt.set(key, prompt);
      index.set(segmentNumber, byPrompt);
    }
    return index;
  }, [
    frameRecipeFingerprintBySegment,
    frameSegmentIdentityIndex,
    frameSegmentPlanPromptForCards,
    prompts,
  ]);
  useEffect(() => {
    if (workspaceTab !== "video") setSegmentConfigOpen(false);
  }, [workspaceTab]);
  const selectionValidationReady = loadedDataKey === videoDataKey;
  const videoDataLoading = videoDataKey != null && loading;
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
  const promptInputSelectionError =
    (invalidGenerationReferenceIds.length > 0
      ? t("video.chapter.invalidGenerationReferenceSelection", {
          count: invalidGenerationReferenceIds.length,
        })
      : null) ??
    (invalidVideoLibraryImageIds.length > 0
      ? t("video.chapter.invalidLibraryReferenceSelection", {
          count: invalidVideoLibraryImageIds.length,
        })
      : null);
  const videoOptionsStateError = videoOptionsValidationError(options, t);
  const videoPersistedStateError =
    chapterContentHashError ??
    segmentCardState.error ??
    segmentRecipeState.error ??
    (selectedSegmentNumber == null
      ? null
      : recipeSaveErrors.get(selectedSegmentNumber)) ??
    workspaceTabState.storageError ??
    selectedSegmentState.storageError ??
    selectedPromptState.storageError ??
    selectedVideoState.storageError ??
    pendingVideoState.storageError ??
    null;
  const effectiveDurationLimits = useMemo(() => {
    if (workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO) {
      return CHAT_VIDEO_DURATION_LIMITS;
    }
    return videoOptionsStateError
      ? {
          ...durationLimits,
          configError: videoOptionsStateError,
        }
      : durationLimits;
  }, [durationLimits, videoOptionsStateError, workflowMode]);

  const updateReferenceMediaDraft = (
    updater: (previous: ReferenceMediaDraft) => ReferenceMediaDraft,
  ) => {
    const next = updater(referenceMediaDraftRef.current);
    referenceMediaDraftRef.current = next;
    setReferenceMediaDraft(next);
    setReferenceMediaDraftDirty(true);
  };
  const setReferenceMediaPrompt = (prompt: string) =>
    updateReferenceMediaDraft((previous) => ({ ...previous, prompt }));
  const setReferenceMediaImageRows = (imageRows: ReferenceMediaBindingRow[]) =>
    updateReferenceMediaDraft((previous) => ({ ...previous, imageRows }));
  const setReferenceMediaVideoRows = (videoRows: ReferenceMediaBindingRow[]) =>
    updateReferenceMediaDraft((previous) => ({ ...previous, videoRows }));
  const setReferenceMediaAudioRows = (audioRows: ReferenceMediaBindingRow[]) =>
    updateReferenceMediaDraft((previous) => ({ ...previous, audioRows }));
  const confirmSegmentRecipeChange = useCallback(
    async (segmentNumber: number, operation: string): Promise<boolean> => {
      const recipe = effectiveRecipeBySegmentRef.current.get(segmentNumber);
      const fingerprint = recipe
        ? await sha256Hex(videoSegmentRecipeGenerationFingerprintJson(recipe))
        : null;
      if (recipe && !fingerprint) {
        addToast(t("video.chapter.executionIdentityUnavailable"), "error");
        return false;
      }
      const hasCurrentPrompt = prompts.some(
        (prompt) =>
          prompt.prompt_type === VIDEO_FINAL_PROMPT_TYPE &&
          videoExecutionSegmentIdentityMatchesPrompt(
            prompt,
            frameSegmentPlanPromptForCards,
            segmentNumber,
            fingerprint,
            frameSegmentIdentityIndex,
          ),
      );
      const hasCurrentVideo = videos.some((video) =>
          videoExecutionSegmentIdentityMatchesVideo(
            video,
            frameSegmentPlanPromptForCards,
            segmentNumber,
            fingerprint,
            frameSegmentIdentityIndex,
          ),
      );
      const discardsPromptDraft =
        segmentNumber === selectedSegmentNumber && selectedPromptDirty;
      if (
        !hasCurrentPrompt &&
        !hasCurrentVideo &&
        recipe?.adopted_video_id == null &&
        !discardsPromptDraft
      ) {
        return true;
      }
      return confirmDialog({
        title: operation,
        message: t("video.chapter.changePlan", {
          number: segmentNumber,
          draft: discardsPromptDraft ? t("video.chapter.unsavedPromptDraft") : "",
        }),
        confirmText: t("video.chapter.continueAction"),
        dialogType: "warning",
      });
    },
    [
      addToast,
      frameSegmentIdentityIndex,
      frameSegmentPlanPromptForCards,
      prompts,
      selectedPromptDirty,
      selectedSegmentNumber,
      videos,
      t,
    ],
  );
  const setGenerationReferenceIds = useCallback(
    async (ids: number[]): Promise<boolean> => {
      const segmentNumber = selectedSegmentNumber;
      if (segmentNumber == null) return false;
      const current = effectiveRecipeBySegmentRef.current.get(segmentNumber);
      if (
        !current ||
        JSON.stringify(current.reference_image_ids) === JSON.stringify(ids)
      )
        return true;
      if (
        !(await confirmSegmentRecipeChange(
          segmentNumber,
          t("video.reference.generationReference"),
        ))
      )
        return false;
      try {
        await updateSegmentRecipe(segmentNumber, (recipe) => ({
          ...recipe,
          reference_image_ids: ids,
        }));
        return true;
      } catch {
        return false;
      }
    },
    [confirmSegmentRecipeChange, selectedSegmentNumber, t, updateSegmentRecipe],
  );
  const setVideoLibraryImageIds = useCallback(
    async (ids: number[]): Promise<boolean> => {
      const segmentNumber = selectedSegmentNumber;
      if (segmentNumber == null) return false;
      const current = effectiveRecipeBySegmentRef.current.get(segmentNumber);
      if (
        !current ||
        JSON.stringify(current.video_library_image_ids) === JSON.stringify(ids)
      )
        return true;
      if (
        !(await confirmSegmentRecipeChange(
          segmentNumber,
          t("video.reference.library"),
        ))
      )
        return false;
      try {
        await updateSegmentRecipe(segmentNumber, (recipe) => ({
          ...recipe,
          video_library_image_ids: ids,
        }));
        return true;
      } catch {
        return false;
      }
    },
    [confirmSegmentRecipeChange, selectedSegmentNumber, t, updateSegmentRecipe],
  );
  const finalPromptBlockedReasonForSegment = useCallback(
    (segmentNumber: number): string | null => {
      const recipe = effectiveRecipeBySegmentRef.current.get(segmentNumber);
      if (!recipe) {
        return t("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        });
      }
      if (recipe.workflow_mode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA) {
        return null;
      }
      const unavailableReferenceIds = [
        ...recipe.reference_image_ids,
        ...recipe.video_library_image_ids,
      ].filter((id) => !selectableReferenceIds.has(id));
      if (unavailableReferenceIds.length > 0) {
        return t("video.chapter.referenceInvalid", {
          ids: [...new Set(unavailableReferenceIds)].join(", "),
        });
      }
      if (recipe.workflow_mode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO) return null;
      const configurationError = videoGuidanceConfigurationError(
        recipe.workflow_mode,
        segmentNumber,
        recipe.guidance_mode,
        t,
      );
      if (configurationError) return configurationError;
      const fingerprint = frameRecipeFingerprintBySegment.get(segmentNumber);
      if (!fingerprint) {
        return (
          recipeFingerprintErrorsBySegment.get(segmentNumber) ??
          t("video.chapter.referenceChecking")
        );
      }
      const framePrompt = (promptType: string, shotNumber: number) =>
        latestFramePromptBySegment.get(segmentNumber)?.get(
          `${promptType}:${shotNumber}`,
        ) ?? null;
      const frameReady = (prompt: ChapterVideoPrompt | null): boolean =>
        prompt != null &&
        latestVideoFrameImageForPrompt(prompt, images, t) != null;
      if (recipe.workflow_mode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES) {
        const first = framePrompt(VIDEO_FRAME_PROMPT_TYPE, 1);
        const last = framePrompt(VIDEO_FRAME_PROMPT_TYPE, 2);
        if (!first || !last) {
          return t("video.chapter.framePromptMustPrepare", {
            label: t("video.chapter.frameBoundary"),
          });
        }
        if (!frameReady(first) || !frameReady(last)) {
          return t("video.chapter.frameImageMustPrepare", {
            label: t("video.chapter.frameBoundary"),
          });
        }
      } else if (recipe.workflow_mode === VIDEO_WORKFLOW_MODE_STORYBOARD) {
        const storyboard = framePrompt(
          VIDEO_STORYBOARD_PROMPT_TYPE,
          segmentNumber,
        );
        if (!storyboard) {
          return t("video.chapter.framePromptMustPrepare", {
            label: t("video.chapter.storyboardInput"),
          });
        }
        if (!frameReady(storyboard)) {
          return t("video.chapter.frameImageMustPrepare", {
            label: t("video.chapter.storyboardInput"),
          });
        }
      } else if (
        recipe.workflow_mode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN &&
        recipe.guidance_mode === "system_frames"
      ) {
        const first = framePrompt(VIDEO_FIRST_FRAME_PROMPT_TYPE, 1);
        if (!first) {
          return t("video.chapter.framePromptMustPrepare", {
            label: t("video.chapter.frameInputStart"),
          });
        }
        if (!frameReady(first)) {
          return t("video.chapter.frameImageMustPrepare", {
            label: t("video.chapter.frameInputStart"),
          });
        }
      } else if (
        recipe.workflow_mode !== VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
      ) {
        return t("video.chapter.framePlanUnsupported");
      }
      if (recipe.guidance_mode === "continuity") {
        const sourceVideoId = recipe.continuity_source_video_id;
        if (sourceVideoId == null)
          return t("video.chapter.continuitySourceMissing");
        const sourceRecipeEntry = [
          ...effectiveRecipeBySegmentRef.current.entries(),
        ].find(
          ([sourceSegment, candidate]) =>
            sourceSegment < segmentNumber &&
            candidate.adopted_video_id === sourceVideoId,
        );
        const sourceVideo = videosById.get(sourceVideoId);
        const sourceFingerprint =
          sourceRecipeEntry == null
            ? null
            : recipeFingerprintBySegment.get(sourceRecipeEntry[0]);
        if (sourceRecipeEntry && !sourceFingerprint) {
          return (
            recipeFingerprintErrorsBySegment.get(sourceRecipeEntry[0]) ??
            t("video.chapter.continuityChecking")
          );
        }
        if (
          !sourceRecipeEntry ||
          !sourceVideo ||
          sourceVideo.status !== "completed" ||
          !sourceVideo.file_path?.trim() ||
          !videoExecutionSegmentIdentityMatchesVideo(
            sourceVideo,
            frameSegmentPlanPromptForCards,
            sourceRecipeEntry[0],
            sourceFingerprint,
            frameSegmentIdentityIndex,
          )
        ) {
          return t("video.chapter.invalidSourceVideo", { id: sourceVideoId });
        }
        const hasTailFrame = images.some(
          (image) =>
            videoTailFrameSourceVideoId(image) === sourceVideoId &&
            Boolean(image.file_path?.trim()),
        );
        return hasTailFrame
          ? null
          : t("video.chapter.sourceVideoNoTail", { id: sourceVideoId });
      }
      return null;
    },
    [
      frameRecipeFingerprintBySegment,
      frameSegmentIdentityIndex,
      frameSegmentPlanPromptForCards,
      images,
      latestFramePromptBySegment,
      recipeFingerprintBySegment,
      recipeFingerprintErrorsBySegment,
      selectableReferenceIds,
      videosById,
      t,
    ],
  );
  const finalPromptBlockedReasons = useMemo(
    () =>
      new Map(
        segmentCardState.cards.flatMap((card) => {
          const reason = finalPromptBlockedReasonForSegment(card.segmentNumber);
          return reason == null ? [] : [[card.segmentNumber, reason] as const];
        }),
      ),
    [finalPromptBlockedReasonForSegment, segmentCardState.cards],
  );
  const saveReferenceMediaDraft = useCallback(
    async (segmentNumber: number) => {
      const expectedScope = referenceMediaDraftScopeForSegment(segmentNumber);
      if (
        expectedScope == null ||
        referenceMediaDraftScopeRef.current !== expectedScope
      ) {
        addToast(
          t("video.chapter.referenceDraftWrongSegment", {
            number: segmentNumber,
          }),
          "error",
        );
        return false;
      }
      const draftToSave = referenceMediaDraftRef.current;
      if (referenceMediaDraftDirty) {
        if (
          !(await confirmSegmentRecipeChange(
            segmentNumber,
            t("video.reference.saveDraft"),
          ))
        )
          return false;
        await updateSegmentRecipe(segmentNumber, (recipe) => ({
          ...recipe,
          reference_media: referenceMediaRecipeFromDraft(draftToSave),
        }));
        if (referenceMediaDraftRef.current === draftToSave) {
          setReferenceMediaDraftDirty(false);
        }
      } else {
        await awaitSegmentRecipeSaved(segmentNumber);
      }
      return true;
    },
    [
      addToast,
      awaitSegmentRecipeSaved,
      confirmSegmentRecipeChange,
      referenceMediaDraftDirty,
      referenceMediaDraftScopeForSegment,
      updateSegmentRecipe,
      t,
    ],
  );

  const handleSelectSegment = useCallback(
    async (segmentNumber: number) => {
      if (segmentNumber === selectedSegmentNumber) return true;
      if (selectedPromptDirty) {
        const discard = await confirmDialog({
          title: t("video.chapter.discardPromptTitle"),
          message: t("video.chapter.discardPromptMessage", {
            action: t("video.chapter.segmentTab"),
          }),
          confirmText: t("video.chapter.discardAndSwitch"),
          dialogType: "warning",
        });
        if (!discard) return false;
      }
      if (
        selectedSegmentNumber != null &&
        selectedSegmentNumber !== segmentNumber &&
        referenceMediaDraftDirty
      ) {
        try {
          if (!(await saveReferenceMediaDraft(selectedSegmentNumber)))
            return false;
        } catch (error) {
          addToast(
            extractErrorMessage(error, t("video.chapter.saveReferenceDraftFailed")),
            "error",
          );
          return false;
        }
      }
      setSelectedSegmentNumber(segmentNumber);
      return true;
    },
    [
      addToast,
      referenceMediaDraftDirty,
      saveReferenceMediaDraft,
      selectedPromptDirty,
      selectedSegmentNumber,
      setSelectedSegmentNumber,
      t,
    ],
  );
  const handleSelectPrompt = useCallback(
    async (promptId: number): Promise<void> => {
      if (promptId === selectedPromptId) return;
      if (selectedPromptDirty) {
        const discard = await confirmDialog({
          title: t("video.chapter.discardPromptTitle"),
          message: t("video.chapter.discardPromptMessage", {
            action: t("video.chapter.promptRecord"),
          }),
          confirmText: t("video.chapter.discardAndSwitch"),
          dialogType: "warning",
        });
        if (!discard) return;
      }
      setSelectedPromptId(promptId);
    },
    [selectedPromptDirty, selectedPromptId, setSelectedPromptId, t],
  );
  const handleActivateSegment = useCallback(
    async (segmentNumber: number) => {
      if (segmentNumber === selectedSegmentNumber) {
        void loadAll();
        setSegmentConfigOpen(true);
        return;
      }

      if (await handleSelectSegment(segmentNumber)) void loadAll();
    },
    [handleSelectSegment, loadAll, selectedSegmentNumber],
  );
  const handleCloseSegmentConfig = useCallback(async () => {
    if (!segmentConfigOpen) return;
    if (selectedPromptDirty) {
      const discard = await confirmDialog({
        title: t("video.chapter.discardPromptTitle"),
        message: t("video.chapter.discardPromptMessage", {
          action: t("common.close"),
        }),
        confirmText: t("video.chapter.discardAndClose"),
        dialogType: "warning",
      });
      if (!discard) return;
      setDraft(
        selectedPrompt
          ? {
              visual_prompt: selectedPrompt.visual_prompt,
              motion_prompt: textInputValue(selectedPrompt.motion_prompt),
              negative_prompt: textInputValue(selectedPrompt.negative_prompt),
              duration_secs: selectedPrompt.duration_secs,
            }
          : DEFAULT_DRAFT,
      );
    }
    if (selectedSegmentNumber != null && referenceMediaDraftDirty) {
      try {
        if (!(await saveReferenceMediaDraft(selectedSegmentNumber))) return;
      } catch (error) {
        addToast(
          extractErrorMessage(error, t("video.chapter.saveReferenceDraftFailed")),
          "error",
        );
        return;
      }
    }
    setSegmentConfigOpen(false);
  }, [
    addToast,
    referenceMediaDraftDirty,
    saveReferenceMediaDraft,
    segmentConfigOpen,
    selectedPrompt,
    selectedPromptDirty,
    selectedSegmentNumber,
    t,
  ]);

  const refreshSegmentPlanDraft = useCallback(async () => {
    if (chapterId == null || !videoDataKey) return null;
    const requestSeq = segmentPlanDraftLoadSeqRef.current + 1;
    const requestDataKey = videoDataKey;
    segmentPlanDraftLoadSeqRef.current = requestSeq;
    setSegmentPlanDraftLoading(true);
    setSegmentPlanDraftError(null);
    try {
      const nextDraft = await loadVideoSegmentPlanDraft(projectId, chapterId, t);
      if (
        segmentPlanDraftLoadSeqRef.current !== requestSeq ||
        activeVideoDataKeyRef.current !== requestDataKey
      ) {
        return null;
      }
      setSegmentPlanDraft(nextDraft);
      setSegmentPlanPreview(nextDraft?.segments ?? []);
      return nextDraft;
    } catch (err) {
      if (
        segmentPlanDraftLoadSeqRef.current === requestSeq &&
        activeVideoDataKeyRef.current === requestDataKey
      ) {
        setSegmentPlanDraftError(
          extractErrorMessage(err, t("video.chapter.loadSavedPlanFailed")),
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
  }, [chapterId, projectId, t, videoDataKey]);

  async function loadAll() {
    if (!chapter || !videoDataKey) return;
    void refreshSegmentPlanDraft().catch(() => undefined);
    const requestSeq = loadAllSeqRef.current + 1;
    const requestDataKey = videoDataKey;
    const requestImageProjectIds = imageProjectIds ?? [projectId];
    const promptRequestSeq = promptLoadSeqRef.current + 1;
    loadAllSeqRef.current = requestSeq;
    promptLoadSeqRef.current = promptRequestSeq;
    setLoading(true);
    setVideoDataError(null);
    try {
      const [
        {
          prompts: promptRows,
          videos: videoRows,
          images: imageRows,
          activeVideoConfig: nextActiveVideoConfig,
          timeline: nextTimeline,
        },
        shootingScriptData,
      ] = await Promise.all([
        loadVideoTabData(projectId, chapter, requestImageProjectIds),
        loadShootingScriptOrProgress(projectId, chapter.id),
      ]);
      if (
        loadAllSeqRef.current !== requestSeq ||
        activeVideoDataKeyRef.current !== requestDataKey
      ) {
        return;
      }
      setActiveVideoConfig(nextActiveVideoConfig);
      setVideos(videoRows);
      setImages(imageRows);
      setTimeline(nextTimeline);
      setShootingScenes(
        shootingScriptData.unitProgress?.script.scenes ??
          shootingScriptData.view?.script.scenes ??
          [],
      );
      setLoadedDataKey(requestDataKey);
      if (promptLoadSeqRef.current === promptRequestSeq) {
        setPrompts(promptRows);
      }
    } catch (err) {
      if (
        loadAllSeqRef.current !== requestSeq ||
        activeVideoDataKeyRef.current !== requestDataKey
      ) {
        return;
      }
      const message = extractErrorMessage(err, t("video.chapter.loadFailed"));
      setVideoDataError(message);
      addToast(message, "error");
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
      if (document.visibilityState === "visible") void loadAll();
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
              title: t("video.actions.segmentPlanResumeTitle"),
              startMessage: t("video.actions.segmentPlanResumeStart"),
              successMessage: t("video.actions.segmentPlanCompleted"),
              errorMessage: t("video.actions.segmentPlanFailed"),
            },
            refreshPrompts,
            t,
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
          );
        } catch (error) {
          if (
            !disposed &&
            activeVideoDataKeyRef.current === requestDataKey &&
            !isCancelledOperation(extractErrorMessage(error, t("video.actions.segmentPlanFailed")))
          ) {
            addToast(extractErrorMessage(error, t("video.actions.segmentPlanFailed")), "error");
          }
        } finally {
          if (attachedSegmentPlanTaskRef.current === taskKey) {
            attachedSegmentPlanTaskRef.current = null;
          }
          if (!disposed && activeVideoDataKeyRef.current === requestDataKey) {
            await refreshSegmentPlanDraft().catch(() => undefined);
          }
          finishVideoAction(actionKey);
        }
      })
      .catch((error) => {
        if (!disposed && activeVideoDataKeyRef.current === requestDataKey) {
          setSegmentPlanDraftError(
            extractErrorMessage(error, t("video.chapter.resumePlanFailed")),
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
    t,
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

  useEffect(() => {
    promptRequestPreviewScopeRef.current += 1;
    imageRequestPreviewScopeRef.current += 1;
    setPromptRequestPreviewOpen(false);
    setPromptRequestPreviewLoading(false);
    setPromptRequestPreviewData(null);
    setImageRequestPreviewOpen(false);
    setImageRequestPreviewLoading(false);
    setImageRequestPreviewData(null);
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

  const clearInvalidGenerationReferences = async () => {
    if (invalidGenerationReferenceIds.length === 0) return;
    const next = generationReferenceIds.filter((id) =>
      selectableReferenceIds.has(id),
    );
    if (await setGenerationReferenceIds(next)) {
      addToast(t("video.reference.clearInvalid"), "success");
    }
  };
  const clearInvalidVideoLibraryImages = async () => {
    if (invalidVideoLibraryImageIds.length === 0) return;
    const next = videoLibraryImageIds.filter((id) =>
      selectableReferenceIds.has(id),
    );
    if (await setVideoLibraryImageIds(next)) {
      addToast(t("video.reference.clearInvalid"), "success");
    }
  };
  const {
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
    handlePollVideo,
    handleToggleVideoFavorite,
    handleUpdateVideo,
    handleDeleteVideo,
    handleTrim,
    requestPreviewOpen: videoRequestPreviewOpen,
    requestPreviewLoading: videoRequestPreviewLoading,
    requestPreviewData: videoRequestPreviewData,
    setRequestPreviewOpen: setVideoRequestPreviewOpen,
  } = useVideoRecordActions({
    projectId,
    chapter,
    selectedPrompt,
    selectedPromptId,
    frameStageLabel:
      workflowMode == null
        ? t("video.chapter.tabGeneration")
        : videoWorkflowModeLabel(workflowMode, t),
    selectedPromptStale,
    currentContentHash,
    selectedVideo,
    pendingVideoId,
    previewScopeKey: segmentStorageKey,
    actionScopeKey: videoActionScopeKey,
    prompts,
    frameImages: videoFrameImages,
    draft,
    selectedPromptDirty,
    validPromptReferenceIds,
    durationLimits: effectiveDurationLimits,
    t,
    addToast,
    runProgressTask,
    loadAll,
    setPrompts,
    setDraft,
    setSelectedPromptId,
    setSelectedVideoId,
    setPendingVideoId,
    beginAction: beginVideoAction,
    isCurrentActionScope: isCurrentVideoActionScope,
    finishAction: finishVideoAction,
  });

  const {
    handleGenerateSegmentPlan,
    handleGenerateFramePrompts: runGenerateFramePrompts,
    handleGenerateVideoPrompts: runGenerateVideoPrompts,
    handleGenerateFrameImages,
    handleImportFrameImage,
  } = useVideoPromptGenerationActions({
    projectId,
    chapter,
    options,
    segmentPlanOptions: planningOptions,
    durationLimits,
    promptInputSelectionError,
    referenceIds: generationReferenceIds,
    videoLibraryImageIds,
    frameImages: videoFrameImages,
    segmentPlanPrompt: frameSegmentPlanPromptForCards,
    segmentPlanDraft,
    framePrompts: currentLanguageFramePrompts,
    allFramePrompts: allCurrentFramePrompts,
    allFinalVideoPrompts: allSegmentPrompts,
    currentContentHash,
    actionScopeKey: videoActionScopeKey,
    locale,
    t,
    addToast,
    runProgressTask,
    loadAll,
    refreshPrompts,
    refreshSegmentPlanDraft,
    setSegmentPlanPreview,
    beginAction: beginVideoAction,
    isCurrentActionScope: isCurrentVideoActionScope,
    finishAction: finishVideoAction,
  });

  const handleSaveSegmentPlan = useCallback(
    async (segments: VideoSegmentPlanV2[]): Promise<boolean> => {
      const prompt = frameSegmentPlanPromptForCards;
      if (!prompt || prompt.status === "running" || acting != null) return false;
      const downstreamStrategy = await chooseDownstreamConflictStrategy({
        title: t("video.actions.downstreamTitle"),
        assetLabel: `${prompt.title ?? t("video.actions.segmentPlanLabel")} #${prompt.id}`,
        downstreamLabel: t("video.actions.downstreamLabel"),
        downstreamCount: videoPromptDownstreamCount(
          prompt,
          prompts,
          videoFrameImages,
        ),
        operation: t("video.chapter.savePlan"),
        t,
      });
      if (downstreamStrategy == null || !beginVideoAction(`edit-segment-plan-${prompt.id}`)) {
        return false;
      }
      const actionKey = `edit-segment-plan-${prompt.id}`;
      try {
        const updated = await updateChapterVideoSegmentPlan(prompt.id, {
          expected_updated_at: prompt.updated_at,
          segments,
          downstream_strategy: downstreamStrategy,
        });
        setPrompts((previous) =>
          previous.map((item) => (item.id === updated.id ? updated : item)),
        );
        await loadAll();
        addToast(t("video.chapter.planSaved"), "success");
        return true;
      } catch (error) {
        addToast(
          extractErrorMessage(error, t("video.chapter.planSaveFailed")),
          "error",
        );
        return false;
      } finally {
        finishVideoAction(actionKey);
      }
    },
    [
      acting,
      addToast,
      beginVideoAction,
      finishVideoAction,
      frameSegmentPlanPromptForCards,
      loadAll,
      prompts,
      t,
      videoFrameImages,
    ],
  );

  const recipeForSegmentAction = (
    segmentNumber: number,
  ): VideoSegmentExecutionRecipe | null => {
    const recipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber) ?? null;
    if (!recipe) {
      addToast(
        t("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        }),
        "error",
      );
    }
    return recipe;
  };

  const targetForSegmentAction = (segmentNumber: number) => {
    const card = segmentCardState.cards.find(
      (item) => item.segmentNumber === segmentNumber,
    );
    const sourceShotId = card?.raw.source_shot_id;
    const segmentIndexInShot = card?.raw.segment_index_in_shot;
    if (
      typeof sourceShotId !== "string" ||
      !sourceShotId.trim() ||
      typeof segmentIndexInShot !== "number" ||
      !Number.isInteger(segmentIndexInShot) ||
      segmentIndexInShot <= 0
    ) {
      addToast(
        t("video.chapter.segmentIdentityMissing", { number: segmentNumber }),
        "error",
      );
      return null;
    }
    return {
      source_shot_id: sourceShotId.trim(),
      segment_index_in_shot: segmentIndexInShot,
    };
  };

  const handleGenerateFramePrompts = async (segmentNumber: number) => {
    try {
      await awaitSegmentRecipeSaved(segmentNumber);
    } catch {
      return;
    }
    const recipe = recipeForSegmentAction(segmentNumber);
    const targetSegment = targetForSegmentAction(segmentNumber);
    if (!recipe || !targetSegment) return;
    await runGenerateFramePrompts(recipe, {
      targetSegment,
    });
  };

  const handleGenerateVideoPrompts = async (segmentNumber: number) => {
    try {
      await awaitSegmentRecipeSaved(segmentNumber);
    } catch {
      return;
    }
    const recipe = recipeForSegmentAction(segmentNumber);
    const targetSegment = targetForSegmentAction(segmentNumber);
    if (!recipe || !targetSegment) return;
    if (recipe.workflow_mode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA) {
      addToast(t("video.chapter.referenceMediaSelectionRequired"), "error");
      return;
    }
    const blockedReason = finalPromptBlockedReasonForSegment(segmentNumber);
    if (blockedReason) {
      addToast(blockedReason, "error");
      return;
    }
    await runGenerateVideoPrompts(recipe, {
      targetSegment,
    });
  };

  const handleAdoptSegmentVideo = async (video: GeneratedVideo) => {
    const identity = videoExecutionIdentityOfVideo(video);
    const shotNumber = videoExecutionSegmentNumberInPlanByLineage(
      identity,
      frameSegmentPlanPromptForCards,
      frameSegmentIdentityIndex,
    );
    if (!chapter || !frameSegmentPlanPromptForCards || shotNumber == null) {
      addToast(t("video.chapter.selectedVideoMissingRecipe"), "error");
      return;
    }
    const targetSegment = targetForSegmentAction(shotNumber);
    if (!targetSegment) return;
    if (selectedSegmentNumber !== shotNumber) {
      addToast(
        t("video.chapter.videoBelongsToOtherSegment", {
          id: video.id,
          number: shotNumber,
        }),
        "error",
      );
      return;
    }
    const fingerprint = recipeFingerprintBySegment.get(shotNumber);
    if (
      !videoExecutionSegmentIdentityMatchesVideo(
        video,
        frameSegmentPlanPromptForCards,
        shotNumber,
        fingerprint,
        frameSegmentIdentityIndex,
      )
    ) {
      addToast(t("video.chapter.videoNotLatestRecipe"), "error");
      return;
    }
    const actionKey = `adopt-video-${video.id}`;
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    try {
      await selectSegmentVideo(
        projectId,
        chapter.id,
        video.id,
        frameSegmentPlanPromptForCards.id,
        targetSegment,
      );
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(t("video.chapter.adoptedSegment", { number: shotNumber }), "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, t("video.chapter.adoptVideoFailed")), "error");
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleSegmentWorkflowModeChange = async (
    mode: VideoSegmentWorkflowMode,
    segmentNumber: number,
  ): Promise<void> => {
    const currentRecipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber);
    if (!currentRecipe) {
      addToast(
        t("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        }),
        "error",
      );
      return;
    }
    const guidanceMode = guidanceModeForWorkflowChange(mode, segmentNumber);
    if (currentRecipe.workflow_mode === mode) {
      await handleSelectSegment(segmentNumber);
      return;
    }
    const savesReferenceMediaDraft =
      currentRecipe.workflow_mode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA &&
      mode !== VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA &&
      segmentNumber === selectedSegmentNumber &&
      referenceMediaDraftScopeRef.current ===
        referenceMediaDraftScopeForSegment(segmentNumber) &&
      referenceMediaDraftDirty;
    if (savesReferenceMediaDraft) {
      try {
        if (!(await saveReferenceMediaDraft(segmentNumber))) return;
      } catch {
        return;
      }
    }
    if (
      !(await confirmSegmentRecipeChange(
        segmentNumber,
        t("video.chapter.workflowMode"),
      ))
    ) {
      return;
    }
    if (!(await handleSelectSegment(segmentNumber))) return;
    try {
      await updateSegmentRecipe(segmentNumber, (recipe) => {
        return {
          ...recipe,
          workflow_mode: mode,
          guidance_mode: guidanceMode,
          guidance_reference_image_ids: [],
          continuity_source_video_id: null,
        };
      });
      setWorkspaceTab("video");
    } catch {
      // persistSegmentRecipe 已保留可见错误并恢复服务端状态。
    }
  };

  const handleFirstFrameSourceChange = async (
    segmentNumber: number,
    guidanceMode: Extract<
      VideoSegmentExecutionRecipe["guidance_mode"],
      "continuity" | "system_frames"
    >,
  ) => {
    const currentRecipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber);
    if (!currentRecipe) {
      addToast(
        t("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        }),
        "error",
      );
      return;
    }
    if (
      currentRecipe.workflow_mode !== VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN ||
      segmentNumber <= 1
    ) {
      addToast(t("video.chapter.firstFrameChainFollowupInputRequired"), "error");
      return;
    }
    if (currentRecipe.guidance_mode === guidanceMode) return;
    if (
      !(await confirmSegmentRecipeChange(
        segmentNumber,
        t("video.chapter.startFrameSource"),
      ))
    ) {
      return;
    }
    try {
      await updateSegmentRecipe(segmentNumber, (recipe) => ({
        ...recipe,
        guidance_mode: guidanceMode,
        guidance_reference_image_ids: [],
        continuity_source_video_id: null,
      }));
    } catch {
      // persistSegmentRecipe 已显示错误。
    }
  };

  const handleContinuitySourceChange = async (
    segmentNumber: number,
    continuitySourceVideoId: number | null,
  ) => {
    const currentRecipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber);
    if (!currentRecipe) {
      addToast(
        t("video.chapter.segmentRecipeMissingForAction", {
          number: segmentNumber,
        }),
        "error",
      );
      return;
    }
    const sourceChanged =
      currentRecipe.guidance_mode !== "continuity" ||
      (currentRecipe.continuity_source_video_id ?? null) !==
        continuitySourceVideoId;
    if (!sourceChanged) return;
    if (
      !(await confirmSegmentRecipeChange(
        segmentNumber,
        t("video.chapter.sourceVideo"),
      ))
    ) {
      return;
    }
    try {
      await updateSegmentRecipe(segmentNumber, (recipe) => ({
        ...recipe,
        guidance_mode: "continuity",
        guidance_reference_image_ids: [],
        continuity_source_video_id: continuitySourceVideoId,
      }));
    } catch {
      // persistSegmentRecipe 已显示错误。
    }
  };

  const handleGenerateInsertedSegment = async (
    input: InsertedSegmentGenerationOptions,
  ): Promise<number | null> => {
    const actionKey = "generate-inserted-segment";
    const requestScope = videoActionScopeKey;
    if (!chapter) {
      addToast(t("video.chapter.noChapter"), "error");
      return null;
    }
    if (!beginVideoAction(actionKey)) return null;
    try {
      const handle = await startChapterVideoInsertedSegmentDraft(
        projectId,
        chapter.id,
        input,
      );
      const workflowRunId = await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("video.chapter.insertedDescription"),
          startMessage: t("video.chapter.insertedDescriptionPreparing"),
          successMessage: t("video.chapter.insertedDescriptionGenerated"),
          errorMessage: t("video.chapter.insertedDescriptionFailed"),
          cancelLabel: t("video.chapter.stopGeneration"),
        },
        async (task) => {
          const result = task.resultRef;
          if (result?.kind !== "inserted_segment_draft") {
            throw new Error(t("video.chapter.insertedDraftMissing"));
          }
          const parsed = Number(result.id);
          if (!Number.isSafeInteger(parsed) || parsed <= 0) {
            throw new Error(t("video.chapter.invalidInsertedWorkflowId"));
          }
          return parsed;
        },
      );
      if (!isCurrentVideoActionScope(requestScope)) return null;
      addToast(t("video.chapter.insertedDescriptionConfirm"), "success");
      return workflowRunId;
    } catch (error) {
      if (isCurrentVideoActionScope(requestScope)) {
        const message = extractErrorMessage(
          error,
          t("video.chapter.insertedDescriptionFailed"),
        );
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
      return null;
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleCommitInsertedSegment = async (
    timelineId: number,
    expectedRevision: number,
    workflowRunId: number,
  ): Promise<boolean> => {
    const actionKey = "commit-inserted-segment";
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return false;
    try {
      const nextTimeline = await commitInsertedSegmentDraft({
        timeline_id: timelineId,
        expected_timeline_revision: expectedRevision,
        workflow_run_id: workflowRunId,
      });
      if (!isCurrentVideoActionScope(requestScope)) return false;
      setTimeline(nextTimeline);
      addToast(t("video.chapter.insertedSubmitted"), "success");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(
            refreshError,
            t("video.chapter.insertedSubmitRefreshFailed"),
          ),
          "error",
        );
      }
      return true;
    } catch (error) {
      if (!isCurrentVideoActionScope(requestScope)) return false;
      const message = extractErrorMessage(error, t("video.chapter.operationFailed"));
      addToast(
        message.includes("timeline_revision_conflict")
          ? t("video.chapter.timelineRevisionConflict", { error: message })
          : message,
        "error",
      );
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(refreshError, t("video.chapter.refreshTimelineFailed")),
          "error",
        );
      }
      return false;
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleMoveInsertedSegment = async (input: {
    timeline_id: number;
    expected_timeline_revision: number;
    timeline_item_id: number;
    previous_item_id: number | null;
    next_item_id: number | null;
  }): Promise<void> => {
    const actionKey = `move-inserted-segment-${input.timeline_item_id}`;
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    try {
      const nextTimeline = await moveInsertedSegment(input);
      if (!isCurrentVideoActionScope(requestScope)) return;
      setTimeline(nextTimeline);
      addToast(t("video.chapter.insertedPositionUpdated"), "success");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(
            refreshError,
            t("video.chapter.insertedMoveRefreshFailed"),
          ),
          "error",
        );
      }
    } catch (error) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(error, t("video.chapter.operationFailed")), "error");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(refreshError, t("video.chapter.refreshTimelineFailed")),
          "error",
        );
      }
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleDeleteInsertedSegment = async (input: {
    timeline_id: number;
    expected_timeline_revision: number;
    timeline_item_id: number;
  }): Promise<void> => {
    const actionKey = `delete-inserted-segment-${input.timeline_item_id}`;
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    try {
      const nextTimeline = await deleteInsertedSegment(input);
      if (!isCurrentVideoActionScope(requestScope)) return;
      setTimeline(nextTimeline);
      addToast(t("video.chapter.insertedRemoved"), "success");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(
            refreshError,
            t("video.chapter.insertedDeleteRefreshFailed"),
          ),
          "error",
        );
      }
    } catch (error) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(error, t("video.chapter.operationFailed")), "error");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(refreshError, t("video.chapter.refreshTimelineFailed")),
          "error",
        );
      }
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleGenerateInsertedVideo = async (
    prompt: ChapterVideoPrompt,
  ): Promise<void> => {
    const actionKey = `generate-inserted-video-${prompt.id}`;
    const requestScope = videoActionScopeKey;
    if (!chapter || !beginVideoAction(actionKey)) return;
    try {
      const handle = await startVideoGeneration({
        project_id: projectId,
        operation: {
          kind: "chapter",
          chapter_id: chapter.id,
          prompt_id: prompt.id,
          config_id: activeVideoConfig?.id ?? null,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("video.chapter.insertedVideoPreparing"),
          startMessage: t("video.chapter.insertedVideoPreparing"),
          successMessage: t("video.chapter.insertedVideoGenerated"),
          errorMessage: t("video.chapter.insertedVideoFailed"),
          cancelLabel: t("video.chapter.stopGeneration"),
        },
        async (task) => {
          await verifyVideoGenerationResult(task, projectId, chapter.id);
        },
      );
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(t("video.chapter.insertedVideoGenerated"), "success");
      try {
        await loadAll();
      } catch (refreshError) {
        addToast(
          extractErrorMessage(
            refreshError,
            t("video.chapter.videoDataGeneratedRefreshFailed"),
          ),
          "error",
        );
      }
    } catch (error) {
      if (isCurrentVideoActionScope(requestScope)) {
        const message = extractErrorMessage(
          error,
          t("video.chapter.insertedVideoFailed"),
        );
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleUpdateInsertedPrompt = async (
    prompt: ChapterVideoPrompt,
    input: {
      visual_prompt: string;
      motion_prompt: string;
      negative_prompt: string;
    },
  ): Promise<void> => {
    await handleSavePromptRow(prompt, {
      ...input,
      duration_secs: prompt.duration_secs,
    });
  };

  const handleCopyChatVideoPrompt = async (prompt: ChapterVideoPrompt) => {
    const text = prompt.visual_prompt.trim();
    if (!text) {
      addToast(t("video.chapter.chatPromptEmpty"), "error");
      return;
    }
    try {
      await copyTextToClipboard(text);
      addToast(t("video.chapter.copiedChatPrompt"), "success");
    } catch (err) {
      addToast(
        extractErrorMessage(err, t("video.chapter.copyChatPromptFailed")),
        "error",
      );
    }
  };

  const buildFrameImageGenerationInput = (
    prompt: ChapterVideoPrompt,
  ): Record<string, unknown> => {
    const frameContract = videoFrameImageContractForPromptType(
      prompt.prompt_type,
      t,
    );
    const trimmedPrompt = prompt.visual_prompt.trim();
    const previewValidationError = promptLengthError(
      frameContract.promptLabel,
      trimmedPrompt,
      VIDEO_FRAME_IMAGE_PROMPT_MIN_CHARS,
      t,
    );
    if (previewValidationError) {
      throw new Error(previewValidationError);
    }
    const references = frameImageReferencesForPrompt(
      prompt,
      videoFrameImages,
      currentLanguageFramePrompts,
      t,
    );
    if (references.error) {
      throw new Error(references.error);
    }
    const effectiveVisualPrompt = shiftReferenceLabels(
      trimmedPrompt,
      references.promptReferenceOffset,
    );
    const referenceOrderError = referenceLabelOrderError(
      effectiveVisualPrompt,
      references.ids.length,
      t,
    );
    if (referenceOrderError) {
      throw new Error(referenceOrderError);
    }
    return buildVideoFrameImageRequest(
      frameContract.imageType,
      effectiveVisualPrompt,
      prompt.chapter_id || chapter?.id || 0,
      prompt.aspect_ratio,
      buildVideoFrameImageTraceMetadata(prompt, frameContract.source, t),
      references.ids,
      references.note,
      t,
    );
  };

  const buildFrameImagePreviewInputs = (
    prompt: ChapterVideoPrompt,
  ): {
    requests: Array<Record<string, unknown>>;
    note: string | null;
  } | null => {
    if (!isBoundaryFramePrompt(prompt)) {
      return {
        requests: [buildFrameImageGenerationInput(prompt)],
        note: null,
      };
    }
    const chain = boundaryFramePromptChainTo(
      prompt,
      currentLanguageFramePrompts,
      t("video.chapter.requestPreviewOperation"),
      t,
    );
    const missingPrompts = chain.filter(
      (item) => !latestVideoFrameImageForPrompt(item, videoFrameImages, t),
    );
    if (missingPrompts.length === 0) {
      addToast(
        t("video.chapter.boundaryImagesAlreadyExist", {
          number: prompt.shot_number,
        }),
        "info",
      );
      return null;
    }
    const firstMissing = missingPrompts[0];
    if (!firstMissing) return null;
    const dynamicTailCount = missingPrompts.length - 1;
    const extraPreviewNote =
      dynamicTailCount > 0
        ? t("video.chapter.boundaryPreviewNotice", {
            first: firstMissing.shot_number,
            count: dynamicTailCount,
          })
        : null;
    return {
      requests: [buildFrameImageGenerationInput(firstMissing)],
      note: extraPreviewNote,
    };
  };

  const previewImageRequests = async (
    requests: Array<Record<string, unknown>>,
    note: string | null = null,
  ) => {
    if (!chapter) return;
    if (requests.length === 0) {
      addToast(t("video.chapter.previewImageRequestsEmpty"), "error");
      return;
    }
    const previewScope = imageRequestPreviewScopeRef.current;
    setImageRequestPreviewOpen(true);
    setImageRequestPreviewLoading(true);
    setImageRequestPreviewData(null);
    try {
      const preview = await previewBatchImagesRequest(projectId, requests);
      if (imageRequestPreviewScopeRef.current !== previewScope) return;
      const previewNote = note?.trim();
      setImageRequestPreviewData(
        previewNote && preview.steps[0]
          ? {
              ...preview,
              steps: preview.steps.map((step, index) =>
                index === 0
                  ? {
                      ...step,
                      note: [step.note?.trim(), previewNote]
                        .filter(Boolean)
                        .join("\n"),
                    }
                  : step,
              ),
            }
          : preview,
      );
    } catch (err) {
      if (imageRequestPreviewScopeRef.current !== previewScope) return;
      addToast(extractErrorMessage(err, t("video.chapter.viewImageRequestFailed")), "error");
      setImageRequestPreviewOpen(false);
    } finally {
      if (imageRequestPreviewScopeRef.current === previewScope) {
        setImageRequestPreviewLoading(false);
      }
    }
  };

  const handlePreviewFrameImageRequest = async (prompt: ChapterVideoPrompt) => {
    let previewInput: ReturnType<typeof buildFrameImagePreviewInputs>;
    try {
      previewInput = buildFrameImagePreviewInputs(prompt);
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.chapter.previewImageRequestFailed")), "error");
      return;
    }
    if (!previewInput) return;
    await previewImageRequests(previewInput.requests, previewInput.note);
  };

  const buildReferenceMediaVideoInput = (
    segment: ReferenceMediaSegmentState,
    draft: ReferenceMediaDraft,
  ): ReferenceMediaVideoInput => {
    const imageRows = normalizeReferenceMediaRows(draft.imageRows);
    const videoRows = normalizeReferenceMediaRows(draft.videoRows);
    const audioRows = normalizeReferenceMediaRows(draft.audioRows);
    return {
      segment_plan_prompt_id: segment.segmentPlanPromptId,
      target_segment: segment.targetSegment,
      image_uploads: imageRows.flatMap((row) =>
        row.upload ? [row.upload] : [],
      ),
      video_uploads: videoRows.flatMap((row) =>
        row.upload ? [row.upload] : [],
      ),
      audio_uploads: audioRows.flatMap((row) =>
        row.upload ? [row.upload] : [],
      ),
      config_id: null,
    };
  };

  const referenceMediaSegmentFor = (
    requireProvider = true,
  ): ReferenceMediaSegmentState | null => {
    if (!chapter) return null;
    if (
      !frameSegmentPlanPromptForCards ||
      selectedSegmentNumber == null ||
      workflowMode !== VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA
    ) {
      addToast(t("video.chapter.referencePreviewOnly"), "error");
      return null;
    }
    if (videoOptionsStateError) {
      addToast(videoOptionsStateError, "error");
      return null;
    }
    const targetSegment = targetForSegmentAction(selectedSegmentNumber);
    if (!targetSegment) return null;
    const segment = {
      segmentPlanPromptId: frameSegmentPlanPromptForCards.id,
      segmentNumber: selectedSegmentNumber,
      targetSegment,
    };
    if (!requireProvider) return segment;
    if (effectiveDurationLimits.configError) {
      addToast(effectiveDurationLimits.configError, "error");
      return null;
    }
    if (!isDurationAllowed(options.durationSecs, effectiveDurationLimits)) {
      addToast(t("video.reference.durationUnsupported"), "error");
      return null;
    }
    return segment;
  };

  const prepareReferenceMediaVideoInput = async () => {
    const segment = referenceMediaSegmentFor();
    if (!segment) return null;
    const draft = referenceMediaDraftRef.current;
    const inputError = referenceMediaDraftError(
      draft.prompt,
      referenceMediaRowsByKindFromDraft(draft),
      t,
    );
    if (inputError) {
      addToast(inputError, "error");
      return null;
    }
    try {
      if (!(await saveReferenceMediaDraft(segment.segmentNumber))) return null;
    } catch {
      return null;
    }
    return buildReferenceMediaVideoInput(segment, draft);
  };

  const handlePreviewReferenceMediaVideoRequest = async () => {
    if (!chapter) return;
    const input = await prepareReferenceMediaVideoInput();
    if (!input) return;
    const previewScope = promptRequestPreviewScopeRef.current;
    setPromptRequestPreviewOpen(true);
    setPromptRequestPreviewLoading(true);
    setPromptRequestPreviewData(null);
    try {
      const preview = await previewReferenceMediaVideoRequest(
        projectId,
        chapter.id,
        input,
      );
      if (promptRequestPreviewScopeRef.current !== previewScope) return;
      setPromptRequestPreviewData(preview);
    } catch (err) {
      if (promptRequestPreviewScopeRef.current !== previewScope) return;
      addToast(
        extractErrorMessage(err, t("video.chapter.previewReferenceMediaFailed")),
        "error",
      );
      setPromptRequestPreviewOpen(false);
    } finally {
      if (promptRequestPreviewScopeRef.current === previewScope) {
        setPromptRequestPreviewLoading(false);
      }
    }
  };

  const handleSaveReferenceMediaDraft = async () => {
    if (selectedSegmentNumber == null) {
      addToast(t("video.chapter.selectReferenceMediaSegment"), "error");
      return;
    }
    try {
      if (!(await saveReferenceMediaDraft(selectedSegmentNumber))) return;
      addToast(t("video.chapter.referenceMediaDraftSaved"), "success");
    } catch {
      // persistSegmentRecipe 已显示错误。
    }
  };

  const handleGenerateReferenceMediaVideo = async () => {
    if (!chapter) return;
    const input = await prepareReferenceMediaVideoInput();
    if (!input) return;
    const actionKey = "reference-media-video";
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    try {
      const handle = await startVideoGeneration({
        project_id: projectId,
        operation: {
          kind: "reference_media",
          chapter_id: chapter.id,
          input,
        },
      });
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("video.chapter.referenceMediaVideoTitle"),
          startMessage: t("video.chapter.referenceMediaVideoPreparing"),
          successMessage: t("video.chapter.referenceMediaVideoGenerated"),
          errorMessage: t("video.chapter.referenceMediaVideoFailed"),
          cancelLabel: t("video.chapter.stopGeneration"),
        },
        async (task) => {
          const [row] = await verifyVideoGenerationResult(
            task,
            projectId,
            chapter.id,
          );
          if (!isCurrentVideoActionScope(requestScope)) return;
          setSelectedVideoId(row.id);
          setPendingVideoId(null);
        },
      );
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(t("video.chapter.referenceMediaVideoGenerated"), "success");
    } catch (err) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      const message = extractErrorMessage(
        err,
        t("video.chapter.referenceMediaVideoFailed"),
      );
      if (!isCancelledOperation(message)) addToast(message, "error");
    } finally {
      if (isCurrentVideoActionScope(requestScope)) await loadAll();
      finishVideoAction(actionKey);
    }
  };

  const handleImportReferenceMediaVideo = async () => {
    if (!chapter) return;
    const segment = referenceMediaSegmentFor(false);
    if (!segment) return;
    if (!referenceMediaDraft.prompt.trim()) {
      addToast(t("video.chapter.mediaPromptEmpty"), "error");
      return;
    }
    try {
      if (!(await saveReferenceMediaDraft(segment.segmentNumber))) return;
    } catch {
      return;
    }

    const actionKey = "reference-media-import";
    const requestScope = videoActionScopeKey;
    if (!beginVideoAction(actionKey)) return;
    let upload: VideoUploadFile | null = null;
    try {
      upload = await pickVideoUploadFile(t, t("video.reference.video"));
    } catch (err) {
      if (isCurrentVideoActionScope(requestScope)) {
        addToast(
          extractErrorMessage(err, t("video.chapter.filePickerFailed")),
          "error",
        );
      }
      finishVideoAction(actionKey);
      return;
    }
    if (!upload) {
      finishVideoAction(actionKey);
      return;
    }
    const { file, mimeType } = upload;

    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentVideoActionScope(requestScope)) return;
      const row = await uploadReferenceMediaVideo(projectId, chapter.id, {
        file_name: file.name,
        mime_type: mimeType,
        data_base64: dataBase64,
        segment_plan_prompt_id: segment.segmentPlanPromptId,
        target_segment: segment.targetSegment,
      });
      if (!isCurrentVideoActionScope(requestScope)) return;
      setSelectedVideoId(row.id);
      setPendingVideoId(null);
      addToast(t("video.chapter.importedReferenceMediaVideo"), "success");
      await loadAll();
    } catch (err) {
      if (isCurrentVideoActionScope(requestScope)) {
        addToast(
          extractErrorMessage(err, t("video.chapter.importReferenceMediaVideoFailed")),
          "error",
        );
      }
    } finally {
      finishVideoAction(actionKey);
    }
  };

  if (!chapter) {
    return <EmptyHint text={t("video.chapter.noChapterSelectedForVideo")} />;
  }

  const referenceMediaWorkspace =
    workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA ? (
      <ReferenceMediaWorkspace
        prompt={referenceMediaDraft.prompt}
        imageRows={referenceMediaDraft.imageRows}
        videoRows={referenceMediaDraft.videoRows}
        audioRows={referenceMediaDraft.audioRows}
        durationSecs={options.durationSecs}
        aspectRatio={options.aspectRatio}
        durationLimits={effectiveDurationLimits}
        activeVideoConfig={activeVideoConfig}
        acting={acting}
        previewing={promptRequestPreviewLoading}
        draftDirty={referenceMediaDraftDirty}
        savingDraft={
          selectedSegmentNumber != null &&
          savingRecipeSegments.has(selectedSegmentNumber)
        }
        onPromptChange={setReferenceMediaPrompt}
        onImageRowsChange={setReferenceMediaImageRows}
        onVideoRowsChange={setReferenceMediaVideoRows}
        onAudioRowsChange={setReferenceMediaAudioRows}
        onSaveDraft={handleSaveReferenceMediaDraft}
        onPreview={handlePreviewReferenceMediaVideoRequest}
        onGenerate={handleGenerateReferenceMediaVideo}
        onImport={handleImportReferenceMediaVideo}
      />
    ) : null;
  const referenceInputs =
    workflowMode == null || workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA ? undefined : (
      <VideoReferenceInputSlots
        images={selectableReferenceImages}
        chapter={chapter}
        workflowMode={workflowMode}
        currentContentHash={currentContentHash}
        generationReferenceIds={generationReferenceIds}
        videoLibraryImageIds={videoLibraryImageIds}
        invalidGenerationReferenceIds={invalidGenerationReferenceIds}
        invalidVideoLibraryImageIds={invalidVideoLibraryImageIds}
        disabled={
          acting != null ||
          selectedSegmentNumber == null ||
          savingRecipeSegments.has(selectedSegmentNumber)
        }
        onGenerationReferenceIdsChange={setGenerationReferenceIds}
        onVideoLibraryImageIdsChange={setVideoLibraryImageIds}
        onReferenceLimit={() =>
          addToast(
            t("video.chapter.referenceLimit", {
              label:
                workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO
                  ? t("video.reference.chatUpload")
                  : t("video.reference.generationReference"),
              count:
                workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO
                  ? MAX_CHAT_REFERENCE_IMAGES
                  : MAX_REFERENCE_IMAGES,
            }),
            "error",
          )
        }
        onSelectionError={(message) => addToast(message, "error")}
        onClearInvalidGenerationReferences={clearInvalidGenerationReferences}
        onClearInvalidVideoLibraryImages={clearInvalidVideoLibraryImages}
      />
    );

  return (
    <>
      <div className="flex h-full min-h-0 flex-col gap-3">
        {videoPersistedStateError && (
          <div
            role="alert"
            className="shrink-0 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-xs leading-relaxed text-error"
          >
            {videoPersistedStateError}
            {t("video.chapter.invalidStateContinue")}
          </div>
        )}
        {videoDataError && (
          <div
            role="alert"
            className="flex shrink-0 items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-xs leading-relaxed text-error"
          >
            <span className="min-w-0 flex-1">
              {t("video.chapter.loadDataFailed", { error: videoDataError })}
            </span>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void loadAll()}
              disabled={videoDataLoading}
            >
              <RotateCcw size={12} />
              {t("video.chapter.retryLoad")}
            </BookButton>
          </div>
        )}
        <VideoWorkspaceHeader
          tabs={workspaceTabs}
          activeTab={workspaceTab}
          onTabChange={setWorkspaceTab}
        >
          {workflowMode == null ? (
            <span>{t("video.chapter.noExecutionRecipeSelected")}</span>
          ) : workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA ? (
            <span>{t("video.chapter.currentReferenceMediaSegment")}</span>
          ) : (
            <span>
              {t("video.chapter.currentWorkflowSegment", {
                workflow: videoWorkflowModeLabel(workflowMode, t),
              })}
            </span>
          )}
        </VideoWorkspaceHeader>

        {workspaceTab === "materials" ? (
          <div className="min-h-0 flex-1">
            <Suspense fallback={<VideoWorkspaceFallback text={t("video.chapter.loadingMaterialsWorkspace")} />}>
              <VideoMaterialWorkspace
                projectId={profileMaterialProjectId}
                onToast={addToast}
                runProgressTask={runProgressTask}
                onRefreshVideoData={loadAll}
              />
            </Suspense>
          </div>
        ) : workspaceTab === "frames" ? (
          planningOptionsState.options ? (
            <Suspense fallback={<VideoWorkspaceFallback text={t("video.chapter.loadingPlanningWorkspace")} />}>
              <VideoPlanningWorkspace
                prompt={frameSegmentPlanPromptForCards}
                previewSegments={segmentPlanPreview}
                draft={segmentPlanDraft}
                options={planningOptionsState.options}
                durationLimits={durationLimits}
                loading={segmentPlanDraftLoading}
                loadError={segmentPlanDraftError}
                dataReady={selectionValidationReady}
                acting={acting}
                shootingScenes={shootingScenes}
                supportedAspectRatios={supportedVideoAspectRatios}
                aspectRatioLoading={imageSizeLoading}
                onGenerate={(targetNarrativeUnitIndex, regenerateExisting) =>
                  handleGenerateSegmentPlan({
                    targetNarrativeUnitIndex,
                    regenerateExisting,
                  })
                }
                onOptionsChange={setPlanningOptions}
                onRetryLoad={() => {
                  void refreshSegmentPlanDraft().catch(() => undefined);
                }}
                onDelete={(prompt) => void handleDeletePromptRow(prompt)}
                onSavePlan={handleSaveSegmentPlan}
              />
            </Suspense>
          ) : (
            <div
              role="alert"
              className="flex min-h-0 flex-1 items-center justify-center"
            >
              <div className="flex max-w-xl flex-col gap-3 rounded-xl border border-error/40 bg-error-bg px-4 py-3 text-sm text-error">
                <span>{planningOptionsState.storageError}</span>
                <BookButton
                  variant="secondary"
                  size="sm"
                  onClick={resetPlanningOptions}
                >
                  <RotateCcw size={12} />
                  {t("video.chapter.resetPlanningConfig")}
                </BookButton>
              </div>
            </div>
          )
        ) : workspaceTab === "assembly" ? (
          <Suspense fallback={<VideoWorkspaceFallback text={t("video.chapter.loadingAssemblyWorkspace")} />}>
            <VideoAssemblyWorkspace
              projectId={projectId}
              chapter={chapter}
              videos={videos}
              timeline={timeline}
              adoptedVideoIds={mixedAdoptedVideoIds}
              unavailableSegmentNumbers={unavailableMixedSegmentNumbers}
              onToast={addToast}
              onRefresh={loadAll}
            />
          </Suspense>
        ) : workspaceTab === "postprocess" ? (
          <Suspense fallback={<VideoWorkspaceFallback text={t("video.chapter.loadingPostprocessWorkspace")} />}>
            <VideoPostProcessWorkspace
              projectId={projectId}
              chapter={chapter}
              videos={videos}
              onToast={addToast}
              onRefresh={loadAll}
            />
          </Suspense>
        ) : (
          <div className="min-h-0 flex-1">
            <VideoGenerationWorkspace
              prompts={allSegmentPrompts}
              segmentCards={segmentCardState.cards}
              segmentRecipes={effectiveRecipeBySegment}
              selectedSegmentNumber={selectedSegmentNumber}
              planningDraftVisible={segmentPlanDraftVisible}
              finalPromptBlockedReasons={finalPromptBlockedReasons}
              timeline={timeline}
              selectedPrompt={selectedPrompt}
              selectedPromptId={selectedPromptId}
              selectedPromptStale={selectedPromptStale}
              durationLimits={effectiveDurationLimits}
              loading={loading}
              acting={acting}
              chapterContentReady={chapterContentReady}
              savingRecipeSegments={savingRecipeSegments}
              draft={draft}
              selectedPromptRunning={selectedPromptRunning}
              selectedPromptDirty={selectedPromptDirty}
              staleSavedReferenceCount={staleSavedReferenceCount}
              savedReferenceIdsError={savedReferenceIdsError}
              framePrompts={currentLanguageFramePrompts}
              frameImages={videoFrameImages}
              imageBusyId={imageBusyId}
              videos={workflowVideos}
              previousAdoptedVideos={previousAdoptedVideos}
              referenceInputs={referenceInputs}
              selectedVideo={selectedVideo}
              selectedVideoPath={selectedVideoPath}
              trimStart={trimStart}
              trimEnd={trimEnd}
              onLoad={loadAll}
              onGeneratePromptRequests={handleGenerateVideoPrompts}
              onSelectPrompt={handleSelectPrompt}
              segmentConfigOpen={segmentConfigOpen}
              onActivateSegment={handleActivateSegment}
              onCloseSegmentConfig={handleCloseSegmentConfig}
              onDeletePrompt={(prompt) => void handleDeletePromptRow(prompt)}
              onInsertSegment={(gap) =>
                insertedSegmentPanelRef.current?.openCreate(gap)
              }
              onOpenInsertedSegment={(timelineItemId) =>
                insertedSegmentPanelRef.current?.openEdit(timelineItemId)
              }
              onWorkflowModeChange={handleSegmentWorkflowModeChange}
              onFirstFrameSourceChange={handleFirstFrameSourceChange}
              onContinuitySourceChange={handleContinuitySourceChange}
              onGenerateFramePrompts={handleGenerateFramePrompts}
              onPreviewFrameImage={handlePreviewFrameImageRequest}
              onGenerateFrameImage={(prompt) =>
                handleGenerateFrameImages(prompt)
              }
              onImportFrameImage={(prompt) => handleImportFrameImage(prompt)}
              onDeleteFrameImage={(image) => handleDeleteImage(image)}
              onDraftChange={setDraft}
              onSavePrompt={() => void handleSavePrompt()}
              onSaveFramePrompt={(prompt, visualPrompt) =>
                handleSavePromptRow(prompt, {
                  visual_prompt: visualPrompt,
                  motion_prompt: prompt.motion_prompt ?? "",
                  negative_prompt: prompt.negative_prompt ?? "",
                  duration_secs: prompt.duration_secs,
                })
              }
              onOptimizePrompt={handleOptimizePromptRow}
              onPreviewGenerateVideo={handlePreviewGenerateVideoRequest}
              onCopyPrompt={handleCopyChatVideoPrompt}
              onPreviewVideoRequest={handlePreviewVideoRequestForVideo}
              onGenerateVideo={handleGenerateVideo}
              onImportVideo={handleImportVideo}
              onAdoptVideo={handleAdoptSegmentVideo}
              onPollVideo={(video) => void handlePollVideo(video)}
              onToggleVideoFavorite={(video) =>
                void handleToggleVideoFavorite(video)
              }
              onUpdateVideo={handleUpdateVideo}
              onDeleteVideo={(video) => void handleDeleteVideo(video)}
              onTrimStartChange={setTrimStart}
              onTrimEndChange={setTrimEnd}
              onTrim={handleTrim}
              referenceMediaWorkspace={referenceMediaWorkspace}
            />
          </div>
        )}
      </div>
      {workspaceTab === "video" ? (
        <InsertedSegmentTimelinePanel
          ref={insertedSegmentPanelRef}
          timeline={timeline}
          prompts={prompts}
          videos={videos}
          referenceImages={selectableReferenceImages}
          defaultStyle={options.style}
          defaultPromptOutputLanguage={options.promptOutputLanguage}
          defaultDurationSecs={options.durationSecs}
          defaultDurationOptions={options.durationOptions}
          defaultAspectRatio={options.aspectRatio}
          supportedAspectRatios={supportedVideoAspectRatios}
          aspectRatioLoading={imageSizeLoading}
          loading={loading}
          acting={acting}
          onGenerate={handleGenerateInsertedSegment}
          onCommit={handleCommitInsertedSegment}
          onMove={handleMoveInsertedSegment}
          onDelete={handleDeleteInsertedSegment}
          onGenerateVideo={handleGenerateInsertedVideo}
          onUpdatePrompt={handleUpdateInsertedPrompt}
          onPreviewVideo={handlePreviewVideoRequestForVideo}
          onDeleteVideo={handleDeleteVideo}
        />
      ) : null}
      {imageDetails ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
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
        </Suspense>
      ) : null}
      {promptRequestPreviewOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <RequestPreviewModal
            open
            loading={promptRequestPreviewLoading}
            data={promptRequestPreviewData}
            fallbackTitle={t("video.chapter.modelRequestPreview")}
            onClose={() => setPromptRequestPreviewOpen(false)}
          />
        </Suspense>
      ) : null}
      {imageRequestPreviewOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <RequestPreviewModal
            open
            loading={imageRequestPreviewLoading}
            data={imageRequestPreviewData}
            fallbackTitle={t("video.chapter.imageRequestPreview")}
            onClose={() => setImageRequestPreviewOpen(false)}
          />
        </Suspense>
      ) : null}
      {videoRequestPreviewOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <RequestPreviewModal
            open
            loading={videoRequestPreviewLoading}
            data={videoRequestPreviewData}
            fallbackTitle={t("video.chapter.videoRequestPreview")}
            onClose={() => setVideoRequestPreviewOpen(false)}
          />
        </Suspense>
      ) : null}
    </>
  );
};

const VideoWorkspaceFallback: React.FC<{ text: string }> = ({ text }) => (
  <div className="flex h-full min-h-48 items-center justify-center text-xs text-book-text-muted">
    <LoadingSpinner size="sm" className="mr-2" />
    {text}
  </div>
);
