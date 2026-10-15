import {
  useCallback,
  useEffect,
  useRef,
  useState,
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
import {
  cancelWorkflowTask,
  getWorkflowRunDetail,
  listWorkflowRuns,
} from "../../api/streaming";
import type { ModelRequestPreview } from "../../api/requestPreview";
import type { ToastType } from "../../components/feedback/Toast";
import { choiceDialog } from "../../components/feedback/ConfirmDialog";
import { createWorkflowTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { useI18n } from "../../i18n";
import type { ReloadChapters } from "./chapterIndex";

type AddToast = (message: string, type: ToastType) => void;

export const waitForWorkflowRun = async (workflowRunId: number) => {
  for (;;) {
    const detail = await getWorkflowRunDetail(workflowRunId, false);
    if (detail.run.status !== "running" && detail.run.status !== "pending") {
      return detail.run;
    }
    await new Promise((resolve) => globalThis.setTimeout(resolve, 500));
  }
};

interface UseChapterAnalysisActionsParams {
  projectId: number;
  activeChapter: ChapterSummary | null;
  activeChapterBusy: boolean;
  activeChapterGeneratingMessage: string;
  analysisBlockedReason: string | null;
  dirtyRef: { current: boolean };
  addToast: AddToast;
  reloadChapters: ReloadChapters;
  reloadOutlines: () => Promise<boolean>;
  applyChapter: (chapter: ChapterSummary) => void;
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
  applyChapter,
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
  projectIdRef.current = projectId;
  activeChapterIdRef.current = activeChapter?.id ?? null;
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

  const isCurrentChapter = useCallback(
    (requestProjectId: number, requestChapterId: number) =>
      mountedRef.current &&
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
      if (operation === "generate" && activeChapter.analysis_sync_status === "running") {
        addToast(t("writing.analysisRunning"), "error");
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
      if (
        operation === "resume" &&
        !activeChapter.analysis_sync_can_resume &&
        activeChapter.analysis_sync_status !== "running"
      ) {
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

  const runAnalysisProgressTask = useCallback(
    <T,>(taskId: string | null, task: () => Promise<T>) =>
      runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: taskId ? cancelWorkflowTask : undefined,
          pollTaskProgress: taskId != null,
          streaming: taskId != null,
          title: t("writing.resumeAnalysisTitle"),
          startMessage: t("writing.resumeAnalysisPreparing"),
          successMessage: t("writing.resumeAnalysisComplete"),
          errorMessage: t("writing.resumeAnalysisFailed"),
          total: CHAPTER_ANALYSIS_PROGRESS_TOTAL,
          cancelLabel: t("writing.analysisStop"),
          runningHint: t("writing.analysisRunningHint"),
        },
        task,
      ),
    [runProgressTask, t],
  );

  const refreshAnalysisReadModels = useCallback(
    (chapterId: number) => {
      void reloadChapters({ force: true, chapterId });
      void reloadOutlines();
    },
    [reloadChapters, reloadOutlines],
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
      const requestProjectId = projectId;
      const requestChapterId = chapter.id;
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
        async () => {
          if (!isCurrentChapter(requestProjectId, requestChapterId)) {
            throw new Error(t("writing.analysisStopped"));
          }
          return generateChapterAnalysis(
            {
              chapter_id: chapter.id,
            },
            taskId,
          );
        },
      );
      if (!isCurrentChapter(requestProjectId, requestChapterId)) return;
      const updated = mutation.chapter;
      applyChapter(updated);
      refreshProfiles();
      refreshAnalysisReadModels(updated.id);
      if (mutation.warnings.length > 0) {
        addToast(
          t("writing.analysisWarningsRetry", {
            warnings: mutation.warnings.join("\n"),
          }),
          "info",
        );
      } else {
        addToast(t("writing.analysisSaved"), "success");
      }
    } catch (err) {
      if (!isCurrentChapter(projectId, activeChapterIdRef.current ?? -1)) return;
      const errorMessage = extractErrorMessage(err, t("writing.analysisFailed"));
      const refreshed = await reloadChapters({ force: true });
      await reloadOutlines();
      refreshProfiles();
      const refreshedChapter =
        refreshed?.index.find((item) => item.id === activeChapterIdRef.current) ?? null;
      if (isCancelledOperation(errorMessage)) {
        addToast(t("writing.analysisStopped"), "info");
        return;
      }
      if (refreshedChapter?.analysis_sync_status === "running") {
        addToast(t("writing.analysisRunningInBackground"), "info");
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
    refreshAnalysisReadModels,
    requireSavedAnalyzableChapter,
    runProgressTask,
    isCurrentChapter,
    applyChapter,
    t,
  ]);

  const handleResumeAnalysisSync = useCallback(async () => {
    if (analysisActionRef.current) {
      addToast(t("writing.analysisRunning"), "error");
      return;
    }
    analysisActionRef.current = "resume";
    let readModelRefreshAttempted = false;
    try {
      const chapter = requireSavedAnalyzableChapter("resume");
      if (!chapter) return;
      const requestProjectId = projectId;
      const requestChapterId = chapter.id;

      setAnalysisBusy(true);
      const runningRun = (
        await listWorkflowRuns({
          chapter_id: chapter.id,
          limit: 1,
          project_id: projectId,
          resumable_only: false,
          status: "running",
          workflow_kind: "chapter_analysis",
        })
      )[0];
      let updated: ChapterSummary | null = null;
      if (runningRun) {
        const takeoverChoice = await choiceDialog({
          title: t("writing.analysisRunningTakeoverTitle"),
          message: t("writing.analysisRunningTakeoverMessage"),
          confirmText: t("writing.analysisRunningTakeoverConfirm"),
          cancelText: t("writing.analysisRunningTakeoverCancel"),
          secondaryText: t("writing.analysisRunningTakeoverCancel"),
          dialogType: "warning",
        });
        if (takeoverChoice !== "confirm" || !isCurrentChapter(requestProjectId, requestChapterId)) return;

        const taskId = runningRun.active_task_id;
        const terminalRun = await runAnalysisProgressTask(taskId, () =>
          isCurrentChapter(requestProjectId, requestChapterId)
            ? waitForWorkflowRun(runningRun.id)
            : Promise.reject(new Error(t("writing.analysisStopped"))),
        );
        if (terminalRun.status !== "completed") {
          throw new Error(
            terminalRun.status === "cancelled" || terminalRun.status === "interrupted"
              ? t("writing.resumeAnalysisStopped")
              : terminalRun.error_message || t("writing.resumeAnalysisFailed"),
          );
        }
      } else {
        const taskId = createWorkflowTaskId("chapter-analysis-resume", projectId);
        updated = await runAnalysisProgressTask(
          taskId,
          () =>
            isCurrentChapter(requestProjectId, requestChapterId)
              ? resumeChapterAnalysisSync({ chapter_id: chapter.id }, taskId)
              : Promise.reject(new Error(t("writing.analysisStopped"))),
        );
      }
      if (!isCurrentChapter(requestProjectId, requestChapterId)) return;
      if (updated) {
        applyChapter(updated);
      }
      refreshAnalysisReadModels(chapter.id);
      refreshProfiles();
      addToast(t("writing.resumeAnalysisComplete"), "success");
    } catch (err) {
      if (!isCurrentChapter(projectId, activeChapterIdRef.current ?? -1)) return;
      const errorMessage = extractErrorMessage(err, t("writing.resumeAnalysisFailed"));
      if (!readModelRefreshAttempted) {
        void reloadChapters({ force: true });
        void reloadOutlines();
        refreshProfiles();
      }
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
    refreshAnalysisReadModels,
    requireSavedAnalyzableChapter,
    runAnalysisProgressTask,
    isCurrentChapter,
    applyChapter,
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
