import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertCircle,
  CheckCircle2,
  Copy,
  Eye,
  Image as ImageIcon,
  ListChecks,
  Play,
  Save,
  Trash2,
  Upload,
  Wand2,
} from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { formatNumber, useI18n } from "../../../i18n";
import type { GeneratedImage } from "../../../api/image";
import {
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type ChapterVideoPrompt,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentWorkflowMode,
} from "../../../api/video";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown as MenuDropdown } from "../../../components/ui/Dropdown";
import { EmptyHint } from "../shared";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import {
  durationChoicesForLimits,
  durationLimitsLabel,
  isDurationAllowed,
  promptByteCount,
  promptCharCount,
  staleVideoPromptMessage,
} from "./videoOptionUtils";
import {
  videoGuidanceConfigurationError,
  VIDEO_FINAL_PROMPT_TYPE,
  videoWorkflowModeLabel,
  type PromptDraft,
  type VideoDurationLimits,
} from "./videoTabConstants";
import { videoPromptSourceFrameNumbers } from "./videoFramePromptUtils";
import { type VideoSegmentCardPlan } from "./videoSegmentPlan";
import { type VideoFrameImageIndexes } from "./videoImageUtils";
import { directorSections } from "./videoGenerationDisplay";

const MEDIA_SLOT_GRID_CLASS =
  "grid max-w-lg grid-cols-2 gap-2 [&>*:only-child]:col-span-2 [&>*:only-child]:max-w-64";

type FrameSlotRenderer = (
  label: string,
  prompt: ChapterVideoPrompt | null,
  image: GeneratedImage | null,
  emptyText?: string,
) => React.ReactNode;

export interface VideoGenerationEditorProps {
  selectedSegmentNumber: number | null;
  selectedSegmentCard: VideoSegmentCardPlan | undefined;
  planningDraftVisible: boolean;
  finalPromptBlockedReason: string | null;
  workflowMode: VideoSegmentWorkflowMode | null;
  segmentRecipe: VideoSegmentExecutionRecipe | null;
  selectedPrompt: ChapterVideoPrompt | null;
  selectedPromptStale: boolean;
  framePromptByNumber: Map<number, ChapterVideoPrompt>;
  frameImageIndexes: VideoFrameImageIndexes;
  referenceInputs?: React.ReactNode;
  acting: string | null;
  selectedPromptRunning: boolean;
  selectedPromptDirty: boolean;
  staleSavedReferenceCount: number;
  savedReferenceIdsError: string | null;
  chapterContentReady: boolean;
  durationLimits: VideoDurationLimits;
  draft: PromptDraft;
  renderFrameSlot: FrameSlotRenderer;
  renderContinuitySlot: () => React.ReactNode;
  renderFirstFrameSourceSelector: () => React.ReactNode;
  onGeneratePromptRequests: (segmentNumber: number) => void | Promise<void>;
  onSavePrompt: () => void | Promise<void>;
  onCopyPrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onGenerateVideo: () => void | Promise<void>;
  onOptimizePrompt: (
    prompt: ChapterVideoPrompt,
  ) => Promise<ChapterVideoPrompt | null>;
  onPreviewGenerateVideo: () => void | Promise<void>;
  onImportVideo: () => void | Promise<void>;
  onDeletePrompt: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onDraftChange: (draft: PromptDraft) => void;
}

export type SegmentInputDialog =
  | {
      kind: "frame";
      label: string;
      promptId: number | null;
      emptyText: string;
    }
  | { kind: "continuity" }
  | null;

export const VideoGenerationEditor: React.FC<VideoGenerationEditorProps> = ({
  selectedSegmentNumber,
  selectedSegmentCard,
  planningDraftVisible,
  finalPromptBlockedReason,
  workflowMode,
  segmentRecipe,
  selectedPrompt,
  selectedPromptStale,
  framePromptByNumber,
  frameImageIndexes,
  referenceInputs,
  acting,
  selectedPromptRunning,
  selectedPromptDirty,
  staleSavedReferenceCount,
  savedReferenceIdsError,
  chapterContentReady,
  durationLimits,
  draft,
  renderFrameSlot,
  renderContinuitySlot,
  renderFirstFrameSourceSelector,
  onGeneratePromptRequests,
  onSavePrompt,
  onCopyPrompt,
  onGenerateVideo,
  onOptimizePrompt,
  onPreviewGenerateVideo,
  onImportVideo,
  onDeletePrompt,
  onDraftChange,
}) => {
  const { locale, t } = useI18n();
  const updateDraft = (patch: Partial<PromptDraft>) =>
    onDraftChange({ ...draft, ...patch });
  const [durationText, setDurationText] = useState(String(draft.duration_secs));
  const durationFocused = useRef(false);
  useEffect(() => {
    if (!durationFocused.current) setDurationText(String(draft.duration_secs));
  }, [draft.duration_secs]);

  const frameInputStateForPrompt = (
    prompt: ChapterVideoPrompt,
    recipe: VideoSegmentExecutionRecipe,
  ) => {
    const firstFrameChainMode =
      workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN;
    const chatVideoMode = workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO;
    const storyboardMode = workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD;
    const usesContinuity =
      firstFrameChainMode && recipe.guidance_mode === "continuity";
    const { startFrameNumber, endFrameNumber } = videoPromptSourceFrameNumbers(
      prompt,
      workflowMode,
      recipe.guidance_mode,
    );
    const startFramePrompt = usesContinuity
      ? null
      : (framePromptByNumber.get(startFrameNumber ?? prompt.shot_number) ??
        null);
    const endFramePrompt =
      firstFrameChainMode || storyboardMode
        ? null
        : endFrameNumber == null
          ? null
          : (framePromptByNumber.get(endFrameNumber) ?? null);
    const generatedStartFrameImage =
      startFramePrompt == null
        ? null
        : (frameImageIndexes.boundaryByFrameNumber.get(
            startFramePrompt.shot_number,
          ) ?? null);
    const continuitySourceVideoId = usesContinuity
      ? recipe.continuity_source_video_id
      : null;
    const continuityTailFrameImage =
      continuitySourceVideoId == null
        ? null
        : (frameImageIndexes.tailByVideoId.get(continuitySourceVideoId) ??
          null);
    const actualStartFrameImage = usesContinuity
      ? continuityTailFrameImage
      : generatedStartFrameImage;
    const endFrameImage =
      endFramePrompt == null
        ? null
        : (frameImageIndexes.boundaryByFrameNumber.get(
            endFramePrompt.shot_number,
          ) ?? null);
    const startFrameMissingMessage =
      firstFrameChainMode && !actualStartFrameImage
        ? usesContinuity
          ? continuitySourceVideoId == null
            ? t("video.chapter.continuitySourceMissing")
            : t("video.chapter.sourceVideoNoTail", { id: continuitySourceVideoId })
          : t("video.chapter.frameImageMustPrepare", {
              label: t("video.chapter.frameInputStart"),
            })
        : null;
    const boundaryFrameMissingMessage =
      workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
        ? [
            startFramePrompt
              ? generatedStartFrameImage
                ? null
                : t("video.chapter.frameNumber", {
                    number: startFrameNumber ?? prompt.shot_number,
                  })
              : t("video.chapter.framePromptMissing", {
                  label: t("video.chapter.frameBoundary"),
                }),
            endFramePrompt
              ? endFrameImage
                ? null
                : t("video.chapter.frameNumber", {
                    number: endFrameNumber ?? prompt.shot_number,
                  })
              : t("video.chapter.framePromptMissing", {
                  label: t("video.chapter.frameBoundary"),
                }),
          ]
            .filter((value): value is string => Boolean(value))
            .join("、")
        : "";
    const storyboardFrameMissingMessage = storyboardMode
      ? startFramePrompt
        ? generatedStartFrameImage
          ? null
          : t("video.chapter.frameNumber", { number: prompt.shot_number })
        : t("video.chapter.framePromptMissing", {
            label: t("video.chapter.storyboardInput"),
          })
      : null;
    const frameInputMissingMessage = chatVideoMode
      ? null
      : storyboardFrameMissingMessage
        ? t("video.chapter.clickSlotToGenerate", {
            value: storyboardFrameMissingMessage,
          })
        : startFrameMissingMessage ||
          (boundaryFrameMissingMessage
            ? t("video.chapter.clickSlotToGenerate", {
                value: boundaryFrameMissingMessage,
              })
            : null);

    return {
      startFramePrompt,
      endFramePrompt,
      generatedStartFrameImage,
      actualStartFrameImage,
      endFrameImage,
      startFrameMissingMessage,
      frameInputMissingMessage,
      usesContinuity,
    };
  };

  const selectedPromptCharStats = useMemo(() => {
    if (!selectedPrompt) return null;
    const visualPrompt = draft.visual_prompt ?? selectedPrompt.visual_prompt;
    const motionPrompt =
      draft.motion_prompt ?? selectedPrompt.motion_prompt ?? "";
    const negativePrompt =
      draft.negative_prompt ?? selectedPrompt.negative_prompt ?? "";
    return {
      visualChars: promptCharCount(visualPrompt),
      visualBytes: promptByteCount(visualPrompt),
      motionChars: promptCharCount(motionPrompt),
      motionBytes: promptByteCount(motionPrompt),
      negativeChars: promptCharCount(negativePrompt),
      negativeBytes: promptByteCount(negativePrompt),
    };
  }, [selectedPrompt, draft]);

  const promptLengthText = (chars: number, bytes: number): string =>
    t("video.chapter.charsAndBytes", {
      chars: formatNumber(chars, locale),
      bytes: formatNumber(bytes, locale),
    });

  if (selectedSegmentNumber == null) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center">
        <EmptyHint text={t("video.chapter.chooseSegment")} />
      </div>
    );
  }
  if (!selectedSegmentCard) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center">
        <EmptyHint text={t("video.chapter.segmentNotInPlan")} />
      </div>
    );
  }
  if (workflowMode == null || segmentRecipe == null) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center">
        <EmptyHint
          text={
            planningDraftVisible
              ? t("video.chapter.segmentPlanPending")
              : t("video.chapter.segmentRecipeMissing")
          }
        />
      </div>
    );
  }
  if (!selectedPrompt) {
    const mode = segmentRecipe.workflow_mode;
    const startFramePrompt =
      framePromptByNumber.get(
        mode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES ||
          mode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN
          ? 1
          : selectedSegmentNumber,
      ) ?? null;
    const endFramePrompt =
      mode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES
        ? (framePromptByNumber.get(2) ?? null)
        : null;
    const startFrameImage = startFramePrompt
      ? (frameImageIndexes.boundaryByFrameNumber.get(
          startFramePrompt.shot_number,
        ) ?? null)
      : null;
    const endFrameImage = endFramePrompt
      ? (frameImageIndexes.boundaryByFrameNumber.get(
          endFramePrompt.shot_number,
        ) ?? null)
      : null;
    const ready = finalPromptBlockedReason == null;

    return (
      <div className="flex h-full min-h-0 flex-1 flex-col">
        <div className="custom-scrollbar min-h-0 flex-1 space-y-3 overflow-y-auto pr-2">
          <section className="a4n-flat-surface p-4 sm:p-5">
            <div className="flex min-w-0 flex-wrap items-start gap-3">
              <div className="min-w-0 flex-1">
                <div className="text-caption font-medium text-book-primary">
                  {t("video.chapter.workflowMode")} · {videoWorkflowModeLabel(mode, t)}
                </div>
                <h3 className="mt-1 text-base font-semibold text-book-text-main">
                  {t("video.chapter.segmentTitle", {
                    number: selectedSegmentNumber,
                    title: selectedSegmentCard.title,
                  })}
                </h3>
                {selectedSegmentCard.sceneSummary ? (
                  <p className="mt-1 line-clamp-2 text-small leading-relaxed text-book-text-muted">
                    {selectedSegmentCard.sceneSummary}
                  </p>
                ) : null}
              </div>
              <span
                className={`inline-flex shrink-0 items-center gap-1 rounded-full px-2.5 py-1 text-caption font-medium ${
                  ready ? "bg-success/10 text-success" : "bg-warning-bg text-warning"
                }`}
              >
                {ready ? <CheckCircle2 size={12} /> : <AlertCircle size={12} />}
                {ready ? t("video.chapter.statusReady") : t("video.chapter.statusRequired")}
              </span>
            </div>
            <div
              role="status"
              className={`mt-3 rounded-xl border px-3 py-2.5 text-small leading-relaxed ${
                ready
                  ? "border-success/25 bg-success/5 text-book-text-sub"
                  : "border-warning/35 bg-warning-bg text-warning"
              }`}
            >
              {finalPromptBlockedReason ?? t("video.chapter.requiredInputsReady")}
            </div>
          </section>

          {mode !== VIDEO_WORKFLOW_MODE_CHAT_VIDEO ? (
            <section
              aria-label={t("video.chapter.requiredInput")}
              className="a4n-flat-surface p-3 sm:p-4"
            >
              <div className="mb-3">
                <h3 className="text-xs font-medium text-book-text-main">
                  {t("video.chapter.requiredInput")}
                </h3>
                <div className="mt-0.5 text-caption text-book-text-muted">
                  {t("video.chapter.requiredInputHelp")}
                </div>
              </div>
              {renderFirstFrameSourceSelector()}
              <div className={MEDIA_SLOT_GRID_CLASS}>
                {mode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES ? (
                  <>
                    {renderFrameSlot(
                      t("video.chapter.frameInputStart"),
                      startFramePrompt,
                      startFrameImage,
                    )}
                    {renderFrameSlot(
                      t("video.chapter.frameInputEnd"),
                      endFramePrompt,
                      endFrameImage,
                    )}
                  </>
                ) : mode === VIDEO_WORKFLOW_MODE_STORYBOARD ? (
                  renderFrameSlot(
                    t("video.chapter.storyboardInput"),
                    startFramePrompt,
                    startFrameImage,
                    t("video.chapter.waitingStoryboard"),
                  )
                ) : segmentRecipe.guidance_mode === "system_frames" ? (
                  renderFrameSlot(
                    t("video.chapter.frameInputStart"),
                    startFramePrompt,
                    startFrameImage,
                    t("video.chapter.waitingFrame"),
                  )
                ) : (
                  renderContinuitySlot()
                )}
              </div>
            </section>
          ) : null}

          {referenceInputs}
        </div>
          <section className="mt-3 flex shrink-0 flex-wrap items-center justify-between gap-3 border-t border-book-border/40 pt-3">
            <span className="text-small text-book-text-muted">
              {t("video.chapter.generateAfterEdit")}
            </span>
            <BookButton
              variant={ready ? "primary" : "secondary"}
              size="sm"
              disabled={acting != null || !ready}
              title={finalPromptBlockedReason ?? undefined}
              onClick={() => void onGeneratePromptRequests(selectedSegmentNumber)}
            >
              <Wand2 size={12} />
              {t("video.chapter.generateFinalPrompt")}
            </BookButton>
          </section>
      </div>
    );
  }

  const chatVideoMode = workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO;
  const firstFrameChainMode =
    workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN;
  const storyboardMode = workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD;
  const guidanceConfigurationError = videoGuidanceConfigurationError(
    workflowMode,
    selectedSegmentNumber,
    segmentRecipe.guidance_mode,
    t,
  );
  const frameStageLabel = videoWorkflowModeLabel(workflowMode, t);
  const {
    startFramePrompt,
    endFramePrompt,
    generatedStartFrameImage,
    actualStartFrameImage,
    endFrameImage,
    startFrameMissingMessage,
    frameInputMissingMessage,
    usesContinuity,
  } = frameInputStateForPrompt(selectedPrompt, segmentRecipe);
  const selectedPromptHasVideo = selectedPrompt.generated_video_id != null;
  const selectedPromptLockedReason = selectedPromptRunning
    ? t("video.chapter.promptRunningCannot", {
        operation: `${t("common.edit")} / ${t("common.save")}`,
      })
    : null;
  const selectedPromptDownstreamNotice = selectedPromptHasVideo
    ? t("video.chapter.promptHasDownstream")
    : null;
  const durationConfigError = durationLimits.configError;
  const saveBlockedReason = selectedPromptLockedReason ?? undefined;
  const optimizeBlockedReason =
    (selectedPromptDirty ? t("video.chapter.promptDirty") : undefined) ??
    selectedPromptLockedReason ??
    undefined;
  const videoGenerationInFlight =
    acting === `generate-video-${selectedPrompt.id}`;
  const selectedGenerateBlockedReason =
    durationConfigError ??
    (selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE
      ? t("video.chapter.promptTypeRequired")
      : undefined) ??
    (selectedPromptStale ? t("video.chapter.promptStale") : undefined) ??
    (savedReferenceIdsError
      ? t("video.chapter.savedReferenceError", { error: savedReferenceIdsError })
      : undefined) ??
    (staleSavedReferenceCount > 0
      ? t("video.chapter.staleReferenceNotice", {
          count: staleSavedReferenceCount,
        })
      : undefined) ??
    (selectedPromptDirty
      ? t("video.chapter.promptDirtyBeforeGenerate")
      : undefined) ??
    guidanceConfigurationError ??
    frameInputMissingMessage ??
    (selectedPromptRunning ? t("video.chapter.promptRunningShort") : undefined) ??
    (!chapterContentReady ? t("video.chapter.currentChapterEmpty") : undefined);
  const selectedImportBlockedReason =
    (selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE
      ? t("video.chapter.promptTypeRequired")
      : undefined) ??
    (selectedPromptStale ? t("video.chapter.promptStale") : undefined) ??
    (selectedPromptDirty
      ? t("video.chapter.promptDirtyBeforeGenerate")
      : undefined) ??
    (selectedPromptRunning ? t("video.chapter.promptRunningShort") : undefined) ??
    (!chapterContentReady
      ? t("video.chapter.currentChapterEmptyImport")
      : undefined);
  const deletePromptBlockedReason = selectedPromptRunning
    ? t("video.chapter.promptRunningShort")
    : undefined;
  const savingPrompt = acting === `save-prompt-${selectedPrompt.id}`;
  const showSavePrimary = savingPrompt || selectedPromptDirty;
  const showCopyPrimary = !showSavePrimary && chatVideoMode;
  const showGeneratePrimary =
    !showSavePrimary &&
    !chatVideoMode;
  const editorMetadata = [
    t("video.chapter.seconds", { count: draft.duration_secs }),
    getOperationStatusLabel(selectedPrompt.status, locale),
    selectedPromptStale ? t("video.chapter.bodyChanged") : null,
    selectedPrompt.generated_video_id != null
      ? t("video.chapter.generatedVideo")
      : null,
    selectedPromptDirty ? t("common.unsavedChangesTitle") : null,
  ].filter((value): value is string => Boolean(value));
  const sections = directorSections(selectedPrompt.metadata_json, t);
  const selectedPromptSceneSummary = selectedPrompt.scene_summary?.trim() ?? "";

  const actions = (
      <footer className="mt-3 flex min-w-0 shrink-0 flex-wrap items-center gap-2 border-t border-book-border/40 pt-3">
        <span className="min-w-0 flex-1 text-small text-book-text-muted">
          {selectedGenerateBlockedReason ?? editorMetadata.join(" · ")}
        </span>
        <div className="flex shrink-0 items-center gap-2">
          {showSavePrimary && (
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onSavePrompt()}
              disabled={acting != null || savingPrompt || selectedPromptRunning}
              title={saveBlockedReason}
              className="!min-h-9 !px-3 sm:!min-h-7"
            >
              <span className="inline-flex items-center gap-1">
                {savingPrompt ? <LoadingSpinner size="sm" /> : <Save size={12} />}
                {savingPrompt
                  ? t("video.chapter.saving")
                  : t("video.chapter.saveChanges")}
              </span>
            </BookButton>
          )}
          {showCopyPrimary && (
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onCopyPrompt(selectedPrompt)}
              disabled={acting != null || selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE}
              className="!min-h-9 !px-3 sm:!min-h-7"
            >
              <span className="inline-flex items-center gap-1">
                <Copy size={12} />
                {t("video.chapter.copyPrompt")}
              </span>
            </BookButton>
          )}
          {showGeneratePrimary && (
            <BookButton
              variant="primary"
              size="sm"
              onClick={() => void onGenerateVideo()}
              disabled={
                acting != null ||
                videoGenerationInFlight ||
                selectedGenerateBlockedReason != null
              }
              title={selectedGenerateBlockedReason}
              className="!min-h-9 !px-3 sm:!min-h-7"
            >
              <span className="inline-flex items-center gap-1">
                {videoGenerationInFlight ? (
                  <LoadingSpinner size="sm" />
                ) : (
                  <Play size={12} />
                )}
                {videoGenerationInFlight
                  ? t("video.chapter.generatingVideo")
                  : t("video.chapter.generateVideo")}
              </span>
            </BookButton>
          )}
          <MenuDropdown
            items={[
              {
                label:
                  acting === `optimize-prompt-${selectedPrompt.id}`
                    ? t("video.chapter.optimizingPrompt")
                    : t("video.chapter.optimizePrompt"),
                icon:
                  acting === `optimize-prompt-${selectedPrompt.id}` ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Wand2 size={13} />
                  ),
                disabled:
                  acting != null ||
                  acting === `optimize-prompt-${selectedPrompt.id}` ||
                  optimizeBlockedReason != null,
                onClick: () => void onOptimizePrompt(selectedPrompt),
              },
              ...(!chatVideoMode
                ? [
                    {
                      label: t("video.chapter.viewModelRequest"),
                      icon: <Eye size={13} />,
                      disabled:
                        selectedPrompt.prompt_type !== VIDEO_FINAL_PROMPT_TYPE ||
                        staleSavedReferenceCount > 0 ||
                        savedReferenceIdsError != null,
                      onClick: () => void onPreviewGenerateVideo(),
                    },
                    {
                      label:
                        acting === `import-video-${selectedPrompt.id}`
                          ? t("video.chapter.importingVideo")
                          : t("video.chapter.importVideo"),
                      icon:
                        acting === `import-video-${selectedPrompt.id}` ? (
                          <LoadingSpinner size="sm" />
                        ) : (
                          <Upload size={13} />
                        ),
                      disabled:
                        acting != null ||
                        acting === `import-video-${selectedPrompt.id}` ||
                        selectedImportBlockedReason != null,
                      onClick: () => void onImportVideo(),
                    },
                  ]
                : []),
              ...(deletePromptBlockedReason == null
                ? [
                    { type: "divider" as const },
                    {
                      label: t("video.chapter.deletePrompt"),
                      icon: <Trash2 size={13} />,
                      danger: true,
                      disabled:
                        acting != null ||
                        acting === `delete-prompt-${selectedPrompt.id}`,
                      onClick: () => void onDeletePrompt(selectedPrompt),
                    },
                  ]
                : []),
            ]}
          />
        </div>
      </footer>
  );

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col">
      <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto pr-2">
      {selectedPromptStale && (
        <div className="mb-2 text-small text-error">
          {staleVideoPromptMessage(frameStageLabel, t)}
        </div>
      )}
      {frameInputMissingMessage && (
        <div className="mb-2 text-small text-warning">{frameInputMissingMessage}</div>
      )}
      {selectedPromptLockedReason && (
        <div className="mb-2 text-small text-warning">{selectedPromptLockedReason}</div>
      )}
      {selectedPromptDownstreamNotice && (
        <div className="mb-2 text-small text-warning">{selectedPromptDownstreamNotice}</div>
      )}
      {durationConfigError && (
        <div className="mb-2 text-small text-error">{durationConfigError}</div>
      )}
      {selectedPromptDirty && (
        <div className="mb-2 text-small text-warning">
          {t("video.chapter.promptDirtyNotice")}
        </div>
      )}

      {selectedPromptRunning ? (
        <div className="a4n-flat-surface mb-3 flex items-center gap-2 px-3 py-2.5 text-small text-book-text-muted">
          <LoadingSpinner size="sm" />
          {t("video.chapter.promptRunning")}
        </div>
      ) : null}
      <fieldset
        disabled={selectedPromptRunning || acting != null}
        className="m-0 grid min-w-0 gap-4 border-0 p-0 md:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]"
      >
        <div className="min-w-0">
        {chatVideoMode ? null : firstFrameChainMode ? (
          <div className="a4n-flat-surface mb-3 p-3">
            {renderFirstFrameSourceSelector()}
            <div className="mb-2 flex items-center gap-2">
              <div className="inline-flex items-center gap-1.5 text-xs text-book-text-main">
                <ImageIcon size={12} className="text-book-primary" />
                {t("video.chapter.frameInput")}
                <span className="text-caption text-book-text-muted">
                  · {actualStartFrameImage ? t("video.chapter.frameReady") : t("video.chapter.frameWaiting")}
                </span>
              </div>
            </div>
            {startFrameMissingMessage && (
              <div className="mb-2 text-caption text-warning">{startFrameMissingMessage}</div>
            )}
            <div className={MEDIA_SLOT_GRID_CLASS}>
              {usesContinuity
                ? renderContinuitySlot()
                : renderFrameSlot(
                    t("video.chapter.frameInputStart"),
                    startFramePrompt,
                    actualStartFrameImage,
                    t("video.chapter.waitingFrame"),
                  )}
            </div>
          </div>
        ) : storyboardMode ? (
          <div className="a4n-flat-surface mb-3 p-3">
            <div className="mb-2 flex min-w-0 items-center gap-2">
              <ImageIcon size={12} className="shrink-0 text-book-primary" />
              <div className="min-w-0 flex-1">
                <div className="text-xs font-medium text-book-text-main">
                  {t("video.chapter.storyboardInput")}
                </div>
                <div className="mt-0.5 text-caption text-book-text-muted">
                  {generatedStartFrameImage ? t("video.chapter.frameReady") : t("video.chapter.frameWaiting")}
                </div>
              </div>
            </div>
            {frameInputMissingMessage && (
              <div className="mb-2 text-caption text-warning">{frameInputMissingMessage}</div>
            )}
            <div className={MEDIA_SLOT_GRID_CLASS}>
              {renderFrameSlot(
                t("video.chapter.storyboardInput"),
                startFramePrompt,
                generatedStartFrameImage,
                t("video.chapter.waitingStoryboard"),
              )}
            </div>
          </div>
        ) : (
          <div className="a4n-flat-surface mb-3 p-3">
            <div className="mb-2 flex min-w-0 items-center gap-2">
              <div className="min-w-0 flex-1">
                <div className="text-xs font-medium text-book-text-main">
                  {t("video.chapter.frameBoundary")}
                </div>
                <div className="mt-0.5 text-caption text-book-text-muted">
                  {t("video.chapter.generatedFrames", {
                    count:
                      Number(Boolean(generatedStartFrameImage)) +
                      Number(Boolean(endFrameImage)),
                    total: 2,
                  })}
                </div>
              </div>
            </div>
            <div className={MEDIA_SLOT_GRID_CLASS}>
              {renderFrameSlot(
                t("video.chapter.frameInputStart"),
                startFramePrompt,
                generatedStartFrameImage,
              )}
              {renderFrameSlot(
                t("video.chapter.frameInputEnd"),
                endFramePrompt,
                endFrameImage,
              )}
            </div>
          </div>
        )}

        {staleSavedReferenceCount > 0 ? (
          <div className="mb-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
            {t("video.chapter.staleReferenceNotice", {
              count: staleSavedReferenceCount,
            })}
          </div>
        ) : null}
        {savedReferenceIdsError ? (
          <div className="mb-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
            {t("video.chapter.savedReferenceError", {
              error: savedReferenceIdsError,
            })}
          </div>
        ) : null}
        {referenceInputs ? <div className="mb-3">{referenceInputs}</div> : null}

        {selectedPromptSceneSummary ? (
          <div className="mb-3 text-xs">
            <span className="text-book-text-muted">{t("video.chapter.sceneSummary")}</span>
            <div className="mt-1 min-h-[34px] border-l-2 border-book-border/55 py-1 pl-2.5 leading-relaxed text-book-text-sub">
              {selectedPromptSceneSummary}
            </div>
          </div>
        ) : null}

        {sections.length > 0 && (
          <details className="a4n-flat-surface mb-3 p-2">
            <summary className="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto] items-center gap-2 text-xs text-book-text-main [&::-webkit-details-marker]:hidden">
              <span className="inline-flex min-w-0 items-center gap-1.5">
                <ListChecks size={12} className="text-book-primary" />
                {t("video.chapter.directorConstraints")}
              </span>
              <span className="text-caption text-book-text-muted">
                {t("video.chapter.sectionCount", { count: sections.length })}
              </span>
            </summary>
            <dl className="mt-2 divide-y divide-book-border/40 border-y border-book-border/40">
              {sections.map((section) => (
                <div
                  key={section.key}
                  className="grid min-w-0 gap-1 px-2.5 py-2 md:grid-cols-[110px_minmax(0,1fr)] md:gap-3"
                >
                  <dt className="text-caption text-book-text-muted">{section.label}</dt>
                  <dd className="whitespace-pre-wrap break-words text-small leading-relaxed text-book-text-sub">
                    {section.text}
                  </dd>
                </div>
              ))}
            </dl>
          </details>
        )}

        {selectedPromptCharStats && (
          <details className="mb-3 text-caption text-book-text-muted">
            <summary className="w-fit cursor-pointer hover:text-book-text-main">
              {t("video.chapter.diagnostics")}
            </summary>
            <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 border-l-2 border-book-border/60 pl-2 text-caption">
              <span className="text-book-text-muted">
                {chatVideoMode
                  ? t("video.chapter.chatPrompt")
                  : t("video.chapter.visualPrompt")} {" "}
                {promptLengthText(
                  selectedPromptCharStats.visualChars,
                  selectedPromptCharStats.visualBytes,
                )}
              </span>
              {!chatVideoMode && (
                <span className="text-book-text-muted">
                  {t("video.chapter.motionPrompt")} {" "}
                  {promptLengthText(
                    selectedPromptCharStats.motionChars,
                    selectedPromptCharStats.motionBytes,
                  )}
                </span>
              )}
              <span className="text-book-text-muted">
                {t("video.chapter.constraints")} {" "}
                {promptLengthText(
                  selectedPromptCharStats.negativeChars,
                  selectedPromptCharStats.negativeBytes,
                )}
              </span>
            </div>
          </details>
        )}

        </div>
        <div className="grid min-w-0 content-start gap-3">
          <label className="flex min-w-0 flex-col gap-1">
            <span className="text-xs text-book-text-muted">{t("video.chapter.videoDuration")}</span>
            {durationLimits.durationMode === "fixed" ? (
              <ThemedSelect
                value={draft.duration_secs}
                onChange={(selectedValue) =>
                  updateDraft({ duration_secs: Number(selectedValue) })
                }
                disabled={selectedPromptRunning}
                className="book-control h-9 px-2 text-xs"
              >
                {!isDurationAllowed(draft.duration_secs, durationLimits) && (
                  <option value={draft.duration_secs} disabled>
                    {t("video.chapter.durationNotSupported", {
                      count: draft.duration_secs,
                    })}
                  </option>
                )}
                {durationChoicesForLimits(durationLimits).map((duration) => (
                  <option key={duration} value={duration}>
                    {t("video.chapter.seconds", { count: duration })}
                  </option>
                ))}
              </ThemedSelect>
            ) : (
              <input
                type="number"
                min={durationLimits.minDurationSecs}
                max={durationLimits.maxDurationSecs}
                step={1}
                value={durationText}
                onFocus={() => { durationFocused.current = true; }}
                onChange={(event) => {
                  const nextText = event.currentTarget.value;
                  setDurationText(nextText);
                  if (nextText.trim() && Number.isSafeInteger(Number(nextText))) {
                    updateDraft({ duration_secs: Number(nextText) });
                  }
                }}
                onBlur={(event) => {
                  durationFocused.current = false;
                  const nextText = event.currentTarget.value;
                  const nextValue = Number(nextText);
                  if (nextText.trim() && Number.isSafeInteger(nextValue)) {
                    updateDraft({ duration_secs: nextValue });
                    setDurationText(String(nextValue));
                  } else {
                    setDurationText(String(draft.duration_secs));
                  }
                }}
                disabled={selectedPromptRunning}
                className="book-control h-9 px-2 text-xs"
              />
            )}
            <span className="text-caption text-book-text-muted">
              {t("video.chapter.modelSupports", {
                value: durationLimitsLabel(durationLimits, t),
              })}
            </span>
          </label>
          <label className="flex min-w-0 flex-col gap-1">
            <span className="text-xs text-book-text-muted">
              {chatVideoMode
                ? t("video.chapter.chatPrompt")
                : t("video.chapter.visualPrompt")}
            </span>
            <textarea
              value={draft.visual_prompt}
              onChange={(event) =>
                updateDraft({ visual_prompt: event.target.value })
              }
              disabled={selectedPromptRunning}
              className="book-control min-h-[220px] resize-y p-2 text-xs leading-relaxed"
            />
          </label>
          {!chatVideoMode && (
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-xs text-book-text-muted">{t("video.chapter.motionPrompt")}</span>
              <textarea
                value={draft.motion_prompt}
                onChange={(event) =>
                  updateDraft({ motion_prompt: event.target.value })
                }
                disabled={selectedPromptRunning}
                className="book-control min-h-[140px] resize-y p-2 text-xs leading-relaxed"
              />
            </label>
          )}
          <label className="flex min-w-0 flex-col gap-1">
            <span className="text-xs text-book-text-muted">{t("video.chapter.constraints")}</span>
            <textarea
              value={draft.negative_prompt}
              onChange={(event) =>
                updateDraft({ negative_prompt: event.target.value })
              }
              disabled={selectedPromptRunning}
              className="book-control min-h-[96px] resize-y p-2 text-xs leading-relaxed"
            />
          </label>
        </div>
      </fieldset>
      </div>
      {actions}
    </div>
  );
};
