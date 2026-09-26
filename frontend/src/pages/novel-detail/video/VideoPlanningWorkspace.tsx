import { ThemedSelect } from "../../../components/ui/ThemedSelect";
import React, { useEffect, useMemo, useState } from "react";
import { ListChecks, Pencil, RotateCcw, Save, Trash2, Wand2, X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { formatNumber, useI18n } from "../../../i18n";
import type { ChapterVideoPrompt, VideoSegmentPlanV2 } from "../../../api/video";
import type { ShootingSceneV1 } from "../../../api/shootingScript";
import { BookButton } from "../../../components/ui/BookButton";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { asArray, asJsonRecord } from "../../../utils/jsonRecord";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { EmptyHint } from "../shared";
import { VideoDurationOptionsPicker } from "./VideoDurationOptionsPicker";
import { VideoSegmentPlanEditor } from "./VideoSegmentPlanEditor";
import {
  promptOutputLanguageLabel,
  VIDEO_ASPECT_RATIOS,
  type VideoDurationLimits,
  type VideoGenOptions,
} from "./videoTabConstants";
import type { LoadedSegmentPlanDraft } from "./loadVideoTabData";

interface VideoPlanningWorkspaceProps {
  prompt: ChapterVideoPrompt | null;
  previewSegments: Record<string, unknown>[];
  draft: LoadedSegmentPlanDraft | null;
  options: VideoGenOptions;
  durationLimits: VideoDurationLimits;
  loading: boolean;
  loadError: string | null;
  dataReady: boolean;
  acting: string | null;
  shootingScenes: ShootingSceneV1[];
  onGenerate: (
    targetNarrativeUnitIndex: number,
    regenerateExisting: boolean,
  ) => void | Promise<void>;
  onOptionsChange: React.Dispatch<React.SetStateAction<VideoGenOptions>>;
  onRetryLoad: () => void | Promise<void>;
  onDelete: (prompt: ChapterVideoPrompt) => void | Promise<void>;
  onSavePlan?: (segments: VideoSegmentPlanV2[]) => Promise<boolean>;
}

const textValue = (value: unknown): string | null =>
  typeof value === "string" && value.trim() ? value.trim() : null;

const numberValue = (value: unknown): number | null =>
  typeof value === "number" && Number.isFinite(value) ? value : null;

type PlanningSceneSelection = Pick<ShootingSceneV1, "narrative_unit_index"> & {
  shots: Array<Pick<ShootingSceneV1["shots"][number], "shot_id">>;
};

export const selectVideoPlanningUnitIndex = (
  shootingScenes: PlanningSceneSelection[],
  draftTarget: number | undefined,
  current: number | null,
  plannedShotIds: ReadonlySet<string>,
): number | null => {
  const available = new Set(
    shootingScenes.map((scene) => scene.narrative_unit_index),
  );
  if (draftTarget != null && available.has(draftTarget)) return draftTarget;

  const currentScene = shootingScenes.find(
    (scene) => scene.narrative_unit_index === current,
  );
  const currentScenePlanned =
    currentScene != null &&
    currentScene.shots.length > 0 &&
    currentScene.shots.every((shot) => plannedShotIds.has(shot.shot_id));
  if (currentScene != null && !currentScenePlanned) {
    return currentScene.narrative_unit_index;
  }

  return shootingScenes.find((scene) =>
    scene.shots.some((shot) => !plannedShotIds.has(shot.shot_id)),
  )?.narrative_unit_index ??
    currentScene?.narrative_unit_index ??
    shootingScenes[0]?.narrative_unit_index ??
    null;
};

const mergeVideoPlanningSegments = (
  persistedSegments: Record<string, unknown>[],
  previewSegments: Record<string, unknown>[],
  shootingScenes: PlanningSceneSelection[],
): Record<string, unknown>[] => {
  if (previewSegments.length === 0) return persistedSegments;
  if (persistedSegments.length === 0) return previewSegments;

  const previewShotIds = new Set(
    previewSegments
      .map((segment) => textValue(segment.source_shot_id))
      .filter((value): value is string => value != null),
  );
  const shotOrder = new Map(
    shootingScenes
      .flatMap((scene) => scene.shots)
      .map((shot, index) => [shot.shot_id, index] as const),
  );
  return [
    ...persistedSegments.filter((segment) => {
      const sourceShotId = textValue(segment.source_shot_id);
      return sourceShotId == null || !previewShotIds.has(sourceShotId);
    }),
    ...previewSegments,
  ]
    .sort((left, right) => {
      const leftShotId = textValue(left.source_shot_id);
      const rightShotId = textValue(right.source_shot_id);
      const leftShotOrder =
        (leftShotId == null ? undefined : shotOrder.get(leftShotId)) ??
        Number.MAX_SAFE_INTEGER;
      const rightShotOrder =
        (rightShotId == null ? undefined : shotOrder.get(rightShotId)) ??
        Number.MAX_SAFE_INTEGER;
      if (leftShotOrder !== rightShotOrder) {
        return leftShotOrder - rightShotOrder;
      }
      if (leftShotId != null && leftShotId === rightShotId) {
        return (
          (numberValue(left.segment_index_in_shot) ?? Number.MAX_SAFE_INTEGER) -
          (numberValue(right.segment_index_in_shot) ?? Number.MAX_SAFE_INTEGER)
        );
      }
      return 0;
    })
    .map((segment, index) => ({
      ...segment,
      segment_number: index + 1,
    }));
};

export const VideoPlanningWorkspace: React.FC<VideoPlanningWorkspaceProps> = ({
  prompt,
  previewSegments,
  draft,
  options,
  durationLimits,
  loading,
  loadError,
  dataReady,
  acting,
  shootingScenes,
  onGenerate,
  onOptionsChange,
  onRetryLoad,
  onDelete,
  onSavePlan,
}) => {
  const { locale, t } = useI18n();
  const persistedSegments = useMemo(
    () =>
      asArray(asJsonRecord(prompt?.metadata_json)?.segments)
        .map(asJsonRecord)
        .filter((item): item is Record<string, unknown> => item != null)
        .sort(
          (left, right) =>
            (numberValue(left.segment_number) ?? Number.MAX_SAFE_INTEGER) -
            (numberValue(right.segment_number) ?? Number.MAX_SAFE_INTEGER),
        ),
    [prompt?.metadata_json],
  );
  const draftRunning = draft?.workflowStatus === "running";
  const canContinueDraft = draft != null && !draftRunning;
  const busy = acting === "generate-segment-plan" || draftRunning;
  const showingPreview = previewSegments.length > 0;
  const generatingPreview = acting === "generate-segment-plan" || draftRunning;
  const segments = useMemo(
    () =>
      mergeVideoPlanningSegments(
        persistedSegments,
        previewSegments,
        shootingScenes,
      ),
    [persistedSegments, previewSegments, shootingScenes],
  );
  const editableSegments = segments as unknown as VideoSegmentPlanV2[];
  const [editingPlan, setEditingPlan] = useState(false);
  const [savingPlan, setSavingPlan] = useState(false);
  const [planDraft, setPlanDraft] = useState<VideoSegmentPlanV2[] | null>(null);
  const planDirty =
    planDraft != null &&
    JSON.stringify(planDraft) !== JSON.stringify(editableSegments);
  const canEditPlan = Boolean(
    onSavePlan &&
      prompt &&
      prompt.status === "completed" &&
      !showingPreview &&
      segments.length > 0 &&
      !draft,
  );
  useEffect(() => {
    setEditingPlan(false);
    setPlanDraft(null);
  }, [prompt?.id]);
  const enterPlanEdit = () => {
    if (!canEditPlan || busy || loading) return;
    setPlanDraft(structuredClone(editableSegments));
    setEditingPlan(true);
  };
  const cancelPlanEdit = async () => {
    if (!editingPlan || savingPlan) return;
    if (planDirty) {
      const confirmed = await confirmDialog({
        title: t("video.chapter.cancelPlanEditTitle"),
        message: t("video.chapter.cancelPlanEditMessage"),
        confirmText: t("video.chapter.cancelPlanEdit"),
        dialogType: "warning",
      });
      if (!confirmed) return;
    }
    setPlanDraft(null);
    setEditingPlan(false);
  };
  const savePlanEdit = async () => {
    if (!editingPlan || !planDraft || !planDirty || !onSavePlan || savingPlan) return;
    setSavingPlan(true);
    try {
      if (await onSavePlan(planDraft)) {
        setPlanDraft(null);
        setEditingPlan(false);
      }
    } finally {
      setSavingPlan(false);
    }
  };
  const plannedShotIds = useMemo(
    () =>
      new Set(
        segments
          .map((segment) => textValue(segment.source_shot_id))
          .filter((id) => !asArray(asJsonRecord(prompt?.metadata_json)?.stale_source_shot_ids).includes(id))
          .filter((value): value is string => value != null),
      ),
    [segments, prompt?.metadata_json],
  );
  const staleShotIds = asArray(asJsonRecord(prompt?.metadata_json)?.stale_source_shot_ids);
  const draftTarget = draft?.resumeOptions.target_narrative_unit_index;
  const [selectedUnitIndex, setSelectedUnitIndex] = useState<number | null>(null);
  useEffect(() => {
    setSelectedUnitIndex((current) => {
      return selectVideoPlanningUnitIndex(
        shootingScenes,
        draftTarget,
        current,
        plannedShotIds,
      );
    });
  }, [draftTarget, plannedShotIds, shootingScenes]);
  const selectedScene = shootingScenes.find(
    (scene) => scene.narrative_unit_index === selectedUnitIndex,
  );
  const selectedScenePlanned =
    selectedScene != null &&
    selectedScene.shots.length > 0 &&
    selectedScene.shots.every((shot) => plannedShotIds.has(shot.shot_id));
  const selectedSceneStale = (selectedScene?.shots.some((shot) => staleShotIds.includes(shot.shot_id)) ?? false)
    || staleShotIds.some((id) => !shootingScenes.some((scene) => scene.shots.some((shot) => shot.shot_id === id)));
  const needsLoadRetry = Boolean(loadError && !prompt);
  const shotProgress =
    draft && draft.completedShotCount > 0
      ? t("video.chapter.shotProgress", {
          completed: formatNumber(draft.completedShotCount, locale),
          total: formatNumber(draft.totalShotCount, locale),
        })
      : "";
  const workflowProgress = draft?.workflowProgress
    ? `${formatNumber(Math.round(draft.workflowProgress.percent), locale)}%${draft.workflowProgress.message.trim() ? ` · ${draft.workflowProgress.message.trim()}` : ""} · `
    : "";

  return (
    <section className="flex h-full min-h-0 flex-col" aria-label={t("video.chapter.planningAria")}>
      {/* 头部：标题 + 状态 + 主操作 */}
      <div className="mb-4 flex min-w-0 flex-wrap items-center gap-3 border-b border-book-border/60 pb-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-baseline gap-2">
            <h2 className="inline-flex items-center gap-1.5 text-base font-semibold text-book-text-main">
              <ListChecks size={16} className="text-book-primary" />
              {t("video.chapter.planningTitle")}
            </h2>
            <span className="text-small text-book-text-muted">
              {showingPreview
                ? `${generatingPreview ? t("video.chapter.planningStatusGenerating") : t("video.chapter.planningStatusSaved")} · ${t("video.chapter.segments", { count: segments.length })}`
                : prompt
                ? `${editingPlan ? t("video.chapter.editingPlan") : getOperationStatusLabel(prompt.status, locale)} · ${t("video.chapter.segments", { count: segments.length })}${planDirty ? ` · ${t("writing.unsavedChanges")}` : ""}`
                : t("video.chapter.planningStatusUnplanned")}
            </span>
          </div>
          {(workflowProgress || shotProgress || loadError) && (
            <div className="mt-1 text-small text-book-text-muted">
              {workflowProgress}{shotProgress}
              {loadError && <span className="text-error">{t("video.chapter.planningLoadFailed", { error: loadError })}</span>}
            </div>
          )}
          {staleShotIds.length > 0 ? <p className="mt-1 text-small text-warning">{t("shootingScript.videoUpdateHint", { count: staleShotIds.length })}</p> : null}
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2">
          {canEditPlan ? (
            <>
              <BookButton
                variant={editingPlan ? "ghost" : "primary"}
                size="md"
                onClick={() => void (editingPlan ? cancelPlanEdit() : enterPlanEdit())}
                disabled={busy || loading || savingPlan}
              >
                {editingPlan ? <X size={14} /> : <Pencil size={14} />}
                {editingPlan ? t("video.chapter.cancelPlanEdit") : t("video.chapter.editPlan")}
              </BookButton>
              {editingPlan ? (
                <BookButton
                  variant="primary"
                  size="md"
                  onClick={() => void savePlanEdit()}
                  disabled={busy || loading || savingPlan || !planDirty}
                >
                  {savingPlan ? <LoadingSpinner size="sm" /> : <Save size={14} />}
                  {savingPlan ? t("video.chapter.savingPlan") : t("video.chapter.savePlan")}
                </BookButton>
              ) : null}
            </>
          ) : null}
          <ThemedSelect
            aria-label={t("video.chapter.chooseSceneAria")}
            value={selectedUnitIndex ?? ""}
            onChange={(selectedValue) => setSelectedUnitIndex(Number(selectedValue))}
            disabled={busy || loading || editingPlan || draft != null || shootingScenes.length === 0}
            className="pyqt-input max-w-64 !px-2 !py-2 text-xs"
          >
            {shootingScenes.length === 0 ? (
              <option value="">{t("video.chapter.noCompletedScenes")}</option>
            ) : (
              shootingScenes.map((scene) => {
                const planned =
                  scene.shots.length > 0 &&
                  scene.shots.every((shot) => plannedShotIds.has(shot.shot_id));
                return (
                  <option
                    key={scene.narrative_unit_index}
                    value={scene.narrative_unit_index}
                  >
                    {t("video.chapter.scene", { number: scene.scene_index, title: scene.title })}{planned ? ` · ${t("video.chapter.scenePlanned")}` : ""}
                  </option>
                );
              })
            )}
          </ThemedSelect>
          <BookButton
            variant={canEditPlan ? "secondary" : "primary"}
            size="md"
            onClick={() =>
              void (needsLoadRetry
                ? onRetryLoad()
                : selectedUnitIndex == null
                  ? undefined
                  : onGenerate(selectedUnitIndex, selectedScenePlanned && !selectedSceneStale))
            }
            disabled={
              busy ||
              loading ||
              editingPlan ||
              selectedUnitIndex == null ||
              (!dataReady && !needsLoadRetry)
            }
          >
            {busy || loading ? <LoadingSpinner size="sm" /> : selectedScenePlanned || needsLoadRetry ? <RotateCcw size={14} /> : <Wand2 size={14} />}
            {canContinueDraft
              ? selectedScenePlanned ? t("video.chapter.continueReplan") : t("video.chapter.continueGenerate")
              : selectedSceneStale ? t("shootingScript.updateVideoPlan") : selectedScenePlanned ? t("video.chapter.replanScene") : needsLoadRetry ? t("video.chapter.retryLoad") : t("video.chapter.planScene")}
          </BookButton>
          {prompt ? (
            <BookButton
              variant="ghost"
              size="md"
              onClick={() => void onDelete(prompt)}
              disabled={busy || editingPlan || savingPlan}
              title={t("video.chapter.deletePlanTitle")}
            >
              <Trash2 size={14} />
            </BookButton>
          ) : null}
        </div>
      </div>

      {/* 配置区：左右结构 */}
      <details className="mb-4 rounded-xl border border-book-border/50 bg-book-bg-paper/60 open:shadow-[var(--shadow-card)]">
        <summary className="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-sm font-medium text-book-text-main hover:bg-book-bg-tertiary/40 [&::-webkit-details-marker]:hidden">
          <span className="flex-1">{t("video.chapter.planningConfig")}</span>
          <span className="text-small font-normal text-book-text-muted">
            {options.aspectRatio} · {promptOutputLanguageLabel(options.promptOutputLanguage, t)} · {options.durationSecs}s
          </span>
        </summary>
        <div className="border-t border-book-border/40 px-4 py-3">
          <div className="grid gap-4 text-xs lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]">
            {/* 左侧：参数配置 */}
            <div className="flex flex-col gap-2.5">
              <div className="grid gap-2.5 grid-cols-3">
                <label className="flex min-w-0 flex-col gap-1">
                  <span className="text-book-text-muted">{t("video.chapter.aspectRatioLabel")}</span>
                  <ThemedSelect
                    value={options.aspectRatio}
                    onChange={(selectedValue) => onOptionsChange((previous) => ({
                      ...previous,
                      aspectRatio: selectedValue as VideoGenOptions["aspectRatio"],
                    }))}
                    disabled={busy}
                    className="pyqt-input !px-2 !py-1.5"
                  >
                    {VIDEO_ASPECT_RATIOS.map((ratio) => <option key={ratio} value={ratio}>{ratio}</option>)}
                  </ThemedSelect>
                </label>
                <label className="flex min-w-0 flex-col gap-1">
                  <span className="text-book-text-muted">{t("video.chapter.promptLanguageLabel")}</span>
                  <ThemedSelect
                    value={options.promptOutputLanguage}
                    onChange={(selectedValue) => onOptionsChange((previous) => ({
                      ...previous,
                      promptOutputLanguage: selectedValue as VideoGenOptions["promptOutputLanguage"],
                    }))}
                    disabled={busy}
                    className="pyqt-input !px-2 !py-1.5"
                  >
                    {(["zh_cn", "en"] as const).map((value) => (
                      <option key={value} value={value}>{promptOutputLanguageLabel(value, t)}</option>
                    ))}
                  </ThemedSelect>
                </label>
                <label className="flex min-w-0 flex-col gap-1">
                  <span className="text-book-text-muted">{t("video.chapter.defaultDuration")}</span>
                  <ThemedSelect
                    value={options.durationSecs}
                    onChange={(selectedValue) => onOptionsChange((previous) => ({
                      ...previous,
                      durationSecs: Number(selectedValue),
                    }))}
                    disabled={busy}
                    className="pyqt-input !px-2 !py-1.5"
                  >
                    {options.durationOptions.map((duration) => (
                      <option key={duration} value={duration}>{t("video.chapter.seconds", { count: duration })}</option>
                    ))}
                  </ThemedSelect>
                </label>
              </div>
              <div className="flex-1 flex flex-col">
                <div className="mb-1 text-book-text-muted">{t("video.chapter.candidateDurations")}</div>
                <VideoDurationOptionsPicker
                  options={options}
                  durationLimits={durationLimits}
                  disabled={busy}
                  onOptionsChange={onOptionsChange}
                />
              </div>
            </div>

            {/* 右侧：风格模板 */}
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-book-text-muted">{t("video.chapter.sharedStyle")}</span>
              <textarea
                value={options.style}
                onChange={(event) => onOptionsChange((previous) => ({ ...previous, style: event.target.value }))}
                disabled={busy}
                className="book-control flex-1 resize-none p-2 text-xs leading-relaxed"
                placeholder={t("video.chapter.stylePlaceholder")}
              />
            </label>
          </div>
        </div>
      </details>

      {/* 片段列表：紧凑卡片 */}
      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-visible custom-scrollbar pr-2">
        {!prompt && !showingPreview ? (
          <EmptyHint text={loadError ? t("video.chapter.savedPlanFailed", { error: loadError }) : loading ? t("video.chapter.loadingSavedPlan") : busy ? t("video.chapter.planningEmptyValidation") : shootingScenes.length === 0 ? t("video.chapter.needShootingScene") : t("video.chapter.selectSceneAndGenerate")} />
        ) : segments.length === 0 ? (
          <EmptyHint text={t("video.chapter.planEmpty")} />
        ) : editingPlan && planDraft ? (
          <VideoSegmentPlanEditor
            segments={planDraft}
            onChange={setPlanDraft}
            disabled={savingPlan}
          />
        ) : (
          <ol className="space-y-2">
            {segments.map((segment, index) => {
              const number = numberValue(segment.segment_number) ?? index + 1;
              const title = textValue(segment.title) ?? t("video.chapter.planningPromptTitle", { number });
              const duration = numberValue(segment.duration_secs);
              const summary = textValue(segment.scene_summary);
              const blocking = textValue(segment.blocking_summary);
              return (
                <li
                  key={`${showingPreview ? "preview" : "saved"}:${number}:${index}`}
                  className="group rounded-lg border border-book-border/40 bg-book-bg-paper/80 px-4 py-3 transition-colors hover:border-book-primary/30 hover:bg-book-bg-tertiary/50"
                >
                  <div className="flex items-start gap-3">
                    <div className="flex shrink-0 items-baseline gap-2">
                      <span className="text-sm font-semibold text-book-primary">#{number}</span>
                      {duration ? (
                        <span className="text-caption text-book-text-muted tabular-nums">{duration}s</span>
                      ) : null}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-medium text-book-text-main">{title}</div>
                      {summary ? (
                        <p className="mt-1.5 text-xs leading-relaxed text-book-text-sub line-clamp-2">
                          {summary}
                        </p>
                      ) : null}
                      {blocking ? (
                        <p className="mt-1 text-caption leading-relaxed text-book-text-muted line-clamp-1">
                          {t("video.chapter.blocking", { value: blocking })}
                        </p>
                      ) : null}
                    </div>
                  </div>
                </li>
              );
            })}
          </ol>
        )}
      </div>
    </section>
  );
};
