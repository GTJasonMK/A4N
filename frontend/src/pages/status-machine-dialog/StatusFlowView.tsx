import React from "react";
import { ChevronRight } from "lucide-react";
import type { ProjectStatus } from "../../api/project";
import { PROJECT_STATUSES } from "../../api/generated/host";
import { useI18n } from "../../i18n";
import { projectStatusLabel } from "./statusLabels";

interface StatusFlowViewProps {
  currentStatus: ProjectStatus;
}

export const StatusFlowView: React.FC<StatusFlowViewProps> = ({ currentStatus }) => {
  const { t } = useI18n();
  const currentIndex = PROJECT_STATUSES.indexOf(currentStatus);

  return (
    <section className="a4n-context-bar !block">
      <div className="flex min-w-0 items-baseline gap-2">
        <span className="text-caption text-book-text-muted">{t("statusMachine.currentStage")}</span>
        <strong className="truncate text-sm font-semibold text-book-text-main">
          {projectStatusLabel(currentStatus, t)}
        </strong>
      </div>
      <p className="mt-1 text-caption text-book-text-muted">
        {t("statusMachine.stageHint")}
      </p>
      <details className="group mt-2 text-caption text-book-text-muted">
        <summary className="flex w-fit cursor-pointer list-none items-center gap-1 transition-colors hover:text-book-text-main [&::-webkit-details-marker]:hidden">
          <span>{t("statusMachine.viewFullFlow")}</span>
          <ChevronRight
            size={11}
            className="transition-transform duration-200 group-open:rotate-90"
          />
        </summary>
        <div
          className="a4n-mobile-scroll-rail custom-scrollbar mt-2 max-w-full overflow-x-auto pb-1"
          aria-label={t("statusMachine.projectStageSummary")}
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
                    {projectStatusLabel(status, t)}
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
