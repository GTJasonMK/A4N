import React from "react";
import { BookOpen, RefreshCw } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";

interface StoryEventsSummaryCardProps {
  analyzedChapters: number;
  totalChapters: number;
  eventCount: number;
  characterCount: number;
  foreshadowingCount: number;
  loading: boolean;
  onRefresh: () => void;
}

export const StoryEventsSummaryCard: React.FC<StoryEventsSummaryCardProps> = ({
  analyzedChapters,
  totalChapters,
  eventCount,
  characterCount,
  foreshadowingCount,
  loading,
  onRefresh,
}) => (
  <BookCard className="p-4">
    <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <BookOpen size={14} className="text-book-primary" />
          <h3 className="text-sm font-serif font-semibold text-book-text-main">事件脉络</h3>
        </div>
      </div>
      <BookButton variant="secondary" size="sm" onClick={onRefresh} disabled={loading}>
        <span className="inline-flex items-center gap-1.5">
          <RefreshCw size={12} className={loading ? "animate-spin" : undefined} /> 刷新
        </span>
      </BookButton>
    </div>

    <div className="grid grid-cols-2 gap-2 text-xs md:grid-cols-4">
      <Metric label="已分析章节" value={`${analyzedChapters}/${totalChapters}`} />
      <Metric label="关键事件" value={String(eventCount)} />
      <Metric label="涉及角色" value={String(characterCount)} />
      <Metric label="伏笔/悬念" value={String(foreshadowingCount)} />
    </div>
  </BookCard>
);

const Metric: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="a4n-stat-tile px-3 py-2">
    <div className="text-[10px] text-book-text-muted">{label}</div>
    <div className="mt-0.5 font-mono text-base font-semibold text-book-text-main">{value}</div>
  </div>
);
