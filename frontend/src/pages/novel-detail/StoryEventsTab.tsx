import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import { listChapters, type ChapterSummary } from "../../api/chapter";
import { listForeshadowing, type Foreshadowing } from "../../api/foreshadowing";
import { ForeshadowingIndexCard } from "./story-events/ForeshadowingIndexCard";
import { StoryEventsListCard } from "./story-events/StoryEventsListCard";
import { StoryEventsSummaryCard } from "./story-events/StoryEventsSummaryCard";
import {
  countAnalyzedChapters,
  DEFAULT_STORY_EVENT_FILTERS,
  extractChapterStorySection,
  filterStorySections,
  getStoryEventFilterOptions,
} from "./story-events/storyEventUtils";
import type { StoryEventFilters } from "./story-events/types";
import { useChapterAnalysisLoader } from "./useNovelDetailLoader";
import { useI18n } from "../../i18n";

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
  const { t } = useI18n();
  const [chapters, setChapters] = useState<ChapterSummary[]>([]);
  const [foreshadowing, setForeshadowing] = useState<Foreshadowing[]>([]);
  const [filters, setFilters] = useState<StoryEventFilters>(DEFAULT_STORY_EVENT_FILTERS);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);

  const loadChapters = useCallback(
    async (isCurrent: () => boolean) => {
      try {
        const [nextChapters, nextForeshadowing] = await Promise.all([
          listChapters(projectId),
          listForeshadowing(projectId),
        ]);
        if (!isCurrent()) return;
        setChapters(nextChapters);
        setForeshadowing(nextForeshadowing);
        setLoadedProjectId(projectId);
      } catch (err) {
        if (isCurrent()) {
          setChapters([]);
          setForeshadowing([]);
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
    t("novel.storyEvents.loadFailed"),
  );

  useEffect(() => {
    setFilters(DEFAULT_STORY_EVENT_FILTERS);
  }, [projectId]);

  const displayChapters = loadedProjectId === projectId ? chapters : [];
  const displayForeshadowing = loadedProjectId === projectId ? foreshadowing : [];
  const displayLoading = loading || loadedProjectId !== projectId;

  const sections = useMemo(
    () => displayChapters.map((chapter) => extractChapterStorySection(chapter, t)),
    [displayChapters, t],
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
  const openWritingDesk = useCallback(
    (chapterNumber?: number) => {
      if (!canOpenWritingDesk) {
        addToast(t("novel.writingDeskPermissionDenied"), "error");
        return;
      }
      navigate(chapterNumber ? `/write/${projectId}?chapter=${chapterNumber}` : `/write/${projectId}`);
    },
    [addToast, canOpenWritingDesk, navigate, projectId, t],
  );

  if (displayLoading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> {t("novel.storyEvents.loading")}
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-4">
      <StoryEventsSummaryCard
        analyzedChapters={analyzedChapters}
        totalChapters={displayChapters.length}
        eventCount={allEvents.length}
        foreshadowingCount={displayForeshadowing.length}
        loading={loading}
        onRefresh={() => void load()}
      />

      <ForeshadowingIndexCard
        projectId={projectId}
        records={displayForeshadowing}
        canEdit={canOpenWritingDesk}
        loading={loading}
        onReload={() => load({ silent: true, force: true })}
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
