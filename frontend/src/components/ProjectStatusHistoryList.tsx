import React from "react";
import { ArrowRight } from "lucide-react";
import type { ProjectStatusHistory } from "../api/project";
import { formatUiDateTime } from "../utils/dateTime";
import { getProjectStatusText } from "../utils/projectStatus";

interface ProjectStatusHistoryListProps {
  history: ProjectStatusHistory[];
  className?: string;
}

export const ProjectStatusHistoryList: React.FC<ProjectStatusHistoryListProps> = ({
  history,
  className = "",
}) => (
  <ul className={`divide-y divide-book-border/35 ${className}`.trim()}>
    {history.map((item) => (
      <li key={item.id} className="px-3 py-2.5 text-xs">
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <span className="font-medium text-book-text-sub">
            {getProjectStatusText(item.from_status)}
          </span>
          <ArrowRight size={11} className="shrink-0 text-book-text-muted" />
          <span className="font-semibold text-book-text-main">
            {getProjectStatusText(item.to_status)}
          </span>
          <time className="ml-auto shrink-0 text-[10px] tabular-nums text-book-text-muted">
            {formatUiDateTime(item.created_at)}
          </time>
        </div>
        {item.reason ? (
          <div className="mt-1 break-words text-[11px] leading-relaxed text-book-text-muted">
            {item.reason}
          </div>
        ) : null}
      </li>
    ))}
  </ul>
);
