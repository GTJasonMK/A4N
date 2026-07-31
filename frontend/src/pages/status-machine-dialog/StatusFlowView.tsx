import React from "react";
import { ChevronRight } from "lucide-react";
import { STATUS_FLOW, projectStatusIndex } from "./statusMachineUtils";

interface StatusFlowViewProps {
  currentStatus: string;
}

export const StatusFlowView: React.FC<StatusFlowViewProps> = ({ currentStatus }) => {
  const currentIndex = projectStatusIndex(currentStatus);
  const currentStep = currentIndex >= 0 ? STATUS_FLOW[currentIndex] : null;
  const completedSteps = currentIndex >= 0 ? currentIndex + 1 : 0;
  const progress = STATUS_FLOW.length > 0 ? (completedSteps / STATUS_FLOW.length) * 100 : 0;

  return (
    <section className="a4n-context-bar !block">
      <div className="flex min-w-0 items-baseline gap-2">
        <span className="text-caption text-book-text-muted">当前阶段</span>
        <strong className="truncate text-sm font-semibold text-book-text-main">
          {currentStep?.label ?? "未知阶段"}
        </strong>
        <span className="ml-auto shrink-0 text-caption tabular-nums text-book-text-muted">
          {completedSteps} / {STATUS_FLOW.length}
        </span>
      </div>
      <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-book-bg-tertiary">
        <div
          className="h-full rounded-full bg-book-primary/75 transition-[width] duration-300"
          style={{ width: `${progress}%` }}
        />
      </div>
      <details className="group mt-2 text-caption text-book-text-muted">
        <summary className="flex w-fit cursor-pointer list-none items-center gap-1 transition-colors hover:text-book-text-main [&::-webkit-details-marker]:hidden">
          <span>查看完整流程</span>
          <ChevronRight
            size={11}
            className="transition-transform duration-200 group-open:rotate-90"
          />
        </summary>
        <div
          className="a4n-mobile-scroll-rail custom-scrollbar mt-2 max-w-full overflow-x-auto pb-1"
          aria-label="完整项目阶段流程"
        >
          <ol className="flex min-w-max flex-nowrap items-baseline gap-x-1.5 leading-relaxed">
            {STATUS_FLOW.map((step, index) => {
              const isCurrent = currentIndex === index;
              const isPast = currentIndex > index;
              return (
                <React.Fragment key={step.id}>
                  <li
                    aria-current={isCurrent ? "step" : undefined}
                    className={`shrink-0 border-b py-0.5 transition-colors ${
                      isCurrent
                        ? "border-book-primary font-semibold text-book-primary"
                        : isPast
                          ? "border-transparent text-book-text-muted"
                          : "border-transparent text-book-text-sub"
                    }`}
                  >
                    {step.label}
                  </li>
                  {index < STATUS_FLOW.length - 1 ? (
                    <span aria-hidden="true" className="shrink-0 text-book-border">
                      ·
                    </span>
                  ) : null}
                </React.Fragment>
              );
            })}
          </ol>
        </div>
      </details>
    </section>
  );
};
