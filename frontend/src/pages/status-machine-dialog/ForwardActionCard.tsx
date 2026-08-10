import React from "react";
import { ArrowRight } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import type { ProjectStatus } from "../../api/project";
import { getProjectStatusText } from "../../utils/projectStatus";

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
    <section className="a4n-context-bar min-w-0 justify-between">
      <div className="min-w-0">
        <div className="text-caption text-book-text-muted">下一阶段</div>
        <div className="truncate text-xs font-medium text-book-text-main">
          {getProjectStatusText(target)}
        </div>
      </div>
      <BookButton
        variant="primary"
        size="sm"
        onClick={() => void onForward(target)}
        disabled={acting !== null}
        className="shrink-0 !min-h-8 !text-xs"
      >
        {acting === actionKey ? (
          <span className="inline-flex items-center gap-1.5">
            <LoadingSpinner size="sm" /> 推进中…
          </span>
        ) : (
          <span className="inline-flex items-center gap-1.5">
            <ArrowRight size={14} /> 推进
          </span>
        )}
      </BookButton>
    </section>
  );
};
