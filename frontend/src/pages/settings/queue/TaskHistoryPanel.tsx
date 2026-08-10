import React from "react";
import { X } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { Dropdown } from "../../../components/ui/Dropdown";
import type { TaskInfoSummary } from "../../../api/task";
import { taskProgressDisplay } from "./queueTabUtils";
import {
  getOperationStatusLabel,
  getTaskPriorityLabel,
  getTaskTypeLabel,
} from "../../../utils/workflowPresentation";

interface TaskHistoryPanelProps {
  tasks: TaskInfoSummary[];
  loading: boolean;
  cancellingTaskIds: ReadonlySet<string>;
  onCancel: (taskId: string) => void;
}

export const TaskHistoryPanel: React.FC<TaskHistoryPanelProps> = ({
  tasks,
  loading,
  cancellingTaskIds,
  onCancel,
}) => (
  <section>
    <div className="flex items-baseline gap-2 px-1 pb-2">
      <h3 className="text-xs font-semibold text-book-text-main">最近任务</h3>
      <span className="text-small text-book-text-muted">
        {tasks.length > 0 ? `显示最近 ${tasks.length} 条` : "执行记录"}
      </span>
      {loading ? <LoadingSpinner size="sm" className="ml-auto  text-book-text-muted" /> : null}
    </div>
    {loading && tasks.length === 0 ? (
      <div className="a4n-flat-surface flex items-center justify-center gap-1.5 px-3 py-5 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" />
        正在加载最近任务…
      </div>
    ) : tasks.length === 0 ? (
      <div className="a4n-empty-state a4n-empty-state--framed px-3 py-5 text-center text-xs text-book-text-muted">
        暂无任务
      </div>
    ) : (
      <ul className="space-y-2.5">
        {tasks.map((task) => {
          const canCancel = task.status === "pending" || task.status === "running";
          const progressDisplay = taskProgressDisplay(task);
          const cancelling = cancellingTaskIds.has(task.id);
          const taskTime = formatTaskTime(task.completed_at ?? task.started_at ?? task.created_at);
          return (
            <li key={task.id} className="a4n-flat-surface px-3 py-3 text-xs">
              <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-3">
                <div className="min-w-0">
                  <div className="truncate font-medium text-book-text-main">
                    {getTaskTypeLabel(task.task_type)}
                  </div>
                  <div className="mt-0.5 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-small text-book-text-muted">
                    <span className={task.priority === "high" ? "text-error" : undefined}>
                      {getTaskPriorityLabel(task.priority)}
                    </span>
                    <span aria-hidden="true">·</span>
                    <span className={taskStatusTone(task.status)}>
                      {getOperationStatusLabel(task.status)}
                    </span>
                    {task.progress ? (
                      <>
                        <span aria-hidden="true">·</span>
                        <span>{task.progress.current} / {task.progress.total}</span>
                      </>
                    ) : null}
                    {taskTime ? (
                      <>
                        <span aria-hidden="true">·</span>
                        <time>{taskTime}</time>
                      </>
                    ) : null}
                  </div>
                  <div className="mt-1.5 min-w-0">
                    <span className="block truncate text-small text-book-text-sub">
                      {progressDisplay.stage}
                    </span>
                    {progressDisplay.detail ? (
                      <span className="block truncate text-small leading-4 text-book-text-muted">
                        {progressDisplay.detail}
                      </span>
                    ) : null}
                  </div>
                </div>
                {cancelling ? (
                  <span className="inline-flex h-8 w-8 items-center justify-center text-book-text-muted">
                    <LoadingSpinner size="sm" />
                  </span>
                ) : canCancel ? (
                  <Dropdown
                    items={[
                      {
                        label: "取消任务",
                        icon: <X size={13} />,
                        danger: true,
                        onClick: () => onCancel(task.id),
                      },
                    ]}
                  />
                ) : null}
              </div>
              <details className="mt-1 text-caption text-book-text-muted">
                <summary className="cursor-pointer">诊断信息</summary>
                <div className="mt-1 flex flex-wrap gap-x-4 gap-y-1 font-mono">
                  <span>任务编号 {task.id}</span>
                  {task.task_scope ? <span>任务范围 {task.task_scope}</span> : null}
                  {task.root_project_id != null ? <span>根项目 {task.root_project_id}</span> : null}
                  {task.source_snapshot_id != null ? <span>来源快照 {task.source_snapshot_id}</span> : null}
                </div>
              </details>
            </li>
          );
        })}
      </ul>
    )}
  </section>
);

function taskStatusTone(status: string): string {
  if (status === "completed") return "text-success";
  if (status === "failed") return "text-error";
  if (status === "running") return "text-book-primary";
  return "text-book-text-muted";
}

function formatTaskTime(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return "";
  return parsed.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}
