import React from "react";
import { ArrowRight, Loader2 } from "lucide-react";
import { BookButton } from "../../components/ui/BookButton";
import { statusLabel } from "./statusMachineUtils";
import type { ProjectStatus } from "../../utils/projectStatus";

interface ForwardActionCardProps {
  target: ProjectStatus;
  acting: string | null;
  onForward: (target: ProjectStatus) => void | Promise<void>;
}

export const ForwardActionCard: React.FC<ForwardActionCardProps> = ({
  target,
  acting,
  onForward,
}) => {
  const actionKey = `forward-${target}`;

  return (
    <div className="pyqt-card p-3">
      <div className="text-xs text-book-text-muted mb-2">下一阶段</div>
      <div className="flex items-center justify-between gap-2">
        <div className="text-sm text-book-text-main">推进到「{statusLabel(target)}」</div>
        <BookButton
          variant="primary"
          onClick={() => void onForward(target)}
          disabled={acting !== null}
        >
          {acting === actionKey ? (
            <Loader2 size={14} className="animate-spin" />
          ) : (
            <span className="inline-flex items-center gap-1.5">
              <ArrowRight size={14} /> 推进
            </span>
          )}
        </BookButton>
      </div>
    </div>
  );
};
