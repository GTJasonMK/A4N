import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { extractErrorMessage } from "../../api";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import { listStoryEventChapters, type ChapterStoryEventEntry } from "../../api/chapter";
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
  reconcileStoryEventFilters,
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
  const [chapters, setChapters] = useState<ChapterStoryEventEntry[]>([]);
  const [foreshadowing, setForeshadowing] = useState<Foreshadowing[]>([]);
  const [filters, setFilters] = useState<StoryEventFilters>(DEFAULT_STORY_EVENT_FILTERS);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const [loadErrors, setLoadErrors] = useState<string[]>([]);
  const [foreshadowingLoadError, setForeshadowingLoadError] = useState<string | null>(null);
  const [foreshadowingLoading, setForeshadowingLoading] = useState(false);
  const chaptersProjectIdRef = useRef<number | null>(null);
  const foreshadowingProjectIdRef = useRef<number | null>(null);
  const foreshadowingRequestedProjectIdRef = useRef<number | null>(null);
  const foreshadowingRequestRef = useRef<{
    projectId: number;
    promise: Promise<Foreshadowing[]>;
  } | null>(null);
  const currentProjectIdRef = useRef(projectId);
  currentProjectIdRef.current = projectId;

  const getForeshadowingRequest = useCallback(
    (force: boolean) => {
      const activeRequest = foreshadowingRequestRef.current;
      if (activeRequest?.projectId === projectId) return activeRequest.promise;
      if (!force && foreshadowingProjectIdRef.current === projectId) return null;

      const promise = listForeshadowing(projectId);
      const request = { projectId, promise };
      foreshadowingRequestRef.current = request;
      void promise.finally(() => {
        if (foreshadowingRequestRef.current === request) {
          foreshadowingRequestRef.current = null;
        }
      }).catch(() => undefined);
      return promise;
    },
    [projectId],
  );

  const loadChapters = useCallback(
    async (isCurrent: () => boolean) => {
      const foreshadowingPromise = foreshadowingRequestedProjectIdRef.current === projectId
        ? getForeshadowingRequest(true)
        : null;
      if (foreshadowingPromise) setForeshadowingLoading(true);
      const [chaptersResult, foreshadowingResult] = await Promise.all([
        listStoryEventChapters(projectId).then(
          (value) => ({ status: "fulfilled" as const, value }),
          (reason) => ({ status: "rejected" as const, reason }),
        ),
        foreshadowingPromise
          ? foreshadowingPromise.then(
            (value) => ({ status: "fulfilled" as const, value }),
            (reason) => ({ status: "rejected" as const, reason }),
          )
          : Promise.resolve(null),
      ]);
      if (!isCurrent()) return;
      const errors: string[] = [];
      if (chaptersResult.status === "fulfilled") {
        setChapters(chaptersResult.value);
        chaptersProjectIdRef.current = projectId;
      }
      else {
        if (chaptersProjectIdRef.current !== projectId) setChapters([]);
        errors.push(extractErrorMessage(chaptersResult.reason, t("novel.storyEvents.chaptersLoadFailed")));
      }
      if (foreshadowingResult) {
        if (foreshadowingResult.status === "fulfilled") {
          setForeshadowing(foreshadowingResult.value);
          foreshadowingProjectIdRef.current = projectId;
          setForeshadowingLoadError(null);
        } else {
          if (foreshadowingProjectIdRef.current !== projectId) setForeshadowing([]);
          const message = extractErrorMessage(foreshadowingResult.reason, t("novel.storyEvents.foreshadowingLoadFailed"));
          setForeshadowingLoadError(message);
          errors.push(message);
        }
      }
      if (foreshadowingPromise) setForeshadowingLoading(false);
      setLoadErrors(errors);
      setLoadedProjectId(projectId);
    },
    [getForeshadowingRequest, projectId, t],
  );
  const { loading, load } = useChapterAnalysisLoader(
    projectId,
    loadChapters,
    t("novel.storyEvents.loadFailed"),
  );

  useEffect(() => {
    setFilters(DEFAULT_STORY_EVENT_FILTERS);
    foreshadowingRequestedProjectIdRef.current = null;
    foreshadowingProjectIdRef.current = null;
    foreshadowingRequestRef.current = null;
    setForeshadowing([]);
    setForeshadowingLoadError(null);
    setForeshadowingLoading(false);
  }, [projectId]);

  const loadForeshadowing = useCallback(async () => {
    foreshadowingRequestedProjectIdRef.current = projectId;
    if (foreshadowingProjectIdRef.current === projectId) return;
    setForeshadowingLoading(true);
    try {
      const request = getForeshadowingRequest(false);
      if (!request) return;
      const records = await request;
      if (currentProjectIdRef.current !== projectId) return;
      setForeshadowing(records);
      foreshadowingProjectIdRef.current = projectId;
      setForeshadowingLoadError(null);
    } catch (error) {
      if (currentProjectIdRef.current !== projectId) return;
      const message = extractErrorMessage(error, t("novel.storyEvents.foreshadowingLoadFailed"));
      setForeshadowingLoadError(message);
    } finally {
      if (currentProjectIdRef.current === projectId) setForeshadowingLoading(false);
    }
  }, [getForeshadowingRequest, projectId, t]);

  const displayChapters = loadedProjectId === projectId ? chapters : [];
  const displayForeshadowing = loadedProjectId === projectId ? foreshadowing : [];
  const foreshadowingLoaded = foreshadowingProjectIdRef.current === projectId;
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
    () => getStoryEventFilterOptions(allEvents),
    [allEvents],
  );
  const activeFilters = useMemo(
    () => reconcileStoryEventFilters(filters, filterOptions),
    [filterOptions, filters],
  );
  useEffect(() => {
    if (activeFilters.character === filters.character && activeFilters.type === filters.type && activeFilters.importance === filters.importance) return;
    setFilters(activeFilters);
  }, [activeFilters, filters]);
  const filteredSections = useMemo(
    () => filterStorySections(sections, activeFilters),
    [activeFilters, sections],
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
  const handleRefresh = async () => {
    setLoadErrors([]);
    await load();
  };

  if (displayLoading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> {t("novel.storyEvents.loading")}
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-4">
      {loadErrors.length > 0 ? (
        <div role="alert" className="space-y-1 border-l-2 border-error px-3 py-2 text-xs text-error">
          {loadErrors.map((error, index) => <p key={`${index}:${error}`}>{error}</p>)}
        </div>
      ) : null}
      <StoryEventsSummaryCard
        analyzedChapters={analyzedChapters}
        totalChapters={displayChapters.length}
        eventCount={allEvents.length}
        foreshadowingCount={displayForeshadowing.length}
        loading={loading}
        onRefresh={() => void handleRefresh()}
      />

      <details
        className="min-w-0"
        onToggle={(event) => {
          if (event.currentTarget.open) void loadForeshadowing();
        }}
      >
        <summary className="mb-2 cursor-pointer text-sm font-medium text-book-text-main">
          {t("novel.foreshadowing.title")}
          {!foreshadowingLoadError && foreshadowingLoaded ? ` · ${displayForeshadowing.length}` : ""}
        </summary>
        <ForeshadowingIndexCard
        projectId={projectId}
        records={displayForeshadowing}
        canEdit={canOpenWritingDesk}
        loading={loading || foreshadowingLoading}
        loadError={foreshadowingLoadError}
        onReload={() => load({ silent: true, force: true })}
        onDeleted={(id) => setForeshadowing((current) => current.filter((record) => record.id !== id))}
      />
      </details>

      <StoryEventsListCard
        chaptersCount={displayChapters.length}
        analyzedChapters={analyzedChapters}
        sections={filteredSections}
        filters={activeFilters}
        filterOptions={filterOptions}
        onFiltersChange={(nextFilters) => setFilters(reconcileStoryEventFilters(nextFilters, filterOptions))}
        onOpenChapter={canOpenWritingDesk ? openWritingDesk : undefined}
      />
    </div>
  );
};
