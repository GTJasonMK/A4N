import React from "react";
import { ArrowRight } from "lucide-react";
import { STATUS_FLOW, projectStatusIndex } from "./statusMachineUtils";

interface StatusFlowViewProps {
  currentStatus: string;
}

export const StatusFlowView: React.FC<StatusFlowViewProps> = ({ currentStatus }) => {
  const currentIndex = projectStatusIndex(currentStatus);

  return (
    <div>
      <div className="text-xs text-book-text-muted mb-2">当前状态</div>
      <div className="flex items-center gap-1.5 flex-wrap">
        {STATUS_FLOW.map((step, index) => {
          const isCurrent = currentIndex === index;
          const isPast = currentIndex > index;

          return (
            <React.Fragment key={step.id}>
              <span
                className={`pyqt-pill !text-[11px] ${
                  isCurrent
                    ? "!bg-book-primary !text-white !border-book-primary"
                    : isPast
                      ? "!text-book-text-muted"
                      : "!text-book-text-sub"
                }`}
              >
                {step.label}
              </span>
              {index < STATUS_FLOW.length - 1 && (
                <ArrowRight
                  size={11}
                  className={isPast ? "text-book-text-muted" : "text-book-border"}
                />
              )}
            </React.Fragment>
          );
        })}
      </div>
    </div>
  );
};
