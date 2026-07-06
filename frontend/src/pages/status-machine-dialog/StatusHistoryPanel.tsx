import React from "react";
import { ArrowRight, History, Loader2 } from "lucide-react";
import type { ProjectStatusHistory } from "../../api/project";
import { statusLabel } from "./statusMachineUtils";

interface StatusHistoryPanelProps {
  history: ProjectStatusHistory[];
  loading: boolean;
}

export const StatusHistoryPanel: React.FC<StatusHistoryPanelProps> = ({
  history,
  loading,
}) => {
  return (
    <div className="pyqt-card p-3">
      <div className="flex items-center justify-between mb-2">
        <div className="text-xs text-book-text-muted inline-flex items-center gap-1.5">
          <History size={12} /> 状态变更历史
        </div>
        {loading && <Loader2 size={11} className="animate-spin" />}
      </div>
      {loading ? (
        <div className="inline-flex items-center gap-1.5 text-xs text-book-text-muted">
          <Loader2 size={11} className="animate-spin" />
          正在加载历史记录…
        </div>
      ) : history.length === 0 ? (
        <div className="text-xs text-book-text-muted">无历史记录</div>
      ) : (
        <ul className="space-y-1 max-h-40 overflow-y-auto custom-scrollbar">
          {history.map((item) => (
            <li
              key={item.id}
              className="text-xs text-book-text-sub flex items-center gap-2 px-2 py-1 rounded hover:bg-book-bg-tertiary/40"
            >
              <span className="text-book-text-muted shrink-0 font-mono text-[10px]">
                {item.created_at ? new Date(item.created_at).toLocaleString() : "—"}
              </span>
              <span>{statusLabel(item.from_status)}</span>
              <ArrowRight size={10} className="text-book-text-muted" />
              <span className="font-semibold text-book-text-main">
                {statusLabel(item.to_status)}
              </span>
              {item.reason && (
                <span className="text-book-text-muted truncate">· {item.reason}</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
