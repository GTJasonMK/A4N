import React from "react";
import { ListChecks, Loader2, X } from "lucide-react";
import { BookIconButton } from "../../../components/ui/BookButton";
import type { TaskInfo } from "../../../api/task";
import { taskProgressDisplay } from "./queueTabUtils";
import { ProgressEventList } from "./ProgressEventList";

interface TaskHistoryPanelProps {
  tasks: TaskInfo[];
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
  <div className="pyqt-card p-4">
    <div className="flex items-center justify-between mb-2">
      <h4 className="text-sm font-serif text-book-text-main inline-flex items-center gap-2">
        <ListChecks size={14} className="text-book-primary" /> 最近任务（30 条）
      </h4>
      {loading && <Loader2 size={12} className="animate-spin text-book-text-muted" />}
    </div>
    {loading && tasks.length === 0 ? (
      <div className="inline-flex items-center justify-center gap-1.5 py-4 text-xs text-book-text-muted">
        <Loader2 size={12} className="animate-spin" />
        正在加载最近任务…
      </div>
    ) : tasks.length === 0 ? (
      <div className="text-xs text-book-text-muted py-4 text-center">暂无任务</div>
    ) : (
      <ul className="max-h-72 overflow-y-auto custom-scrollbar divide-y divide-book-border/30">
        {tasks.map((task) => {
          const canCancel = task.status === "pending" || task.status === "running";
          const progressDisplay = taskProgressDisplay(task);
          const cancelling = cancellingTaskIds.has(task.id);
          return (
            <li key={task.id} className="py-2 text-xs">
              <div className="flex items-center gap-2">
                <span className="font-mono text-[10px] text-book-text-muted shrink-0 w-20 truncate">
                  {task.id.slice(0, 16)}
                </span>
                <span className="pyqt-pill !text-[10px] shrink-0">{task.task_type}</span>
                <span
                  className={`pyqt-pill !text-[10px] shrink-0 ${
                    task.priority === "high"
                      ? "!border-error/50 !text-error"
                      : task.priority === "low"
                        ? "!text-book-text-muted"
                        : ""
                  }`}
                >
                  {task.priority}
                </span>
                <span
                  className={`pyqt-pill !text-[10px] shrink-0 ${
                    task.status === "completed"
                      ? "!border-success/50 !text-success"
                      : task.status === "failed"
                        ? "!border-error/50 !text-error"
                        : task.status === "running"
                          ? "!border-book-primary !text-book-primary"
                          : ""
                  }`}
                >
                  {task.status}
                </span>
                {task.task_scope && (
                  <span className="pyqt-pill !text-[10px] shrink-0">{task.task_scope}</span>
                )}
                {task.root_project_id != null && (
                  <span className="text-book-text-muted shrink-0">
                    root #{task.root_project_id}
                  </span>
                )}
                {task.source_snapshot_id != null && (
                  <span className="text-book-text-muted shrink-0">
                    snap #{task.source_snapshot_id}
                  </span>
                )}
                {task.progress && (
                  <span className="text-book-text-muted shrink-0">
                    {task.progress.current}/{task.progress.total}
                  </span>
                )}
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-book-text-sub">
                    {progressDisplay.stage}
                  </span>
                  {progressDisplay.detail ? (
                    <span className="block truncate text-[10px] leading-4 text-book-text-muted">
                      {progressDisplay.detail}
                    </span>
                  ) : null}
                </span>
                {canCancel && (
                  <BookIconButton
                    label="取消任务"
                    onClick={() => onCancel(task.id)}
                    variant="secondary"
                    size="sm"
                    disabled={cancelling}
                  >
                    {cancelling ? (
                      <Loader2 size={12} className="animate-spin" />
                    ) : (
                      <X size={12} />
                    )}
                  </BookIconButton>
                )}
              </div>
              <ProgressEventList events={task.progress_events} limit={2} />
            </li>
          );
        })}
      </ul>
    )}
  </div>
);
