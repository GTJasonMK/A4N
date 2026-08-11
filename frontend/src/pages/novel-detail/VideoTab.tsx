import React, {
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
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { useToast } from "../../components/feedback/Toast";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../../api";
import { generationClient } from "../../api/generationTasks";
import {
  previewBatchImagesRequest,
  type GeneratedImage,
} from "../../api/image";
import {
  getChapterShootingScript,
  getChapterShootingScriptUnitProgress,
  type ShootingSceneV1,
} from "../../api/shootingScript";
import type { ModelRequestPreview } from "../../api/requestPreview";
import {
  listChapterVideoPrompts,
  previewReferenceMediaVideoRequest,
  selectSegmentVideo,
  startVideoGeneration,
  updateChapterVideoSegmentRecipe,
  updateVideoSegmentPlanDraftRecipe,
  uploadReferenceMediaVideo,
  verifyVideoGenerationResult,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type ReferenceMediaVideoInput,
  type VideoGenConfig,
  type VideoSegmentExecutionRecipe,
} from "../../api/video";
import { usePersistedState } from "../../hooks/usePersistedState";
import { EmptyHint, type ChapterRow } from "./shared";
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
import { ImageActionDialog } from "./images/ImageActionDialog";
import { VideoAssemblyWorkspace } from "./video/VideoAssemblyWorkspace";
import { VideoPlanningWorkspace } from "./video/VideoPlanningWorkspace";
import { VideoGenerationWorkspace } from "./video/VideoGenerationWorkspace";
import { VideoMaterialWorkspace } from "./video/VideoMaterialWorkspace";
import { VideoPostProcessWorkspace } from "./video/VideoPostProcessWorkspace";
import {
  createReferenceMediaDraft,
  normalizeReferenceMediaRows,
  REFERENCE_MEDIA_KIND_META,
  REFERENCE_MEDIA_LIMITS,
  ReferenceMediaWorkspace,
  referenceMediaDraftScopeKey,
  referenceMediaGenerateDraftError,
  referenceMediaDraftFromRecipe,
  referenceMediaRecipeFromDraft,
  referenceMediaRowsByKindFromDraft,
  validReferenceMediaUrl,
  VideoReferenceInputSlots,
  type ReferenceMediaBindingRow,
  type ReferenceMediaDraft,
  type ReferenceMediaInputPurpose,
  type ReferenceMediaKind,
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
  VIDEO_FRAME_PROMPT_TYPE,
  VIDEO_FIRST_FRAME_PROMPT_TYPE,
  VIDEO_WORKFLOW_MODE_LABELS,
  VIDEO_STORYBOARD_PROMPT_TYPE,
  latestVideoSegmentPlanPrompt,
  readVideoSegmentRecipes,
  videoOptionsFromSegmentRecipe,
  videoSegmentCardsFromPlanPrompt,
  videoSegmentCardsFromSegments,
  videoSegmentRecipeFrameFingerprintJson,
  videoSegmentRecipeGenerationFingerprintJson,
  videoExecutionSegmentIdentityMatchesPrompt,
  videoExecutionSegmentIdentityMatchesVideo,
  videoExecutionIdentityOfPrompt,
  videoExecutionIdentityOfVideo,
  videoExecutionSegmentNumberInPlanByLineage,
  videoGuidanceConfigurationError,
  videoTailFrameSourceVideoId,
  finalVideoExecutionIdentityIsComplete,
  videoOptionsValidationError,
  type ChapterVideoWorkflowMode,
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
import { useVideoRecordActions } from "./video/useVideoRecordActions";
import { useVideoTabDerivedState } from "./video/useVideoTabDerivedState";
import {
  VideoWorkspaceHeader,
  type VideoWorkspaceTab,
} from "./video/VideoWorkspaceHeader";

interface VideoTabProps {
  projectId: number;
  fixedChapter?: ChapterRow | null;
  imageProjectIds?: number[];
  materialProjectId?: number;
}

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

interface PersistedWorkspaceTabState {
  tab: VideoWorkspaceTab;
  storageError: string | null;
}

interface PersistedNullableIdState {
  id: number | null;
  storageError: string | null;
}

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
): PersistedWorkspaceTabState => {
  if (allowedTabs.includes(raw as VideoWorkspaceTab)) {
    return createPersistedWorkspaceTabState(raw as VideoWorkspaceTab);
  }
  return createPersistedWorkspaceTabState(
    defaultTab,
    `视频工作区 tab 本地存储损坏：${raw || "<empty>"}`,
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

const serializePersistedNullableIdState = (
  state: PersistedNullableIdState,
): string => serializeNullableNumber(state.id);

export const VideoTab: React.FC<VideoTabProps> = ({
  projectId,
  fixedChapter,
  imageProjectIds,
  materialProjectId,
}) => {
  const { addToast } = useToast();
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
  const planningOptionsInitialValue = useMemo<VideoGenOptions>(
    () => ({
      ...DEFAULT_VIDEO_OPTIONS,
      durationOptions: [...DEFAULT_VIDEO_OPTIONS.durationOptions],
    }),
    [chapterStorageKey],
  );
  const [planningOptions, setPlanningOptions] =
    usePersistedState<VideoGenOptions>(
      chapterStorageKey ? `${chapterStorageKey}:segment-plan-options` : null,
      planningOptionsInitialValue,
      { parse: parseVideoOptions, serialize: serializeVideoOptions },
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
  const workspaceTabIds = VIDEO_WORKSPACE_TABS.map((tab) => tab.id);
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
          ? videoSegmentCardsFromSegments(segmentPlanPreview)
          : videoSegmentCardsFromPlanPrompt(frameSegmentPlanPromptForCards),
        error: null as string | null,
      };
    } catch (error) {
      return {
        cards: [] as VideoSegmentCardPlan[],
        error: extractErrorMessage(error, "读取片段规划失败"),
      };
    }
  }, [
    frameSegmentPlanPromptForCards,
    segmentPlanDraftVisible,
    segmentPlanPreview,
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
        : readVideoSegmentRecipes(frameSegmentPlanPromptForCards),
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
        parse: (raw) => parsePersistedNullableIdState(raw, "选中片段"),
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
                ? [[segmentNumber, "当前环境无法校验片段执行身份"] as const]
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
              "片段执行身份校验失败，请刷新后重试",
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
        parse: (raw) => parsePersistedNullableIdState(raw, "选中视频提示词"),
        serialize: serializePersistedNullableIdState,
      },
    );
  const [selectedVideoState, setSelectedVideoState] =
    usePersistedState<PersistedNullableIdState>(
      segmentStorageKey ? `${segmentStorageKey}:selected-video` : null,
      persistedNullIdInitialValue,
      {
        parse: (raw) => parsePersistedNullableIdState(raw, "选中视频"),
        serialize: serializePersistedNullableIdState,
      },
    );
  const [pendingVideoState, setPendingVideoState] =
    usePersistedState<PersistedNullableIdState>(
      segmentStorageKey ? `${segmentStorageKey}:pending-video` : null,
      persistedNullIdInitialValue,
      {
        parse: (raw) => parsePersistedNullableIdState(raw, "待恢复视频任务"),
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
          ? "当前片段规划草稿缺少 workflow 身份，不能保存执行方案"
          : "当前没有可保存执行方案的片段规划";
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
                throw new Error("当前章节无效，无法保存片段执行方案");
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
                  `片段 ${recipe.segment_number} 缺少稳定草稿身份`,
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
                `保存成功后未返回片段 ${recipe.segment_number} 的执行方案`,
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
              if (!metadata) throw new Error("当前片段规划 metadata_json 无效");
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
            const message = extractErrorMessage(error, "保存片段执行方案失败");
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
        const message = `片段 ${segmentNumber} 缺少可编辑的执行方案`;
        addToast(message, "error");
        return Promise.reject(new Error(message));
      }
      return persistSegmentRecipe(updater(current));
    },
    [addToast, persistSegmentRecipe],
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

  const previousAdoptedVideos = useMemo(() => {
    if (selectedSegmentNumber == null) return [];
    const rows: GeneratedVideo[] = [];
    for (const [segmentNumber, recipe] of effectiveRecipeBySegment) {
      if (
        segmentNumber >= selectedSegmentNumber ||
        recipe.adopted_video_id == null
      )
        continue;
      const video = videos.find((row) => row.id === recipe.adopted_video_id);
      if (
        video &&
        video.status === "completed" &&
        Boolean(video.file_path?.trim()) &&
        videoExecutionSegmentIdentityMatchesVideo(
          video,
          frameSegmentPlanPromptForCards,
          segmentNumber,
          recipeFingerprintBySegment.get(segmentNumber),
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
    videos,
  ]);
  const { adoptedVideoIds, unavailableSegmentNumbers } = useMemo(() => {
    const adoptedVideoIds = new Set<number>();
    const unavailableSegmentNumbers: number[] = [];
    for (const card of segmentCardState.cards) {
      const recipe = effectiveRecipeBySegment.get(card.segmentNumber);
      const fingerprint = recipeFingerprintBySegment.get(card.segmentNumber);
      const video =
        recipe?.adopted_video_id == null
          ? null
          : videos.find((row) => row.id === recipe.adopted_video_id);
      const valid =
        video &&
        video.status === "completed" &&
        Boolean(video.file_path?.trim()) &&
        videoExecutionSegmentIdentityMatchesVideo(
          video,
          frameSegmentPlanPromptForCards,
          card.segmentNumber,
          fingerprint,
        );
      if (valid) adoptedVideoIds.add(video.id);
      else unavailableSegmentNumbers.push(card.segmentNumber);
    }
    return { adoptedVideoIds, unavailableSegmentNumbers };
  }, [
    effectiveRecipeBySegment,
    frameSegmentPlanPromptForCards,
    recipeFingerprintBySegment,
    segmentCardState.cards,
    videos,
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
        );
        return (
          typeof segmentNumber === "number" &&
          Number.isInteger(segmentNumber) &&
          videoExecutionSegmentIdentityMatchesPrompt(
            prompt,
            frameSegmentPlanPromptForCards,
            segmentNumber,
            frameRecipeFingerprintBySegment.get(segmentNumber),
          )
        );
      }),
    [frameRecipeFingerprintBySegment, frameSegmentPlanPromptForCards, prompts],
  );
  const allSegmentPrompts = useMemo(
    () =>
      prompts
        .filter((prompt) => {
          const segmentNumber = videoExecutionSegmentNumberInPlanByLineage(
            videoExecutionIdentityOfPrompt(prompt),
            frameSegmentPlanPromptForCards,
          );
          return (
            segmentNumber != null &&
            prompt.prompt_type === "video_prompt" &&
            videoExecutionSegmentIdentityMatchesPrompt(
              prompt,
              frameSegmentPlanPromptForCards,
              segmentNumber,
              recipeFingerprintBySegment.get(segmentNumber),
            ) &&
            finalVideoExecutionIdentityIsComplete(prompt)
          );
        })
        .sort((a, b) => a.shot_number - b.shot_number || a.id - b.id),
    [frameSegmentPlanPromptForCards, prompts, recipeFingerprintBySegment],
  );
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
      ? `生成参考图中有 ${invalidGenerationReferenceIds.length} 张已不可用，请清除后重选`
      : null) ??
    (invalidVideoLibraryImageIds.length > 0
      ? `视频素材库中有 ${invalidVideoLibraryImageIds.length} 张图片已不可用，请清除后重选`
      : null);
  const videoOptionsStateError =
    options.storageError ?? videoOptionsValidationError(options);
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
    if (workflowMode === "chat_video") return CHAT_VIDEO_DURATION_LIMITS;
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
        addToast(
          "当前环境无法校验片段执行身份，已阻止修改以避免误用下游资产",
          "error",
        );
        return false;
      }
      const hasCurrentPrompt = prompts.some(
        (prompt) =>
          prompt.prompt_type === "video_prompt" &&
          videoExecutionSegmentIdentityMatchesPrompt(
            prompt,
            frameSegmentPlanPromptForCards,
            segmentNumber,
            fingerprint,
          ),
      );
      const hasCurrentVideo = videos.some((video) =>
        videoExecutionSegmentIdentityMatchesVideo(
          video,
          frameSegmentPlanPromptForCards,
          segmentNumber,
          fingerprint,
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
        message: `修改片段 ${segmentNumber} 的执行方案会使当前最终提示词、候选视频和采用状态失效；历史记录仍会保留，但不再参与当前生成和成片。${discardsPromptDraft ? "当前未保存的提示词编辑也会被放弃。" : ""}是否继续？`,
        confirmText: "继续修改",
        dialogType: "warning",
      });
    },
    [
      addToast,
      frameSegmentPlanPromptForCards,
      prompts,
      selectedPromptDirty,
      selectedSegmentNumber,
      videos,
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
        !(await confirmSegmentRecipeChange(segmentNumber, "修改片段生成参考图"))
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
    [confirmSegmentRecipeChange, selectedSegmentNumber, updateSegmentRecipe],
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
        !(await confirmSegmentRecipeChange(segmentNumber, "修改片段素材库输入"))
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
    [confirmSegmentRecipeChange, selectedSegmentNumber, updateSegmentRecipe],
  );
  const finalPromptBlockedReasonForSegment = useCallback(
    (segmentNumber: number): string | null => {
      const recipe = effectiveRecipeBySegmentRef.current.get(segmentNumber);
      if (!recipe) return `片段 ${segmentNumber} 缺少执行方案`;
      if (recipe.workflow_mode === "reference_media") return null;
      const unavailableReferenceIds = [
        ...recipe.reference_image_ids,
        ...recipe.video_library_image_ids,
      ].filter((id) => !selectableReferenceIds.has(id));
      if (unavailableReferenceIds.length > 0) {
        return `片段参考素材已失效：${[...new Set(unavailableReferenceIds)].join(", ")}`;
      }
      if (recipe.workflow_mode === "chat_video") return null;
      const configurationError = videoGuidanceConfigurationError(
        recipe.workflow_mode,
        segmentNumber,
        recipe.guidance_mode,
      );
      if (configurationError) return configurationError;
      const fingerprint = frameRecipeFingerprintBySegment.get(segmentNumber);
      if (!fingerprint) {
        return (
          recipeFingerprintErrorsBySegment.get(segmentNumber) ??
          "正在核对当前片段的素材，请稍候"
        );
      }
      const framePrompt = (promptType: string, shotNumber: number) =>
        prompts
          .filter(
            (prompt) =>
              prompt.prompt_type === promptType &&
              prompt.shot_number === shotNumber &&
              videoExecutionSegmentIdentityMatchesPrompt(
                prompt,
                frameSegmentPlanPromptForCards,
                segmentNumber,
                fingerprint,
              ),
          )
          .sort((left, right) => right.id - left.id)[0] ?? null;
      const frameReady = (prompt: ChapterVideoPrompt | null): boolean =>
        prompt != null &&
        latestVideoFrameImageForPrompt(prompt, images) != null;
      if (recipe.workflow_mode === "boundary_frames") {
        const first = framePrompt(VIDEO_FRAME_PROMPT_TYPE, 1);
        const last = framePrompt(VIDEO_FRAME_PROMPT_TYPE, 2);
        if (!first || !last) return "请先准备当前片段的首尾帧提示词";
        if (!frameReady(first) || !frameReady(last)) {
          return "请先生成当前片段的首尾帧图片";
        }
      } else if (recipe.workflow_mode === "storyboard") {
        const storyboard = framePrompt(
          VIDEO_STORYBOARD_PROMPT_TYPE,
          segmentNumber,
        );
        if (!storyboard) return "请先准备当前片段的分镜提示词";
        if (!frameReady(storyboard)) {
          return "请先生成当前片段的分镜板";
        }
      } else if (
        recipe.workflow_mode === "first_frame_chain" &&
        recipe.guidance_mode === "system_frames"
      ) {
        const first = framePrompt(VIDEO_FIRST_FRAME_PROMPT_TYPE, 1);
        if (!first) return "请先准备当前片段的首帧提示词";
        if (!frameReady(first)) {
          return "请先生成当前片段的首帧图片";
        }
      } else if (recipe.workflow_mode !== "first_frame_chain") {
        return "当前片段执行方案不支持帧提示词规划";
      }
      if (recipe.guidance_mode === "continuity") {
        const sourceVideoId = recipe.continuity_source_video_id;
        if (sourceVideoId == null)
          return "请选择更早片段已采用的视频作为续接来源";
        const sourceRecipeEntry = [
          ...effectiveRecipeBySegmentRef.current.entries(),
        ].find(
          ([sourceSegment, candidate]) =>
            sourceSegment < segmentNumber &&
            candidate.adopted_video_id === sourceVideoId,
        );
        const sourceVideo = videos.find((video) => video.id === sourceVideoId);
        const sourceFingerprint =
          sourceRecipeEntry == null
            ? null
            : recipeFingerprintBySegment.get(sourceRecipeEntry[0]);
        if (sourceRecipeEntry && !sourceFingerprint) {
          return (
            recipeFingerprintErrorsBySegment.get(sourceRecipeEntry[0]) ??
            "正在核对续接来源视频，请稍候"
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
          )
        ) {
          return `续接来源视频 #${sourceVideoId} 已失效，请重新选择`;
        }
        const hasTailFrame = images.some(
          (image) =>
            videoTailFrameSourceVideoId(image) === sourceVideoId &&
            Boolean(image.file_path?.trim()),
        );
        return hasTailFrame
          ? null
          : `续接来源视频 #${sourceVideoId} 尚无可用尾帧`;
      }
      return null;
    },
    [
      frameRecipeFingerprintBySegment,
      frameSegmentPlanPromptForCards,
      images,
      prompts,
      recipeFingerprintBySegment,
      recipeFingerprintErrorsBySegment,
      selectableReferenceIds,
      videos,
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
          `参考媒体草稿不属于当前片段规划的片段 ${segmentNumber}，请重新编辑`,
          "error",
        );
        return false;
      }
      const draftToSave = referenceMediaDraftRef.current;
      if (referenceMediaDraftDirty) {
        if (
          !(await confirmSegmentRecipeChange(segmentNumber, "保存参考媒体方案"))
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
    ],
  );

  const handleSelectSegment = useCallback(
    async (segmentNumber: number) => {
      if (segmentNumber === selectedSegmentNumber) return true;
      if (selectedPromptDirty) {
        const discard = await confirmDialog({
          title: "放弃未保存的提示词编辑",
          message:
            "当前最终提示词有未保存编辑。切换片段会放弃这些修改，是否继续？",
          confirmText: "放弃并切换",
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
            extractErrorMessage(error, "保存当前片段参考媒体草稿失败"),
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
    ],
  );
  const handleSelectPrompt = useCallback(
    async (promptId: number): Promise<void> => {
      if (promptId === selectedPromptId) return;
      if (selectedPromptDirty) {
        const discard = await confirmDialog({
          title: "放弃未保存的提示词编辑",
          message:
            "当前最终提示词有未保存编辑。切换提示词会放弃这些修改，是否继续？",
          confirmText: "放弃并切换",
          dialogType: "warning",
        });
        if (!discard) return;
      }
      setSelectedPromptId(promptId);
    },
    [selectedPromptDirty, selectedPromptId, setSelectedPromptId],
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
        title: "放弃未保存的提示词编辑",
        message:
          "当前最终提示词有未保存编辑。关闭片段配置会放弃这些修改，是否继续？",
        confirmText: "放弃并关闭",
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
          extractErrorMessage(error, "保存当前片段参考媒体草稿失败"),
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
  ]);

  const refreshSegmentPlanDraft = useCallback(async () => {
    if (chapterId == null || !videoDataKey) return null;
    const requestSeq = segmentPlanDraftLoadSeqRef.current + 1;
    const requestDataKey = videoDataKey;
    segmentPlanDraftLoadSeqRef.current = requestSeq;
    setSegmentPlanDraftLoading(true);
    setSegmentPlanDraftError(null);
    try {
      const nextDraft = await loadVideoSegmentPlanDraft(projectId, chapterId);
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
          extractErrorMessage(err, "加载已保存的片段规划失败"),
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
        },
        shootingScript,
        shootingProgress,
      ] = await Promise.all([
        loadVideoTabData(projectId, chapter, requestImageProjectIds),
        getChapterShootingScript(projectId, chapter.id),
        getChapterShootingScriptUnitProgress(projectId, chapter.id),
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
      setShootingScenes(
        shootingProgress?.script.scenes ?? shootingScript?.script.scenes ?? [],
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
      const message = extractErrorMessage(err, "加载视频数据失败");
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
              title: "继续生成片段规划",
              startMessage: "正在恢复当前片段规划任务",
              successMessage: "片段规划已完成",
              errorMessage: "片段规划失败",
              total: 100,
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
          );
        } catch (error) {
          if (
            !disposed &&
            activeVideoDataKeyRef.current === requestDataKey &&
            !isCancelledOperation(extractErrorMessage(error, "片段规划失败"))
          ) {
            addToast(extractErrorMessage(error, "片段规划失败"), "error");
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
            extractErrorMessage(error, "恢复片段规划任务失败"),
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
      addToast("已清除不可用生成参考图片选择", "success");
    }
  };
  const clearInvalidVideoLibraryImages = async () => {
    if (invalidVideoLibraryImageIds.length === 0) return;
    const next = videoLibraryImageIds.filter((id) =>
      selectableReferenceIds.has(id),
    );
    if (await setVideoLibraryImageIds(next)) {
      addToast("已清除不可用素材库图片选择", "success");
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
        ? "视频生成"
        : VIDEO_WORKFLOW_MODE_LABELS[workflowMode],
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

  const recipeForSegmentAction = (
    segmentNumber: number,
  ): VideoSegmentExecutionRecipe | null => {
    const recipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber) ?? null;
    if (!recipe) addToast(`片段 ${segmentNumber} 缺少执行方案`, "error");
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
      addToast(`片段 ${segmentNumber} 缺少稳定身份`, "error");
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
    if (recipe.workflow_mode === "reference_media") {
      addToast("参考媒体方案直接生成视频，不生成最终提示词", "error");
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
    );
    if (!chapter || !frameSegmentPlanPromptForCards || shotNumber == null) {
      addToast("该视频缺少完整片段执行身份，不能采用", "error");
      return;
    }
    const targetSegment = targetForSegmentAction(shotNumber);
    if (!targetSegment) return;
    if (selectedSegmentNumber !== shotNumber) {
      addToast(
        `视频 #${video.id} 属于片段 ${shotNumber}，不能作为当前片段采用`,
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
      )
    ) {
      addToast("该视频不属于当前片段的最新执行方案，不能采用", "error");
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
      addToast(`已采用片段 ${shotNumber} 的当前视频`, "success");
      await loadAll();
    } catch (err) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast(extractErrorMessage(err, "采用片段视频失败"), "error");
    } finally {
      finishVideoAction(actionKey);
    }
  };

  const handleSegmentWorkflowModeChange = async (
    mode: ChapterVideoWorkflowMode,
    segmentNumber: number,
  ): Promise<void> => {
    const currentRecipe =
      effectiveRecipeBySegmentRef.current.get(segmentNumber);
    if (!currentRecipe) {
      addToast(`片段 ${segmentNumber} 缺少执行方案`, "error");
      return;
    }
    const guidanceMode = guidanceModeForWorkflowChange(mode, segmentNumber);
    if (currentRecipe.workflow_mode === mode) {
      await handleSelectSegment(segmentNumber);
      return;
    }
    const savesReferenceMediaDraft =
      currentRecipe.workflow_mode === "reference_media" &&
      mode !== "reference_media" &&
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
      !(await confirmSegmentRecipeChange(segmentNumber, "切换片段生成方案"))
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
      addToast(`片段 ${segmentNumber} 缺少执行方案`, "error");
      return;
    }
    if (
      currentRecipe.workflow_mode !== "first_frame_chain" ||
      segmentNumber <= 1
    ) {
      addToast("只有首帧续接的后续片段可以切换首帧来源", "error");
      return;
    }
    if (currentRecipe.guidance_mode === guidanceMode) return;
    if (
      !(await confirmSegmentRecipeChange(segmentNumber, "切换片段首帧来源"))
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
      addToast(`片段 ${segmentNumber} 缺少执行方案`, "error");
      return;
    }
    const sourceChanged =
      currentRecipe.guidance_mode !== "continuity" ||
      (currentRecipe.continuity_source_video_id ?? null) !==
        continuitySourceVideoId;
    if (!sourceChanged) return;
    if (
      !(await confirmSegmentRecipeChange(segmentNumber, "修改片段续接来源"))
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
    const storyboardPrompt =
      prompt.prompt_type === VIDEO_STORYBOARD_PROMPT_TYPE;
    const label = firstFramePrompt
      ? "首帧提示词"
      : storyboardPrompt
        ? "分镜提示词"
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
    const referenceOrderError = referenceLabelOrderError(
      effectiveVisualPrompt,
      references.ids.length,
    );
    if (referenceOrderError) {
      throw new Error(referenceOrderError);
    }
    const requestPreviewNote = [extraPreviewNote].filter(Boolean).join(" ");
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
      addToast(
        `第 1-${prompt.shot_number} 张边界帧图片已存在，无需新增成图请求`,
        "info",
      );
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

  const previewImageRequests = async (
    requests: Array<Record<string, unknown>>,
  ) => {
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
    if (
      purpose === "generate" &&
      activeRows.length > REFERENCE_MEDIA_LIMITS[kind]
    ) {
      addToast(
        `${meta.label}最多允许 ${REFERENCE_MEDIA_LIMITS[kind]} 个输入`,
        "error",
      );
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
    const imageRows = collectReferenceMediaRows(
      "image",
      referenceMediaDraft.imageRows,
      purpose,
    );
    if (!imageRows) return null;
    const videoRows = collectReferenceMediaRows(
      "video",
      referenceMediaDraft.videoRows,
      purpose,
    );
    if (!videoRows) return null;
    const audioRows = collectReferenceMediaRows(
      "audio",
      referenceMediaDraft.audioRows,
      purpose,
    );
    if (!audioRows) return null;
    if (
      imageRows.length === 0 &&
      videoRows.length === 0 &&
      audioRows.length === 0
    ) {
      if (purpose !== "preview") {
        addToast("参考媒体模式至少需要一个图片、视频或音频输入", "error");
        return null;
      }
    }
    const input: ReferenceMediaVideoInput = {
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
    return input;
  };

  const referenceMediaSegmentFor = (
    requireProvider = true,
  ): ReferenceMediaSegmentState | null => {
    if (!chapter) return null;
    if (
      !frameSegmentPlanPromptForCards ||
      selectedSegmentNumber == null ||
      workflowMode !== "reference_media"
    ) {
      addToast("请从片段提示词卡片选择参考媒体方案", "error");
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
      addToast("视频时长不在默认视频模型支持范围内", "error");
      return null;
    }
    return segment;
  };

  const handlePreviewReferenceMediaVideoRequest = async () => {
    if (!chapter) return;
    const segment = referenceMediaSegmentFor(false);
    if (!segment) return;
    try {
      if (!(await saveReferenceMediaDraft(segment.segmentNumber))) return;
    } catch {
      return;
    }
    const input = buildReferenceMediaVideoInput(segment, "preview");
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
      addToast(extractErrorMessage(err, "预览参考媒体视频请求失败"), "error");
      setPromptRequestPreviewOpen(false);
    } finally {
      if (promptRequestPreviewScopeRef.current === previewScope) {
        setPromptRequestPreviewLoading(false);
      }
    }
  };

  const handleSaveReferenceMediaDraft = async () => {
    if (selectedSegmentNumber == null) {
      addToast("请先选择参考媒体片段", "error");
      return;
    }
    try {
      if (!(await saveReferenceMediaDraft(selectedSegmentNumber))) return;
      addToast("参考媒体草稿已保存", "success");
    } catch {
      // persistSegmentRecipe 已显示错误。
    }
  };

  const handleGenerateReferenceMediaVideo = async () => {
    if (!chapter) return;
    const segment = referenceMediaSegmentFor();
    if (!segment) return;
    const generateInputError = referenceMediaGenerateDraftError(
      referenceMediaDraft.prompt,
      referenceMediaRowsByKindFromDraft(referenceMediaDraft),
    );
    if (generateInputError) {
      addToast(generateInputError, "error");
      return;
    }
    try {
      if (!(await saveReferenceMediaDraft(segment.segmentNumber))) return;
    } catch {
      return;
    }
    const input = buildReferenceMediaVideoInput(segment);
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
          if (!row)
            throw new Error("参考媒体视频生成已结束，但没有正式视频结果");
          if (!isCurrentVideoActionScope(requestScope)) return;
          setSelectedVideoId(row.id);
          setPendingVideoId(null);
        },
      );
      if (!isCurrentVideoActionScope(requestScope)) return;
      addToast("参考媒体视频生成完成", "success");
    } catch (err) {
      if (!isCurrentVideoActionScope(requestScope)) return;
      const message = extractErrorMessage(err, "参考媒体视频生成失败");
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
      addToast("参考媒体视频 prompt 不能为空", "error");
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
      upload = await pickVideoUploadFile("参考媒体视频");
    } catch (err) {
      if (isCurrentVideoActionScope(requestScope)) {
        addToast(extractErrorMessage(err, "打开文件选择器失败"), "error");
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
      addToast("参考媒体视频已导入", "success");
      await loadAll();
    } catch (err) {
      if (isCurrentVideoActionScope(requestScope)) {
        addToast(extractErrorMessage(err, "导入参考媒体视频失败"), "error");
      }
    } finally {
      finishVideoAction(actionKey);
    }
  };

  if (!chapter) {
    return <EmptyHint text="请先在写作桌选择已生成的章节。" />;
  }

  const referenceMediaWorkspace =
    workflowMode === "reference_media" ? (
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
    workflowMode == null || workflowMode === "reference_media" ? undefined : (
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
            ；请修正当前片段执行方案、工作区或条目后继续。
          </div>
        )}
        {videoDataError && (
          <div
            role="alert"
            className="flex shrink-0 items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-xs leading-relaxed text-error"
          >
            <span className="min-w-0 flex-1">
              加载视频数据失败：{videoDataError}
            </span>
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => void loadAll()}
              disabled={videoDataLoading}
            >
              <RotateCcw size={12} />
              重试加载
            </BookButton>
          </div>
        )}
        <VideoWorkspaceHeader
          tabs={VIDEO_WORKSPACE_TABS}
          activeTab={workspaceTab}
          onTabChange={setWorkspaceTab}
        >
          {workflowMode == null ? (
            <span>未选择片段执行方案</span>
          ) : workflowMode === "reference_media" ? (
            <span>参考媒体 · 当前片段</span>
          ) : (
            <span>{VIDEO_WORKFLOW_MODE_LABELS[workflowMode]} · 当前片段</span>
          )}
        </VideoWorkspaceHeader>

        {workspaceTab === "materials" ? (
          <div className="min-h-0 flex-1">
            <VideoMaterialWorkspace
              projectId={profileMaterialProjectId}
              onToast={addToast}
              runProgressTask={runProgressTask}
              onRefreshVideoData={loadAll}
            />
          </div>
        ) : workspaceTab === "frames" ? (
          <VideoPlanningWorkspace
            prompt={frameSegmentPlanPromptForCards}
            previewSegments={segmentPlanPreview}
            draft={segmentPlanDraft}
            options={planningOptions}
            durationLimits={durationLimits}
            loading={segmentPlanDraftLoading}
            loadError={segmentPlanDraftError}
            dataReady={selectionValidationReady}
            acting={acting}
            shootingScenes={shootingScenes}
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
          />
        ) : workspaceTab === "assembly" ? (
          <VideoAssemblyWorkspace
            projectId={projectId}
            chapter={chapter}
            videos={videos}
            adoptedVideoIds={adoptedVideoIds}
            unavailableSegmentNumbers={unavailableSegmentNumbers}
            onToast={addToast}
            onRefresh={loadAll}
          />
        ) : workspaceTab === "postprocess" ? (
          <VideoPostProcessWorkspace
            projectId={projectId}
            chapter={chapter}
            videos={videos}
            onToast={addToast}
            onRefresh={loadAll}
          />
        ) : (
          <div className="min-h-0 flex-1">
            <VideoGenerationWorkspace
              prompts={allSegmentPrompts}
              segmentCards={segmentCardState.cards}
              segmentRecipes={effectiveRecipeBySegment}
              selectedSegmentNumber={selectedSegmentNumber}
              planningDraftVisible={segmentPlanDraftVisible}
              finalPromptBlockedReasons={finalPromptBlockedReasons}
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
