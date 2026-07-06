import React from "react";
import { FileText, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import type { Blueprint } from "../../../api/blueprint";
import { EmptyHint } from "../shared";

interface SynopsisCardProps {
  blueprint: Blueprint | null;
  loading: boolean;
  refining: boolean;
  canEditBlueprint: boolean;
  onOpenSynopsis: () => void;
  onRefine: () => void;
  onCreateBlueprint: () => void;
}

export const SynopsisCard: React.FC<SynopsisCardProps> = ({
  blueprint,
  loading,
  refining,
  canEditBlueprint,
  onOpenSynopsis,
  onRefine,
  onCreateBlueprint,
}) => (
  <BookCard className="p-4">
    <div className="flex items-center justify-between mb-2">
      <div className="flex items-center gap-2">
        <FileText size={14} className="text-book-primary" />
        <h3 className="text-sm font-serif font-semibold text-book-text-main">剧情梗概</h3>
      </div>
      {blueprint && (
        <div className="flex items-center gap-2">
          <BookButton variant="secondary" size="sm" onClick={onOpenSynopsis}>
            查看完整
          </BookButton>
          {canEditBlueprint ? (
            <BookButton variant="secondary" size="sm" onClick={onRefine} disabled={refining}>
              {refining ? (
                <span className="inline-flex items-center gap-1.5">
                  <Loader2 size={12} className="animate-spin" /> 优化中…
                </span>
              ) : (
                "AI 优化"
              )}
            </BookButton>
          ) : null}
        </div>
      )}
    </div>
    {loading ? (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <Loader2 size={12} className="animate-spin" /> 加载蓝图…
      </div>
    ) : !blueprint ? (
      <EmptyHint
        text="尚未生成蓝图，剧情梗概空缺"
        actionText={canEditBlueprint ? "去灵感对话生成" : undefined}
        onAction={canEditBlueprint ? onCreateBlueprint : undefined}
      />
    ) : (
      <p className="text-xs text-book-text-sub leading-relaxed whitespace-pre-wrap line-clamp-6">
        {blueprint.full_synopsis || "蓝图未包含完整梗概字段"}
      </p>
    )}
  </BookCard>
);
