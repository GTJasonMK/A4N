import React, { useEffect, useMemo, useRef, useState } from "react";
import { Pencil, RefreshCw, Save, Trash2, Wand2, X } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";

import { extractErrorMessage } from "../../api";
import type { ChapterSummary } from "../../api/chapter";
import type { ChapterVisualPlanView } from "../../api/chapterVisualPlan";
import {
  deleteChapterShootingScript,
  editChapterShootingScript,
  editChapterShootingScriptProgress,
  readChapterShootingScriptGenerationResult,
  startChapterShootingScriptGeneration,
  type ChapterShootingScriptUnitProgressView,
  type ChapterShootingScriptV2,
  type ChapterShootingScriptView,
  type ShootingScriptGenerationMode,
} from "../../api/shootingScript";
import { generationClient } from "../../api/generationTasks";
import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { BookButton } from "../../components/ui/BookButton";
import { cancelWorkflow } from "../../api/streaming";
import { useProgressTask } from "../../hooks/useProgressTask";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { ScriptContent } from "./shooting-script/ScriptContent";
import { StructuredScriptEditor } from "./shooting-script/StructuredScriptEditor";
import { EmptyHint } from "./shared";
import { useI18n } from "../../i18n";
import { loadShootingScriptData, loadShootingScriptOrProgress } from "./loadShootingScriptData";
import { useNovelDetailDirty } from "./novelDetailDirty";

interface ShootingScriptTabProps {
  projectId: number;
  canEdit: boolean;
  chapter: ChapterSummary | null;
}

export const ShootingScriptTab: React.FC<ShootingScriptTabProps> = ({
  projectId,
  canEdit,
  chapter,
}) => {
  const { addToast } = useToast();
  const { t } = useI18n();
  const { runProgressTask } = useProgressTask();
  const chapterId = chapter?.id ?? null;
  const sourceKey = `${projectId}:${chapterId}`;
  const sourceRef = useRef(0);
  const followingTask = useRef<string | null>(null);
  const dataVersion = useRef(0);
  const [view, setView] = useState<ChapterShootingScriptView | null>(null);
  const [unitProgress, setUnitProgress] =
    useState<ChapterShootingScriptUnitProgressView | null>(null);
  const [scriptLoading, setScriptLoading] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [editing, setEditing] = useState(false);
  const [confirmedVisualPlan, setConfirmedVisualPlan] = useState<ChapterVisualPlanView | null>(null);
  const confirmedVisualPlanId = confirmedVisualPlan?.record.id ?? null;
  const [rebindShotIds, setRebindShotIds] = useState<string[]>([]);
  const [focusShotId, setFocusShotId] = useState<string | null>(null);
  const affectedShotIds = view ? [...view.plan_sync.stale_shot_ids, ...view.plan_sync.missing_shot_ids] : [];
  const [draft, setDraft] = useState<ChapterShootingScriptV2 | null>(null);
  const [invalidatedProjectIds, setInvalidatedProjectIds] = useState<number[]>(
    [],
  );
  const isDirty = useMemo(
    () =>
      draft != null &&
      (view != null || unitProgress != null) &&
      (rebindShotIds.length > 0 || JSON.stringify(view?.script ?? unitProgress?.script) !==
        JSON.stringify(draft)),
    [draft, unitProgress, view, rebindShotIds],
  );
  useNovelDetailDirty(`shooting-script:${projectId}:${chapterId ?? "none"}`, isDirty);

  const applyScriptData = (
    nextView: ChapterShootingScriptView | null,
    nextProgress: ChapterShootingScriptUnitProgressView | null,
  ) => {
    dataVersion.current += 1;
    setView(nextView);
    setUnitProgress(nextView ? null : nextProgress);
    const script = nextView?.script ?? nextProgress?.script;
    setDraft(script ? structuredClone(script) : null);
    setRebindShotIds([]);
    setInvalidatedProjectIds([]);
  };

  const followGeneration = async (
    handle: GenerationTaskHandle,
    mode: ShootingScriptGenerationMode = "full",
    keepEditing = false,
  ) => {
    if (chapterId == null) return;
    const key = `${handle.task_id}:${handle.attempt}`;
    const scope = sourceRef.current;
    if (followingTask.current === key) return;
    followingTask.current = key;
    const isCurrent = () => sourceRef.current === scope && followingTask.current === key;
    setGenerating(true);
    try {
      const generated = await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t(mode === "next_scene" ? "shootingScript.generateNextScene" : "shootingScript.generateStructured"),
          startMessage: t("shootingScript.preparing"),
          successMessage: t("shootingScript.completed"),
          errorMessage: t("shootingScript.failed"),
          cancelLabel: t("workflow.stop"),
          runningHint: t("shootingScript.stopHint"),
        },
        (task) => readChapterShootingScriptGenerationResult(projectId, chapterId, task.resultRef),
      );
      if (!isCurrent()) return;
      if ("record" in generated) {
        applyScriptData(generated, null);
        addToast(t("shootingScript.allScenesCompleted"), "success");
      } else {
        applyScriptData(null, generated);
        addToast(t("shootingScript.sceneProgress", {
          current: generated.completed_unit_count, total: generated.total_unit_count,
        }), "success");
      }
      setEditing(keepEditing);
    } catch (error) {
      if (!isCurrent()) return;
      // A stopped or failed task can still have durable completed scenes.
      try {
        const saved = await loadShootingScriptOrProgress(projectId, chapterId);
        if (isCurrent()) applyScriptData(saved.view, saved.unitProgress);
      } catch (readError) {
        if (isCurrent()) addToast(extractErrorMessage(readError, t("shootingScript.loadFailed")), "error");
      }
      if (!isCurrent()) return;
      const message = extractErrorMessage(error, t("shootingScript.failed"));
      addToast(isCancelledOperation(message) ? t("shootingScript.stopped") : message,
        isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrent()) {
        followingTask.current = null;
        setGenerating(false);
      }
    }
  };
  const followGenerationRef = useRef(followGeneration);
  followGenerationRef.current = followGeneration;

  useEffect(() => {
    sourceRef.current += 1;
    followingTask.current = null;
    const initialVersion = ++dataVersion.current;
    setGenerating(false);
    setSaving(false);
    setDeleting(false);
    setRebindShotIds([]);
    setFocusShotId(null);
    setView(null);
    setUnitProgress(null);
    setDraft(null);
    setEditing(false);
    setInvalidatedProjectIds([]);
    setConfirmedVisualPlan(null);
    if (chapterId == null) {
      setScriptLoading(false);
      return;
    }
    let active = true;
    setScriptLoading(true);
    void loadShootingScriptData(projectId, chapterId)
      .then(({ view: value, unitProgress: progress, visualPlan, scriptError, visualPlanError }) => {
        if (!active) return;
        if (scriptError !== undefined) {
          addToast(extractErrorMessage(scriptError, t("shootingScript.loadFailed")), "error");
        } else if (dataVersion.current === initialVersion) {
          applyScriptData(value, progress);
        }
        if (visualPlan) {
          setConfirmedVisualPlan(visualPlan.confirmed);
        } else if (visualPlanError !== undefined) {
          addToast(extractErrorMessage(visualPlanError, t("shootingScript.loadFailed")), "error");
        }
      })
      .catch((error) => {
        if (active) {
          addToast(
            extractErrorMessage(error, t("shootingScript.loadFailed")),
            "error",
          );
        }
      })
      .finally(() => {
        if (active) setScriptLoading(false);
      });
    void generationClient.listActive(projectId).then((tasks) => {
      if (!active || followingTask.current != null) return;
      const task = tasks.find((task) => task.started.target?.kind === "chapter_shooting_script"
        && task.started.target?.id === `chapter:${chapterId}`);
      if (task) void followGenerationRef.current(task.handle);
    }).catch((error) => {
      if (active) addToast(extractErrorMessage(error, t("shootingScript.loadFailed")), "error");
    });
    return () => {
      active = false;
      sourceRef.current += 1;
    };
  }, [addToast, chapterId, projectId]);

  const savePartialDraft = async () => {
    if (view || !unitProgress || !draft || chapterId == null || !isDirty) return;
    const scope = sourceRef.current;
    const result = await editChapterShootingScriptProgress(
      projectId,
      chapterId,
      unitProgress,
      draft,
    );
    if (sourceRef.current !== scope) return;
    setUnitProgress(result.progress);
    setDraft(structuredClone(result.progress.script));
    setInvalidatedProjectIds(result.invalidated_project_ids);
  };

  const generate = async (mode: ShootingScriptGenerationMode, shotIds: string[] = []) => {
    if (
      chapterId == null ||
      confirmedVisualPlanId == null ||
      generating ||
      followingTask.current != null ||
      saving ||
      scriptLoading
    )
      return;
    const scope = sourceRef.current;
    const keepEditing = mode === "next_scene" && editing;
    if (mode === "full" && view && (shotIds.length === 0 || isDirty)) {
      const confirmed = await confirmDialog({
        title: t("shootingScript.regenerate"),
        message: isDirty
          ? t("shootingScript.unsavedWillBeLost")
          : t("shootingScript.downstreamWillRegenerate"),
        confirmText: t("shootingScript.regenerate"),
        dialogType: "warning",
      });
      if (!confirmed) return;
    }
    if (followingTask.current != null || sourceRef.current !== scope) return;
    const pendingKey = `starting:${sourceKey}`;
    followingTask.current = pendingKey;
    setGenerating(true);
    try {
      await savePartialDraft();
      if (sourceRef.current !== scope) return;
      const handle = await startChapterShootingScriptGeneration(
        projectId, chapterId, confirmedVisualPlanId, mode, shotIds,
      );
      if (sourceRef.current !== scope) return;
      await followGeneration(handle, mode, keepEditing);
    } catch (error) {
      if (sourceRef.current !== scope) return;
      const message = extractErrorMessage(error, t("shootingScript.failed"));
      addToast(
        isCancelledOperation(message) ? t("shootingScript.stopped") : message,
        isCancelledOperation(message) ? "info" : "error",
      );
    } finally {
      if (sourceRef.current === scope && followingTask.current === pendingKey) {
        followingTask.current = null;
        setGenerating(false);
      }
    }
  };

  const save = async () => {
    const editable = view ?? unitProgress;
    if (
      !editable ||
      !draft ||
      chapterId == null ||
      saving ||
      generating ||
      scriptLoading
    )
      return;
    const scope = sourceRef.current;
    setSaving(true);
    try {
      const result = "record" in editable
        ? await editChapterShootingScript(projectId, chapterId, editable, draft, rebindShotIds)
        : await editChapterShootingScriptProgress(projectId, chapterId, editable, draft);
      if (sourceRef.current !== scope) return;
      if ("shooting_script" in result) {
        setView(result.shooting_script);
        setUnitProgress(null);
        setDraft(structuredClone(result.shooting_script.script));
      } else {
        setUnitProgress(result.progress);
        setDraft(structuredClone(result.progress.script));
      }
      setInvalidatedProjectIds(result.invalidated_project_ids);
      setRebindShotIds([]);
      addToast(
        result.invalidated_project_ids.length > 0
          ? t("shootingScript.downstreamInvalidated", {
              count: result.invalidated_project_ids.length,
            })
          : t("shootingScript.saveCompleted"),
        "success",
      );
    } catch (error) {
      if (sourceRef.current !== scope) return;
      addToast(
        extractErrorMessage(error, t("shootingScript.saveFailed")),
        "error",
      );
    } finally {
      if (sourceRef.current === scope) setSaving(false);
    }
  };

  const enterEdit = (shotId: string | null = null) => {
    const source = view?.script ?? unitProgress?.script;
    if (!source || actionsDisabled) return;
    setDraft(structuredClone(source));
    setRebindShotIds([]);
    setFocusShotId(shotId);
    setEditing(true);
  };

  const cancelEdit = async () => {
    if (!editing || actionsDisabled) return;
    const scope = sourceRef.current;
    if (isDirty) {
      const confirmed = await confirmDialog({
        title: t("shootingScript.cancelEditTitle"),
        message: t("shootingScript.cancelEditMessage"),
        confirmText: t("shootingScript.cancelEdit"),
        dialogType: "warning",
      });
      if (!confirmed) return;
    }
    if (sourceRef.current !== scope) return;
    const source = view?.script ?? unitProgress?.script;
    setDraft(source ? structuredClone(source) : null);
    setEditing(false);
    setRebindShotIds([]);
  };

  const removeProgress = async () => {
    if (chapterId == null || !unitProgress || deleting || generating || saving || scriptLoading)
      return;
    const scope = sourceRef.current;
    const confirmed = await confirmDialog({
      title: t("shootingScript.discardProgressTitle"),
      message: t("shootingScript.discardProgressMessage"),
      confirmText: t("shootingScript.discardProgress"),
      dialogType: "danger",
    });
    if (!confirmed || sourceRef.current !== scope) return;
    setDeleting(true);
    try {
      await cancelWorkflow({
        task_id: null,
        workflow_run_id: unitProgress.workflow_run_id,
      });
      if (sourceRef.current !== scope) return;
      const refreshed = await loadShootingScriptOrProgress(projectId, chapterId);
      if (sourceRef.current !== scope) return;
      if (refreshed.scriptError || refreshed.view || refreshed.unitProgress) {
        throw new Error(t("shootingScript.discardProgressFailed"));
      }
      setUnitProgress(null);
      setDraft(null);
      setEditing(false);
      setInvalidatedProjectIds([]);
      addToast(t("shootingScript.discardProgressCompleted"), "success");
    } catch (error) {
      if (sourceRef.current !== scope) return;
      addToast(
        extractErrorMessage(error, t("shootingScript.discardProgressFailed")),
        "error",
      );
    } finally {
      if (sourceRef.current === scope) setDeleting(false);
    }
  };

  const remove = async () => {
    if (
      !view ||
      chapterId == null ||
      deleting ||
      generating ||
      saving ||
      scriptLoading
    )
      return;
    const scope = sourceRef.current;
    const confirmed = await confirmDialog({
      title: t("shootingScript.deleteTitle"),
      message: t("shootingScript.deleteMessage"),
      confirmText: t("shootingScript.delete"),
      dialogType: "danger",
    });
    if (!confirmed || sourceRef.current !== scope) return;
    setDeleting(true);
    try {
      await deleteChapterShootingScript(projectId, chapterId, view);
      if (sourceRef.current !== scope) return;
      const refreshed = await loadShootingScriptOrProgress(projectId, chapterId);
      if (sourceRef.current !== scope) return;
      if (refreshed.scriptError || refreshed.view || refreshed.unitProgress) {
        throw new Error(t("shootingScript.deleteFailed"));
      }
      setView(null);
      setDraft(null);
      setUnitProgress(null);
      setInvalidatedProjectIds([]);
      setEditing(false);
      addToast(t("shootingScript.deleteCompleted"), "success");
    } catch (error) {
      if (sourceRef.current !== scope) return;
      addToast(
        extractErrorMessage(error, t("shootingScript.deleteFailed")),
        "error",
      );
    } finally {
      if (sourceRef.current === scope) setDeleting(false);
    }
  };

  if (!chapter) {
    return <EmptyHint text={t("shootingScript.chooseChapter")} />;
  }

  const actionsDisabled = generating || saving || deleting || scriptLoading;

  return (
    <section className="min-w-0 space-y-3">
      <div className={`flex min-w-0 flex-wrap items-center justify-between gap-2 px-1 ${editing ? "sticky top-0 z-10 bg-[rgb(var(--color-bg-primary))] py-2" : ""}`}>
        <div className="text-small text-book-text-muted">
          {scriptLoading
            ? t("shootingScript.loading")
            : generating
              ? view
                ? t("shootingScript.regenerating")
                : t("shootingScript.generating")
              : saving
                ? t("shootingScript.saving")
                : deleting
                  ? t("shootingScript.deleting")
                  : editing
                    ? t("shootingScript.editing")
                    : view
                      ? t("shootingScript.currentRevision", {
                          revision: view.record.revision,
                          dirty: isDirty
                            ? ` · ${t("writing.unsavedChanges")}`
                            : "",
                        })
                      : unitProgress
                        ? `${t("shootingScript.completedScenes", { current: unitProgress.completed_unit_count, total: unitProgress.total_unit_count })}${isDirty ? ` · ${t("writing.unsavedChanges")}` : ""}`
                        : confirmedVisualPlanId == null
                          ? t("shootingScript.choosePlan")
                          : t("shootingScript.notGenerated")}
        </div>
        {canEdit ? (
          <div className="ml-auto flex min-w-0 flex-wrap items-center gap-2">
            {view && draft ? (
              <>
                <BookButton
                  size="sm"
                  variant={editing ? "ghost" : "primary"}
                  onClick={() => void (editing ? cancelEdit() : enterEdit())}
                  disabled={actionsDisabled}
                >
                  {editing ? <X size={13} /> : <Pencil size={13} />}
                  {editing
                    ? t("shootingScript.cancelEdit")
                    : t("shootingScript.edit")}
                </BookButton>
                <BookButton
                  size="sm"
                  variant="secondary"
                  onClick={() => void generate("full")}
                  disabled={actionsDisabled}
                >
                  {generating ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <RefreshCw size={13} />
                  )}
                  {generating
                    ? t("shootingScript.regenerating")
                    : t("shootingScript.regenerate")}
                </BookButton>
                <BookButton
                  size="sm"
                  variant="ghost"
                  className="text-error hover:bg-error-bg hover:text-error"
                  onClick={() => void remove()}
                  disabled={actionsDisabled}
                >
                  {deleting ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Trash2 size={13} />
                  )}
                  {deleting
                    ? t("shootingScript.deleting")
                    : t("shootingScript.delete")}
                </BookButton>
                {editing ? (
                  <BookButton
                    size="sm"
                    onClick={() => void save()}
                    disabled={actionsDisabled || !isDirty}
                  >
                    {saving ? <LoadingSpinner size="sm" /> : <Save size={13} />}
                    {saving
                      ? t("shootingScript.saving")
                      : t("shootingScript.saveAdaptation")}
                  </BookButton>
                ) : null}
              </>
            ) : (
              <>
                {unitProgress ? (
                  <>
                    <BookButton
                      size="sm"
                      variant="secondary"
                      onClick={() => void generate("full")}
                      disabled={actionsDisabled}
                    >
                      <RefreshCw size={13} />
                      {t("shootingScript.generateRemaining")}
                    </BookButton>
                    <BookButton
                      size="sm"
                      variant="danger"
                      onClick={() => void removeProgress()}
                      disabled={actionsDisabled}
                    >
                      {deleting ? (
                        <LoadingSpinner size="sm" />
                      ) : (
                        <Trash2 size={13} />
                      )}
                      {deleting
                        ? t("shootingScript.deleting")
                        : t("shootingScript.discardProgress")}
                    </BookButton>
                  </>
                ) : null}
                {unitProgress ? (
                  <BookButton
                    size="sm"
                    variant={editing ? "ghost" : "primary"}
                    onClick={() => void (editing ? cancelEdit() : enterEdit())}
                    disabled={actionsDisabled}
                  >
                    {editing ? <X size={13} /> : <Pencil size={13} />}
                    {editing
                      ? t("shootingScript.cancelEdit")
                      : t("shootingScript.edit")}
                  </BookButton>
                ) : null}
                {unitProgress && editing ? (
                  <BookButton
                    size="sm"
                    onClick={() => void save()}
                    disabled={actionsDisabled || !isDirty}
                  >
                    {saving ? <LoadingSpinner size="sm" /> : <Save size={13} />}
                    {saving
                      ? t("shootingScript.saving")
                      : t("shootingScript.saveAdaptation")}
                  </BookButton>
                ) : null}
                <BookButton
                  size="sm"
                  onClick={() => void generate("next_scene")}
                  disabled={actionsDisabled || confirmedVisualPlanId == null}
                >
                  {generating ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Wand2 size={13} />
                  )}
                  {generating
                    ? t("shootingScript.generatingShort")
                    : unitProgress
                      ? t("shootingScript.nextScene")
                      : t("shootingScript.firstScene")}
                </BookButton>
              </>
            )}
          </div>
        ) : null}
      </div>
      {scriptLoading ? (
        <div className="a4n-flat-surface py-12 text-center">
          <LoadingSpinner
            size="md"
            text={t("shootingScript.loading")}
            inline={false}
          />
        </div>
      ) : !view && !unitProgress ? (
        <div className="a4n-flat-surface p-3 sm:p-4">
          <EmptyHint
            text={
              confirmedVisualPlanId == null
                ? t("shootingScript.choosePlan")
                : t("shootingScript.editDescription")
            }
          />
        </div>
      ) : view ? (
        <>
          {affectedShotIds.length > 0 ? (
            <section className="a4n-flat-surface space-y-3 p-3 sm:p-4" aria-label={t("shootingScript.needsUpdate")}>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <p className="text-sm font-semibold text-warning">{t("shootingScript.updateCount", { count: affectedShotIds.length })}</p>
                {canEdit ? <BookButton size="sm" onClick={() => void generate("full", affectedShotIds)} disabled={actionsDisabled || editing}>
                  <Wand2 size={13} />{t("shootingScript.updateAffected")}
                </BookButton> : null}
              </div>
              <p className="text-small text-book-text-muted">{editing ? t("shootingScript.manualUpdateHint") : t("shootingScript.localUpdateHint")}</p>
              <ul className="divide-y divide-book-border/45">
                {affectedShotIds.map((id) => {
                  const planned = confirmedVisualPlan?.plan.segments.find((shot) => shot.id === id);
                  const missing = view.plan_sync.missing_shot_ids.includes(id);
                  return <li key={id} className="flex min-w-0 flex-wrap items-center justify-between gap-2 py-2">
                    <div className="min-w-0 flex-1 basis-48">
                      <p className="break-words text-xs font-medium">{planned?.title || id} · {missing ? t("shootingScript.missingShot") : t("shootingScript.staleShot")}</p>
                      <p className="mt-1 break-words text-small text-book-text-muted">{planned?.prompt}</p>
                    </div>
                    {canEdit ? <div className="flex flex-wrap items-center gap-2">
                      {editing && !missing ? <label className="flex cursor-pointer items-center gap-2 text-xs">
                        <input type="checkbox" checked={rebindShotIds.includes(id)} disabled={actionsDisabled}
                          onChange={(event) => setRebindShotIds((previous) => event.target.checked ? [...previous, id] : previous.filter((shotId) => shotId !== id))} />
                        {t("shootingScript.manualUpdated", { id: planned?.title || id })}
                      </label> : !editing ? <>
                        {!missing ? <BookButton size="sm" variant="secondary" disabled={actionsDisabled} onClick={() => enterEdit(id)} aria-label={t("shootingScript.editShot", { id })}><Pencil size={13} />{t("shootingScript.manualUpdate")}</BookButton> : null}
                        <BookButton size="sm" variant="secondary" disabled={actionsDisabled} onClick={() => void generate("full", [id])} aria-label={t("shootingScript.generateShot", { id })}><Wand2 size={13} />{t("shootingScript.modelUpdate")}</BookButton>
                      </> : null}
                    </div> : null}
                  </li>;
                })}
              </ul>
            </section>
          ) : null}
          {canEdit && editing && draft ? (
            <StructuredScriptEditor
              script={draft}
              onChange={setDraft}
              disabled={actionsDisabled}
              focusShotId={focusShotId}
            />
          ) : (
            <ScriptContent script={view.script} />
          )}
          {invalidatedProjectIds.length > 0 ? (
            <div className="px-1 text-small leading-relaxed text-book-text-muted">
              <p>
                {t("shootingScript.invalidatedHint", {
                  count: invalidatedProjectIds.length,
                })}
              </p>
              <details className="mt-1">
                <summary className="w-fit cursor-pointer hover:text-book-text-main">
                  {t("writing.diagnostics")}
                </summary>
                <p className="mt-1 break-all font-mono text-caption">
                  {t("shootingScript.downstreamIds", {
                    ids: invalidatedProjectIds.join(", "),
                  })}
                </p>
              </details>
            </div>
          ) : null}
        </>
      ) : unitProgress ? (
        <>
          <p className="px-1 text-small leading-relaxed text-book-text-muted">
            {canEdit
              ? editing
                ? t("shootingScript.partialEditHint")
                : t("shootingScript.partialViewHint")
              : t("shootingScript.partialReadOnlyHint")}
          </p>
          {canEdit && editing && draft ? (
            <StructuredScriptEditor
              script={draft}
              onChange={setDraft}
              disabled={actionsDisabled}
            />
          ) : (
            <ScriptContent script={unitProgress.script} />
          )}
        </>
      ) : null}
    </section>
  );
};
