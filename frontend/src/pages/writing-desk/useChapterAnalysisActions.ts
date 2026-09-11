import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { extractErrorMessage } from "../../api";
import {
  CHAPTER_ANALYSIS_PROGRESS_TOTAL,
  generateChapterAnalysis,
  previewChapterAnalysisRequest,
  previewResumeChapterAnalysisSyncRequest,
  resumeChapterAnalysisSync,
  type ChapterSummary,
} from "../../api/chapter";
import type { ModelRequestPreview } from "../../api/requestPreview";
import type { ToastType } from "../../components/feedback/Toast";
import { cancelWorkflowTask } from "../../api/streaming";
import { createWorkflowTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { useI18n } from "../../i18n";

type AddToast = (message: string, type: ToastType) => void;

interface UseChapterAnalysisActionsParams {
  projectId: number;
  activeChapter: ChapterSummary | null;
  activeChapterBusy: boolean;
  activeChapterGeneratingMessage: string;
  analysisBlockedReason: string | null;
  dirtyRef: { current: boolean };
  addToast: AddToast;
  reloadChapters: (options?: { force?: boolean }) => Promise<ChapterSummary[] | null>;
  reloadOutlines: () => Promise<void>;
  setChapters: Dispatch<SetStateAction<ChapterSummary[]>>;
  refreshProfiles: () => void;
}

export function useChapterAnalysisActions({
  projectId,
  activeChapter,
  activeChapterBusy,
  activeChapterGeneratingMessage,
  analysisBlockedReason,
  dirtyRef,
  addToast,
  reloadChapters,
  reloadOutlines,
  setChapters,
  refreshProfiles,
}: UseChapterAnalysisActionsParams) {
  const { runProgressTask } = useProgressTask();
  const { t } = useI18n();
  const [analysisBusy, setAnalysisBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const mountedRef = useRef(false);
  const projectIdRef = useRef(projectId);
  const activeChapterIdRef = useRef(activeChapter?.id ?? null);
  const analysisActionRef = useRef<"generate" | "resume" | null>(null);
  const requestPreviewBusyRef = useRef(false);
  const requestPreviewSeqRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      analysisActionRef.current = null;
      requestPreviewBusyRef.current = false;
      requestPreviewSeqRef.current += 1;
    };
  }, []);

  useEffect(() => {
    projectIdRef.current = projectId;
    activeChapterIdRef.current = activeChapter?.id ?? null;
    requestPreviewBusyRef.current = false;
    requestPreviewSeqRef.current += 1;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [activeChapter?.id, projectId]);

  const isCurrentPreview = useCallback(
    (requestSeq: number, requestProjectId: number, requestChapterId: number) =>
      mountedRef.current &&
      requestPreviewSeqRef.current === requestSeq &&
      projectIdRef.current === requestProjectId &&
      activeChapterIdRef.current === requestChapterId,
    [],
  );

  const closeRequestPreview = useCallback((open: boolean) => {
    if (open) {
      setRequestPreviewOpen(true);
      return;
    }
    requestPreviewSeqRef.current += 1;
    requestPreviewBusyRef.current = false;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
  }, []);

  const requireSavedAnalyzableChapter = useCallback(
    (operation: "generate" | "resume"): ChapterSummary | null => {
      if (!activeChapter) {
        addToast(t("writing.analysisNoChapter"), "error");
        return null;
      }
      if (activeChapterBusy) {
        addToast(activeChapterGeneratingMessage, "error");
        return null;
      }
      if (dirtyRef.current) {
        addToast(
          operation === "generate"
            ? t("writing.analysisSaveFirstGenerate")
            : t("writing.analysisSaveFirstResume"),
          "error",
        );
        return null;
      }
      if (!activeChapter.content.trim()) {
        addToast(
          operation === "generate"
            ? t("writing.analysisEmptyGenerate")
            : t("writing.analysisEmptyResume"),
          "error",
        );
        return null;
      }
      if (analysisBlockedReason) {
        addToast(analysisBlockedReason, "error");
        return null;
      }
      if (operation === "resume" && !activeChapter.analysis_sync_can_resume) {
        addToast(
          activeChapter.analysis_sync_resume_block_reason ||
            t("writing.analysisNoResume"),
          "error",
        );
        return null;
      }
      return activeChapter;
    },
    [
      activeChapter,
      activeChapterBusy,
      activeChapterGeneratingMessage,
      addToast,
      analysisBlockedReason,
      dirtyRef,
      t,
    ],
  );

  const handleGenerateAnalysis = useCallback(async () => {
    if (analysisActionRef.current) {
      addToast(t("writing.analysisRunning"), "error");
      return;
    }
    analysisActionRef.current = "generate";
    try {
      const chapter = requireSavedAnalyzableChapter("generate");
      if (!chapter) return;

      const taskId = createWorkflowTaskId("chapter-analysis", projectId);
      setAnalysisBusy(true);
      const mutation = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          streaming: true,
          title: t("writing.analysisTaskTitle"),
          startMessage: t("writing.analysisPreparing"),
          successMessage: t("writing.analysisSaved"),
          errorMessage: t("writing.analysisFailed"),
          total: CHAPTER_ANALYSIS_PROGRESS_TOTAL,
          cancelLabel: t("writing.analysisStop"),
          runningHint: t("writing.analysisRunningHint"),
        },
        () =>
          generateChapterAnalysis(
            {
              chapter_id: chapter.id,
            },
            taskId,
          ),
      );
      const updated = mutation.chapter;
      setChapters((prev) =>
        prev.map((item) => (item.id === updated.id ? updated : item)),
      );
      await reloadChapters({ force: true });
      await reloadOutlines();
      refreshProfiles();
      if (mutation.warnings.length > 0) {
        addToast(
          t("writing.analysisWarningsRetry", {
            warnings: mutation.warnings.join("\n"),
          }),
          "info",
        );
      } else {
        addToast(t("writing.analysisSavedSynced"), "success");
      }
    } catch (err) {
      const errorMessage = extractErrorMessage(err, t("writing.analysisFailed"));
      const refreshed = await reloadChapters({ force: true });
      await reloadOutlines();
      refreshProfiles();
      const refreshedChapter =
        refreshed?.find((item) => item.id === activeChapterIdRef.current) ?? null;
      if (isCancelledOperation(errorMessage)) {
        addToast(t("writing.analysisStopped"), "info");
        return;
      }
      if (refreshedChapter?.analysis_sync_can_resume) {
        addToast(
          refreshedChapter.analysis_sync_has_saved_analysis
            ? t("writing.analysisResumeSavedRetry", { error: errorMessage })
            : t("writing.analysisResumeRetry", { error: errorMessage }),
          "info",
        );
        return;
      }
      addToast(errorMessage, "error");
    } finally {
      analysisActionRef.current = null;
      setAnalysisBusy(false);
    }
  }, [
    addToast,
    projectId,
    refreshProfiles,
    reloadChapters,
    reloadOutlines,
    requireSavedAnalyzableChapter,
    runProgressTask,
    setChapters,
    t,
  ]);

  const handleResumeAnalysisSync = useCallback(async () => {
    if (analysisActionRef.current) {
      addToast(t("writing.analysisRunning"), "error");
      return;
    }
    analysisActionRef.current = "resume";
    try {
      const chapter = requireSavedAnalyzableChapter("resume");
      if (!chapter) return;

      const taskId = createWorkflowTaskId("chapter-analysis-resume", projectId);
      setAnalysisBusy(true);
      const updated = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          streaming: true,
          title: t("writing.resumeAnalysisTitle"),
          startMessage: t("writing.resumeAnalysisPreparing"),
          successMessage: t("writing.resumeAnalysisComplete"),
          errorMessage: t("writing.resumeAnalysisFailed"),
          total: CHAPTER_ANALYSIS_PROGRESS_TOTAL,
          cancelLabel: t("writing.analysisStop"),
          runningHint: t("writing.analysisRunningHint"),
        },
        () =>
          resumeChapterAnalysisSync(
            {
              chapter_id: chapter.id,
            },
            taskId,
          ),
      );
      setChapters((prev) =>
        prev.map((item) => (item.id === updated.id ? updated : item)),
      );
      await reloadChapters({ force: true });
      await reloadOutlines();
      refreshProfiles();
      addToast(t("writing.resumeAnalysisComplete"), "success");
    } catch (err) {
      const errorMessage = extractErrorMessage(err, t("writing.resumeAnalysisFailed"));
      void reloadChapters({ force: true });
      void reloadOutlines();
      refreshProfiles();
      addToast(
        isCancelledOperation(errorMessage) ? t("writing.resumeAnalysisStopped") : errorMessage,
        isCancelledOperation(errorMessage) ? "info" : "error",
      );
    } finally {
      analysisActionRef.current = null;
      setAnalysisBusy(false);
    }
  }, [
    addToast,
    projectId,
    refreshProfiles,
    reloadChapters,
    reloadOutlines,
    requireSavedAnalyzableChapter,
    runProgressTask,
    setChapters,
    t,
  ]);

  const handlePreviewAnalysisRequest = useCallback(async () => {
    if (requestPreviewBusyRef.current) {
      addToast(t("writing.requestPreviewBusy"), "error");
      return;
    }
    const chapter = requireSavedAnalyzableChapter("generate");
    if (!chapter) return;
    const requestProjectId = projectId;
    const requestChapterId = chapter.id;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    requestPreviewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewChapterAnalysisRequest({ chapter_id: chapter.id });
      if (!isCurrentPreview(requestSeq, requestProjectId, requestChapterId)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentPreview(requestSeq, requestProjectId, requestChapterId)) return;
      addToast(extractErrorMessage(err, t("writing.analysisPreviewFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentPreview(requestSeq, requestProjectId, requestChapterId)) {
        requestPreviewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  }, [addToast, isCurrentPreview, projectId, requireSavedAnalyzableChapter, t]);

  const handlePreviewResumeAnalysisSyncRequest = useCallback(async () => {
    if (requestPreviewBusyRef.current) {
      addToast(t("writing.requestPreviewBusy"), "error");
      return;
    }
    const chapter = requireSavedAnalyzableChapter("resume");
    if (!chapter) return;
    const requestProjectId = projectId;
    const requestChapterId = chapter.id;
    const requestSeq = requestPreviewSeqRef.current + 1;
    requestPreviewSeqRef.current = requestSeq;
    requestPreviewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewResumeChapterAnalysisSyncRequest({
        chapter_id: chapter.id,
      });
      if (!isCurrentPreview(requestSeq, requestProjectId, requestChapterId)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (!isCurrentPreview(requestSeq, requestProjectId, requestChapterId)) return;
      addToast(extractErrorMessage(err, t("writing.resumePreviewFailed")), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentPreview(requestSeq, requestProjectId, requestChapterId)) {
        requestPreviewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  }, [addToast, isCurrentPreview, projectId, requireSavedAnalyzableChapter, t]);

  return {
    analysisBusy,
    handleGenerateAnalysis,
    handleResumeAnalysisSync,
    handlePreviewAnalysisRequest,
    handlePreviewResumeAnalysisSyncRequest,
    requestPreviewOpen,
    requestPreviewLoading,
    requestPreviewData,
    setRequestPreviewOpen: closeRequestPreview,
  };
}
