import { useCallback, useEffect, useMemo } from "react";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type GeneratedVideo,
} from "../../../api/video";
import type { ChapterRow } from "../shared";
import { textInputValue } from "../../../utils/text";
import { sha256Hex } from "../../../utils/contentHash";
import type { MessageTranslator } from "../../../i18n";
import type { VideoTabState } from "./useVideoTabState";
import type { useVideoTabDerivedState } from "./useVideoTabDerivedState";
import type {
  ReferenceMediaBindingRow,
  ReferenceMediaDraft,
} from "./referenceMediaWorkspace";
import { referenceMediaRecipeFromDraft } from "./referenceMediaWorkspace";
import {
  CHAT_VIDEO_DURATION_LIMITS,
  DEFAULT_DRAFT,
  VIDEO_FINAL_PROMPT_TYPE,
  VIDEO_FIRST_FRAME_PROMPT_TYPE,
  VIDEO_FRAME_PROMPT_TYPE,
  VIDEO_STORYBOARD_PROMPT_TYPE,
  videoExecutionSegmentIdentityMatchesPrompt,
  videoExecutionSegmentIdentityMatchesVideo,
  videoExecutionSegmentIdentityIndex,
  videoExecutionIdentityOfPrompt,
  videoExecutionSegmentNumberInPlanByLineage,
  videoGuidanceConfigurationError,
  videoTailFrameSourceVideoId,
  finalVideoExecutionIdentityIsComplete,
  latestVideoFrameImageForPrompt,
  videoOptionsValidationError,
  videoSegmentRecipeGenerationFingerprintJson,
} from "./videoTabUtils";

type DerivedVideoTabState = ReturnType<typeof useVideoTabDerivedState>;
export interface VideoTabViewStateOptions
  extends VideoTabState,
    DerivedVideoTabState {
  chapter: ChapterRow | null;
  currentContentHash: string | null;
  chapterContentHashError: string | null;
  addToast: (message: string, type: "success" | "error" | "info") => void;
  t: MessageTranslator;
  loadAll: () => Promise<void>;
}

export function useVideoTabViewState({
  chapter,
  currentContentHash,
  chapterContentHashError,
  addToast,
  t,
  loadAll,
  ...state
}: VideoTabViewStateOptions) {
  const {
    prompts,
    videos,
    images,
    timeline,
    frameSegmentPlanPromptForCards,
    segmentCardState,
    effectiveRecipeBySegment,
    effectiveRecipeBySegmentRef,
    recipeFingerprintBySegment,
    frameRecipeFingerprintBySegment,
    recipeFingerprintErrorsBySegment,
    selectedSegmentNumber,
    selectedPrompt,
    workflowMode,
    selectedPromptId,
    draft,
    referenceMediaDraftRef,
    referenceMediaDraftDirty,
    setReferenceMediaDraft,
    setReferenceMediaDraftDirty,
    updateSegmentRecipe,
    awaitSegmentRecipeSaved,
    referenceMediaDraftScopeForSegment,
    selectableReferenceIds,
    durationLimits,
    options,
    workspaceTab,
    videoDataKey,
    loadedDataKey,
    loading,
    generationReferenceIds,
    videoLibraryImageIds,
    workspaceTabState,
    selectedSegmentState,
    selectedPromptState,
    selectedVideoState,
    pendingVideoState,
    segmentRecipeState,
    recipeSaveErrors,
    referenceMediaDraftScopeRef,
    setSegmentConfigOpen,
    setDraft,
    setSelectedPromptId,
    setSelectedSegmentNumber,
    segmentConfigOpen,
  } = state;
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


  return {
    selectedPromptDirty,
    videosById,
    frameSegmentIdentityIndex,
    previousAdoptedVideos,
    mixedAdoptedVideoIds,
    unavailableMixedSegmentNumbers,
    allCurrentFramePrompts,
    allSegmentPrompts,
    latestFramePromptBySegment,
    selectionValidationReady,
    videoDataLoading,
    invalidGenerationReferenceIds,
    invalidVideoLibraryImageIds,
    promptInputSelectionError,
    videoOptionsStateError,
    videoPersistedStateError,
    effectiveDurationLimits,
    setReferenceMediaPrompt,
    setReferenceMediaImageRows,
    setReferenceMediaVideoRows,
    setReferenceMediaAudioRows,
    confirmSegmentRecipeChange,
    setGenerationReferenceIds,
    setVideoLibraryImageIds,
    finalPromptBlockedReasons,
    saveReferenceMediaDraft,
    handleSelectSegment,
    handleSelectPrompt,
    handleActivateSegment,
    handleCloseSegmentConfig,
  };
}
