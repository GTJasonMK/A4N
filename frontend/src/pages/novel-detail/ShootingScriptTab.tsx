import React, { useEffect, useMemo, useState } from "react";
import { RefreshCw, Save, Trash2, Wand2 } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";

import { extractErrorMessage } from "../../api";
import type { ChapterSummary } from "../../api/chapter";
import {
  deleteChapterShootingScript,
  editChapterShootingScript,
  generateChapterShootingScript,
  generateNextChapterShootingScriptUnit,
  type ChapterShootingScriptUnitProgressView,
  type ChapterShootingScriptV1,
  type ChapterShootingScriptView,
} from "../../api/shootingScript";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import { cancelWorkflowTask } from "../../api/streaming";
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
  const [confirmedVisualPlanId, setConfirmedVisualPlanId] = useState<number | null>(null);
  const [draft, setDraft] = useState<ChapterShootingScriptV1 | null>(null);
  const [invalidatedProjectIds, setInvalidatedProjectIds] = useState<number[]>(
    [],
  );
  const isDirty = useMemo(
    () =>
      view != null &&
      draft != null &&
      JSON.stringify(view.script) !== JSON.stringify(draft),
    [draft, view],
  );

  useEffect(() => {
    if (chapterId == null) {
      setView(null);
      setUnitProgress(null);
      setDraft(null);
      setInvalidatedProjectIds([]);
      setConfirmedVisualPlanId(null);
      setScriptLoading(false);
      return;
    }
    let active = true;
    setView(null);
    setScriptLoading(true);
    void loadShootingScriptData(projectId, chapterId)
      .then(({ view: value, unitProgress: progress, visualPlan }) => {
        if (!active) return;
        setView(value);
        setUnitProgress(value ? null : progress);
        setDraft(value ? structuredClone(value.script) : null);
        setInvalidatedProjectIds([]);
        setConfirmedVisualPlanId(visualPlan.confirmed?.record.id ?? null);
      })
      .catch((error) => {
        if (active) {
          addToast(extractErrorMessage(error, t("shootingScript.loadFailed")), "error");
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
    ) return;
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
        () => generateChapterShootingScript(
          projectId,
          chapterId,
          taskId,
          confirmedVisualPlanId,
        ),
      );
      setView(generated);
      setUnitProgress(null);
      setDraft(structuredClone(generated.script));
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
    ) return;
    setGenerating(true);
    try {
      const taskId = createWorkflowTaskId("chapter-shooting-script-unit", projectId);
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
        () => generateNextChapterShootingScriptUnit(
          projectId,
          chapterId,
          taskId,
          confirmedVisualPlanId,
        ),
      );
      if (generated.completed_record) {
        const completedView = {
          record: generated.completed_record,
          script: generated.script,
        } satisfies ChapterShootingScriptView;
        setView(completedView);
        setUnitProgress(null);
        setDraft(structuredClone(generated.script));
      } else {
        setUnitProgress(generated);
      }
      setInvalidatedProjectIds([]);
      addToast(
        generated.completed_record
          ? t("shootingScript.allScenesCompleted")
          : t("shootingScript.sceneProgress", { current: generated.completed_unit_count, total: generated.total_unit_count }),
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
    if (!view || !draft || chapterId == null || saving || generating || scriptLoading) return;
    setSaving(true);
    try {
      const result = await editChapterShootingScript(
        projectId,
        chapterId,
        view,
        draft,
      );
      setView(result.shooting_script);
      setDraft(structuredClone(result.shooting_script.script));
      setInvalidatedProjectIds(result.invalidated_project_ids);
      addToast(
        result.invalidated_project_ids.length > 0
          ? t("shootingScript.downstreamInvalidated", { count: result.invalidated_project_ids.length })
          : t("shootingScript.saveCompleted"),
        "success",
      );
    } catch (error) {
      addToast(extractErrorMessage(error, t("shootingScript.saveFailed")), "error");
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!view || chapterId == null || deleting || generating || saving || scriptLoading) return;
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
      addToast(t("shootingScript.deleteCompleted"), "success");
    } catch (error) {
      addToast(extractErrorMessage(error, t("shootingScript.deleteFailed")), "error");
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
      <div className="flex min-w-0 flex-wrap items-center justify-between gap-2 px-1">
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
                  : view
                  ? t("shootingScript.currentRevision", { revision: view.record.revision, dirty: isDirty ? ` · ${t("writing.unsavedChanges")}` : "" })
                  : unitProgress
                    ? t("shootingScript.completedScenes", { current: unitProgress.completed_unit_count, total: unitProgress.total_unit_count })
                  : confirmedVisualPlanId == null
                    ? t("shootingScript.choosePlan")
                    : t("shootingScript.notGenerated")}
        </div>
        {canEdit ? (
          <div className="ml-auto flex items-center gap-2">
            {view && draft ? (
              <>
                <Dropdown
                  items={[
                    {
                      label: generating ? t("shootingScript.regenerating") : t("shootingScript.regenerate"),
                      onClick: () => void generate(),
                      disabled: actionsDisabled,
                      icon: generating ? (
                        <LoadingSpinner size="sm" />
                      ) : (
                        <RefreshCw size={12} />
                      ),
                    },
                    { type: "divider" },
                    {
                      label: deleting ? t("shootingScript.deleting") : t("shootingScript.delete"),
                      onClick: () => void remove(),
                      disabled: actionsDisabled,
                      danger: true,
                      icon: deleting ? <LoadingSpinner size="sm" /> : <Trash2 size={12} />,
                    },
                  ]}
                />
                <BookButton
                  size="sm"
                  onClick={() => void save()}
                  disabled={actionsDisabled || !isDirty}
                >
                  {saving ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Save size={13} />
                  )}
                  {saving ? t("shootingScript.saving") : t("shootingScript.saveAdaptation")}
                </BookButton>
              </>
            ) : (
              <>
                {unitProgress ? (
                  <Dropdown
                    items={[
                      {
                        label: t("shootingScript.generateRemaining"),
                        onClick: () => void generate(),
                        disabled: actionsDisabled,
                        icon: <RefreshCw size={12} />,
                      },
                    ]}
                  />
                ) : null}
                <BookButton
                  size="sm"
                  onClick={() => void generateNextUnit()}
                  disabled={actionsDisabled || confirmedVisualPlanId == null}
                >
                  {generating ? <LoadingSpinner size="sm" /> : <Wand2 size={13} />}
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
          <LoadingSpinner size="md" text={t("shootingScript.loading")} inline={false} />
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
          {canEdit && draft ? (
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
                {t("shootingScript.invalidatedHint", { count: invalidatedProjectIds.length })}
              </p>
              <details className="mt-1">
                <summary className="w-fit cursor-pointer hover:text-book-text-main">
                  {t("writing.diagnostics")}
                </summary>
                <p className="mt-1 break-all font-mono text-caption">
                  {t("shootingScript.downstreamIds", { ids: invalidatedProjectIds.join(", ") })}
                </p>
              </details>
            </div>
          ) : null}
        </>
      ) : unitProgress ? (
        <ScriptContent script={unitProgress.script} />
      ) : null}
    </section>
  );
};
