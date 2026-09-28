import React, { useEffect, useState } from "react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { GalgamePipelineStatusSummary } from "../../../api/galgame";
import { formatDateTime, useI18n, type MessageTranslator } from "../../../i18n";

/**
 * Galgame 生成状态面板
 *
 * 显示当前生成流水线的运行状态、步骤进度与错误信息。
 */

interface PipelineStatusPanelProps {
  status: GalgamePipelineStatusSummary | null;
  loading: boolean;
}

function pipelineStatusLabel(status: string, t: MessageTranslator): string {
  switch (status) {
    case "running":
      return t("workflow.runningStatus");
    case "completed":
      return t("workflow.successStatus");
    case "failed":
      return t("workflow.error");
    case "cancelled":
      return t("workflow.cancelled");
    case "interrupted":
      return t("workflow.interrupted");
    case "superseded":
      return t("workflow.superseded");
    default:
      return t("workflow.unknownStatus");
  }
}

function pipelineStepLabel(stepKey: string, t: MessageTranslator): string {
  switch (stepKey) {
    case "adaptation_plan":
      return t("galgame.startStage.adaptationPlan");
    case "shooting_script_projection":
    case "scenes":
      return t("workflow.shootingScriptProjection");
    case "script_adaptation":
    case "lines_draft":
      return t("workflow.scriptAdaptation");
    case "character_staging":
    case "lines_staged":
      return t("workflow.characterStaging");
    case "visual_planning":
    case "visual_plan":
      return t("workflow.visualPlanning");
    case "branch_design":
    case "branch_plan":
      return t("workflow.branchDesign");
    case "persist_script":
      return t("workflow.persistScript");
    case "asset_generation":
      return t("workflow.assetGeneration");
    default:
      return t("workflow.unknownStep");
  }
}

export const PipelineStatusPanel: React.FC<PipelineStatusPanelProps> = ({
  status,
  loading,
}) => {
  const { locale, t } = useI18n();
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
      aria-label={t("workflow.progress")}
      className="a4n-flat-surface overflow-hidden text-xs"
      open={open}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-3 font-medium text-book-text-main hover:bg-book-bg-tertiary/55 [&::-webkit-details-marker]:hidden">
        <span>{t("workflow.progress")}</span>
        <span className="flex-1" />
        {loading ? <LoadingSpinner size="sm" /> : null}
        {run ? (
          <span className="text-small font-normal text-book-text-muted">
            {pipelineStatusLabel(run.status, t)}
          </span>
        ) : null}
      </summary>
      <div className="border-t border-book-border/40 px-3 py-3">
        {!run ? (
          <div className="text-small text-book-text-muted">{t("common.loading")}</div>
        ) : (
          <div className="space-y-1 text-small text-book-text-muted">
            <div className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-2">
              <span>{pipelineStatusLabel(run.status, t)}</span>
              <span className="min-w-0 truncate text-right">
                {formatDateTime(run.updated_at, locale)}
              </span>
            </div>
            {run.current_step_key ? (
              <div className="truncate">
                {t("workflow.currentStep", { step: pipelineStepLabel(run.current_step_key, t) })}
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
                      {pipelineStepLabel(checkpoint.step_key, t)}
                    </span>
                    <span>{pipelineStatusLabel(checkpoint.status, t)}</span>
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
