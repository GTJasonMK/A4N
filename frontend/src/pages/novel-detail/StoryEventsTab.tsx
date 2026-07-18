import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Loader2 } from "lucide-react";
import { useToast } from "../../components/feedback/Toast";
import { extractErrorMessage } from "../../api";
import {
  listenAppEvent,
  type UnlistenFn,
} from "../../api/runtime";
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
import type { ChapterAnalysisUpdatedPayload, StoryEventFilters } from "./story-events/types";

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
  const [loading, setLoading] = useState(true);
  const [filters, setFilters] = useState<StoryEventFilters>(DEFAULT_STORY_EVENT_FILTERS);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const activeProjectIdRef = useRef(projectId);
  const loadSeqRef = useRef(0);
  const loadInFlightProjectRef = useRef<number | null>(null);

  activeProjectIdRef.current = projectId;

  const isCurrentLoad = useCallback((requestProjectId: number, seq: number) => {
    return activeProjectIdRef.current === requestProjectId && loadSeqRef.current === seq;
  }, []);

  const load = useCallback(async (silent = false) => {
    const requestProjectId = projectId;
    if (loadInFlightProjectRef.current === requestProjectId) return;
    loadInFlightProjectRef.current = requestProjectId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    if (!silent) setLoading(true);
    try {
      const nextChapters = await listChapters(requestProjectId);
      if (!isCurrentLoad(requestProjectId, requestSeq)) return;
      setChapters(nextChapters);
    } catch (err) {
      if (!isCurrentLoad(requestProjectId, requestSeq)) return;
      setChapters([]);
      addToast(extractErrorMessage(err, "加载事件脉络失败"), "error");
    } finally {
      if (isCurrentLoad(requestProjectId, requestSeq)) {
        setLoadedProjectId(requestProjectId);
        if (!silent) setLoading(false);
        if (loadInFlightProjectRef.current === requestProjectId) {
          loadInFlightProjectRef.current = null;
        }
      }
    }
  }, [addToast, isCurrentLoad, projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setFilters(DEFAULT_STORY_EVENT_FILTERS);
  }, [projectId]);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | null = null;

    void listenAppEvent<ChapterAnalysisUpdatedPayload>("chapter-analysis-updated", (event) => {
      if (Number(event.payload?.project_id) === projectId) {
        void load(true);
      }
    }).then((detach) => {
      if (disposed) {
        detach();
      } else {
        unlisten = detach;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [load, projectId]);

  useEffect(() => {
    const refreshOnFocus = () => {
      if (document.visibilityState === "visible") void load(true);
    };
    window.addEventListener("focus", refreshOnFocus);
    document.addEventListener("visibilitychange", refreshOnFocus);
    return () => {
      window.removeEventListener("focus", refreshOnFocus);
      document.removeEventListener("visibilitychange", refreshOnFocus);
    };
  }, [load, projectId]);

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
        <Loader2 size={12} className="animate-spin" /> 加载事件脉络…
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
