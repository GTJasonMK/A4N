import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import { listChapters, type ChapterData } from "../../api/chapter";
import { StoryEventsListCard } from "./story-events/StoryEventsListCard";
import { StoryEventsSummaryCard } from "./story-events/StoryEventsSummaryCard";
import {
  countAnalyzedChapters,
  countForeshadowingSignals,
  DEFAULT_STORY_EVENT_FILTERS,
  extractChapterStorySection,
  filterStorySections,
  getStoryEventFilterOptions,
} from "./story-events/storyEventUtils";
import type { StoryEventFilters } from "./story-events/types";
import { useChapterAnalysisLoader } from "./useNovelDetailLoader";

interface StoryEventsTabProps {
  projectId: number;
  canOpenWritingDesk: boolean;
}

export const StoryEventsTab: React.FC<StoryEventsTabProps> = ({
  projectId,
  canOpenWritingDesk,
}) => {
  const navigate = useNavigate();
  const { addToast } = useToast();
  const [chapters, setChapters] = useState<ChapterData[]>([]);
  const [filters, setFilters] = useState<StoryEventFilters>(DEFAULT_STORY_EVENT_FILTERS);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);

  const loadChapters = useCallback(
    async (isCurrent: () => boolean) => {
      try {
        const nextChapters = await listChapters(projectId);
        if (!isCurrent()) return;
        setChapters(nextChapters);
        setLoadedProjectId(projectId);
      } catch (err) {
        if (isCurrent()) {
          setChapters([]);
          setLoadedProjectId(projectId);
        }
        throw err;
      }
    },
    [projectId],
  );
  const { loading, load } = useChapterAnalysisLoader(
    projectId,
    loadChapters,
    "加载事件脉络失败",
  );

  useEffect(() => {
    setFilters(DEFAULT_STORY_EVENT_FILTERS);
  }, [projectId]);

  const displayChapters = loadedProjectId === projectId ? chapters : [];
  const displayLoading = loading || loadedProjectId !== projectId;

  const sections = useMemo(
    () => displayChapters.map(extractChapterStorySection),
    [displayChapters],
  );

  const allEvents = useMemo(
    () => sections.flatMap((section) => section.events),
    [sections],
  );

  const filterOptions = useMemo(
    () => getStoryEventFilterOptions(sections, allEvents),
    [sections, allEvents],
  );
  const filteredSections = useMemo(
    () => filterStorySections(sections, filters),
    [sections, filters],
  );
  const analyzedChapters = useMemo(() => countAnalyzedChapters(sections), [sections]);
  const totalForeshadowing = useMemo(() => countForeshadowingSignals(sections), [sections]);

  const openWritingDesk = useCallback(
    (chapterNumber?: number) => {
      if (!canOpenWritingDesk) {
        addToast("当前账号没有进入写作桌的权限", "error");
        return;
      }
      navigate(chapterNumber ? `/write/${projectId}?chapter=${chapterNumber}` : `/write/${projectId}`);
    },
    [addToast, canOpenWritingDesk, navigate, projectId],
  );

  if (displayLoading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> 加载事件脉络…
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-4">
      <StoryEventsSummaryCard
        analyzedChapters={analyzedChapters}
        totalChapters={displayChapters.length}
        eventCount={allEvents.length}
        foreshadowingCount={totalForeshadowing}
        loading={loading}
        onRefresh={() => void load()}
      />

      <StoryEventsListCard
        chaptersCount={displayChapters.length}
        analyzedChapters={analyzedChapters}
        sections={filteredSections}
        filters={filters}
        filterOptions={filterOptions}
        onFiltersChange={setFilters}
        onOpenWritingDesk={canOpenWritingDesk ? () => openWritingDesk() : undefined}
        onOpenChapter={canOpenWritingDesk ? openWritingDesk : undefined}
      />
    </div>
  );
};
