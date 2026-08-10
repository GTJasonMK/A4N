import React, { useEffect, useState } from "react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { formatUiDateTime } from "../../../utils/dateTime";
import type { GalgamePipelineStatusSummary } from "../../../api/galgame";

/**
 * Galgame 生成状态面板
 *
 * 显示当前生成流水线的运行状态、步骤进度与错误信息。
 */

interface PipelineStatusPanelProps {
  status: GalgamePipelineStatusSummary | null;
  loading: boolean;
}

function pipelineStatusLabel(status: string): string {
  switch (status) {
    case "running":
      return "运行中";
    case "completed":
      return "已完成";
    case "failed":
      return "失败";
    case "cancelled":
      return "已取消";
    case "interrupted":
      return "已中断";
    case "superseded":
      return "已取代";
    default:
      return "状态未知";
  }
}

function pipelineStepLabel(stepKey: string): string {
  switch (stepKey) {
    case "shooting_script_projection":
      return "拍摄剧本投影";
    case "script_adaptation":
      return "剧本改编";
    case "character_staging":
      return "角色演出";
    case "visual_planning":
      return "视觉规划";
    case "branch_design":
      return "分支设计";
    case "persist_script":
      return "写入剧本";
    case "asset_generation":
      return "资产意图";
    default:
      return "其他步骤";
  }
}

function formatPipelineTime(value: string): string {
  return formatUiDateTime(value);
}

export const PipelineStatusPanel: React.FC<PipelineStatusPanelProps> = ({
  status,
  loading,
}) => {
  const run = status?.run ?? null;
  const [open, setOpen] = useState(
    run?.status === "running" || run?.status === "failed",
  );

  useEffect(() => {
    if (run?.status === "running" || run?.status === "failed") setOpen(true);
  }, [run?.status]);

  if (!run && !loading) return null;

  return (
    <details
      aria-label="生成状态"
      className="a4n-flat-surface overflow-hidden text-xs"
      open={open}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-3 font-medium text-book-text-main hover:bg-book-bg-tertiary/55 [&::-webkit-details-marker]:hidden">
        <span>生成状态</span>
        <span className="flex-1" />
        {loading ? <LoadingSpinner size="sm" /> : null}
        {run ? (
          <span className="text-small font-normal text-book-text-muted">
            {pipelineStatusLabel(run.status)}
          </span>
        ) : null}
      </summary>
      <div className="border-t border-book-border/40 px-3 py-3">
        {!run ? (
          <div className="text-small text-book-text-muted">正在读取状态…</div>
        ) : (
          <div className="space-y-1 text-small text-book-text-muted">
            <div className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-2">
              <span>{pipelineStatusLabel(run.status)}</span>
              <span className="min-w-0 truncate text-right">
                {formatPipelineTime(run.updated_at)}
              </span>
            </div>
            {run.current_step_key ? (
              <div className="truncate">
                步骤：{pipelineStepLabel(run.current_step_key)}
              </div>
            ) : null}
            {run.error_message ? (
              <div
                role="alert"
                className="rounded-xl border border-error/35 bg-error-bg/65 p-2.5 text-error"
              >
                {run.error_message}
              </div>
            ) : null}
            {status?.checkpoints.length ? (
              <div className="mt-2 space-y-1 border-t border-book-border pt-2">
                {status.checkpoints.map((checkpoint) => (
                  <div
                    key={checkpoint.id}
                    className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2"
                  >
                    <span className="min-w-0 truncate">
                      {pipelineStepLabel(checkpoint.step_key)}
                    </span>
                    <span>{pipelineStatusLabel(checkpoint.status)}</span>
                  </div>
                ))}
              </div>
            ) : null}
          </div>
        )}
      </div>
    </details>
  );
};
