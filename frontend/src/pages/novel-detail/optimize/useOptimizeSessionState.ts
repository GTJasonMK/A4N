import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type SetStateAction,
} from "react";
import { extractErrorMessage } from "../../../api";
import { listChapters } from "../../../api/chapter";
import type { ChapterData } from "../../../api/chapter";
import {
  getOptimizeSession,
  listOptimizePatchSets,
  listOptimizeSessions,
  listOptimizeTurns,
  type OptimizeSession,
  type OptimizeTurn,
  type ProposeOptimizationPatchesResponse,
} from "../../../api/optimize";
import type { ToastType } from "../../../components/feedback/Toast";
import {
  initialSelectedChapter,
  latestPendingPatchProposal,
} from "./optimizeTabUtils";

type AddToast = (message: unknown, type: ToastType) => void;

interface UseOptimizeSessionStateParams {
  projectId: number;
  fixedChapterId?: number | null;
  taskRunning: boolean;
  addToast: AddToast;
  setPendingPatchState: (
    proposal: ProposeOptimizationPatchesResponse | null,
    baseContent: string | null,
  ) => void;
  clearPendingPatchState: () => void;
}

export function useOptimizeSessionState({
  projectId,
  fixedChapterId,
  taskRunning,
  addToast,
  setPendingPatchState,
  clearPendingPatchState,
}: UseOptimizeSessionStateParams) {
  const [chapters, setChapters] = useState<ChapterData[]>([]);
  const [sessions, setSessions] = useState<OptimizeSession[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedChapter, setSelectedChapterState] = useState<number | "">(
    initialSelectedChapter(fixedChapterId),
  );
  const [activeSessionId, setActiveSessionId] = useState<number | null>(null);
  const [activeSession, setActiveSession] = useState<OptimizeSession | null>(
    null,
  );
  const [turns, setTurns] = useState<OptimizeTurn[]>([]);
  const selectedChapterRef = useRef<number | "">(
    initialSelectedChapter(fixedChapterId),
  );
  const activeSessionIdRef = useRef<number | null>(null);
  const loadRequestRef = useRef(0);
  const refreshSessionsRequestRef = useRef(0);
  const openSessionRequestRef = useRef(0);

  const setSelectedChapter = useCallback(
    (value: SetStateAction<number | "">) => {
      if (typeof value !== "function") {
        selectedChapterRef.current = value;
        setSelectedChapterState(value);
        return;
      }
      setSelectedChapterState((current) => {
        const next = value(current);
        selectedChapterRef.current = next;
        return next;
      });
    },
    [],
  );

  const chapterContentById = useCallback(
    (chapterId: number): string | null =>
      chapters.find((chapter) => chapter.id === chapterId)?.content ?? null,
    [chapters],
  );

  useEffect(() => {
    activeSessionIdRef.current = activeSessionId;
  }, [activeSessionId]);

  const setActiveSessionIdentity = useCallback((id: number | null) => {
    activeSessionIdRef.current = id;
    setActiveSessionId(id);
  }, []);

  const activeChapter = useMemo(
    () =>
      typeof selectedChapter === "number"
        ? chapters.find((chapter) => chapter.id === selectedChapter) ?? null
        : null,
    [chapters, selectedChapter],
  );

  const chapterSessions = useMemo(
    () =>
      typeof selectedChapter === "number"
        ? sessions.filter((session) => session.chapter_id === selectedChapter)
        : [],
    [selectedChapter, sessions],
  );

  const openSession = useCallback(
    async (id: number) => {
      const requestId = openSessionRequestRef.current + 1;
      const switchingSession = activeSessionIdRef.current !== id;
      openSessionRequestRef.current = requestId;
      setActiveSessionIdentity(id);
      if (switchingSession) {
        setActiveSession(null);
        setTurns([]);
        clearPendingPatchState();
      }
      const [session, sessionTurns, patchSets] = await Promise.all([
        getOptimizeSession(id),
        listOptimizeTurns(id),
        listOptimizePatchSets(id),
      ]);
      if (openSessionRequestRef.current !== requestId || activeSessionIdRef.current !== id) {
        return session;
      }
      setActiveSession(session);
      setTurns(sessionTurns);
      const proposal = latestPendingPatchProposal(patchSets, session.project_id);
      setPendingPatchState(
        proposal,
        proposal ? chapterContentById(proposal.chapter_id) : null,
      );
      return session;
    },
    [
      chapterContentById,
      clearPendingPatchState,
      setActiveSessionIdentity,
      setPendingPatchState,
    ],
  );

  const refreshSessions = useCallback(async () => {
    if (typeof selectedChapter !== "number") {
      refreshSessionsRequestRef.current += 1;
      setSessions([]);
      return [];
    }
    const requestId = refreshSessionsRequestRef.current + 1;
    refreshSessionsRequestRef.current = requestId;
    const targetChapterId = selectedChapter;
    const nextSessions = await listOptimizeSessions(projectId, targetChapterId);
    if (
      refreshSessionsRequestRef.current !== requestId ||
      selectedChapterRef.current !== targetChapterId
    ) {
      return [];
    }
    setSessions(nextSessions);
    return nextSessions;
  }, [projectId, selectedChapter]);

  const load = useCallback(async () => {
    const requestId = loadRequestRef.current + 1;
    loadRequestRef.current = requestId;
    setLoading(true);
    try {
      const nextChapters = await listChapters(projectId);
      if (loadRequestRef.current !== requestId) return;
      setChapters(nextChapters);
      if (fixedChapterId != null) {
        setSelectedChapter(fixedChapterId);
      } else if (nextChapters.length > 0) {
        setSelectedChapter((current) =>
          typeof current === "number" &&
          nextChapters.some((chapter) => chapter.id === current)
            ? current
            : nextChapters[0].id,
        );
      } else {
        setSelectedChapter("");
      }
    } catch (err) {
      if (loadRequestRef.current === requestId) {
        addToast(extractErrorMessage(err, "加载内容优化 Agent 失败"), "error");
      }
    } finally {
      if (loadRequestRef.current === requestId) {
        setLoading(false);
      }
    }
  }, [addToast, fixedChapterId, projectId]);

  useEffect(() => {
    loadRequestRef.current += 1;
    refreshSessionsRequestRef.current += 1;
    openSessionRequestRef.current += 1;
    setChapters([]);
    setSessions([]);
    setSelectedChapter(initialSelectedChapter(fixedChapterId));
    setActiveSessionIdentity(null);
    setActiveSession(null);
    setTurns([]);
    clearPendingPatchState();
  }, [
    clearPendingPatchState,
    fixedChapterId,
    projectId,
    setActiveSessionIdentity,
  ]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (fixedChapterId != null) {
      setSelectedChapter(fixedChapterId);
    }
  }, [fixedChapterId]);

  useEffect(() => {
    void refreshSessions().catch((err) => {
      addToast(extractErrorMessage(err, "加载章节优化会话失败"), "error");
    });
  }, [addToast, refreshSessions]);

  useEffect(() => {
    if (typeof selectedChapter !== "number") {
      setActiveSessionIdentity(null);
      setActiveSession(null);
      setTurns([]);
      clearPendingPatchState();
      return;
    }

    if (activeSession?.chapter_id === selectedChapter) {
      return;
    }

    const latest = sessions.find(
      (session) => session.chapter_id === selectedChapter,
    );
    if (!latest) {
      if (activeSession?.chapter_id !== selectedChapter) {
        setActiveSessionIdentity(null);
        setActiveSession(null);
        setTurns([]);
        clearPendingPatchState();
      }
      return;
    }
    if (activeSessionId !== latest.id) {
      void openSession(latest.id).catch((err) => {
        addToast(extractErrorMessage(err, "打开优化会话失败"), "error");
      });
    }
  }, [
    activeSession,
    activeSessionId,
    addToast,
    clearPendingPatchState,
    openSession,
    selectedChapter,
    setActiveSessionIdentity,
    sessions,
  ]);

  useEffect(() => {
    if (!taskRunning || activeSessionId == null) return;
    let stopped = false;
    const refreshActiveSessionMessages = async () => {
      try {
        const [session, sessionTurns] = await Promise.all([
          getOptimizeSession(activeSessionId),
          listOptimizeTurns(activeSessionId),
        ]);
        if (stopped || activeSessionIdRef.current !== activeSessionId) return;
        setActiveSession(session);
        setTurns(sessionTurns);
      } catch {
        // 运行中的刷新只用于让已落库的 Agent 消息更快出现；终态由 generation task 统一处理。
      }
    };
    void refreshActiveSessionMessages();
    const timer = window.setInterval(() => {
      void refreshActiveSessionMessages();
    }, 900);
    return () => {
      stopped = true;
      window.clearInterval(timer);
    };
  }, [activeSessionId, taskRunning]);

  return {
    activeChapter,
    activeSession,
    activeSessionId,
    activeSessionIdRef,
    chapterContentById,
    chapterSessions,
    chapters,
    load,
    loading,
    openSession,
    refreshSessions,
    selectedChapter,
    sessions,
    setActiveSession,
    setActiveSessionIdentity,
    setSelectedChapter,
    setTurns,
    turns,
  };
}
