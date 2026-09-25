import React, { useEffect, useMemo, useState } from "react";
import { Pencil, RefreshCw, Save, Trash2, Wand2, X } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";

import { extractErrorMessage } from "../../api";
import type { ChapterSummary } from "../../api/chapter";
import {
  deleteChapterShootingScript,
  editChapterShootingScript,
  editChapterShootingScriptProgress,
  generateChapterShootingScript,
  generateNextChapterShootingScriptUnit,
  type ChapterShootingScriptUnitProgressView,
  type ChapterShootingScriptV1,
  type ChapterShootingScriptView,
} from "../../api/shootingScript";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { BookButton } from "../../components/ui/BookButton";
import { cancelWorkflow, cancelWorkflowTask } from "../../api/streaming";
import {
  createWorkflowTaskId,
  useProgressTask,
} from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { ScriptContent } from "./shooting-script/ScriptContent";
import { StructuredScriptEditor } from "./shooting-script/StructuredScriptEditor";
import { EmptyHint } from "./shared";
import { useI18n } from "../../i18n";
import { loadShootingScriptData } from "./loadShootingScriptData";

const mergePartialScriptEdits = (
  generated: ChapterShootingScriptV1,
  partial: ChapterShootingScriptV1,
  completedUnitCount: number,
): ChapterShootingScriptV1 => {
  const partialScenes = new Map(
    partial.scenes.map((scene) => [scene.scene_identity, scene]),
  );
  const presentNarrativeUnits = new Set(
    partial.scenes.map((scene) => scene.narrative_unit_index),
  );
  const deletedNarrativeUnits = new Set(
    Array.from({ length: completedUnitCount }, (_, index) => index + 1).filter(
      (index) => !presentNarrativeUnits.has(index),
    ),
  );
  const scenes = generated.scenes
    .filter(
      (scene) =>
        !deletedNarrativeUnits.has(scene.narrative_unit_index) &&
        (scene.narrative_unit_index > completedUnitCount ||
          partialScenes.has(scene.scene_identity)),
    )
    .map((scene, index) => {
      const edited = partialScenes.get(scene.scene_identity);
      const editedShots = edited
        ? new Map(edited.shots.map((item) => [item.shot_id, item]))
        : null;
      const merged = edited
        ? {
            ...scene,
            ...structuredClone(edited),
            shots: scene.shots.map(
              (shot) => editedShots?.get(shot.shot_id) ?? shot,
            ),
          }
        : scene;
      return {
        ...merged,
        scene_index: index + 1,
        sequence_index: index + 1,
        timeline_position: index + 1,
        transition_contracts: merged.transition_contracts.filter(
          (transition) =>
            !deletedNarrativeUnits.has(transition.to_narrative_unit_index),
        ),
      };
    });
  return {
    ...generated,
    directing_statement: structuredClone(partial.directing_statement),
    user_additions: structuredClone(partial.user_additions),
    scenes,
  };
};

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
  const [view, setView] = useState<ChapterShootingScriptView | null>(null);
  const [unitProgress, setUnitProgress] =
    useState<ChapterShootingScriptUnitProgressView | null>(null);
  const [scriptLoading, setScriptLoading] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [editing, setEditing] = useState(false);
  const [confirmedVisualPlanId, setConfirmedVisualPlanId] = useState<
    number | null
  >(null);
  const [draft, setDraft] = useState<ChapterShootingScriptV1 | null>(null);
  const [invalidatedProjectIds, setInvalidatedProjectIds] = useState<number[]>(
    [],
  );
  const isDirty = useMemo(
    () =>
      draft != null &&
      (view != null || unitProgress != null) &&
      JSON.stringify(view?.script ?? unitProgress?.script) !==
        JSON.stringify(draft),
    [draft, unitProgress, view],
  );

  useEffect(() => {
    if (chapterId == null) {
      setView(null);
      setUnitProgress(null);
      setDraft(null);
      setEditing(false);
      setInvalidatedProjectIds([]);
      setConfirmedVisualPlanId(null);
      setScriptLoading(false);
      return;
    }
    let active = true;
    setView(null);
    setEditing(false);
    setScriptLoading(true);
    void loadShootingScriptData(projectId, chapterId)
      .then(({ view: value, unitProgress: progress, visualPlan }) => {
        if (!active) return;
        setView(value);
        setUnitProgress(value ? null : progress);
        setDraft(
          value
            ? structuredClone(value.script)
            : progress
              ? structuredClone(progress.script)
              : null,
        );
        setInvalidatedProjectIds([]);
        setConfirmedVisualPlanId(visualPlan.confirmed?.record.id ?? null);
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
    return () => {
      active = false;
    };
  }, [addToast, chapterId, projectId]);

  const generate = async () => {
    if (
      chapterId == null ||
      confirmedVisualPlanId == null ||
      generating ||
      saving ||
      scriptLoading
    )
      return;
    const partialProgress = editing && !view ? unitProgress : null;
    const partialDraft =
      partialProgress && draft ? structuredClone(draft) : null;
    if (view) {
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
    setGenerating(true);
    try {
      const taskId = createWorkflowTaskId("chapter-shooting-script", projectId);
      const generated = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          streaming: true,
          title: t("shootingScript.generateStructured"),
          startMessage: t("shootingScript.preparing"),
          successMessage: t("shootingScript.completed"),
          errorMessage: t("shootingScript.failed"),
          total: 100,
          cancelLabel: t("workflow.stop"),
          runningHint: t("shootingScript.stopHint"),
        },
        () =>
          generateChapterShootingScript(
            projectId,
            chapterId,
            taskId,
            confirmedVisualPlanId,
          ),
      );
      const script =
        partialDraft && partialProgress
          ? mergePartialScriptEdits(
              generated.script,
              partialDraft,
              partialProgress.completed_unit_count,
            )
          : generated.script;
      const completedView =
        script === generated.script ? generated : { ...generated, script };
      setView(completedView);
      setUnitProgress(null);
      setDraft(structuredClone(script));
      setEditing(false);
      setInvalidatedProjectIds([]);
      addToast(t("shootingScript.completed"), "success");
    } catch (error) {
      const message = extractErrorMessage(error, t("shootingScript.failed"));
      addToast(
        isCancelledOperation(message) ? t("shootingScript.stopped") : message,
        isCancelledOperation(message) ? "info" : "error",
      );
    } finally {
      setGenerating(false);
    }
  };

  const generateNextUnit = async () => {
    if (
      chapterId == null ||
      confirmedVisualPlanId == null ||
      generating ||
      saving ||
      scriptLoading
    )
      return;
    const wasEditing = editing;
    const partialProgress = wasEditing ? unitProgress : null;
    const partialDraft =
      partialProgress && draft ? structuredClone(draft) : null;
    setGenerating(true);
    try {
      const taskId = createWorkflowTaskId(
        "chapter-shooting-script-unit",
        projectId,
      );
      const generated = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          streaming: true,
          title: t("shootingScript.generateNextScene"),
          startMessage: t("shootingScript.prepareScene"),
          successMessage: t("shootingScript.nextSceneCompleted"),
          errorMessage: t("shootingScript.failed"),
          total: 100,
          cancelLabel: t("workflow.stop"),
          runningHint: t("shootingScript.nextSceneHint"),
        },
        () =>
          generateNextChapterShootingScriptUnit(
            projectId,
            chapterId,
            taskId,
            confirmedVisualPlanId,
          ),
      );
      const script =
        partialDraft && partialProgress
          ? mergePartialScriptEdits(
              generated.script,
              partialDraft,
              partialProgress.completed_unit_count,
            )
          : generated.script;
      if (generated.completed_record) {
        const completedView = {
          record: generated.completed_record,
          script,
        } satisfies ChapterShootingScriptView;
        setView(completedView);
        setUnitProgress(null);
        setDraft(structuredClone(script));
      } else {
        setUnitProgress({ ...generated, script });
        setDraft(structuredClone(script));
      }
      setEditing(wasEditing);
      setInvalidatedProjectIds([]);
      addToast(
        generated.completed_record
          ? t("shootingScript.allScenesCompleted")
          : t("shootingScript.sceneProgress", {
              current: generated.completed_unit_count,
              total: generated.total_unit_count,
            }),
        "success",
      );
    } catch (error) {
      const message = extractErrorMessage(error, t("shootingScript.failed"));
      addToast(
        isCancelledOperation(message) ? t("shootingScript.stopped") : message,
        isCancelledOperation(message) ? "info" : "error",
      );
    } finally {
      setGenerating(false);
    }
  };

  const save = async () => {
    if (
      (!view && !unitProgress) ||
      !draft ||
      chapterId == null ||
      saving ||
      generating ||
      scriptLoading
    )
      return;
    setSaving(true);
    try {
      const result = view
        ? await editChapterShootingScript(projectId, chapterId, view, draft)
        : unitProgress
          ? await editChapterShootingScriptProgress(
              projectId,
              chapterId,
              unitProgress,
              draft,
            )
          : null;
      if (!result) return;
      if ("shooting_script" in result) {
        setView(result.shooting_script);
        setUnitProgress(null);
        setDraft(structuredClone(result.shooting_script.script));
      } else {
        setUnitProgress(result.progress);
        setDraft(structuredClone(result.progress.script));
      }
      setInvalidatedProjectIds(result.invalidated_project_ids);
      addToast(
        result.invalidated_project_ids.length > 0
          ? t("shootingScript.downstreamInvalidated", {
              count: result.invalidated_project_ids.length,
            })
          : t("shootingScript.saveCompleted"),
        "success",
      );
    } catch (error) {
      addToast(
        extractErrorMessage(error, t("shootingScript.saveFailed")),
        "error",
      );
    } finally {
      setSaving(false);
    }
  };

  const enterEdit = () => {
    const source = view?.script ?? unitProgress?.script;
    if (!source || actionsDisabled) return;
    setDraft(structuredClone(source));
    setEditing(true);
  };

  const cancelEdit = async () => {
    if (!editing || actionsDisabled) return;
    if (isDirty) {
      const confirmed = await confirmDialog({
        title: t("shootingScript.cancelEditTitle"),
        message: t("shootingScript.cancelEditMessage"),
        confirmText: t("shootingScript.cancelEdit"),
        dialogType: "warning",
      });
      if (!confirmed) return;
    }
    const source = view?.script ?? unitProgress?.script;
    setDraft(source ? structuredClone(source) : null);
    setEditing(false);
  };

  const removeProgress = async () => {
    if (!unitProgress || deleting || generating || saving || scriptLoading)
      return;
    const confirmed = await confirmDialog({
      title: t("shootingScript.discardProgressTitle"),
      message: t("shootingScript.discardProgressMessage"),
      confirmText: t("shootingScript.discardProgress"),
      dialogType: "danger",
    });
    if (!confirmed) return;
    setDeleting(true);
    try {
      await cancelWorkflow({
        task_id: null,
        workflow_run_id: unitProgress.workflow_run_id,
      });
      setUnitProgress(null);
      setDraft(null);
      setEditing(false);
      setInvalidatedProjectIds([]);
      addToast(t("shootingScript.discardProgressCompleted"), "success");
    } catch (error) {
      addToast(
        extractErrorMessage(error, t("shootingScript.discardProgressFailed")),
        "error",
      );
    } finally {
      setDeleting(false);
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
    const confirmed = await confirmDialog({
      title: t("shootingScript.deleteTitle"),
      message: t("shootingScript.deleteMessage"),
      confirmText: t("shootingScript.delete"),
      dialogType: "danger",
    });
    if (!confirmed) return;
    setDeleting(true);
    try {
      await deleteChapterShootingScript(projectId, chapterId, view);
      setView(null);
      setDraft(null);
      setUnitProgress(null);
      setInvalidatedProjectIds([]);
      setEditing(false);
      addToast(t("shootingScript.deleteCompleted"), "success");
    } catch (error) {
      addToast(
        extractErrorMessage(error, t("shootingScript.deleteFailed")),
        "error",
      );
    } finally {
      setDeleting(false);
    }
  };

  if (!chapter) {
    return <EmptyHint text={t("shootingScript.chooseChapter")} />;
  }

  const actionsDisabled = generating || saving || deleting || scriptLoading;

  return (
    <section className="min-w-0 space-y-3">
      <div className={`flex min-w-0 flex-wrap items-center justify-between gap-2 px-1 ${editing ? "sticky top-0 z-10 bg-book-bg py-2" : ""}`}>
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
                  onClick={() => void generate()}
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
                      onClick={() => void generate()}
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
                  onClick={() => void generateNextUnit()}
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
          {canEdit && editing && draft ? (
            <StructuredScriptEditor
              script={draft}
              onChange={setDraft}
              disabled={actionsDisabled}
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
