import React from "react";
import { RefreshCw } from "lucide-react";

interface StoryEventsSummaryCardProps {
  analyzedChapters: number;
  totalChapters: number;
  eventCount: number;
  foreshadowingCount: number;
  loading: boolean;
  onRefresh: () => void;
}

export const StoryEventsSummaryCard: React.FC<StoryEventsSummaryCardProps> = ({
  analyzedChapters,
  totalChapters,
  eventCount,
  foreshadowingCount,
  loading,
  onRefresh,
}) => (
  <div className="flex min-h-8 items-center gap-3 text-[11px] text-book-text-muted">
    <div className="min-w-0 flex-1 truncate">
      {totalChapters > 0 ? `已分析 ${analyzedChapters}/${totalChapters} 章` : "暂无章节"}
      {eventCount > 0 ? ` · ${eventCount} 个事件` : ""}
      {foreshadowingCount > 0 ? ` · ${foreshadowingCount} 条伏笔线索` : ""}
    </div>
    <button
      type="button"
      onClick={onRefresh}
      disabled={loading}
      className="pyqt-btn-icon ml-auto !h-9 !w-9 sm:!h-7 sm:!w-7"
      title="刷新事件脉络"
      aria-label="刷新事件脉络"
    >
      <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
    </button>
  </div>
);
