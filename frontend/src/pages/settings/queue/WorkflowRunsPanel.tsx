import React from "react";
import {
  ChevronDown,
  ChevronRight,
  Loader2,
  RefreshCw,
} from "lucide-react";
import type { WorkflowRun, WorkflowRunDetail } from "../../../api/streaming";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import { progressDisplayModel } from "../../../utils/progressPresentation";
import {
  getWorkflowKindLabel,
  getWorkflowModeLabel,
  getWorkflowStatusLabel,
} from "../../../utils/workflowPresentation";

const PANEL_RESUMABLE_WORKFLOW_KINDS = new Set([
  "video_provider_job",
  "chapter_analysis",
]);

interface WorkflowRunsPanelProps {
  runs: WorkflowRun[];
  loading: boolean;
  resumingRunIds: ReadonlySet<number>;
  expandedRunIds: ReadonlySet<number>;
  loadingDetailRunIds: ReadonlySet<number>;
  details: ReadonlyMap<number, WorkflowRunDetail>;
  onResume: (run: WorkflowRun) => void;
  onToggleDetail: (run: WorkflowRun) => void;
}

export const WorkflowRunsPanel: React.FC<WorkflowRunsPanelProps> = ({
  runs,
  loading,
  resumingRunIds,
  expandedRunIds,
  loadingDetailRunIds,
  details,
  onResume,
  onToggleDetail,
}) => (
  <section>
    <div className="flex items-baseline gap-2 px-1 pb-2">
      <h3 className="text-xs font-semibold text-book-text-main">持久化工作流</h3>
      <span className="text-[11px] text-book-text-muted">
        {runs.length > 0 ? `最近 ${runs.length} 条` : "可恢复的运行记录"}
      </span>
      {loading ? <Loader2 size={12} className="ml-auto animate-spin text-book-text-muted" /> : null}
    </div>
    {loading && runs.length === 0 ? (
      <div className="a4n-flat-surface flex items-center justify-center gap-1.5 px-3 py-5 text-xs text-book-text-muted">
        <Loader2 size={12} className="animate-spin" />
        正在加载工作流…
      </div>
    ) : runs.length === 0 ? (
      <div className="a4n-empty-state a4n-empty-state--framed px-3 py-5 text-center text-xs text-book-text-muted">
        暂无持久化工作流记录
      </div>
    ) : (
      <ul className="space-y-2.5">
        {runs.map((run) => (
          <WorkflowRunRow
            key={run.id}
            run={run}
            resuming={resumingRunIds.has(run.id)}
            expanded={expandedRunIds.has(run.id)}
            detailLoading={loadingDetailRunIds.has(run.id)}
            detail={details.get(run.id)}
            onResume={onResume}
            onToggleDetail={onToggleDetail}
          />
        ))}
      </ul>
    )}
  </section>
);

const WorkflowRunRow: React.FC<{
  run: WorkflowRun;
  resuming: boolean;
  expanded: boolean;
  detailLoading: boolean;
  detail?: WorkflowRunDetail;
  onResume: (run: WorkflowRun) => void;
  onToggleDetail: (run: WorkflowRun) => void;
}> = ({
  run,
  resuming,
  expanded,
  detailLoading,
  detail,
  onResume,
  onToggleDetail,
}) => {
  const display = run.progress
    ? progressDisplayModel({
        ...run.progress,
        status:
          run.status === "completed"
            ? "success"
            : run.status === "failed"
              ? "error"
              : run.status === "cancelled"
                ? "cancelled"
                : "running",
        fallbackStage: getWorkflowKindLabel(run.workflow_kind),
      })
    : null;
  const kindLabel = getWorkflowKindLabel(run.workflow_kind);
  const displayStage = readableWorkflowText(display?.stage, kindLabel);
  const canResumeLabel = run.can_resume
    ? "可恢复"
    : run.resume_block_reason
      ? "不可恢复"
      : null;
  const canResumeFromPanel =
    run.can_resume && PANEL_RESUMABLE_WORKFLOW_KINDS.has(run.workflow_kind);
  const resumeTitle = run.can_resume
    ? canResumeFromPanel
      ? "可在此处恢复工作流"
      : "该工作流需要在对应业务页面恢复"
    : (run.resume_block_reason ?? undefined);
  const fallbackDetail =
    (run.can_resume && !canResumeFromPanel
      ? "请到对应业务页面恢复"
      : (run.error_message ?? run.updated_at));
  const detailText = readableWorkflowText(display?.detail, fallbackDetail);
  const menuItems: DropdownItem[] = [
    {
      label: expanded ? "收起运行详情" : "查看运行详情",
      icon: expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />,
      onClick: () => onToggleDetail(run),
    },
  ];
  if (canResumeFromPanel && !resuming) {
    menuItems.push({ type: "divider" });
    menuItems.push({
      label: "继续运行",
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
          <div className="mt-0.5 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-[11px] text-book-text-muted">
            <span className={runStatusTone(run.status)}>{getWorkflowStatusLabel(run.status)}</span>
            {run.mode ? (
              <>
                <span aria-hidden="true">·</span>
                <span>{getWorkflowModeLabel(run.mode)}</span>
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
          <div className="mt-1.5 truncate text-[11px] text-book-text-sub">
            {displayStage}
          </div>
          <div className="truncate text-[11px] leading-4 text-book-text-muted" title={detailText}>
            {formatWorkflowDetail(detailText)}
          </div>
        </div>
        {detailLoading || resuming ? (
          <span className="inline-flex h-8 w-8 shrink-0 items-center justify-center text-book-text-muted">
            <Loader2 size={13} className="animate-spin" />
          </span>
        ) : (
          <Dropdown items={menuItems} />
        )}
      </div>
      {expanded && (
        <WorkflowRunDetailBlock detail={detail} loading={detailLoading} />
      )}
    </li>
  );
};

const WorkflowRunDetailBlock: React.FC<{
  detail?: WorkflowRunDetail;
  loading: boolean;
}> = ({ detail, loading }) => {
  if (loading && !detail) {
    return (
      <div className="mt-3 flex items-center gap-1.5 border-t border-book-border/30 pt-3 text-[11px] text-book-text-muted">
        <Loader2 size={11} className="animate-spin" />
        正在加载调用详情…
      </div>
    );
  }
  if (!detail) {
    return (
      <div className="mt-3 border-t border-book-border/30 pt-3 text-[11px] text-book-text-muted">
        暂无调用详情
      </div>
    );
  }
  return (
    <div className="mt-3 space-y-3 border-t border-book-border/30 pt-3 text-[11px] text-book-text-muted">
      <div className="flex flex-wrap items-center gap-x-1.5 gap-y-1">
        <span className="text-book-text-sub">{detail.checkpoints.length} 个检查点</span>
        <span aria-hidden="true">·</span>
        <span className="text-book-text-sub">{detail.generation_runs.length} 次生成调用</span>
      </div>
      {detail.checkpoints.length > 0 && (
        <div>
          <div className="mb-1.5 font-medium text-book-text-sub">运行进度</div>
          <ol className="divide-y divide-book-border/25 rounded-xl bg-book-bg-tertiary/35 px-2.5">
            {detail.checkpoints.slice(0, 6).map((checkpoint) => (
              <li
                key={checkpoint.id}
                className="flex min-w-0 items-center gap-2 py-2"
              >
                <span className="min-w-0 flex-1 text-book-text-sub">
                  第 {checkpoint.step_order} 步
                </span>
                <span className={runStatusTone(checkpoint.status)}>
                  {getWorkflowStatusLabel(checkpoint.status)}
                </span>
              </li>
            ))}
          </ol>
        </div>
      )}
      {detail.generation_runs.length > 0 && (
        <div>
          <div className="mb-1.5 font-medium text-book-text-sub">生成调用</div>
          <ol className="divide-y divide-book-border/25 rounded-xl bg-book-bg-tertiary/35 px-2.5">
            {detail.generation_runs.slice(0, 8).map((run, index) => (
              <li
                key={run.id}
                className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 py-2"
              >
                <span className="text-book-text-muted">{index + 1}</span>
                <span className="min-w-0 truncate text-book-text-sub">
                  {run.title || "生成请求"}
                </span>
                <span className={runStatusTone(run.status)}>
                  {getWorkflowStatusLabel(run.status)}
                  {run.attempt > 1 ? ` · 第 ${run.attempt} 次` : ""}
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

const WorkflowRunDiagnostics: React.FC<{ detail: WorkflowRunDetail }> = ({ detail }) => (
  <details className="text-[10px] text-book-text-muted">
    <summary className="w-fit cursor-pointer select-none py-1 hover:text-book-text-main">
      诊断信息
    </summary>
    <div className="mt-1 space-y-2 rounded-xl bg-book-bg-tertiary/45 p-2.5 font-mono">
      <div className="grid grid-cols-[72px_minmax(0,1fr)] gap-x-2 gap-y-1">
        <span>运行编号</span>
        <span className="break-all text-book-text-sub">{detail.run.id}</span>
        <span>业务键</span>
        <span className="break-all text-book-text-sub">{detail.run.business_key}</span>
        <span>工作流类型</span>
        <span className="break-all text-book-text-sub">{detail.run.workflow_kind}</span>
        {detail.run.current_step_key ? (
          <>
            <span>当前步骤</span>
            <span className="break-all text-book-text-sub">{detail.run.current_step_key}</span>
          </>
        ) : null}
      </div>
      {detail.checkpoints.length > 0 ? (
        <div className="border-t border-book-border/30 pt-2">
          {detail.checkpoints.slice(0, 6).map((checkpoint) => (
            <div key={checkpoint.id} className="break-all">
              checkpoint {checkpoint.id} · {checkpoint.step_key} · {checkpoint.status}
            </div>
          ))}
        </div>
      ) : null}
      {detail.generation_runs.length > 0 ? (
        <div className="border-t border-book-border/30 pt-2">
          {detail.generation_runs.slice(0, 8).map((run) => (
            <div key={run.id} className="break-all">
              generation {run.id} · {run.call_key ?? run.step_key} · {run.status}
              {run.raw_response_json != null ? " · raw" : ""}
              {run.parsed_output_json != null ? " · parsed" : ""}
              {run.validation_json != null ? " · validated" : ""}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  </details>
);

function runStatusTone(status: string): string {
  if (["completed", "success"].includes(status)) return "text-success";
  if (["failed", "error"].includes(status)) return "text-error";
  if (["interrupted", "paused", "blocked"].includes(status)) return "text-warning";
  if (status === "running") return "text-book-primary";
  return "text-book-text-muted";
}

function formatWorkflowDetail(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

function readableWorkflowText(value: string | null | undefined, fallback: string): string {
  const normalized = value?.trim();
  if (!normalized || looksLikeInternalIdentifier(normalized)) return fallback;
  return normalized;
}

function looksLikeInternalIdentifier(value: string): boolean {
  return /^[a-z][a-z0-9]*(?:[_:-][a-z0-9]+)+$/i.test(value);
}
