import React from "react";
import { Sparkles } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { Dropdown } from "../../../components/ui/Dropdown";
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
  <BookCard className="flex min-h-0 flex-col overflow-hidden">
    <header className="flex flex-col gap-2 px-4 pb-3 pt-4 sm:flex-row sm:items-center sm:justify-between">
      <div className="min-w-0">
        <h2 className="text-sm font-semibold text-book-text-main">故事摘要</h2>
        <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
          {blueprint
            ? [blueprint.genre, blueprint.tone].filter(Boolean).join(" · ") || "蓝图核心叙事摘要"
            : "从灵感对话建立故事蓝图"}
        </p>
      </div>
      {blueprint ? (
        <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
          {canEditBlueprint ? (
            <Dropdown
              items={[
                {
                  label: refining ? "正在优化蓝图…" : "优化蓝图",
                  icon: refining ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Sparkles size={12} />
                  ),
                  onClick: onRefine,
                  disabled: refining,
                },
              ]}
            />
          ) : null}
          <BookButton
            variant="secondary"
            size="sm"
            onClick={onOpenSynopsis}
            className="min-w-0 flex-1 !text-xs sm:flex-none"
          >
            查看完整
          </BookButton>
        </div>
      ) : null}
    </header>

    <div className="min-h-0 flex-1 border-t border-book-border/40 px-4 py-3">
      {loading ? (
        <div className="flex items-center gap-2 text-xs text-book-text-muted">
          <LoadingSpinner size="sm" /> 加载蓝图…
        </div>
      ) : !blueprint ? (
        <EmptyHint
          text="还没有故事蓝图"
          actionText={canEditBlueprint ? "开始灵感对话" : undefined}
          onAction={canEditBlueprint ? onCreateBlueprint : undefined}
        />
      ) : (
        <div className="space-y-2">
          {blueprint.one_sentence_summary ? (
            <p className="text-xs font-medium leading-relaxed text-book-text-main">
              {blueprint.one_sentence_summary}
            </p>
          ) : null}
          <p className="line-clamp-7 whitespace-pre-wrap text-xs leading-relaxed text-book-text-sub">
            {blueprint.full_synopsis || "蓝图未包含完整梗概字段"}
          </p>
        </div>
      )}
    </div>
  </BookCard>
);
