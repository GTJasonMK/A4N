import React from "react";
import { ChevronRight, History } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import type { ProjectStatusHistory } from "../../api/project";
import { ProjectStatusHistoryList } from "../../components/ProjectStatusHistoryList";

interface StatusHistoryPanelProps {
  history: ProjectStatusHistory[];
  loading: boolean;
}

export const StatusHistoryPanel: React.FC<StatusHistoryPanelProps> = ({
  history,
  loading,
}) => {
  return (
    <details className="group border-t border-book-border/35 pt-2">
      <summary className="flex cursor-pointer list-none items-center gap-2 py-1 text-xs text-book-text-sub [&::-webkit-details-marker]:hidden">
        <History size={12} className="shrink-0 text-book-text-muted" />
        <span className="font-medium">状态变更历史</span>
        <span className="text-caption text-book-text-muted">
          {loading ? "加载中…" : history.length > 0 ? `${history.length} 条记录` : "暂无记录"}
        </span>
        <span className="ml-auto inline-flex items-center gap-1 text-book-text-muted">
          {loading ? <LoadingSpinner size="sm" /> : null}
          <ChevronRight
            size={11}
            className="transition-transform duration-200 group-open:rotate-90"
          />
        </span>
      </summary>
      <div className="mt-2 sm:pl-5">
        {loading ? (
          <div className="inline-flex items-center gap-1.5 py-2 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" />
            正在加载历史记录…
          </div>
        ) : history.length === 0 ? (
          <div className="py-2 text-xs text-book-text-muted">暂无阶段变更记录。</div>
        ) : (
          <ProjectStatusHistoryList history={history} className="border-y border-book-border/35" />
        )}
      </div>
    </details>
  );
};
