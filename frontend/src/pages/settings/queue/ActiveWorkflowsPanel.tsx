import React from "react";
import { Loader2, Wifi, X } from "lucide-react";
import type { WorkflowState } from "../../../api/streaming";
import { BookIconButton } from "../../../components/ui/BookButton";
import { progressDisplayModel } from "../../../utils/progressPresentation";
import { ProgressEventList } from "./ProgressEventList";

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
  <div className="pyqt-card p-4">
    <div className="flex items-center justify-between mb-2">
      <h4 className="text-sm font-serif text-book-text-main inline-flex items-center gap-2">
        <Wifi size={14} className="text-book-primary" /> 活跃工作流（{workflows.length}）
      </h4>
      <span className="text-[10px] text-book-text-muted">含进度条与阶段信息</span>
    </div>
    {loading && workflows.length === 0 ? (
      <div className="inline-flex items-center justify-center gap-1.5 py-3 text-xs text-book-text-muted">
        <Loader2 size={12} className="animate-spin" />
        正在加载活跃工作流…
      </div>
    ) : workflows.length === 0 ? (
      <div className="text-xs text-book-text-muted py-3 text-center">
        当前无运行中的流式工作流
      </div>
    ) : (
      <ul className="space-y-1.5">
        {workflows.map((workflow) => {
          const hasProgress = workflow.progress != null;
          const cancelling = cancellingTaskIds.has(workflow.task_id);
          return (
            <li
              key={workflow.task_id}
              className="a4n-stat-tile px-3 py-2"
            >
              <div className="mb-1 flex min-w-0 items-center gap-2 text-xs">
                <span className="shrink-0 font-mono text-[10px] text-book-text-muted">
                  {workflow.task_id.slice(0, 12)}
                </span>
                <span
                  className={`pyqt-pill !text-[10px] shrink-0 ${
                    workflow.status === "completed"
                      ? "!border-success/50 !text-success"
                      : workflow.status === "failed"
                        ? "!border-error/50 !text-error"
                        : workflow.status === "running"
                          ? "!border-book-primary !text-book-primary"
                          : ""
                  }`}
                >
                  {workflow.status}
                </span>
                {workflow.stage && !hasProgress && (
                  <span className="pyqt-pill min-w-0 !text-[10px]">
                    <span className="block max-w-40 truncate">{workflow.stage}</span>
                  </span>
                )}
                {workflow.task_scope && (
                  <span className="pyqt-pill !text-[10px] shrink-0">{workflow.task_scope}</span>
                )}
                {workflow.root_project_id != null && (
                  <span className="shrink-0 text-book-text-muted text-[10px]">
                    root #{workflow.root_project_id}
                  </span>
                )}
                {workflow.source_snapshot_id != null && (
                  <span className="shrink-0 text-book-text-muted text-[10px]">
                    snap #{workflow.source_snapshot_id}
                  </span>
                )}
                {workflow.progress && (
                  <span className="shrink-0 text-book-text-muted text-[10px]">
                    {workflow.progress.current} / {workflow.progress.total}
                  </span>
                )}
                <span className="min-w-0 flex-1" />
                <BookIconButton
                  label="取消工作流"
                  onClick={() => onCancel(workflow.task_id)}
                  variant="secondary"
                  size="sm"
                  disabled={
                    cancelling ||
                    workflow.status === "completed" ||
                    workflow.status === "failed"
                  }
                >
                  {cancelling ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <X size={12} />
                  )}
                </BookIconButton>
              </div>
              {workflow.progress && <WorkflowProgress progress={workflow.progress} />}
              <ProgressEventList events={workflow.progress_events} />
            </li>
          );
        })}
      </ul>
    )}
  </div>
);

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
    <>
      <div className="mb-1 flex min-w-0 items-center justify-between gap-2 text-[10px]">
        <span className="min-w-0 truncate font-semibold text-book-text-sub">
          {display.stage}
        </span>
        <span className="shrink-0 font-mono text-book-text-muted">
          {display.percent == null ? "未返回" : `${Math.round(display.percent)}%`}
        </span>
      </div>
      <div className="relative h-4 w-full overflow-hidden rounded border border-book-border/50 bg-book-bg-tertiary">
        <div
          className="h-full bg-book-primary/35 transition-all"
          style={{ width: display.percent == null ? "0%" : `${display.percent}%` }}
        />
      </div>
      <div className="mt-1 flex min-w-0 items-center justify-between gap-2 text-[10px]">
        <span className="min-w-0 truncate text-book-text-muted" title={detail}>
          {detail}
        </span>
        {display.countLabel ? (
          <span className="shrink-0 font-mono text-book-text-muted">{display.countLabel}</span>
        ) : null}
      </div>
      {display.percent == null ? (
        <div className="mt-1 text-[10px] text-book-text-muted">
          后端未返回进度百分比
        </div>
      ) : null}
    </>
  );
};
