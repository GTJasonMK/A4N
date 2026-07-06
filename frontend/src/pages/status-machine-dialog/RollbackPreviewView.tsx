import React from "react";
import type { RollbackImpact, RollbackPreview } from "../../api/project";

interface RollbackPreviewViewProps {
  preview: RollbackPreview;
}

export const RollbackPreviewView: React.FC<RollbackPreviewViewProps> = ({ preview }) => {
  return (
    <div className="space-y-2 text-xs text-book-text-sub max-h-44 overflow-y-auto custom-scrollbar">
      <div className="rounded border border-book-border/40 bg-book-bg-paper/50 p-2">
        <div className="text-book-text-muted mb-1">回退路径</div>
        <div className="font-mono text-[10px] break-words">{preview.path.join(" -> ")}</div>
      </div>

      <div className="rounded border border-book-border/40 bg-book-bg-paper/50 p-2">
        <div className="text-book-text-muted mb-1">概览</div>
        <div className="whitespace-pre-wrap break-words">{preview.summary}</div>
      </div>

      <div className="space-y-1">
        {preview.impacts.map((impact) => (
          <RollbackImpactRow key={impact.kind} impact={impact} />
        ))}
      </div>
    </div>
  );
};

const RollbackImpactRow: React.FC<{ impact: RollbackImpact }> = ({ impact }) => {
  const scope = formatImpactScope(impact);

  return (
    <div className="a4n-stat-tile p-2">
      <div className="flex items-center justify-between gap-2">
        <div className="text-book-text-main">{impact.title}</div>
        {impact.count != null && <div className="pyqt-pill !text-[10px]">{impact.count}</div>}
      </div>
      <div className="mt-1 text-[10px] text-book-text-muted">
        {impact.tables.join(", ")}
        {scope ? ` · ${scope}` : ""}
      </div>
      {impact.note && <div className="mt-1 text-[10px]">{impact.note}</div>}
    </div>
  );
};

function formatImpactScope(impact: RollbackImpact): string | null {
  if (impact.chapter_start != null && impact.chapter_end != null) {
    return `第 ${impact.chapter_start}-${impact.chapter_end} 章`;
  }

  if (impact.part_start != null && impact.part_end != null) {
    return `第 ${impact.part_start}-${impact.part_end} 部分`;
  }

  return null;
}
