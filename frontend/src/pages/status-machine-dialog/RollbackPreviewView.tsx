import React from "react";
import type {
  ProjectRollbackImpactSummary,
  ProjectRollbackPreview,
} from "../../api/project";
import { PROJECT_STATUS_LABELS } from "../../api/generated/host";

interface RollbackPreviewViewProps {
  preview: ProjectRollbackPreview;
}

export const RollbackPreviewView: React.FC<RollbackPreviewViewProps> = ({ preview }) => {
  return (
    <div className="space-y-2 text-xs text-book-text-sub">
      <div>
        <div className="mb-1 text-book-text-muted">回退路径</div>
        <div className="break-words text-book-text-sub">
          {preview.path.map((status) => PROJECT_STATUS_LABELS[status]).join(" → ")}
        </div>
      </div>

      <div className="border-t border-book-border/30 pt-2">
        <div className="mb-1 text-book-text-muted">影响概览</div>
        {preview.impacts.length > 0 ? (
          <div className="text-warning">回退会删除以下依赖数据，此操作不可恢复。</div>
        ) : null}
      </div>

      <div className="divide-y divide-book-border/35 border-y border-book-border/35">
        {preview.impacts.map((impact) => (
          <RollbackImpactRow key={impact.title} impact={impact} />
        ))}
      </div>
    </div>
  );
};

const RollbackImpactRow: React.FC<{ impact: ProjectRollbackImpactSummary }> = ({ impact }) => {
  const scope = formatImpactScope(impact);

  return (
    <div className="px-2.5 py-2">
      <div className="flex min-w-0 items-baseline gap-2">
        <div className="min-w-0 flex-1 truncate font-medium text-book-text-main">{impact.title}</div>
        <div className="shrink-0 tabular-nums text-caption text-book-text-muted">
          {impact.count.toLocaleString()} 项
        </div>
      </div>
      {scope ? <div className="mt-1 text-caption text-book-text-muted">{scope}</div> : null}
      {impact.note && <div className="mt-1 text-caption">{impact.note}</div>}
    </div>
  );
};

function formatImpactScope(impact: ProjectRollbackImpactSummary): string | null {
  if (impact.chapter_start != null && impact.chapter_end != null) {
    return `第 ${impact.chapter_start}-${impact.chapter_end} 章`;
  }

  if (impact.part_start != null && impact.part_end != null) {
    return `第 ${impact.part_start}-${impact.part_end} 部分`;
  }

  return null;
}
