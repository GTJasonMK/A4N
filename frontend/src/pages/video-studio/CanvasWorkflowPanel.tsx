import React from "react";
import { Loader2, Play, RefreshCw, Square } from "lucide-react";
import type { ProgressEventRecord } from "../../api/progress";
import type { WorkflowRunDetail } from "../../api/streaming";
import { Dropdown, type DropdownItem } from "../../components/ui/Dropdown";
import { getWorkflowStatusLabel } from "../../utils/workflowPresentation";

interface CanvasWorkflowPanelProps {
  latestWorkflowRunId: number | null;
  detail: WorkflowRunDetail | null;
  events: ProgressEventRecord[];
  loading: boolean;
  submittingLabel: string | null;
  cancelling: boolean;
  resuming: boolean;
  onRefresh: () => void;
  onCancel: () => void;
  onResume: () => void;
}

export const CanvasWorkflowPanel: React.FC<CanvasWorkflowPanelProps> = ({
  latestWorkflowRunId,
  detail,
  events,
  loading,
  submittingLabel,
  cancelling,
  resuming,
  onRefresh,
  onCancel,
  onResume,
}) => {
  const run = detail?.run ?? null;
  const activeTaskId = run?.active_task_id ?? null;
  const activeCheckpoint = latestCheckpointWithStatus(detail?.checkpoints ?? [], "running");
  const failedCheckpoint = latestCheckpointWithStatus(detail?.checkpoints ?? [], "failed");
  const currentStepKey = activeCheckpoint?.step_key ?? failedCheckpoint?.step_key ?? run?.current_step_key ?? null;
  const completedCheckpointCount = detail?.checkpoints.filter((checkpoint) => checkpoint.status === "completed").length ?? 0;
  const waitingProvider = activeCheckpoint ? canvasVideoProviderWait(activeCheckpoint) : null;
  const staleRunning = run ? runningWorkflowLooksStale(run.updated_at) : false;
  const canCancel = Boolean(activeTaskId || run?.status === "running" || run?.status === "pending");
  const canResume = Boolean(run?.can_resume && run.status !== "running" && run.status !== "pending");
  const resumeTitle = run?.can_resume
    ? "从未完成节点继续运行"
    : run?.resume_block_reason ?? "该运行当前不可继续";
  const actionItems: DropdownItem[] = [];
  if (latestWorkflowRunId) {
    actionItems.push({
      label: "刷新运行记录",
      icon: loading ? <Loader2 size={12} className="animate-spin" /> : <RefreshCw size={12} />,
      disabled: loading,
      onClick: onRefresh,
    });
  }
  if (canResume) {
    actionItems.push({
      label: "继续运行",
      icon: resuming ? <Loader2 size={12} className="animate-spin" /> : <Play size={12} />,
      disabled: resuming,
      onClick: onResume,
    });
  }
  if (canCancel) {
    actionItems.push({
      label: "停止运行",
      icon: cancelling ? <Loader2 size={12} className="animate-spin" /> : <Square size={12} />,
      danger: true,
      disabled: cancelling,
      onClick: onCancel,
    });
  }
  return (
    <section aria-label="工作流运行记录" className="mt-4 border-t border-book-border/70 pt-3 text-xs">
      <div className="mb-2 flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="text-xs font-semibold text-book-text-main">运行状态</h3>
          <p className="mt-0.5 truncate text-[11px] text-book-text-muted" title={canResume ? resumeTitle : undefined}>
            {run
              ? canvasWorkflowStepLabel(currentStepKey)
              : latestWorkflowRunId
                ? "正在加载最近运行"
                : "尚未运行"}
          </p>
        </div>
        {actionItems.length > 0 ? <Dropdown items={actionItems} /> : null}
      </div>
      {submittingLabel ? (
        <div className="mb-2 space-y-1.5 border-l-2 border-book-primary/50 bg-book-primary/5 px-2.5 py-2">
          <div className="inline-flex items-center gap-1.5 text-[11px] text-book-text-main">
            <Loader2 size={12} className="animate-spin" />
            {submittingLabel}
          </div>
          {!run ? <div className="text-[11px] text-book-text-muted">正在发现运行记录，找到后会自动显示节点进度。</div> : null}
        </div>
      ) : null}
      {!submittingLabel && !latestWorkflowRunId && !run ? (
        <div className="text-[11px] text-book-text-muted">尚未运行画布节点</div>
      ) : submittingLabel && !run ? null : !detail && loading ? (
        <div className="inline-flex items-center gap-1.5 text-[11px] text-book-text-muted">
          <Loader2 size={12} className="animate-spin" />
          加载运行记录
        </div>
      ) : run ? (
        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-2 text-[11px]">
            <span className={workflowStatusTextClass(run.status)}>
              {getWorkflowStatusLabel(run.status)}
            </span>
            <span className="min-w-0 flex-1 truncate text-book-text-sub" title={run.progress?.message ?? ""}>
              {run.progress ? `${Math.round(run.progress.percent ?? 0)}% ${run.progress.message}` : "-"}
            </span>
          </div>
          {run.progress ? (
            <div className="h-1.5 overflow-hidden rounded-full bg-book-bg-tertiary">
              <div
                className="h-full rounded-full bg-book-primary/75 transition-[width] duration-300"
                style={{ width: `${Math.max(0, Math.min(100, run.progress.percent ?? 0))}%` }}
              />
            </div>
          ) : null}
          {run.error_message ? (
            <div role="alert" className="rounded-xl border border-error/50 bg-error-bg px-2.5 py-2 text-[11px] text-error">
              {run.error_message}
            </div>
          ) : null}
          {waitingProvider ? (
            <div role="status" className="rounded-xl border border-warning/50 bg-warning-bg px-2.5 py-2 text-[11px] leading-4 text-warning">
              视频服务正在生成，请稍候
              {staleRunning ? "。该运行已长时间无进度更新，可能是旧版本遗留的挂起任务；请重新运行画布生成新的后台轮询。" : ""}
            </div>
          ) : null}
          <div className="flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-book-text-muted">
            <span>
              已完成 <span className="tabular-nums text-book-text-main">{completedCheckpointCount} / {detail!.checkpoints.length}</span>
            </span>
            {detail!.generation_runs.length > 0 ? (
              <span>
                模型调用 <span className="tabular-nums text-book-text-main">{detail!.generation_runs.length}</span>
              </span>
            ) : null}
          </div>
          {events.length > 0 ? (
            <div className="divide-y divide-book-border/45 border-y border-book-border/45">
              {events.slice(0, 5).map((event) => (
                <div key={event.id} className="grid grid-cols-[42px_minmax(0,1fr)] gap-2 px-2.5 py-1.5 text-[10px]">
                  <span className="font-mono text-book-text-muted">
                    {event.percent == null ? "-" : `${Math.round(event.percent)}%`}
                  </span>
                  <span className="truncate text-book-text-sub" title={event.message}>
                    {event.message}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-[11px] text-book-text-muted">暂无进度事件</div>
          )}
          <details className="rounded border border-book-border/60 bg-book-bg-paper/45 text-[10px]">
            <summary className="cursor-pointer px-2.5 py-2 text-book-text-muted">诊断信息</summary>
            <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 border-t border-book-border/50 px-2.5 py-2">
              <span className="text-book-text-muted">运行编号</span>
              <span className="font-mono text-book-text-sub">{run.id}</span>
              {run.active_task_id ? (
                <>
                  <span className="text-book-text-muted">任务编号</span>
                  <span className="break-all font-mono text-book-text-sub">{run.active_task_id}</span>
                </>
              ) : null}
              {currentStepKey ? (
                <>
                  <span className="text-book-text-muted">步骤标识</span>
                  <span className="break-all font-mono text-book-text-sub">{currentStepKey}</span>
                </>
              ) : null}
              {waitingProvider?.generatedVideoId ? (
                <>
                  <span className="text-book-text-muted">视频编号</span>
                  <span className="font-mono text-book-text-sub">{waitingProvider.generatedVideoId}</span>
                </>
              ) : null}
              {waitingProvider?.providerJobId ? (
                <>
                  <span className="text-book-text-muted">服务任务</span>
                  <span className="break-all font-mono text-book-text-sub">{waitingProvider.providerJobId}</span>
                </>
              ) : null}
            </div>
          </details>
        </div>
      ) : (
        <div className="text-[11px] text-book-text-muted">运行记录不可用</div>
      )}
    </section>
  );
};

function workflowStatusTextClass(status: string): string {
  if (status === "running" || status === "pending") return "font-medium text-book-primary";
  if (status === "completed" || status === "success") return "font-medium text-success";
  if (status === "failed" || status === "error") return "font-medium text-error";
  return "text-book-text-muted";
}

function canvasWorkflowStepLabel(stepKey: string | null): string {
  if (!stepKey) return "等待开始";
  if (/^canvas_node:\d+$/.test(stepKey)) return "处理画布节点";
  const labels: Record<string, string> = {
    prepare: "准备运行",
    validate: "检查画布",
    execute: "运行节点",
    finalize: "整理结果",
    video_provider: "生成视频",
    video_poll: "等待视频服务",
  };
  return labels[stepKey] ?? "处理工作流";
}

function latestCheckpointWithStatus(
  checkpoints: NonNullable<WorkflowRunDetail["checkpoints"]>,
  status: string,
): NonNullable<WorkflowRunDetail["checkpoints"]>[number] | null {
  return checkpoints
    .filter((checkpoint) => checkpoint.status === status)
    .sort((left, right) => right.updated_at.localeCompare(left.updated_at))[0] ?? null;
}

function canvasVideoProviderWait(checkpoint: NonNullable<WorkflowRunDetail["checkpoints"]>[number]): {
  providerJobId: string | null;
  generatedVideoId: number | null;
} | null {
  const artifacts = checkpoint.artifacts_json;
  if (!isRecord(artifacts)) return null;
  const providerJobId = typeof artifacts.provider_job_id === "string" ? artifacts.provider_job_id : null;
  const generatedVideoId = typeof artifacts.generated_video_id === "number" ? artifacts.generated_video_id : null;
  if (!providerJobId && !generatedVideoId) return null;
  return { providerJobId, generatedVideoId };
}

function runningWorkflowLooksStale(updatedAt: string | null | undefined): boolean {
  const updatedAtMs = parseWorkflowPanelTimestampMs(updatedAt);
  if (updatedAtMs === null) return false;
  return Date.now() - updatedAtMs > 10 * 60 * 1000;
}

function parseWorkflowPanelTimestampMs(value: string | null | undefined): number | null {
  if (!value) return null;
  const trimmed = value.trim();
  if (!trimmed) return null;
  const hasTimezone = /(?:Z|[+-]\d{2}:?\d{2})$/.test(trimmed);
  const normalized = trimmed.includes("T") ? trimmed : trimmed.replace(" ", "T");
  const parsed = Date.parse(hasTimezone ? normalized : `${normalized}Z`);
  return Number.isFinite(parsed) ? parsed : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
