import React from "react";
import { RefreshCw } from "lucide-react";
import { useI18n } from "../../../i18n";

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
}) => {
  const { t } = useI18n();
  return (
  <div className="flex min-h-8 items-center gap-3 text-small text-book-text-muted">
    <div className="min-w-0 flex-1 truncate">
      {totalChapters > 0
        ? t("novel.storyEvents.analysisSummary", { analyzed: analyzedChapters, total: totalChapters })
        : t("novel.storyEvents.noChapters")}
      {eventCount > 0 ? ` · ${t("novel.storyEvents.eventCount", { count: eventCount })}` : ""}
      {foreshadowingCount > 0 ? ` · ${t("novel.storyEvents.foreshadowingCount", { count: foreshadowingCount })}` : ""}
    </div>
    <button
      type="button"
      onClick={onRefresh}
      disabled={loading}
      className="pyqt-btn-icon ml-auto !h-9 !w-9 sm:!h-7 sm:!w-7"
      title={t("novel.storyEvents.refresh")}
      aria-label={t("novel.storyEvents.refresh")}
    >
      <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
    </button>
  </div>
  );
};
