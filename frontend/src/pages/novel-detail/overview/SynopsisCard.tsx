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
  aggregating: boolean;
  canEditBlueprint: boolean;
  onOpenSynopsis: () => void;
  onRefine: () => void;
  onEditBlueprint: () => void;
  onAggregateBlueprint: () => void;
  onCreateBlueprint: () => void;
}

export const SynopsisCard: React.FC<SynopsisCardProps> = ({
  blueprint,
  loading,
  refining,
  aggregating,
  canEditBlueprint,
  onOpenSynopsis,
  onRefine,
  onEditBlueprint,
  onAggregateBlueprint,
  onCreateBlueprint,
}) => (
  <BookCard className="flex min-h-0 flex-col overflow-hidden">
    <header className="flex flex-col gap-2 px-4 pb-3 pt-4 sm:flex-row sm:items-center sm:justify-between">
      <div className="min-w-0">
        <h2 className="text-sm font-semibold text-book-text-main">故事摘要</h2>
        <p className="mt-0.5 text-small leading-relaxed text-book-text-muted">
          {blueprint
            ? [blueprint.genre, blueprint.tone].filter(Boolean).join(" · ") || "蓝图核心叙事摘要"
            : "从现有资料归纳、直接录入或从灵感对话开始"}
        </p>
      </div>
      {blueprint ? (
        <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
          {canEditBlueprint ? (
            <Dropdown
              items={[
                {
                  label: "编辑蓝图",
                  onClick: onEditBlueprint,
                  disabled: refining || aggregating,
                },
                {
                  label: aggregating
                    ? "正在依据当前资料更新…"
                    : "依据当前资料更新蓝图",
                  icon: aggregating ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Sparkles size={12} />
                  ),
                  onClick: onAggregateBlueprint,
                  disabled: refining || aggregating,
                },
                {
                  label: refining ? "正在优化蓝图…" : "优化蓝图",
                  icon: refining ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Sparkles size={12} />
                  ),
                  onClick: onRefine,
                  disabled: refining || aggregating,
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
        canEditBlueprint ? (
          <div className="flex h-full min-h-28 flex-col items-center justify-center gap-3 text-center">
            <p className="max-w-md text-xs leading-relaxed text-book-text-muted">
              已有正文或设定可以直接归纳正式蓝图；也可以手工录入。只有想法时再从灵感对话开始。
            </p>
            <div className="flex flex-wrap justify-center gap-2">
              <BookButton
                size="sm"
                disabled={aggregating}
                onClick={onAggregateBlueprint}
              >
                {aggregating ? (
                  <span className="inline-flex items-center gap-1.5">
                    <LoadingSpinner size="sm" /> 正在归纳…
                  </span>
                ) : (
                  "依据现有资料归纳蓝图"
                )}
              </BookButton>
              <BookButton
                variant="secondary"
                size="sm"
                disabled={aggregating}
                onClick={onEditBlueprint}
              >
                直接录入蓝图
              </BookButton>
              <BookButton
                variant="secondary"
                size="sm"
                disabled={aggregating}
                onClick={onCreateBlueprint}
              >
                开始灵感对话
              </BookButton>
            </div>
          </div>
        ) : (
          <EmptyHint text="还没有故事蓝图" />
        )
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
