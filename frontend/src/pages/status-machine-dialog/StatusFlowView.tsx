import React from "react";
import { ChevronRight } from "lucide-react";
import type { ProjectStatus } from "../../api/project";
import { PROJECT_STATUSES, getProjectStatusText } from "../../utils/projectStatus";

interface StatusFlowViewProps {
  currentStatus: ProjectStatus;
}

export const StatusFlowView: React.FC<StatusFlowViewProps> = ({ currentStatus }) => {
  const currentIndex = PROJECT_STATUSES.indexOf(currentStatus);

  return (
    <section className="a4n-context-bar !block">
      <div className="flex min-w-0 items-baseline gap-2">
        <span className="text-caption text-book-text-muted">当前阶段</span>
        <strong className="truncate text-sm font-semibold text-book-text-main">
          {getProjectStatusText(currentStatus)}
        </strong>
      </div>
      <p className="mt-1 text-caption text-book-text-muted">
        阶段只概括当前已有产物；自由创作可以跳过未建立的蓝图或大纲。
      </p>
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
          aria-label="项目阶段摘要选项"
        >
          <ol className="flex min-w-max flex-nowrap items-baseline gap-x-1.5 leading-relaxed">
            {PROJECT_STATUSES.map((status, index) => {
              const isCurrent = currentIndex === index;
              return (
                <React.Fragment key={status}>
                  <li
                    aria-current={isCurrent ? "step" : undefined}
                    className={`shrink-0 border-b py-0.5 transition-colors ${
                      isCurrent
                        ? "border-book-primary font-semibold text-book-primary"
                        : "border-transparent text-book-text-sub"
                    }`}
                  >
                    {getProjectStatusText(status)}
                  </li>
                  {index < PROJECT_STATUSES.length - 1 ? (
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
