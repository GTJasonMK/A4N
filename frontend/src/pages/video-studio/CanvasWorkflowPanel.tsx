import React from "react";
import { Crosshair, Play, RefreshCw, Square } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import type { ProgressEventRecord, ProgressStatus } from "../../api/progress";
import type { WorkflowRunDetailSummary } from "../../api/streaming";
import { Dropdown, type DropdownItem } from "../../components/ui/Dropdown";
import { progressDisplayModel } from "../../utils/progressPresentation";
import { getOperationStatusLabel } from "../../utils/workflowPresentation";
import { parseApiTimestampMs } from "../../utils/timestamp";
import { formatNumber, useI18n, type MessageTranslator } from "../../i18n";
import {
  CANVAS_WORKFLOW_VISIBLE_PROGRESS_EVENT_COUNT,
  canvasNodeIdFromStepKey,
} from "./canvasWorkflowState";
import { nodeDisplayTitle } from "./StudioCanvasNode";
import type { StudioNode } from "./types";

interface CanvasWorkflowPanelProps {
  latestWorkflowRunId: number | null;
  detail: WorkflowRunDetailSummary | null;
  events: ProgressEventRecord[];
  statusError: string | null;
  eventsError: string | null;
  loading: boolean;
  submittingLabel: string | null;
  nodes: StudioNode[];
  draftChanged: boolean;
  cancelling: boolean;
  resuming: boolean;
  onRefresh: () => void;
  onLocateNode: (nodeId: string) => void;
  onCancel: () => void;
  onResume: () => void;
}

export const CanvasWorkflowPanel: React.FC<CanvasWorkflowPanelProps> = ({
  latestWorkflowRunId,
  detail,
  events,
  statusError,
  eventsError,
  loading,
  submittingLabel,
  nodes,
  draftChanged,
  cancelling,
  resuming,
  onRefresh,
  onLocateNode,
  onCancel,
  onResume,
}) => {
  const { locale, t } = useI18n();
  const run = detail?.run ?? null;
  const activeTaskId = run?.active_task_id ?? null;
  const activeCheckpoint = latestCheckpointWithStatus(detail?.checkpoints ?? [], "running");
  const failedCheckpoint = latestCheckpointWithStatus(detail?.checkpoints ?? [], "failed");
  const currentStepKey = activeCheckpoint?.step_key ?? failedCheckpoint?.step_key ?? run?.current_step_key ?? null;
  const checkpointNodeId = canvasNodeIdFromStepKey(activeCheckpoint?.step_key ?? failedCheckpoint?.step_key);
  const checkpointNode = checkpointNodeId ? nodes.find((node) => node.id === checkpointNodeId) ?? null : null;
  const checkpointNodeLabel = checkpointNode ? nodeDisplayTitle(checkpointNode.data, t) : "";
  const failureMessage = run?.error_message ?? failedCheckpoint?.error_message ?? null;
  const completedCheckpointCount = detail?.checkpoints.filter((checkpoint) => checkpoint.status === "completed").length ?? 0;
  const waitingProvider = activeCheckpoint ? canvasVideoProviderWait(activeCheckpoint) : null;
  const staleRunning = run ? runningWorkflowLooksStale(run.updated_at) : false;
  const canCancel = Boolean(activeTaskId || run?.status === "running" || run?.status === "pending");
  const canResume = Boolean(run?.can_resume && run.status !== "running" && run.status !== "pending");
  const runProgressDisplay = run?.progress
    ? progressDisplayModel({
        ...run.progress,
        locale,
        t,
        status: canvasProgressStatus(run.status),
        fallbackStage: canvasWorkflowStepLabel(currentStepKey, t),
      })
    : null;
  const resumeTitle = run?.can_resume
    ? t("video.canvas.workflow.resumeFromIncomplete")
    : run?.resume_block_reason ?? t("video.canvas.workflow.cannotResume");
  const actionItems: DropdownItem[] = [];
  if (latestWorkflowRunId) {
    actionItems.push({
      label: t("video.canvas.workflow.refresh"),
      icon: loading ? <LoadingSpinner size="sm" /> : <RefreshCw size={12} />,
      disabled: loading,
      onClick: onRefresh,
    });
  }
  if (canResume) {
    actionItems.push({
      label: t("video.canvas.workflow.resume"),
      icon: resuming ? <LoadingSpinner size="sm" /> : <Play size={12} />,
      disabled: resuming,
      onClick: onResume,
    });
  }
  if (canCancel) {
    actionItems.push({
      label: t("video.canvas.workflow.stop"),
      icon: cancelling ? <LoadingSpinner size="sm" /> : <Square size={12} />,
      danger: true,
      disabled: cancelling,
      onClick: onCancel,
    });
  }
  return (
    <section aria-label={t("video.canvas.workflow.ariaLabel")} className="mt-4 border-t border-book-border/70 pt-3 text-xs">
      <div className="mb-2 flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="text-xs font-semibold text-book-text-main">{t("video.canvas.workflow.status")}</h3>
          <p className="mt-0.5 truncate text-small text-book-text-muted" title={canResume ? resumeTitle : undefined}>
            {run
              ? checkpointNodeLabel
                ? `${canvasWorkflowStepLabel(currentStepKey, t)} · ${checkpointNodeLabel}`
                : canvasWorkflowStepLabel(currentStepKey, t)
              : statusError
                ? t("video.canvas.workflow.unavailable")
              : latestWorkflowRunId
                ? t("video.canvas.workflow.loadingLatest")
                : t("video.canvas.workflow.notRun")}
          </p>
        </div>
        {actionItems.length > 0 ? <Dropdown items={actionItems} /> : null}
      </div>
      {submittingLabel ? (
        <div className="mb-2 space-y-1.5 border-l-2 border-book-primary/50 bg-book-primary/5 px-2.5 py-2">
          <div className="inline-flex items-center gap-1.5 text-small text-book-text-main">
            <LoadingSpinner size="sm" />
            {submittingLabel}
          </div>
          {!run ? <div className="text-small text-book-text-muted">{t("video.canvas.workflow.discovering")}</div> : null}
        </div>
      ) : null}
      {statusError ? <p role="alert" className="mb-2 text-small text-error">{statusError}</p> : null}
      {!statusError && !submittingLabel && !latestWorkflowRunId && !run ? (
        <div className="text-small text-book-text-muted">{t("video.canvas.workflow.noCanvasRun")}</div>
      ) : submittingLabel && !run ? null : !detail && loading ? (
        <div className="inline-flex items-center gap-1.5 text-small text-book-text-muted">
          <LoadingSpinner size="sm" />
          {t("video.canvas.workflow.loadHistory")}
        </div>
      ) : run ? (
        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-2 text-small">
            <span className={workflowStatusTextClass(run.status)}>
              {getOperationStatusLabel(run.status, locale)}
            </span>
            <span className="min-w-0 flex-1 truncate text-book-text-sub" title={runProgressDisplay?.stage ?? ""}>
              {run.progress ? (
                <>
                  <span>{formatNumber(Math.round(runProgressDisplay?.percent ?? 0), locale)}%</span>{" "}
                  <span>{runProgressDisplay?.detail ?? runProgressDisplay?.stage}</span>
                </>
              ) : "-"}
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
          {draftChanged ? (
            <div role="status" className="rounded-xl border border-warning/50 bg-warning-bg px-2.5 py-2 text-small leading-4 text-warning">
              {t("video.canvas.workflow.draftChanged")}
            </div>
          ) : null}
          {failureMessage ? (
            <div role="alert" className="rounded-xl border border-error/50 bg-error-bg px-2.5 py-2 text-small text-error">
              {failureMessage}
            </div>
          ) : null}
          {checkpointNode ? (
            <div className={`flex flex-wrap items-center justify-between gap-2 rounded-xl border px-2.5 py-2 text-small ${activeCheckpoint ? "border-book-primary/35 bg-book-primary/5 text-book-primary" : "border-error/35 bg-error-bg/60 text-error"}`}>
              <span>{t(activeCheckpoint ? "video.canvas.workflow.activeNode" : "video.canvas.workflow.failedNode", { name: checkpointNodeLabel })}</span>
              <BookButton variant="secondary" size="sm" onClick={() => onLocateNode(checkpointNode.id)}>
                <Crosshair size={13} />
                {t("video.canvas.workflow.locateNode")}
              </BookButton>
            </div>
          ) : null}
          {waitingProvider ? (
            <div role="status" className="rounded-xl border border-warning/50 bg-warning-bg px-2.5 py-2 text-small leading-4 text-warning">
              {t("video.canvas.workflow.waitingProvider")}
              {staleRunning ? t("video.canvas.workflow.staleProvider") : ""}
            </div>
          ) : null}
          <div className="flex flex-wrap gap-x-4 gap-y-1 text-small text-book-text-muted">
            <span>
              {t("video.canvas.workflow.completed", {
                completed: formatNumber(completedCheckpointCount, locale),
                total: formatNumber(detail!.checkpoints.length, locale),
              })}
            </span>
            {detail!.generation_run_count > 0 ? (
              <span>
                {t("video.canvas.workflow.modelCalls", { count: formatNumber(detail!.generation_run_count, locale) })}
              </span>
            ) : null}
          </div>
          <details className="rounded border border-book-border/60 bg-book-bg-paper/45 text-caption">
            <summary className="cursor-pointer px-2.5 py-2 text-book-text-muted">{t("video.canvas.workflow.diagnostics")}</summary>
            <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1 border-t border-book-border/50 px-2.5 py-2">
              <span className="text-book-text-muted">{t("video.canvas.workflow.runId")}</span>
              <span className="font-mono text-book-text-sub">{run.id}</span>
              {run.active_task_id ? (
                <>
                  <span className="text-book-text-muted">{t("video.canvas.workflow.taskId")}</span>
                  <span className="break-all font-mono text-book-text-sub">{run.active_task_id}</span>
                </>
              ) : null}
              {currentStepKey ? (
                <>
                  <span className="text-book-text-muted">{t("video.canvas.workflow.stepId")}</span>
                  <span className="break-all font-mono text-book-text-sub">{currentStepKey}</span>
                </>
              ) : null}
              {waitingProvider?.generatedVideoId ? (
                <>
                  <span className="text-book-text-muted">{t("video.canvas.workflow.videoId")}</span>
                  <span className="font-mono text-book-text-sub">{waitingProvider.generatedVideoId}</span>
                </>
              ) : null}
              {waitingProvider?.providerJobId ? (
                <>
                  <span className="text-book-text-muted">{t("video.canvas.workflow.providerTask")}</span>
                  <span className="break-all font-mono text-book-text-sub">{waitingProvider.providerJobId}</span>
                </>
              ) : null}
            </div>
          </details>
        </div>
      ) : (
        <div className="text-small text-book-text-muted">{t("video.canvas.workflow.unavailable")}</div>
      )}
      {latestWorkflowRunId && !submittingLabel && detail ? (
        eventsError ? (
          <div role="alert" className="mt-2 border-y border-error/35 px-2.5 py-2 text-small text-error">{eventsError}</div>
        ) : events.length > 0 ? (
          <div className="mt-2 divide-y divide-book-border/45 border-y border-book-border/45">
            {events.slice(-CANVAS_WORKFLOW_VISIBLE_PROGRESS_EVENT_COUNT).map((event) => {
              const display = progressDisplayModel({
                ...event,
                locale,
                t,
                fallbackStage: t("video.canvas.workflow.process"),
              });
              return (
                <div key={event.id} className="grid grid-cols-[42px_minmax(0,1fr)] gap-2 px-2.5 py-1.5 text-caption">
                  <span className="font-mono text-book-text-muted">
                    {display.percent == null ? "-" : `${formatNumber(Math.round(display.percent), locale)}%`}
                  </span>
                  <span className="truncate text-book-text-sub" title={display.detail ?? display.stage}>
                    {display.detail ?? display.stage}
                  </span>
                </div>
              );
            })}
          </div>
        ) : loading ? null : (
          <div className="mt-2 text-small text-book-text-muted">{t("video.canvas.workflow.noEvents")}</div>
        )
      ) : null}
    </section>
  );
};

function workflowStatusTextClass(status: string): string {
  if (status === "running" || status === "pending") return "font-medium text-book-primary";
  if (status === "completed" || status === "success") return "font-medium text-success";
  if (status === "failed" || status === "error") return "font-medium text-error";
  return "text-book-text-muted";
}

function canvasProgressStatus(status: string): ProgressStatus {
  if (status === "completed" || status === "success") return "success";
  if (status === "failed" || status === "error") return "error";
  if (status === "cancelled" || status === "canceled") return "cancelled";
  if (status === "pending") return "idle";
  return "running";
}

function canvasWorkflowStepLabel(stepKey: string | null, t: MessageTranslator): string {
  if (!stepKey) return t("video.canvas.workflow.waitingStart");
  if (canvasNodeIdFromStepKey(stepKey)) return t("video.canvas.workflow.processingNode");
  switch (stepKey) {
    case "prepare": return t("video.canvas.workflow.prepare");
    case "validate": return t("video.canvas.workflow.validate");
    case "execute": return t("video.canvas.workflow.execute");
    case "finalize": return t("video.canvas.workflow.finalize");
    case "video_provider": return t("video.canvas.workflow.generateVideo");
    case "video_poll": return t("video.canvas.workflow.waitVideo");
    default: return t("video.canvas.workflow.process");
  }
}

function latestCheckpointWithStatus(
  checkpoints: NonNullable<WorkflowRunDetailSummary["checkpoints"]>,
  status: string,
): NonNullable<WorkflowRunDetailSummary["checkpoints"]>[number] | null {
  return checkpoints
    .filter((checkpoint) => checkpoint.status === status)
    .sort((left, right) => right.updated_at.localeCompare(left.updated_at))[0] ?? null;
}

function canvasVideoProviderWait(checkpoint: NonNullable<WorkflowRunDetailSummary["checkpoints"]>[number]): {
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
  const updatedAtMs = parseApiTimestampMs(updatedAt);
  if (updatedAtMs === null) return false;
  return Date.now() - updatedAtMs > 10 * 60 * 1000;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
