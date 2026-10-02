import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { extractErrorMessage } from "../../../api";
import {
  updateChapterVideoSegmentRecipe,
  updateVideoSegmentPlanDraftRecipe,
  type ChapterVideoPrompt,
  type GeneratedVideo,
  type VideoGenConfig,
  type VideoSegmentExecutionRecipe,
  type VideoTimelineSummary,
} from "../../../api/video";
import type { GeneratedImage } from "../../../api/image";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import type { ShootingSceneV2 } from "../../../api/shootingScript";
import { usePersistedState } from "../../../hooks/usePersistedState";
import { asJsonRecord } from "../../../utils/jsonRecord";
import { sha256Hex } from "../../../utils/contentHash";
import {
  createReferenceMediaDraft,
  referenceMediaDraftFromRecipe,
  referenceMediaDraftScopeKey,
  referenceMediaRecipeFromDraft,
  type ReferenceMediaDraft,
} from "./referenceMediaWorkspace";
import {
  DEFAULT_DRAFT,
  DEFAULT_VIDEO_OPTIONS,
  type PromptDraft,
  type VideoGenOptions,
} from "./videoTabConstants";
import {
  latestVideoSegmentPlanPrompt,
  readVideoSegmentRecipes,
  videoOptionsFromSegmentRecipe,
  videoSegmentCardsFromPlanPrompt,
  videoSegmentCardsFromSegments,
  videoSegmentRecipeFrameFingerprintJson,
  videoSegmentRecipeGenerationFingerprintJson,
  type VideoSegmentCardPlan,
} from "./videoSegmentPlan";
import {
  parseNullableNumber,
  parseVideoOptions,
  serializeNullableNumber,
  serializeVideoOptions,
} from "./videoOptionUtils";
import {
  loadedSegmentPlanDraftIsActive,
  type LoadedSegmentPlanDraft,
} from "./loadVideoTabData";
import type { MessageTranslator } from "../../../i18n";
import type { ToastType } from "../../../components/feedback/Toast";
import type { InsertedSegmentTimelinePanelHandle } from "./InsertedSegmentTimelinePanel";

export type VideoWorkspaceTab = "director" | "screenplay" | "materials" | "frames" | "video" | "assembly" | "postprocess";

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

export interface VideoTabStateInput {
  projectId: number;
  chapterId: number | null;
  chapterStorageKey: string | null;
  videoDataKey: string | null;
  currentContentHash: string | null;
  workspaceTabs: readonly { id: VideoWorkspaceTab }[];
  t: MessageTranslator;
  addToast: (message: string, type: ToastType) => void;
}

export function useVideoTabState({
  projectId,
  chapterId,
  chapterStorageKey,
  videoDataKey,
  currentContentHash,
  workspaceTabs,
  t,
  addToast,
}: VideoTabStateInput) {
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
  const defaultWorkspaceTab: VideoWorkspaceTab = "materials";
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
  const [shootingScenes, setShootingScenes] = useState<ShootingSceneV2[]>([]);
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
  useLayoutEffect(() => {
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

  return {
    planningOptionsState,
    planningOptions,
    setPlanningOptions,
    resetPlanningOptions,
    acting,
    videoActionScopeKey,
    beginVideoAction,
    isCurrentVideoActionScope,
    finishVideoAction,
    workspaceTabState,
    workspaceTab,
    setWorkspaceTab,
    prompts,
    setPrompts,
    timeline,
    setTimeline,
    insertedSegmentPanelRef,
    segmentPlanPreview,
    setSegmentPlanPreview,
    segmentPlanDraft,
    setSegmentPlanDraft,
    videos,
    setVideos,
    images,
    setImages,
    shootingScenes,
    setShootingScenes,
    frameSegmentPlanPromptForCards,
    segmentPlanDraftVisible,
    segmentCardState,
    segmentRecipeState,
    selectedSegmentState,
    selectedSegmentNumber,
    setSelectedSegmentNumber,
    segmentConfigOpen,
    setSegmentConfigOpen,
    savingRecipeSegments,
    recipeSaveErrors,
    effectiveRecipeBySegment,
    effectiveRecipeBySegmentRef,
    recipeFingerprintBySegment,
    frameRecipeFingerprintBySegment,
    recipeFingerprintErrorsBySegment,
    selectedSegmentRecipe,
    workflowMode,
    segmentStorageKey,
    selectedPromptState,
    selectedPromptId,
    setSelectedPromptId,
    selectedVideoState,
    selectedVideoId,
    setSelectedVideoId,
    pendingVideoState,
    pendingVideoId,
    setPendingVideoId,
    selectedVideoSelectionScopeRef,
    options,
    videoLibraryImageIds,
    generationReferenceIds,
    persistSegmentRecipe,
    updateSegmentRecipe,
    awaitSegmentRecipeSaved,
    draft,
    setDraft,
    referenceMediaDraftScopeForSegment,
    referenceMediaDraft,
    setReferenceMediaDraft,
    referenceMediaDraftRef,
    referenceMediaDraftDirty,
    setReferenceMediaDraftDirty,
    referenceMediaDraftScopeRef,
    loading,
    setLoading,
    videoDataError,
    setVideoDataError,
    segmentPlanDraftLoading,
    setSegmentPlanDraftLoading,
    segmentPlanDraftError,
    setSegmentPlanDraftError,
    loadedDataKey,
    setLoadedDataKey,
    videoDataKey,
    activeVideoConfig,
    setActiveVideoConfig,
    promptRequestPreviewOpen,
    setPromptRequestPreviewOpen,
    promptRequestPreviewLoading,
    setPromptRequestPreviewLoading,
    promptRequestPreviewData,
    setPromptRequestPreviewData,
    promptRequestPreviewScopeRef,
    loadAllSeqRef,
    promptLoadSeqRef,
    segmentPlanDraftLoadSeqRef,
    attachedSegmentPlanTaskRef,
    activeVideoDataKeyRef,
    imageRequestPreviewOpen,
    setImageRequestPreviewOpen,
    imageRequestPreviewLoading,
    setImageRequestPreviewLoading,
    imageRequestPreviewData,
    setImageRequestPreviewData,
    imageRequestPreviewScopeRef,
  };
}

export type VideoTabState = ReturnType<typeof useVideoTabState>;
