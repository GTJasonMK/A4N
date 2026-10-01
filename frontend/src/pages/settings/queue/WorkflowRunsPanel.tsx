import React, { useEffect, useState } from "react";
import { ChevronDown,
  ChevronRight,
  RefreshCw, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type {
  WorkflowRunDetailSummary,
  WorkflowRunSummary,
} from "../../../api/streaming";
import { getWorkflowRunDetail } from "../../../api/streaming";
import { extractErrorMessage } from "../../../api";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { progressDisplayModel } from "../../../utils/progressPresentation";
import {
  getWorkflowKindLabel,
  getWorkflowModeLabel,
  getOperationStatusLabel,
} from "../../../utils/workflowPresentation";
import { formatDateTime, useI18n } from "../../../i18n";

const PANEL_RESUMABLE_WORKFLOW_KINDS = new Set([
  "video_provider_job",
  "chapter_analysis",
  "blueprint_generation",
  "blueprint_refine",
]);

interface WorkflowRunsPanelProps {
  runs: WorkflowRunSummary[];
  loading: boolean;
  resumingRunIds: ReadonlySet<number>;
  onResume: (run: WorkflowRunSummary) => void;
}

export const WorkflowRunsPanel: React.FC<WorkflowRunsPanelProps> = ({
  runs,
  loading,
  resumingRunIds,
  onResume,
}) => {
  const { t } = useI18n();
  return (
  <section>
    <div className="flex items-baseline gap-2 px-1 pb-2">
      <h3 className="text-xs font-semibold text-book-text-main">{t("settingsQueue.persistentWorkflows")}</h3>
      <span className="text-small text-book-text-muted">
        {runs.length > 0 ? t("settingsQueue.recentRuns", { count: runs.length }) : t("settingsQueue.resumableRuns")}
      </span>
      {loading ? <LoadingSpinner size="sm" className="ml-auto  text-book-text-muted" /> : null}
    </div>
    {loading && runs.length === 0 ? (
      <div className="a4n-flat-surface flex items-center justify-center gap-1.5 px-3 py-5 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" />
        {t("settingsQueue.loadingWorkflows")}
      </div>
    ) : runs.length === 0 ? (
      <div className="a4n-empty-state a4n-empty-state--framed px-3 py-5 text-center text-xs text-book-text-muted">
        {t("settingsQueue.noPersistentWorkflows")}
      </div>
    ) : (
      <ul className="space-y-2.5">
        {runs.map((run) => (
          <WorkflowRunRow
            key={run.id}
            run={run}
            resuming={resumingRunIds.has(run.id)}
            onResume={onResume}
          />
        ))}
      </ul>
    )}
  </section>
  );
};

const WorkflowRunRow: React.FC<{
  run: WorkflowRunSummary;
  resuming: boolean;
  onResume: (run: WorkflowRunSummary) => void;
}> = ({
  run,
  resuming,
  onResume,
}) => {
  const { t, locale } = useI18n();
  const [expanded, setExpanded] = useState(false);
  const [detail, setDetail] = useState<WorkflowRunDetailSummary>();
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<{ reason: unknown } | null>(null);

  useEffect(() => {
    if (!expanded) return;
    let current = true;
    setDetail(undefined);
    setDetailError(null);
    setDetailLoading(true);
    void getWorkflowRunDetail(run.id, true).then((next) => {
      if (current) setDetail(next);
    }).catch((error: unknown) => {
      if (current) setDetailError({ reason: error });
    }).finally(() => {
      if (current) setDetailLoading(false);
    });
    return () => { current = false; };
  }, [expanded, run.id, run.current_step_key, run.status]);
  const display = run.progress
      ? progressDisplayModel({
        ...run.progress,
        locale,
        t,
        status:
          run.status === "completed"
            ? "success"
            : run.status === "failed"
              ? "error"
              : run.status === "cancelled"
                ? "cancelled"
                : "running",
        fallbackStage: getWorkflowKindLabel(run.workflow_kind, locale),
      })
    : null;
  const kindLabel = getWorkflowKindLabel(run.workflow_kind, locale);
  const displayStage = readableWorkflowText(display?.stage, kindLabel);
  const canResumeLabel = run.can_resume
    ? t("settingsQueue.resumable")
    : run.resume_block_reason
      ? t("settingsQueue.notResumable")
      : null;
  const canResumeFromPanel =
    run.can_resume && PANEL_RESUMABLE_WORKFLOW_KINDS.has(run.workflow_kind);
  const resumeTitle = run.can_resume
    ? canResumeFromPanel
      ? t("settingsQueue.resumeHere")
      : t("settingsQueue.resumeOnBusinessPage")
    : (run.resume_block_reason ?? undefined);
  const fallbackDetail =
    (run.can_resume && !canResumeFromPanel
      ? t("settingsQueue.goToBusinessPage")
      : (run.error_message ?? run.updated_at));
  const detailText = readableWorkflowText(display?.detail, fallbackDetail);
  const menuItems: DropdownItem[] = [
    {
      label: expanded ? t("settingsQueue.collapseDetails") : t("settingsQueue.viewDetails"),
      icon: expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />,
      onClick: () => setExpanded((value) => !value),
    },
  ];
  if (canResumeFromPanel && !resuming) {
    menuItems.push({ type: "divider" });
    menuItems.push({
      label: t("settingsQueue.continueRun"),
      icon: <RefreshCw size={13} />,
      onClick: () => onResume(run),
    });
  }

  return (
    <li className="a4n-flat-surface px-3 py-3 text-xs">
      <div className="flex min-w-0 items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="truncate font-medium text-book-text-main">
            {kindLabel}
          </div>
          <div className="mt-0.5 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-small text-book-text-muted">
            <span className={runStatusTone(run.status)}>{getOperationStatusLabel(run.status, locale)}</span>
            {run.mode ? (
              <>
                <span aria-hidden="true">·</span>
                <span>{getWorkflowModeLabel(run.mode, locale)}</span>
              </>
            ) : null}
            {canResumeLabel ? (
              <>
                <span aria-hidden="true">·</span>
                <span className={run.can_resume ? "text-success" : undefined} title={resumeTitle}>
                  {canResumeLabel}
                </span>
              </>
            ) : null}
            {display?.countLabel ? (
              <>
                <span aria-hidden="true">·</span>
                <span>{display.countLabel}</span>
              </>
            ) : null}
          </div>
          <div className="mt-1.5 truncate text-small text-book-text-sub">
            {displayStage}
          </div>
          <div className="truncate text-small leading-4 text-book-text-muted" title={detailText}>
            {formatDateTime(detailText, locale, detailText)}
          </div>
        </div>
        {resuming ? (
          <span className="inline-flex h-8 w-8 shrink-0 items-center justify-center text-book-text-muted">
            <LoadingSpinner size="sm" />
          </span>
        ) : (
          <Dropdown items={menuItems} />
        )}
      </div>
      {expanded && (detailError
        ? <p role="alert" className="mt-3 text-error">
            {extractErrorMessage(detailError.reason, t("settingsQueue.loadWorkflowFailed"))}
          </p>
        : <WorkflowRunDetailBlock detail={detail} loading={detailLoading} />)}
    </li>
  );
};

const WorkflowRunDetailBlock: React.FC<{
  detail?: WorkflowRunDetailSummary;
  loading: boolean;
}> = ({ detail, loading }) => {
  const { t, locale } = useI18n();
  if (loading && !detail) {
    return (
      <div className="mt-3 flex items-center gap-1.5 border-t border-book-border/30 pt-3 text-small text-book-text-muted">
        <LoadingSpinner size="sm" />
        {t("settingsQueue.loadingCallDetails")}
      </div>
    );
  }
  if (!detail) {
    return (
      <div className="mt-3 border-t border-book-border/30 pt-3 text-small text-book-text-muted">
        {t("settingsQueue.noCallDetails")}
      </div>
    );
  }
  return (
    <div className="mt-3 space-y-3 border-t border-book-border/30 pt-3 text-small text-book-text-muted">
      <div className="flex flex-wrap items-center gap-x-1.5 gap-y-1">
        <span className="text-book-text-sub">{t("settingsQueue.checkpoints", { count: detail.checkpoints.length })}</span>
        <span aria-hidden="true">·</span>
        <span className="text-book-text-sub">{t("settingsQueue.generationCalls", { count: detail.generation_runs.length })}</span>
      </div>
      {detail.checkpoints.length > 0 && (
        <div>
          <div className="mb-1.5 font-medium text-book-text-sub">{t("settingsQueue.runProgress")}</div>
          <ol className="divide-y divide-book-border/25 rounded-xl bg-book-bg-tertiary/35 px-2.5">
            {detail.checkpoints.map((checkpoint) => (
              <li
                key={checkpoint.id}
                className="flex min-w-0 items-center gap-2 py-2"
              >
                <span className="min-w-0 flex-1 text-book-text-sub">
                  {t("settingsQueue.step", { count: checkpoint.step_order })}
                </span>
                <span className={runStatusTone(checkpoint.status)}>
                  {getOperationStatusLabel(checkpoint.status, locale)}
                </span>
              </li>
            ))}
          </ol>
        </div>
      )}
      {detail.generation_runs.length > 0 && (
        <div>
          <div className="mb-1.5 font-medium text-book-text-sub">{t("settingsQueue.generationCall")}</div>
          <ol className="divide-y divide-book-border/25 rounded-xl bg-book-bg-tertiary/35 px-2.5">
            {detail.generation_runs.map((run, index) => (
              <li
                key={run.id}
                className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 py-2"
              >
                <span className="text-book-text-muted">{index + 1}</span>
                <span className="min-w-0 truncate text-book-text-sub">
                  {run.title || t("settingsQueue.generationRequest")}
                </span>
                <span className={runStatusTone(run.status)}>
                  {getOperationStatusLabel(run.status, locale)}
                  {run.attempt > 1 ? ` · ${t("settingsQueue.attempt", { count: run.attempt })}` : ""}
                </span>
              </li>
            ))}
          </ol>
        </div>
      )}
      <WorkflowRunDiagnostics detail={detail} />
    </div>
  );
};

const WorkflowRunDiagnostics: React.FC<{ detail: WorkflowRunDetailSummary }> = ({ detail }) => {
  const { t } = useI18n();
  return (
  <details className="text-caption text-book-text-muted">
    <summary className="w-fit cursor-pointer select-none py-1 hover:text-book-text-main">
      {t("settingsQueue.diagnostic")}
    </summary>
    <div className="mt-1 space-y-2 rounded-xl bg-book-bg-tertiary/45 p-2.5 font-mono">
      <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1">
        <span>{t("settingsQueue.runNumber")}</span>
        <span className="break-all text-book-text-sub">{detail.run.id}</span>
        <span>{t("settingsQueue.businessKey")}</span>
        <span className="break-all text-book-text-sub">{detail.run.business_key}</span>
        <span>{t("settingsQueue.workflowKind")}</span>
        <span className="break-all text-book-text-sub">{detail.run.workflow_kind}</span>
        {detail.run.current_step_key ? (
          <>
            <span>{t("settingsQueue.currentStep")}</span>
            <span className="break-all text-book-text-sub">{detail.run.current_step_key}</span>
          </>
        ) : null}
      </div>
      {detail.checkpoints.length > 0 ? (
        <div className="border-t border-book-border/30 pt-2">
          {detail.checkpoints.map((checkpoint) => (
            <div key={checkpoint.id} className="break-all">
              {t("settingsQueue.checkpoint", { id: checkpoint.id })} · {checkpoint.step_key} · {checkpoint.status}
            </div>
          ))}
        </div>
      ) : null}
      {detail.generation_runs.length > 0 ? (
        <div className="border-t border-book-border/30 pt-2">
          {detail.generation_runs.map((run) => (
            <div key={run.id} className="break-all">
              {t("settingsQueue.generation", { id: run.id })} · {run.call_key ?? run.step_key} · {run.status}
              {run.raw_response_json != null ? ` · ${t("settingsQueue.raw")}` : ""}
              {run.parsed_output_json != null ? ` · ${t("settingsQueue.parsed")}` : ""}
              {run.validation_json != null ? ` · ${t("settingsQueue.validated")}` : ""}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  </details>
  );
};

function runStatusTone(status: string): string {
  if (["completed", "success"].includes(status)) return "text-success";
  if (["failed", "error"].includes(status)) return "text-error";
  if (["interrupted", "paused", "blocked"].includes(status)) return "text-warning";
  if (status === "running") return "text-book-primary";
  return "text-book-text-muted";
}

function readableWorkflowText(value: string | null | undefined, fallback: string): string {
  const normalized = value?.trim();
  if (!normalized || looksLikeInternalIdentifier(normalized)) return fallback;
  return normalized;
}

function looksLikeInternalIdentifier(value: string): boolean {
  return /^[a-z][a-z0-9]*(?:[_:-][a-z0-9]+)+$/i.test(value);
}
