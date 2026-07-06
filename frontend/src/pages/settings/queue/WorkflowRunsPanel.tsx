import React from "react";
import {
  ChevronDown,
  ChevronRight,
  GitBranch,
  Loader2,
  RefreshCw,
} from "lucide-react";
import type { WorkflowRun, WorkflowRunDetail } from "../../../api/streaming";
import { BookIconButton } from "../../../components/ui/BookButton";
import { progressDisplayModel } from "../../../utils/progressPresentation";

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
  <div className="pyqt-card p-4">
    <div className="mb-2 flex items-center justify-between">
      <h4 className="inline-flex items-center gap-2 font-serif text-sm text-book-text-main">
        <GitBranch size={14} className="text-book-primary" /> 持久化工作流（
        {runs.length}）
      </h4>
      {loading && (
        <Loader2 size={12} className="animate-spin text-book-text-muted" />
      )}
    </div>
    {loading && runs.length === 0 ? (
      <div className="inline-flex items-center justify-center gap-1.5 py-4 text-xs text-book-text-muted">
        <Loader2 size={12} className="animate-spin" />
        正在加载工作流…
      </div>
    ) : runs.length === 0 ? (
      <div className="py-4 text-center text-xs text-book-text-muted">
        暂无持久化工作流记录
      </div>
    ) : (
      <ul className="max-h-72 divide-y divide-book-border/30 overflow-y-auto custom-scrollbar">
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
  </div>
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
        fallbackStage: run.current_step_key ?? run.workflow_kind,
      })
    : null;
  const canResumeLabel = run.can_resume
    ? "可恢复"
    : run.resume_block_reason
      ? "不可恢复"
      : null;
  const statusClass =
    run.status === "completed"
      ? "!border-success/50 !text-success"
      : run.status === "failed"
        ? "!border-error/50 !text-error"
        : run.status === "interrupted"
          ? "!border-warning/60 !text-warning"
          : run.status === "running"
            ? "!border-book-primary !text-book-primary"
            : "";
  const canResumeFromPanel =
    run.can_resume && PANEL_RESUMABLE_WORKFLOW_KINDS.has(run.workflow_kind);
  const resumeTitle = run.can_resume
    ? canResumeFromPanel
      ? "可在此处恢复工作流"
      : "该工作流需要在对应业务页面恢复"
    : (run.resume_block_reason ?? undefined);
  const detailText =
    display?.detail ??
    (run.can_resume && !canResumeFromPanel
      ? "请到对应业务页面恢复"
      : (run.error_message ?? run.updated_at));

  return (
    <li className="py-2 text-xs">
      <div className="flex min-w-0 items-center gap-2">
        <span className="w-14 shrink-0 font-mono text-[10px] text-book-text-muted">
          #{run.id}
        </span>
        <span className="pyqt-pill !text-[10px] shrink-0">
          {run.workflow_kind}
        </span>
        <span className={`pyqt-pill !text-[10px] shrink-0 ${statusClass}`}>
          {run.status}
        </span>
        {canResumeLabel && (
          <span
            className={`pyqt-pill !text-[10px] shrink-0 ${
              run.can_resume
                ? "!border-success/50 !text-success"
                : "!text-book-text-muted"
            }`}
            title={resumeTitle}
          >
            {canResumeLabel}
          </span>
        )}
        {run.mode && (
          <span className="pyqt-pill !text-[10px] shrink-0">{run.mode}</span>
        )}
        <span className="min-w-0 flex-1">
          <span
            className="block truncate text-book-text-sub"
            title={run.business_key}
          >
            {display?.stage ?? run.business_key}
          </span>
          <span className="block truncate text-[10px] leading-4 text-book-text-muted">
            {detailText}
          </span>
        </span>
        {display?.countLabel && (
          <span className="shrink-0 font-mono text-[10px] text-book-text-muted">
            {display.countLabel}
          </span>
        )}
        <BookIconButton
          label={expanded ? "收起详情" : "查看详情"}
          onClick={() => onToggleDetail(run)}
          variant="ghost"
          size="sm"
          disabled={detailLoading}
        >
          {detailLoading ? (
            <Loader2 size={12} className="animate-spin" />
          ) : expanded ? (
            <ChevronDown size={12} />
          ) : (
            <ChevronRight size={12} />
          )}
        </BookIconButton>
        {canResumeFromPanel && (
          <BookIconButton
            label="继续"
            onClick={() => onResume(run)}
            variant="secondary"
            size="sm"
            disabled={resuming}
          >
            {resuming ? (
              <Loader2 size={12} className="animate-spin" />
            ) : (
              <RefreshCw size={12} />
            )}
          </BookIconButton>
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
      <div className="mt-2 flex items-center gap-1.5 pl-16 text-[10px] text-book-text-muted">
        <Loader2 size={11} className="animate-spin" />
        正在加载调用详情…
      </div>
    );
  }
  if (!detail) {
    return (
      <div className="mt-2 pl-16 text-[10px] text-book-text-muted">
        暂无调用详情
      </div>
    );
  }
  return (
    <div className="mt-2 space-y-2 pl-16 text-[10px] text-book-text-muted">
      <div className="grid gap-1 sm:grid-cols-2">
        <div>
          <span className="text-book-text-sub">步骤 checkpoint</span>
          <span className="ml-1 font-mono">{detail.checkpoints.length}</span>
        </div>
        <div>
          <span className="text-book-text-sub">生成调用</span>
          <span className="ml-1 font-mono">
            {detail.generation_runs.length}
          </span>
        </div>
      </div>
      {detail.checkpoints.length > 0 && (
        <div className="space-y-1">
          {detail.checkpoints.slice(0, 6).map((checkpoint) => (
            <div
              key={checkpoint.id}
              className="flex min-w-0 items-center gap-2"
            >
              <span className="w-8 shrink-0 font-mono">
                #{checkpoint.step_order}
              </span>
              <span
                className="min-w-0 flex-1 truncate text-book-text-sub"
                title={checkpoint.step_key}
              >
                {checkpoint.step_key}
              </span>
              <span className="pyqt-pill !text-[10px] shrink-0">
                {checkpoint.status}
              </span>
            </div>
          ))}
        </div>
      )}
      {detail.generation_runs.length > 0 && (
        <div className="space-y-1">
          {detail.generation_runs.slice(0, 8).map((run) => (
            <div
              key={run.id}
              className="grid min-w-0 grid-cols-[64px_minmax(0,1fr)_auto] items-center gap-2"
            >
              <span className="font-mono">#{run.id}</span>
              <span
                className="min-w-0 truncate text-book-text-sub"
                title={run.call_key ?? run.step_key}
              >
                {run.call_key ?? run.step_key}
              </span>
              <span className="flex shrink-0 items-center gap-1">
                <span className="pyqt-pill !text-[10px]">{run.status}</span>
                <span className="font-mono">a{run.attempt}</span>
                {run.raw_response_json != null && (
                  <span title="已保存原始响应">raw</span>
                )}
                {run.parsed_output_json != null && (
                  <span title="已保存结构化输出">parsed</span>
                )}
                {run.validation_json != null && (
                  <span title="已保存校验结果">valid</span>
                )}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
