import React from "react";
import { X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { WorkflowState } from "../../../api/streaming";
import { Dropdown } from "../../../components/ui/Dropdown";
import { progressDisplayModel } from "../../../utils/progressPresentation";
import { ProgressEventList } from "./ProgressEventList";
import { getOperationStatusLabel } from "../../../utils/workflowPresentation";

interface ActiveWorkflowsPanelProps {
  workflows: WorkflowState[];
  loading: boolean;
  cancellingTaskIds: ReadonlySet<string>;
  onCancel: (taskId: string) => void;
}

export const ActiveWorkflowsPanel: React.FC<ActiveWorkflowsPanelProps> = ({
  workflows,
  loading,
  cancellingTaskIds,
  onCancel,
}) => (
  <section>
    <div className="flex items-baseline gap-2 px-1 pb-2">
      <h3 className="text-xs font-semibold text-book-text-main">活跃工作流</h3>
      <span className="text-small text-book-text-muted">
        {workflows.length > 0 ? `${workflows.length} 个运行中` : "实时进度"}
      </span>
    </div>
    {loading && workflows.length === 0 ? (
      <div className="a4n-flat-surface flex items-center justify-center gap-1.5 px-3 py-5 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" />
        正在加载活跃工作流…
      </div>
    ) : workflows.length === 0 ? (
      <div className="a4n-empty-state a4n-empty-state--framed px-3 py-5 text-center text-xs text-book-text-muted">
        当前没有运行中的工作流
      </div>
    ) : (
      <ul className="space-y-2.5">
        {workflows.map((workflow) => {
          const cancelling = cancellingTaskIds.has(workflow.task_id);
          const canCancel = !["completed", "failed", "cancelled", "canceled"].includes(
            workflow.status,
          );
          return (
            <li key={workflow.task_id} className="a4n-flat-surface px-3 py-3">
              <div className="flex min-w-0 items-start gap-3">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-xs font-medium text-book-text-main">
                    {activeWorkflowTitle(workflow)}
                  </div>
                  <div className="mt-0.5 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-small text-book-text-muted">
                    <span className={workflowStatusTone(workflow.status)}>
                      {getOperationStatusLabel(workflow.status)}
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
                ) : canCancel ? (
                  <Dropdown
                    items={[
                      {
                        label: "取消工作流",
                        icon: <X size={13} />,
                        danger: true,
                        onClick: () => onCancel(workflow.task_id),
                      },
                    ]}
                  />
                ) : null}
              </div>
              {workflow.progress && <WorkflowProgress progress={workflow.progress} />}
              <ProgressEventList events={workflow.progress_events} />
              <details className="mt-2 text-caption text-book-text-muted">
                <summary className="w-fit cursor-pointer select-none py-1 hover:text-book-text-main">
                  诊断信息
                </summary>
                <div className="mt-1 grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 rounded-xl bg-book-bg-tertiary/45 p-2.5">
                  <span className="text-book-text-muted">任务编号</span>
                  <span className="break-all font-mono text-book-text-sub">{workflow.task_id}</span>
                  {workflow.task_scope ? (
                    <>
                      <span className="text-book-text-muted">任务范围</span>
                      <span className="break-all font-mono text-book-text-sub">{workflow.task_scope}</span>
                    </>
                  ) : null}
                  {workflow.root_project_id != null ? (
                    <>
                      <span className="text-book-text-muted">根项目</span>
                      <span className="font-mono text-book-text-sub">{workflow.root_project_id}</span>
                    </>
                  ) : null}
                  {workflow.source_snapshot_id != null ? (
                    <>
                      <span className="text-book-text-muted">来源快照</span>
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

function activeWorkflowTitle(workflow: WorkflowState): string {
  const scopeLabels: Record<string, string> = {
    project: "项目任务",
    chapter: "章节任务",
    video: "视频任务",
    canvas: "画布任务",
    source_sync: "来源同步",
  };
  return scopeLabels[workflow.task_scope ?? ""] ?? "后台任务";
}

function workflowStatusTone(status: string): string {
  if (status === "failed") return "text-error";
  if (status === "completed") return "text-success";
  if (status === "running") return "text-book-primary";
  return "text-book-text-muted";
}

const WorkflowProgress: React.FC<{ progress: NonNullable<WorkflowState["progress"]> }> = ({
  progress,
}) => {
  const stageHint = progress.stage?.trim() && progress.stage.trim() !== progress.message.trim()
    ? progress.stage
    : null;
  const display = progressDisplayModel({
    ...progress,
    stageHint,
    fallbackStage: "任务执行中",
  });
  const detail = display.detail ?? "等待子阶段更新…";

  return (
    <div className="mt-2.5">
      <div className="flex min-w-0 items-baseline gap-2 text-small">
        <span className="min-w-0 flex-1 truncate text-book-text-sub" title={display.stage}>
          {display.stage}
        </span>
        <span className="shrink-0 text-book-text-muted">
          {display.percent == null ? display.countLabel : `${Math.round(display.percent)}%`}
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
