import React from "react";
import { ArrowRight, ChevronRight, History } from "lucide-react";
import type { ProjectStatusHistorySummary } from "../../api/project";
import { formatUiDateTime } from "../../utils/dateTime";
import { getProjectStatusText } from "../../utils/projectStatus";

interface StatusHistoryPanelProps {
  history: ProjectStatusHistorySummary[];
}

export const StatusHistoryPanel: React.FC<StatusHistoryPanelProps> = ({ history }) => {
  return (
    <details className="group border-t border-book-border/35 pt-2">
      <summary className="flex cursor-pointer list-none items-center gap-2 py-1 text-xs text-book-text-sub [&::-webkit-details-marker]:hidden">
        <History size={12} className="shrink-0 text-book-text-muted" />
        <span className="font-medium">状态变更历史</span>
        <span className="text-caption text-book-text-muted">
          {history.length > 0 ? `${history.length} 条记录` : "暂无记录"}
        </span>
        <span className="ml-auto inline-flex items-center gap-1 text-book-text-muted">
          <ChevronRight
            size={11}
            className="transition-transform duration-200 group-open:rotate-90"
          />
        </span>
      </summary>
      <div className="mt-2 sm:pl-5">
        {history.length === 0 ? (
          <div className="py-2 text-xs text-book-text-muted">暂无阶段变更记录。</div>
        ) : (
          <ul className="divide-y divide-book-border/35 border-y border-book-border/35">
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
                  <time className="ml-auto shrink-0 text-caption tabular-nums text-book-text-muted">
                    {formatUiDateTime(item.created_at)}
                  </time>
                </div>
                {item.reason ? (
                  <div className="mt-1 break-words text-small leading-relaxed text-book-text-muted">
                    {item.reason}
                  </div>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </div>
    </details>
  );
};
