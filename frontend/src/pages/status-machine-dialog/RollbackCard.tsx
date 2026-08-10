import React from "react";
import { ChevronRight, Undo2 } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import type { ProjectRollbackPreview, ProjectStatus } from "../../api/project";
import { RollbackPreviewView } from "./RollbackPreviewView";
import { getProjectStatusText } from "../../utils/projectStatus";

interface RollbackCardProps {
  targets: ProjectStatus[];
  target: ProjectStatus | null;
  preview: ProjectRollbackPreview | null;
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
    <details
      className="group border-t border-book-border/35 pt-2"
      onToggle={(event) => {
        if (!(event.currentTarget as HTMLDetailsElement).open && target) onCancel();
      }}
    >
      <summary className="flex cursor-pointer list-none items-center gap-2 py-1 text-xs text-book-text-sub [&::-webkit-details-marker]:hidden">
        <Undo2 size={12} className="shrink-0 text-book-text-muted" />
        <span className="font-medium">回滚到先前阶段</span>
        <span className="text-caption text-book-text-muted">{targets.length} 个可选目标</span>
        <span className="ml-auto text-caption text-warning">谨慎操作</span>
        <ChevronRight
          size={11}
          className="shrink-0 text-book-text-muted transition-transform duration-200 group-open:rotate-90"
        />
      </summary>
      <div className="mt-2 space-y-2 sm:pl-5">
        <label className="flex min-w-0 flex-col gap-1 text-caption text-book-text-muted">
          目标阶段
          <select
            value={target ?? ""}
            onChange={(event) => {
              const value = event.target.value as ProjectStatus;
              if (value) void onPreview(value);
              else onCancel();
            }}
            disabled={actionBusy}
            className="pyqt-input !px-2 !py-1 text-xs"
          >
            <option value="">选择要恢复的阶段</option>
            {targets.map((status) => (
              <option key={status} value={status}>
                {getProjectStatusText(status)}
              </option>
            ))}
          </select>
        </label>
        {target ? (
          <div className="border-l-2 border-warning/55 py-1 pl-3">
            <div className="mb-1.5 text-xs text-warning">
              回滚到「{getProjectStatusText(target)}」将影响以下内容：
            </div>
            {preview == null ? (
              <div className="inline-flex items-center gap-1.5 text-xs text-book-text-muted">
                <LoadingSpinner size="sm" /> 正在检查影响范围…
              </div>
            ) : (
              <RollbackPreviewView preview={preview} />
            )}
            {preview ? (
              <div className="mt-2 flex justify-end">
                <BookButton
                  variant="danger"
                  size="sm"
                  onClick={() => void onConfirm()}
                  disabled={actionBusy}
                  className="!min-h-8 !text-xs"
                >
                  {acting === actionKey ? (
                    <span className="inline-flex items-center gap-1.5">
                      <LoadingSpinner size="sm" /> 回滚中…
                    </span>
                  ) : (
                    "确认回滚"
                  )}
                </BookButton>
              </div>
            ) : null}
          </div>
        ) : null}
      </div>
    </details>
  );
};
