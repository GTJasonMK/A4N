import React from "react";
import { Loader2 } from "lucide-react";

export const JsonCard: React.FC<{ title: string; data: unknown; loading?: boolean }> = ({
  title,
  data,
  loading = false,
}) => (
  <div className="pyqt-card p-3">
    <h4 className="text-xs font-semibold text-book-text-sub uppercase tracking-wider mb-1.5">
      {title}
    </h4>
    {loading && data == null ? (
      <div className="inline-flex items-center gap-1.5 text-[11px] text-book-text-muted">
        <Loader2 size={12} className="animate-spin" />
        正在加载数据…
      </div>
    ) : data == null ? (
      <div className="text-[11px] text-book-text-muted italic">（无数据）</div>
    ) : (
      <pre className="text-[10px] font-mono whitespace-pre-wrap break-words max-h-40 overflow-y-auto custom-scrollbar bg-book-bg-tertiary/40 rounded p-2 text-book-text-sub">
        {JSON.stringify(data, null, 2)}
      </pre>
    )}
  </div>
);
