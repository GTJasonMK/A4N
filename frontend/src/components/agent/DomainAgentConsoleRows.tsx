import React from "react";
import { Check, CircleDot, ShieldCheck, X } from "lucide-react";
import { compactAgentValue } from "./AgentMessageRow";
import type { AgentApprovalView, AgentExecutionView } from "../../api/agent";
import { BookButton } from "../ui/BookButton";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import { useI18n, type Locale } from "../../i18n";

const RUN_STATUS_LABELS: Record<string, { zh: string; en: string }> = {
  queued: { zh: "排队中", en: "Queued" },
  running: { zh: "运行中", en: "Running" },
  waiting: { zh: "等待中", en: "Waiting" },
  pausing: { zh: "暂停中", en: "Pausing" },
  paused: { zh: "已暂停", en: "Paused" },
  cancelling: { zh: "取消中", en: "Cancelling" },
  cancelled: { zh: "已取消", en: "Cancelled" },
  completed: { zh: "已完成", en: "Completed" },
  failed: { zh: "失败", en: "Failed" },
  interrupted: { zh: "已中断", en: "Interrupted" },
  recovery_required: { zh: "需要恢复", en: "Recovery required" },
  active: { zh: "活跃", en: "Active" },
  archived: { zh: "已归档", en: "Archived" },
  pending: { zh: "待处理", en: "Pending" },
  claimed: { zh: "处理中", en: "Claimed" },
  approved: { zh: "已批准", en: "Approved" },
  rejected: { zh: "已拒绝", en: "Rejected" },
  expired: { zh: "已过期", en: "Expired" },
  consumed: { zh: "已处理", en: "Consumed" },
};

export const labelFor = (value: string | null | undefined, locale: Locale): string => {
  if (!value) return locale === "zh-CN" ? "未知" : "Unknown";
  const known = RUN_STATUS_LABELS[value];
  return known
    ? locale === "zh-CN"
      ? known.zh
      : known.en
    : value.replace(/_/g, " ");
};

function executionTone(execution: AgentExecutionView): string {
  if (execution.status === "succeeded") return "text-success";
  if (
    execution.status === "failed" ||
    execution.status === "rejected" ||
    execution.status === "outcome_unknown"
  ) {
    return "text-error";
  }
  if (execution.status === "awaiting_user") return "text-warning";
  return "text-book-primary";
}

export function ExecutionRow({
  execution,
  locale,
  onSelectThread,
}: {
  execution: AgentExecutionView;
  locale: Locale;
  onSelectThread?: (threadId: string) => void;
}) {
  return (
    <div className="min-w-0 rounded-lg border border-book-border/35 bg-book-bg-paper px-2 py-1.5">
      <div className="flex min-w-0 items-center gap-1.5 text-[10px]">
        <CircleDot size={10} className={executionTone(execution)} />
        <span className="min-w-0 flex-1 truncate text-book-text-sub">
          {execution.tool_name || execution.kind}
        </span>
        <span className="shrink-0 text-book-text-muted">
          {labelFor(execution.status, locale)}
        </span>
      </div>
      {execution.result_refs.filter((ref) => ref.kind === "agent_thread" && ref.id).map((ref) => (
        <button
          key={ref.id}
          type="button"
          className="mt-1 max-w-full truncate text-left font-mono text-[10px] text-book-primary hover:underline"
          onClick={() => onSelectThread?.(ref.id)}
        >
          {locale === "zh-CN" ? "打开子线程" : "Open child thread"} · #{ref.id}
        </button>
      ))}
      {execution.result_summary ? (
        <p className="mt-0.5 break-words text-[10px] leading-relaxed text-book-text-muted">
          {compactAgentValue(execution.result_summary)}
        </p>
      ) : null}
      {execution.error ? (
        <p className="mt-0.5 break-words text-[10px] leading-relaxed text-error">
          {execution.error}
        </p>
      ) : null}
    </div>
  );
}

export function ApprovalRow({
  approval,
  locale,
  busy,
  onResolve,
}: {
  approval: AgentApprovalView;
  locale: Locale;
  busy: boolean;
  onResolve: (
    approval: AgentApprovalView,
    decision: "approve" | "reject",
  ) => void;
}) {
  return (
    <div data-approval-id={approval.id} className="min-w-0 rounded-xl border border-warning/35 bg-warning-bg/55 p-2">
      <div className="flex min-w-0 items-start gap-1.5">
        <ShieldCheck size={13} className="mt-0.5 shrink-0 text-warning" />
        <div className="min-w-0 flex-1">
          <p className="break-words text-xs font-medium text-book-text-main">
            {approval.command}
          </p>
          {approval.reason ? (
            <p className="mt-0.5 break-words text-[10px] leading-relaxed text-book-text-muted">
              {approval.reason}
            </p>
          ) : null}
        </div>
      </div>
      <div className="mt-1.5 flex gap-1.5">
        <BookButton
          size="sm"
          className="min-w-0 flex-1"
          onClick={() => onResolve(approval, "approve")}
          disabled={busy}
        >
          <Check size={12} />
          {locale === "zh-CN" ? "批准" : "Approve"}
        </BookButton>
        <BookButton
          size="sm"
          variant="danger"
          className="min-w-0 flex-1"
          onClick={() => onResolve(approval, "reject")}
          disabled={busy}
        >
          <X size={12} />
          {locale === "zh-CN" ? "拒绝" : "Reject"}
        </BookButton>
        {busy ? <LoadingSpinner size="sm" className="self-center" /> : null}
      </div>
    </div>
  );
}

export function Notice({
  children,
  onRetry,
}: {
  children: React.ReactNode;
  onRetry?: () => void;
}) {
  const { t } = useI18n();
  return (
    <div
      role="alert"
      className="flex min-w-0 shrink-0 items-start gap-2 border-b border-error/30 bg-error-bg px-3 py-2 text-[10px] leading-relaxed text-error"
    >
      <span className="min-w-0 flex-1 break-words">{children}</span>
      {onRetry ? (
        <BookButton size="sm" variant="secondary" onClick={onRetry}>
          {t("common.retry")}
        </BookButton>
      ) : null}
    </div>
  );
}
