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
        addToast("请先选择已保存的章节", "error");
        return null;
      }
      if (activeChapterBusy) {
        addToast(activeChapterGeneratingMessage, "error");
        return null;
      }
      if (dirtyRef.current) {
        addToast(
          operation === "generate"
            ? "请先保存正文，再生成结构化分析"
            : "请先保存正文，再继续章节结构化分析",
          "error",
        );
        return null;
      }
      if (!activeChapter.content.trim()) {
        addToast(
          operation === "generate"
            ? "章节正文为空，不能生成结构化分析"
            : "章节正文为空，不能继续章节结构化分析",
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
            "当前章节分析无需继续",
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
    ],
  );

  const handleGenerateAnalysis = useCallback(async () => {
    if (analysisActionRef.current) {
      addToast("章节结构化分析任务正在进行", "error");
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
          title: "章节结构化分析",
          startMessage: "正在准备章节分析",
          successMessage: "结构化分析已保存",
          errorMessage: "结构化分析失败",
          total: CHAPTER_ANALYSIS_PROGRESS_TOTAL,
          cancelLabel: "停止分析",
          runningHint: "结构化分析过程中已写入的数据会保留",
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
          `${mutation.warnings.join("\n")}\n请点击「继续分析」重试`,
          "info",
        );
      } else {
        addToast("结构化分析已保存，项目状态已同步", "success");
      }
    } catch (err) {
      const errorMessage = extractErrorMessage(err, "结构化分析失败");
      const refreshed = await reloadChapters({ force: true });
      await reloadOutlines();
      refreshProfiles();
      const refreshedChapter =
        refreshed?.find((item) => item.id === activeChapterIdRef.current) ?? null;
      if (isCancelledOperation(errorMessage)) {
        addToast("章节结构化分析已停止", "info");
        return;
      }
      if (refreshedChapter?.analysis_sync_can_resume) {
        addToast(
          refreshedChapter.analysis_sync_has_saved_analysis
            ? `结构化分析已保存，但派生数据同步未完成。请点击「继续分析」重试：${errorMessage}`
            : `章节分析已有可恢复进度。请点击「继续分析」从断点恢复：${errorMessage}`,
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
  ]);

  const handleResumeAnalysisSync = useCallback(async () => {
    if (analysisActionRef.current) {
      addToast("章节结构化分析任务正在进行", "error");
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
          title: "继续章节分析",
          startMessage: "正在准备恢复章节分析",
          successMessage: "章节分析已继续完成",
          errorMessage: "继续分析失败",
          total: CHAPTER_ANALYSIS_PROGRESS_TOTAL,
          cancelLabel: "停止分析",
          runningHint: "结构化分析过程中已写入的数据会保留",
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
      addToast("章节分析已继续完成", "success");
    } catch (err) {
      const errorMessage = extractErrorMessage(err, "继续分析失败");
      void reloadChapters({ force: true });
      void reloadOutlines();
      refreshProfiles();
      addToast(
        isCancelledOperation(errorMessage) ? "章节分析恢复已停止" : errorMessage,
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
  ]);

  const handlePreviewAnalysisRequest = useCallback(async () => {
    if (requestPreviewBusyRef.current) {
      addToast("正在生成请求预览", "error");
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
      addToast(extractErrorMessage(err, "预览结构化分析提示词失败"), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentPreview(requestSeq, requestProjectId, requestChapterId)) {
        requestPreviewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  }, [addToast, isCurrentPreview, projectId, requireSavedAnalyzableChapter]);

  const handlePreviewResumeAnalysisSyncRequest = useCallback(async () => {
    if (requestPreviewBusyRef.current) {
      addToast("正在生成请求预览", "error");
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
      addToast(extractErrorMessage(err, "预览继续同步提示词失败"), "error");
      setRequestPreviewOpen(false);
    } finally {
      if (isCurrentPreview(requestSeq, requestProjectId, requestChapterId)) {
        requestPreviewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  }, [addToast, isCurrentPreview, projectId, requireSavedAnalyzableChapter]);

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
