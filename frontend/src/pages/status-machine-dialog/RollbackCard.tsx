import React from "react";
import { Loader2, Undo2 } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import type { RollbackPreview } from "../../api/project";
import type { ProjectStatus } from "../../utils/projectStatus";
import { RollbackPreviewView } from "./RollbackPreviewView";
import { statusLabel, type StatusFlowStep } from "./statusMachineUtils";

interface RollbackCardProps {
  targets: StatusFlowStep[];
  target: ProjectStatus | null;
  preview: RollbackPreview | null;
  acting: string | null;
  onPreview: (target: ProjectStatus) => void | Promise<void>;
  onCancel: () => void;
  onConfirm: () => void | Promise<void>;
}

export const RollbackCard: React.FC<RollbackCardProps> = ({
  targets,
  target,
  preview,
  acting,
  onPreview,
  onCancel,
  onConfirm,
}) => {
  if (targets.length === 0) return null;

  const actionKey = target ? `rollback-${target}` : null;
  const actionBusy = acting !== null;

  return (
    <div className="pyqt-card p-3">
      <div className="text-xs text-book-text-muted mb-2 inline-flex items-center gap-1.5">
        <Undo2 size={12} /> 回滚到先前阶段
      </div>
      <div className="flex items-center gap-1.5 flex-wrap">
        {targets.map((step) => (
          <button
            key={step.id}
            type="button"
            onClick={() => void onPreview(step.id)}
            disabled={actionBusy}
            className={`pyqt-btn-secondary !min-h-9 !px-2.5 text-[11px] sm:!min-h-[28px] ${
              target === step.id ? "!border-book-primary !text-book-primary" : ""
            }`}
          >
            {step.label}
          </button>
        ))}
      </div>
      {target && (
        <div className="mt-3 rounded border border-warning/40 bg-warning-bg/40 p-2.5">
          <div className="text-xs text-warning mb-1.5">
            回滚到「{statusLabel(target)}」将影响以下内容：
          </div>
          {preview == null ? (
            <div className="text-xs text-book-text-muted inline-flex items-center gap-1.5">
              <Loader2 size={10} className="animate-spin" /> 预览中…
            </div>
          ) : (
            <RollbackPreviewView preview={preview} />
          )}
          <div className="flex items-center justify-end gap-2 mt-2">
            <BookButton variant="secondary" onClick={onCancel} disabled={actionBusy}>
              取消
            </BookButton>
            <button
              type="button"
              onClick={() => void onConfirm()}
              disabled={actionBusy || preview == null || !preview.can_rollback}
              className="pyqt-btn-secondary !min-h-9 !px-3 text-xs !border-error/60 !text-error hover:!bg-error-bg sm:!min-h-[32px]"
            >
              {acting === actionKey ? (
                <Loader2 size={12} className="animate-spin" />
              ) : (
                "确认回滚"
              )}
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
