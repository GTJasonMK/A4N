import React from "react";
import { Loader2, Play, RefreshCw, Square } from "lucide-react";
import type { ProgressEventRecord } from "../../api/progress";
import type { WorkflowRunDetail } from "../../api/streaming";
import { BookButton } from "../../components/ui/BookButton";

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
  return (
    <div className="mt-3 rounded border border-book-border bg-book-bg p-2 text-xs">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="text-[10px] font-medium text-book-text-muted">运行记录</div>
        <div className="flex items-center gap-1">
          <BookButton variant="secondary" size="sm" onClick={onRefresh} disabled={!latestWorkflowRunId || loading}>
            {loading ? <Loader2 size={12} className="animate-spin" /> : <RefreshCw size={12} />}
            刷新
          </BookButton>
          <BookButton
            variant="secondary"
            size="sm"
            onClick={onResume}
            disabled={!canResume || resuming}
            title={resumeTitle}
          >
            {resuming ? <Loader2 size={12} className="animate-spin" /> : <Play size={12} />}
            继续运行
          </BookButton>
          <BookButton variant="secondary" size="sm" onClick={onCancel} disabled={!canCancel || cancelling}>
            {cancelling ? <Loader2 size={12} className="animate-spin" /> : <Square size={12} />}
            取消
          </BookButton>
        </div>
      </div>
      {submittingLabel ? (
        <div className="mb-2 space-y-2 rounded border border-book-border bg-book-bg-paper px-2 py-1.5">
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
          <div className="grid grid-cols-[64px_minmax(0,1fr)] gap-x-2 gap-y-1 text-[11px]">
            <span className="text-book-text-muted">Run</span>
            <span className="font-mono text-book-text-main">#{run.id}</span>
            <span className="text-book-text-muted">状态</span>
            <span className="text-book-text-main">{run.status}</span>
            <span className="text-book-text-muted">当前</span>
            <span className="truncate text-book-text-main" title={currentStepKey ?? ""}>
              {currentStepKey ?? "-"}
            </span>
            <span className="text-book-text-muted">进度</span>
            <span className="truncate text-book-text-main" title={run.progress?.message ?? ""}>
              {run.progress ? `${Math.round(run.progress.percent ?? 0)}% ${run.progress.message}` : "-"}
            </span>
          </div>
          {run.error_message ? (
            <div className="rounded border border-red-300/50 bg-red-50 px-2 py-1 text-[11px] text-red-700">
              {run.error_message}
            </div>
          ) : null}
          {waitingProvider ? (
            <div className="rounded border border-amber-300/50 bg-amber-50 px-2 py-1 text-[11px] leading-4 text-amber-800">
              视频 provider 任务处理中：视频 #{waitingProvider.generatedVideoId ?? "-"}
              {waitingProvider.providerJobId ? ` / job ${waitingProvider.providerJobId}` : ""}
              {staleRunning ? "。该运行已长时间无进度更新，可能是旧版本遗留的挂起任务；请重新运行画布生成新的后台轮询。" : ""}
            </div>
          ) : null}
          <div className="grid grid-cols-2 gap-2 text-[11px] text-book-text-muted">
            <div>
              checkpoint <span className="font-mono text-book-text-main">{detail!.checkpoints.length}</span>
            </div>
            <div>
              完成 <span className="font-mono text-book-text-main">{completedCheckpointCount}</span>
            </div>
            <div>
              generation <span className="font-mono text-book-text-main">{detail!.generation_runs.length}</span>
            </div>
            <div>
              运行中 <span className="font-mono text-book-text-main">{activeCheckpoint ? 1 : 0}</span>
            </div>
          </div>
          {events.length > 0 ? (
            <div className="space-y-1">
              {events.slice(0, 5).map((event) => (
                <div key={event.id} className="grid grid-cols-[42px_minmax(0,1fr)] gap-2 text-[10px]">
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
        </div>
      ) : (
        <div className="text-[11px] text-book-text-muted">运行记录不可用</div>
      )}
    </div>
  );
};

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
