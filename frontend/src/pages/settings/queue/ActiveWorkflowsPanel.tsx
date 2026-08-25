import React from "react";
import { X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { WorkflowStateSummary } from "../../../api/streaming";
import { Dropdown } from "../../../components/ui/Dropdown";
import { progressDisplayModel } from "../../../utils/progressPresentation";
import { ProgressEventList } from "./ProgressEventList";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";
import { formatNumber, useI18n } from "../../../i18n";

interface ActiveWorkflowsPanelProps {
  workflows: WorkflowStateSummary[];
  loading: boolean;
  cancellingTaskIds: ReadonlySet<string>;
  onCancel: (taskId: string) => void;
}

export const ActiveWorkflowsPanel: React.FC<ActiveWorkflowsPanelProps> = ({
  workflows,
  loading,
  cancellingTaskIds,
  onCancel,
}) => {
  const { t, locale } = useI18n();
  return (
  <section>
    <div className="flex items-baseline gap-2 px-1 pb-2">
      <h3 className="text-xs font-semibold text-book-text-main">{t("settingsQueue.activeWorkflows")}</h3>
      <span className="text-small text-book-text-muted">
        {workflows.length > 0 ? t("settingsQueue.activeRunsCount", { count: formatNumber(workflows.length, locale) }) : t("settingsQueue.realtimeProgress")}
      </span>
    </div>
    {loading && workflows.length === 0 ? (
      <div className="a4n-flat-surface flex items-center justify-center gap-1.5 px-3 py-5 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" />
        {t("settingsQueue.loadActiveWorkflows")}
      </div>
    ) : workflows.length === 0 ? (
      <div className="a4n-empty-state a4n-empty-state--framed px-3 py-5 text-center text-xs text-book-text-muted">
        {t("settingsQueue.noRunningWorkflows")}
      </div>
    ) : (
      <ul className="space-y-2.5">
        {workflows.map((workflow) => {
          const cancelling = cancellingTaskIds.has(workflow.task_id);
          return (
            <li key={workflow.task_id} className="a4n-flat-surface px-3 py-3">
              <div className="flex min-w-0 items-start gap-3">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-xs font-medium text-book-text-main">
                    {activeWorkflowTitle(workflow, t)}
                  </div>
                  <div className="mt-0.5 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-small text-book-text-muted">
                    <span className="text-book-primary">
                      {getOperationStatusLabel("running", locale)}
                    </span>
                    {workflow.progress ? (
                      <>
                        <span aria-hidden="true">·</span>
                        <span>
                          {workflow.progress.current} / {workflow.progress.total}
                        </span>
                      </>
                    ) : null}
                  </div>
                </div>
                {cancelling ? (
                  <span className="inline-flex h-8 w-8 shrink-0 items-center justify-center text-book-text-muted">
                    <LoadingSpinner size="sm" />
                  </span>
                ) : (
                  <Dropdown
                    items={[
                      {
                        label: t("settingsQueue.cancelWorkflow"),
                        icon: <X size={13} />,
                        danger: true,
                        onClick: () => onCancel(workflow.task_id),
                      },
                    ]}
                  />
                )}
              </div>
              {workflow.progress && <WorkflowProgress progress={workflow.progress} />}
              <ProgressEventList events={workflow.progress_events} />
              <details className="mt-2 text-caption text-book-text-muted">
                <summary className="w-fit cursor-pointer select-none py-1 hover:text-book-text-main">
                  {t("settingsQueue.diagnostic")}
                </summary>
                <div className="mt-1 grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 rounded-xl bg-book-bg-tertiary/45 p-2.5">
                      <span className="text-book-text-muted">{t("settingsQueue.taskIdLabel")}</span>
                  <span className="break-all font-mono text-book-text-sub">{workflow.task_id}</span>
                  {workflow.task_scope ? (
                    <>
                      <span className="text-book-text-muted">{t("settingsQueue.taskScopeLabel")}</span>
                      <span className="break-all font-mono text-book-text-sub">{workflow.task_scope}</span>
                    </>
                  ) : null}
                  {workflow.root_project_id != null ? (
                    <>
                      <span className="text-book-text-muted">{t("settingsQueue.rootProjectLabel")}</span>
                      <span className="font-mono text-book-text-sub">{workflow.root_project_id}</span>
                    </>
                  ) : null}
                  {workflow.source_snapshot_id != null ? (
                    <>
                      <span className="text-book-text-muted">{t("settingsQueue.sourceSnapshotLabel")}</span>
                      <span className="font-mono text-book-text-sub">{workflow.source_snapshot_id}</span>
                    </>
                  ) : null}
                </div>
              </details>
            </li>
          );
        })}
      </ul>
    )}
  </section>
  );
};

function activeWorkflowTitle(workflow: WorkflowStateSummary, t: ReturnType<typeof useI18n>["t"]): string {
  const scopeLabels: Record<string, string> = {
    project: t("settingsQueue.taskProject"),
    chapter: t("settingsQueue.taskChapter"),
    video: t("settingsQueue.taskVideo"),
    canvas: t("settingsQueue.taskCanvas"),
    source_sync: t("settingsQueue.sourceSync"),
  };
  return scopeLabels[workflow.task_scope ?? ""] ?? t("settingsQueue.backgroundTask");
}

const WorkflowProgress: React.FC<{ progress: NonNullable<WorkflowStateSummary["progress"]> }> = ({
  progress,
}) => {
  const { locale, t } = useI18n();
  const display = progressDisplayModel({
    ...progress,
    locale,
    t,
    fallbackStage: t("settingsQueue.taskRunning"),
  });
  const detail = display.detail ?? t("settingsQueue.substageWaiting");

  return (
    <div className="mt-2.5">
      <div className="flex min-w-0 items-baseline gap-2 text-small">
        <span className="min-w-0 flex-1 truncate text-book-text-sub" title={display.stage}>
          {display.stage}
        </span>
        <span className="shrink-0 text-book-text-muted">
          {display.percent == null ? display.countLabel : `${formatNumber(Math.round(display.percent), locale)}%`}
        </span>
      </div>
      {display.percent != null ? (
        <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-book-bg-tertiary">
          <div
            className="h-full rounded-full bg-book-primary/70 transition-[width] duration-300 ease-out"
            style={{ width: `${display.percent}%` }}
          />
        </div>
      ) : null}
      <div className="mt-1 truncate text-small leading-4 text-book-text-muted" title={detail}>
        {detail}
      </div>
    </div>
  );
};
