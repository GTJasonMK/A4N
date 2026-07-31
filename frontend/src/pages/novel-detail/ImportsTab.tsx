import React, { useEffect, useRef, useState, type ChangeEvent } from "react";
import { useToast } from "../../components/feedback/Toast";
import { ProgressDialog } from "../../components/feedback/ProgressDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../../api";
import {
  acceptImportReview,
  batchAcceptReviews,
  cancelImportAnalysis,
  checkImportDedup,
  deleteImportReview,
  getImportAnalysisStatus,
  getProjectImportAnalysisProgress,
  getImportReview,
  importUploadedFile,
  listImportQueue,
  modifyImportReview,
  previewImportProjectAnalysisRequest,
  previewReverseEngineerBlueprintRequest,
  previewReverseEngineerOutlineRequest,
  previewReverseEngineerPartOutlineRequest,
  rejectImportReview,
  reverseEngineerBlueprint,
  reverseEngineerBlueprintAndApply,
  reverseEngineerOutline,
  reverseEngineerOutlineAndApply,
  reverseEngineerPartOutline,
  reverseEngineerPartOutlineAndApply,
  runImportProjectAnalysis,
  startImportAnalysis,
  updateImportStatus,
  type ImportAnalysisStatus,
  type ImportReview,
  type ImportReviewStatus,
  type ProjectImportAnalysisProgress,
} from "../../api/import";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { allocateHostTaskId, useProgressTask } from "../../hooks/useProgressTask";
import { isCancelledOperation } from "../../utils/cancellation";
import { ImportFilePanel } from "./imports/ImportFilePanel";
import { ImportReviewDetailPanel } from "./imports/ImportReviewDetailPanel";
import {
  ImportReviewQueuePanel,
  type ReviewAction,
} from "./imports/ImportReviewQueuePanel";
import { ImportToolsPanel } from "./imports/ImportToolsPanel";
import {
  REVERSE_CONTEXT_LIMITS,
  dedupStats,
  delay,
  emptyImportAnalysisDialog,
  importAnalysisDialogFromProgress,
  type ImportAnalysisDialogState,
  type ReverseType,
} from "./imports/importTabUtils";
import { readFileAsBase64 } from "../../utils/fileUpload";

const MAX_IMPORT_UPLOAD_BYTES = 100 * 1024 * 1024;

const importFileTypeFromName = (fileName: string): string | null => {
  const extension = fileName.trim().toLowerCase().split(".").pop();
  if (extension === "txt") return "txt";
  if (extension === "md" || extension === "markdown") return "markdown";
  if (extension === "epub") return "epub";
  if (extension === "docx") return "docx";
  return null;
};

interface ImportsTabProps {
  projectId: number;
  onBusyChange?: (busy: boolean) => void;
}

export const ImportsTab: React.FC<ImportsTabProps> = ({ projectId, onBusyChange }) => {
  const { addToast } = useToast();
  const { runProgressTask: runReverseProgressTask } = useProgressTask();
  const [reviews, setReviews] = useState<ImportReview[]>([]);
  const [loading, setLoading] = useState(false);
  const [analysis, setAnalysis] = useState<ImportAnalysisStatus[]>([]);
  const [projectProgress, setProjectProgress] = useState<ProjectImportAnalysisProgress | null>(null);
  const [importBusy, setImportBusy] = useState(false);
  const [analyzeBusy, setAnalyzeBusy] = useState(false);
  const [cancelBusy, setCancelBusy] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [batchBusy, setBatchBusy] = useState(false);
  const [activeReview, setActiveReview] = useState<ImportReview | null>(null);
  const [activeReviewJson, setActiveReviewJson] = useState("");
  const [activeReviewJsonError, setActiveReviewJsonError] = useState<string | null>(null);
  const [activeReviewJsonDirty, setActiveReviewJsonDirty] = useState(false);
  const activeReviewRef = useRef<ImportReview | null>(null);
  const activeReviewJsonDirtyRef = useRef(false);
  const reviewsRef = useRef<ImportReview[]>([]);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const loadRequestRef = useRef(0);
  const importSeqRef = useRef(0);
  const analysisSeqRef = useRef(0);
  const cancelAnalysisSeqRef = useRef(0);
  const batchSeqRef = useRef(0);
  const actSeqRef = useRef(0);
  const detailSeqRef = useRef(0);
  const dedupSeqRef = useRef(0);
  const dedupInputVersionRef = useRef(0);
  const reverseSeqRef = useRef(0);
  const requestPreviewSeqRef = useRef(0);
  const detailPanelRef = useRef<HTMLDivElement | null>(null);
  const importAnalysisDialogDismissedRef = useRef(false);
  const importBusyRef = useRef(false);
  const analyzeBusyRef = useRef(false);
  const cancelBusyRef = useRef(false);
  const batchBusyRef = useRef(false);
  const actBusyRef = useRef<string | null>(null);
  const dedupBusyRef = useRef(false);
  const reverseBusyRef = useRef(false);
  const requestPreviewBusyRef = useRef(false);
  const [actBusy, setActBusy] = useState<string | null>(null);
  const [dedupText, setDedupText] = useState("");
  const [dedupResult, setDedupResult] = useState<unknown>(null);
  const [dedupBusy, setDedupBusy] = useState(false);
  const [reverseType, setReverseType] = useState<ReverseType>("blueprint");
  const [reverseSample, setReverseSample] = useState("");
  const [reverseAutoContext, setReverseAutoContext] = useState(true);
  const [reverseResult, setReverseResult] = useState<unknown>(null);
  const [reverseBusy, setReverseBusy] = useState<"preview" | "apply" | null>(null);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [requestPreviewTitle, setRequestPreviewTitle] = useState("模型请求预览");
  const [importAnalysisDialog, setImportAnalysisDialog] = useState<ImportAnalysisDialogState>(
    emptyImportAnalysisDialog,
  );
  const writeBusy =
    importBusy ||
    analyzeBusy ||
    cancelBusy ||
    batchBusy ||
    actBusy !== null ||
    reverseBusy === "apply";

  // 导入新文件
  const [importFileName, setImportFileName] = useState("");
  const [importUploadFile, setImportUploadFile] = useState<File | null>(null);
  const [fileType, setFileType] = useState("txt");
  const importFileInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    onBusyChange?.(writeBusy);
  }, [onBusyChange, writeBusy]);

  useEffect(
    () => () => {
      onBusyChange?.(false);
    },
    [onBusyChange],
  );

  const setActiveReviewState = (review: ImportReview | null) => {
    activeReviewRef.current = review;
    setActiveReview(review);
  };

  const setActiveReviewJsonState = (value: string, dirty: boolean) => {
    activeReviewJsonDirtyRef.current = dirty;
    setActiveReviewJson(value);
    setActiveReviewJsonDirty(dirty);
  };

  const setProgressDialogFromProgress = (progress: ProjectImportAnalysisProgress) => {
    const dialog = importAnalysisDialogFromProgress(progress);
    setImportAnalysisDialog(
      importAnalysisDialogDismissedRef.current ? { ...dialog, open: false } : dialog,
    );
  };

  const nextSequence = (ref: React.MutableRefObject<number>) => {
    ref.current += 1;
    return ref.current;
  };

  const isCurrentProject = (requestProjectId: number) =>
    mountedRef.current && activeProjectIdRef.current === requestProjectId;

  const isCurrentAction = (
    requestProjectId: number,
    ref: React.MutableRefObject<number>,
    sequence: number,
  ) => isCurrentProject(requestProjectId) && ref.current === sequence;

  const canCancelAnalysis = () =>
    analyzeBusy ||
    projectProgress?.status === "analyzing" ||
    analysis.some((item) => item.status === "analyzing" && item.count > 0);
  const analysisInProgress = canCancelAnalysis();
  const fileExternalBusy =
    analysisInProgress ||
    batchBusy ||
    actBusy !== null ||
    dedupBusy ||
    reverseBusy !== null;
  const reviewExternalBusy = importBusy || dedupBusy || reverseBusy !== null;
  const detailExternalBusy =
    importBusy ||
    analysisInProgress ||
    batchBusy ||
    dedupBusy ||
    reverseBusy !== null;
  const toolsExternalBusy =
    importBusy || analysisInProgress || batchBusy || actBusy !== null;

  const invalidateProjectScopedRequests = () => {
    loadRequestRef.current += 1;
    importSeqRef.current += 1;
    analysisSeqRef.current += 1;
    cancelAnalysisSeqRef.current += 1;
    batchSeqRef.current += 1;
    actSeqRef.current += 1;
    detailSeqRef.current += 1;
    dedupSeqRef.current += 1;
    reverseSeqRef.current += 1;
    requestPreviewSeqRef.current += 1;
    importBusyRef.current = false;
    analyzeBusyRef.current = false;
    cancelBusyRef.current = false;
    batchBusyRef.current = false;
    actBusyRef.current = null;
    dedupBusyRef.current = false;
    reverseBusyRef.current = false;
    requestPreviewBusyRef.current = false;
  };

  const load = async (requestProjectId = projectId) => {
    const requestId = loadRequestRef.current + 1;
    loadRequestRef.current = requestId;
    setLoading(true);
    try {
      const [list, st, progress] = await Promise.all([
        listImportQueue(requestProjectId),
        getImportAnalysisStatus(requestProjectId),
        getProjectImportAnalysisProgress(requestProjectId),
      ]);
      if (!isCurrentProject(requestProjectId) || loadRequestRef.current !== requestId) return;
      reviewsRef.current = list;
      setReviews(list);
      setAnalysis(st);
      setProjectProgress(progress);
      const reviewIds = new Set(list.map((review) => review.id));
      setSelected((current) => {
        const next = new Set([...current].filter((id) => reviewIds.has(id)));
        return next.size === current.size ? current : next;
      });
      const currentActiveReview = activeReviewRef.current;
      if (currentActiveReview) {
        const updatedActiveReview = list.find((review) => review.id === currentActiveReview.id);
        if (updatedActiveReview) {
          setActiveReviewState(updatedActiveReview);
          if (!activeReviewJsonDirtyRef.current) {
            setActiveReviewJsonState(
              JSON.stringify(updatedActiveReview.analysis_result ?? {}, null, 2),
              false,
            );
            setActiveReviewJsonError(null);
          }
        } else {
          setActiveReviewState(null);
          setActiveReviewJsonState("", false);
          setActiveReviewJsonError(null);
        }
      }
    } catch (err) {
      if (isCurrentProject(requestProjectId) && loadRequestRef.current === requestId) {
        addToast(extractErrorMessage(err, "加载导入队列失败"), "error");
      }
    } finally {
      if (isCurrentProject(requestProjectId) && loadRequestRef.current === requestId) {
        setLoading(false);
      }
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      invalidateProjectScopedRequests();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    invalidateProjectScopedRequests();
    reviewsRef.current = [];
    setReviews([]);
    setAnalysis([]);
    setProjectProgress(null);
    setLoading(false);
    setImportBusy(false);
    setImportFileName("");
    setImportUploadFile(null);
    setAnalyzeBusy(false);
    setCancelBusy(false);
    setBatchBusy(false);
    setActBusy(null);
    setDedupBusy(false);
    setDedupResult(null);
    setReverseBusy(null);
    setReverseResult(null);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setRequestPreviewTitle("模型请求预览");
    importAnalysisDialogDismissedRef.current = false;
    setImportAnalysisDialog(emptyImportAnalysisDialog);
    setSelected(new Set());
    setActiveReviewState(null);
    setActiveReviewJsonState("", false);
    setActiveReviewJsonError(null);
    setActiveReviewJsonDirty(false);
    void load(projectId);
    const t = setInterval(() => {
      void load(projectId);
    }, 5000);
    return () => {
      loadRequestRef.current += 1;
      clearInterval(t);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  const handleImport = async () => {
    const requestFile = importUploadFile;
    const requestFileName = importFileName.trim();
    const requestFileType = fileType;
    const requestProjectId = projectId;
    if (!requestFile || !requestFileName) {
      addToast("请选择要导入的文件", "error");
      return;
    }
    if (requestFile.size > MAX_IMPORT_UPLOAD_BYTES) {
      addToast(`导入文件不能超过 ${MAX_IMPORT_UPLOAD_BYTES / 1024 / 1024}MB`, "error");
      return;
    }
    if (importBusyRef.current) return;
    importBusyRef.current = true;
    const requestSeq = nextSequence(importSeqRef);
    setImportBusy(true);
    try {
      const dataBase64 = await readFileAsBase64(requestFile);
      if (!isCurrentAction(requestProjectId, importSeqRef, requestSeq)) return;
      const parsed = await importUploadedFile({
        file_name: requestFileName,
        file_type: requestFileType,
        data_base64: dataBase64,
      });
      if (!isCurrentAction(requestProjectId, importSeqRef, requestSeq)) return;
      if (parsed.chapters.length === 0) {
        addToast("未解析到章节", "error");
        return;
      }
      const stats = dedupStats(parsed.chapters.map((chapter) => chapter.content_hash));
      const reviews = await startImportAnalysis(requestProjectId, {
        filename: requestFileName,
        file_type: requestFileType,
        detected_encoding: parsed.detected_encoding,
        chapters: parsed.chapters,
        dedup_paragraphs: stats.dedup_paragraphs,
        dedup_rate: stats.dedup_rate,
      });
      if (!isCurrentAction(requestProjectId, importSeqRef, requestSeq)) return;
      addToast(`已创建 ${reviews.length} 个审核项`, "success");
      setImportFileName("");
      setImportUploadFile(null);
      await load(requestProjectId);
    } catch (err) {
      if (isCurrentAction(requestProjectId, importSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "导入失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestProjectId, importSeqRef, requestSeq)) {
        setImportBusy(false);
      }
      if (importSeqRef.current === requestSeq) {
        importBusyRef.current = false;
      }
    }
  };

  const handleSelectImportFile = () => {
    if (importBusyRef.current) return;
    importFileInputRef.current?.click();
  };

  const handleImportFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.currentTarget.files?.[0] ?? null;
    event.currentTarget.value = "";
    if (!file) return;
    if (file.size > MAX_IMPORT_UPLOAD_BYTES) {
      addToast(`导入文件不能超过 ${MAX_IMPORT_UPLOAD_BYTES / 1024 / 1024}MB`, "error");
      return;
    }
    setImportUploadFile(file);
    setImportFileName(file.name);
    const inferredType = importFileTypeFromName(file.name);
    if (inferredType) setFileType(inferredType);
  };

  const handleStartAnalysis = async () => {
    if (analyzeBusyRef.current) return;
    analyzeBusyRef.current = true;
    const requestProjectId = projectId;
    const requestSeq = nextSequence(analysisSeqRef);
    setAnalyzeBusy(true);
    let polling = true;
    importAnalysisDialogDismissedRef.current = false;
    setImportAnalysisDialog({
      open: true,
      status: "running",
      current: 0,
      total: 1,
      message: "正在启动导入项目分析",
      error: null,
    });

    const pollProgress = async () => {
      while (polling && isCurrentAction(requestProjectId, analysisSeqRef, requestSeq)) {
        try {
          const progress = await getProjectImportAnalysisProgress(requestProjectId);
          if (!isCurrentAction(requestProjectId, analysisSeqRef, requestSeq)) return;
          if (progress) {
            setProjectProgress(progress);
            setProgressDialogFromProgress(progress);
            if (["completed", "failed", "cancelled"].includes(progress.status)) {
              return;
            }
          }
        } catch {
          // 主命令会返回明确错误；轮询失败不覆盖弹窗主状态。
        }
        await delay(1000);
      }
    };

    void pollProgress();
    try {
      const result = await runImportProjectAnalysis(requestProjectId);
      if (!isCurrentAction(requestProjectId, analysisSeqRef, requestSeq)) return;
      setProjectProgress(result.progress);
      setProgressDialogFromProgress(result.progress);
      if (result.progress.status === "cancelled") {
        addToast("导入项目分析已停止，已完成部分会保留", "success");
      } else {
        addToast(
          `已分析 ${result.analyzed_chapters} 章，更新 ${result.outlined_chapters} 个大纲`,
          "success",
        );
      }
      await load(requestProjectId);
    } catch (err) {
      if (!isCurrentAction(requestProjectId, analysisSeqRef, requestSeq)) return;
      const message = extractErrorMessage(err, "启动失败");
      setImportAnalysisDialog((prev) => ({
        ...prev,
        open: !importAnalysisDialogDismissedRef.current,
        status: "error",
        message: "导入项目分析失败",
        error: message,
      }));
      addToast(message, "error");
    } finally {
      polling = false;
      if (isCurrentAction(requestProjectId, analysisSeqRef, requestSeq)) {
        setAnalyzeBusy(false);
      }
      if (analysisSeqRef.current === requestSeq) {
        analyzeBusyRef.current = false;
      }
    }
  };

  const handlePreviewImportAnalysisRequest = async () => {
    if (requestPreviewBusyRef.current) return;
    requestPreviewBusyRef.current = true;
    const requestProjectId = projectId;
    const requestSeq = nextSequence(requestPreviewSeqRef);
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    setRequestPreviewTitle("导入项目分析请求预览");
    try {
      const preview = await previewImportProjectAnalysisRequest(requestProjectId);
      if (!isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "预览导入项目分析请求失败"), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq)) {
        setRequestPreviewLoading(false);
      }
      if (requestPreviewSeqRef.current === requestSeq) {
        requestPreviewBusyRef.current = false;
      }
    }
  };

  const handleCancelAnalysis = async () => {
    if (cancelBusyRef.current) return;
    if (!canCancelAnalysis()) {
      addToast("当前没有正在运行的导入项目分析", "info");
      return;
    }
    cancelBusyRef.current = true;
    const requestProjectId = projectId;
    const requestSeq = nextSequence(cancelAnalysisSeqRef);
    setCancelBusy(true);
    try {
      const n = await cancelImportAnalysis(requestProjectId);
      if (!isCurrentAction(requestProjectId, cancelAnalysisSeqRef, requestSeq)) return;
      analysisSeqRef.current += 1;
      setAnalyzeBusy(false);
      importAnalysisDialogDismissedRef.current = false;
      addToast(`已取消 ${n} 个待处理任务`, "success");
      const progress = await getProjectImportAnalysisProgress(requestProjectId);
      if (!isCurrentAction(requestProjectId, cancelAnalysisSeqRef, requestSeq)) return;
      if (progress) {
        setProjectProgress(progress);
        setProgressDialogFromProgress(progress);
      } else {
        setImportAnalysisDialog((prev) => ({
          ...prev,
          open: true,
          status: "cancelled",
          message: "导入分析已取消",
        }));
      }
      await load(requestProjectId);
    } catch (err) {
      if (isCurrentAction(requestProjectId, cancelAnalysisSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "取消失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestProjectId, cancelAnalysisSeqRef, requestSeq)) {
        setCancelBusy(false);
      }
      if (cancelAnalysisSeqRef.current === requestSeq) {
        cancelBusyRef.current = false;
      }
    }
  };

  const handleBatchAccept = async () => {
    if (batchBusyRef.current || actBusyRef.current != null) return;
    const requestProjectId = projectId;
    const arr = Array.from(selected);
    if (arr.length === 0) {
      addToast("请勾选要批量接受的审核", "error");
      return;
    }
    batchBusyRef.current = true;
    const requestSeq = nextSequence(batchSeqRef);
    setBatchBusy(true);
    try {
      const result = await batchAcceptReviews(arr);
      if (!isCurrentAction(requestProjectId, batchSeqRef, requestSeq)) return;
      if (result.some((review) => review.project_id !== requestProjectId)) {
        throw new Error("批量接受返回了其他项目的审核项");
      }
      const currentActiveId = activeReviewRef.current?.id;
      const updatedActiveReview = currentActiveId
        ? result.find((review) => review.id === currentActiveId)
        : null;
      if (updatedActiveReview) {
        setActiveReviewState(updatedActiveReview);
      }
      addToast(`已批量接受 ${result.length} 个`, "success");
      setSelected(new Set());
      await load(requestProjectId);
    } catch (err) {
      if (isCurrentAction(requestProjectId, batchSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "批量接受失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestProjectId, batchSeqRef, requestSeq)) {
        setBatchBusy(false);
      }
      if (batchSeqRef.current === requestSeq) {
        batchBusyRef.current = false;
      }
    }
  };

  const toggleSelect = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleAct = async (id: number, action: ReviewAction) => {
    if (batchBusyRef.current || actBusyRef.current != null) return;
    const requestProjectId = projectId;
    const review = reviewsRef.current.find((item) => item.id === id);
    if (!review || review.project_id !== requestProjectId) {
      addToast("审核项已不在当前项目队列中", "error");
      return;
    }
    const actionKey = `${action}-${id}`;
    actBusyRef.current = actionKey;
    const requestSeq = nextSequence(actSeqRef);
    setActBusy(actionKey);
    try {
      if (action === "accept") await acceptImportReview(id);
      else if (action === "reject") await rejectImportReview(id);
      else await deleteImportReview(id);
      if (!isCurrentAction(requestProjectId, actSeqRef, requestSeq)) return;
      addToast(`已${action === "accept" ? "接受" : action === "reject" ? "拒绝" : "删除"}`, "success");
      setSelected((prev) => {
        if (!prev.has(id)) return prev;
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
      if (activeReviewRef.current?.id === id) {
        setActiveReviewState(null);
        setActiveReviewJsonState("", false);
        setActiveReviewJsonError(null);
      }
      await load(requestProjectId);
    } catch (err) {
      if (isCurrentAction(requestProjectId, actSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "操作失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestProjectId, actSeqRef, requestSeq)) {
        setActBusy(null);
      }
      if (actSeqRef.current === requestSeq) {
        actBusyRef.current = null;
      }
    }
  };

  const handleUpdateStatus = async (id: number, status: ImportReviewStatus) => {
    if (batchBusyRef.current || actBusyRef.current != null) return;
    const requestProjectId = projectId;
    const review = reviewsRef.current.find((item) => item.id === id);
    if (!review || review.project_id !== requestProjectId) {
      addToast("审核项已不在当前项目队列中", "error");
      return;
    }
    actBusyRef.current = `status-${id}`;
    const requestSeq = nextSequence(actSeqRef);
    setActBusy(`status-${id}`);
    try {
      const updated = await updateImportStatus(id, status);
      if (!isCurrentAction(requestProjectId, actSeqRef, requestSeq)) return;
      if (updated.project_id !== requestProjectId) {
        throw new Error("状态更新返回了其他项目的审核项");
      }
      setReviews((prev) =>
        prev.map((review) => (review.id === updated.id ? updated : review)),
      );
      reviewsRef.current = reviewsRef.current.map((review) =>
        review.id === updated.id ? updated : review,
      );
      if (activeReviewRef.current?.id === updated.id) {
        setActiveReviewState(updated);
      }
      addToast("状态已更新", "success");
      await load(requestProjectId);
    } catch (err) {
      if (isCurrentAction(requestProjectId, actSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "更新失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestProjectId, actSeqRef, requestSeq)) {
        setActBusy(null);
      }
      if (actSeqRef.current === requestSeq) {
        actBusyRef.current = null;
      }
    }
  };

  const handleViewDetail = async (id: number) => {
    const requestProjectId = projectId;
    const requestSeq = nextSequence(detailSeqRef);
    try {
      const detail = await getImportReview(id);
      if (!isCurrentAction(requestProjectId, detailSeqRef, requestSeq)) return;
      if (detail.project_id !== requestProjectId) {
        throw new Error("详情返回了其他项目的审核项");
      }
      setActiveReviewState(detail);
      setActiveReviewJsonState(JSON.stringify(detail.analysis_result ?? {}, null, 2), false);
      setActiveReviewJsonError(null);
      if (window.matchMedia("(max-width: 1023px)").matches) {
        requestAnimationFrame(() => {
          detailPanelRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
        });
      }
    } catch (err) {
      if (isCurrentAction(requestProjectId, detailSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "查看失败"), "error");
      }
    }
  };

  const handleAnalysisJsonChange = (value: string) => {
    setActiveReviewJsonState(value, true);
    try {
      const parsed = JSON.parse(value);
      const current = activeReviewRef.current;
      if (current) {
        setActiveReviewState({ ...current, analysis_result: parsed });
      }
      setActiveReviewJsonError(null);
    } catch {
      setActiveReviewJsonError("JSON 格式无效");
    }
  };

  const handleModify = async () => {
    if (batchBusyRef.current || actBusyRef.current != null) return;
    if (!activeReviewJsonDirtyRef.current) return;
    const requestReview = activeReviewRef.current;
    if (!requestReview) return;
    const requestProjectId = projectId;
    if (requestReview.project_id !== requestProjectId) {
      addToast("审核项已不在当前项目队列中", "error");
      return;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(activeReviewJson);
      setActiveReviewJsonError(null);
    } catch {
      setActiveReviewJsonError("JSON 格式无效");
      return;
    }
    actBusyRef.current = `modify-${requestReview.id}`;
    const requestSeq = nextSequence(actSeqRef);
    setActBusy(`modify-${requestReview.id}`);
    try {
      const updated = await modifyImportReview(requestReview.id, {
        analysis_result: parsed,
      });
      if (!isCurrentAction(requestProjectId, actSeqRef, requestSeq)) return;
      if (updated.project_id !== requestProjectId) {
        throw new Error("修改返回了其他项目的审核项");
      }
      setActiveReviewState(updated);
      setActiveReviewJsonState(JSON.stringify(updated.analysis_result ?? {}, null, 2), false);
      addToast("修改已保存", "success");
      await load(requestProjectId);
    } catch (err) {
      if (isCurrentAction(requestProjectId, actSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "修改失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestProjectId, actSeqRef, requestSeq)) {
        setActBusy(null);
      }
      if (actSeqRef.current === requestSeq) {
        actBusyRef.current = null;
      }
    }
  };

  const handleDedupTextChange = (value: string) => {
    dedupInputVersionRef.current += 1;
    dedupSeqRef.current += 1;
    dedupBusyRef.current = false;
    setDedupText(value);
    setDedupResult(null);
    setDedupBusy(false);
  };

  const isCurrentDedupRequest = (
    requestProjectId: number,
    requestSeq: number,
    requestInputVersion: number,
  ) =>
    isCurrentAction(requestProjectId, dedupSeqRef, requestSeq) &&
    dedupInputVersionRef.current === requestInputVersion;

  const handleDedup = async () => {
    if (dedupBusyRef.current) return;
    const requestText = dedupText.trim();
    if (!requestText) {
      addToast("请先粘贴需要检查的段落", "error");
      return;
    }
    dedupBusyRef.current = true;
    const requestProjectId = projectId;
    const requestInputVersion = dedupInputVersionRef.current;
    const requestSeq = nextSequence(dedupSeqRef);
    setDedupBusy(true);
    try {
      const result = await checkImportDedup({
        project_id: requestProjectId,
        content: requestText,
      });
      if (!isCurrentDedupRequest(requestProjectId, requestSeq, requestInputVersion)) {
        return;
      }
      setDedupResult(result);
      addToast("已完成去重检查", "success");
    } catch (err) {
      if (isCurrentDedupRequest(requestProjectId, requestSeq, requestInputVersion)) {
        addToast(extractErrorMessage(err, "去重检查失败"), "error");
      }
    } finally {
      if (isCurrentDedupRequest(requestProjectId, requestSeq, requestInputVersion)) {
        setDedupBusy(false);
      }
      if (dedupSeqRef.current === requestSeq) {
        dedupBusyRef.current = false;
      }
    }
  };

  const buildReverseInput = (
    requestProjectId: number,
    requestReverseType: ReverseType,
    requestSample: string,
    requestAutoContext: boolean,
  ) => ({
    project_id: requestProjectId,
    content_sample: requestSample.trim(),
    auto_context: requestAutoContext,
    context_limit: requestAutoContext ? REVERSE_CONTEXT_LIMITS[requestReverseType] : null,
    variables: null,
  });

  const handlePreviewReverseRequest = async () => {
    if (requestPreviewBusyRef.current) return;
    const requestProjectId = projectId;
    const requestReverseType = reverseType;
    const requestSample = reverseSample;
    const requestAutoContext = reverseAutoContext;
    if (!requestAutoContext && !requestSample.trim()) {
      addToast("请粘贴反推样本，或启用项目上下文", "error");
      return;
    }
    const requestSeq = nextSequence(requestPreviewSeqRef);
    const input = buildReverseInput(
      requestProjectId,
      requestReverseType,
      requestSample,
      requestAutoContext,
    );
    requestPreviewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    setRequestPreviewTitle("反推模型请求预览");
    try {
      const preview =
        requestReverseType === "blueprint"
          ? await previewReverseEngineerBlueprintRequest(input)
          : requestReverseType === "outline"
            ? await previewReverseEngineerOutlineRequest(input)
            : await previewReverseEngineerPartOutlineRequest(input);
      if (!isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "预览反推请求失败"), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq)) {
        setRequestPreviewLoading(false);
      }
      if (requestPreviewSeqRef.current === requestSeq) {
        requestPreviewBusyRef.current = false;
      }
    }
  };

  const handleReverseEngineer = async (apply: boolean) => {
    if (reverseBusyRef.current) return;
    const requestProjectId = projectId;
    const requestReverseType = reverseType;
    const requestSample = reverseSample;
    const requestAutoContext = reverseAutoContext;
    if (!requestAutoContext && !requestSample.trim()) {
      addToast("请粘贴反推样本，或启用项目上下文", "error");
      return;
    }
    reverseBusyRef.current = true;
    const requestSeq = nextSequence(reverseSeqRef);
    const taskId = allocateHostTaskId(
      `import-reverse-${requestReverseType}-${apply ? "apply" : "preview"}`,
      requestProjectId,
    );
    const reverseLabel =
      requestReverseType === "blueprint"
        ? "蓝图"
        : requestReverseType === "outline"
          ? "章节大纲"
          : "分部大纲";
    const input = buildReverseInput(
      requestProjectId,
      requestReverseType,
      requestSample,
      requestAutoContext,
    );
    setReverseBusy(apply ? "apply" : "preview");
    setReverseResult(null);
    try {
      const result = await runReverseProgressTask(
        {
          cancelTaskId: taskId,
          title: apply ? `反推并应用${reverseLabel}` : `反推${reverseLabel}`,
          startMessage: `正在准备反推${reverseLabel}`,
          successMessage: apply ? `反推${reverseLabel}结果已应用` : `反推${reverseLabel}完成`,
          errorMessage: "反推失败",
          total: apply ? 4 : 3,
          cancelLabel: "停止反推",
          runningHint: "反推停止后，已经应用的数据会保留",
        },
        () => {
          if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) {
            return Promise.reject(new Error("任务已停止"));
          }
          if (requestReverseType === "blueprint") {
            return apply
              ? reverseEngineerBlueprintAndApply(input, taskId)
              : reverseEngineerBlueprint(input, taskId);
          }
          if (requestReverseType === "outline") {
            return apply
              ? reverseEngineerOutlineAndApply(input, taskId)
              : reverseEngineerOutline(input, taskId);
          }
          return apply
            ? reverseEngineerPartOutlineAndApply(input, taskId)
            : reverseEngineerPartOutline(input, taskId);
        },
      );
      if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) return;
      setReverseResult(result);
      addToast(apply ? "反推结果已应用" : "反推完成", "success");
    } catch (err) {
      if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) return;
      const message = extractErrorMessage(err, "反推失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) {
        setReverseBusy(null);
      }
      if (reverseSeqRef.current === requestSeq) {
        reverseBusyRef.current = false;
      }
    }
  };

  const handleReverseTypeChange = (value: ReverseType) => {
    setReverseType(value);
    setReverseResult(null);
  };

  const handleReverseSampleChange = (value: string) => {
    setReverseSample(value);
    setReverseResult(null);
  };

  const handleReverseAutoContextChange = (value: boolean) => {
    setReverseAutoContext(value);
    setReverseResult(null);
  };

  const handleRequestPreviewClose = () => {
    requestPreviewSeqRef.current += 1;
    requestPreviewBusyRef.current = false;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
  };

  const handleImportAnalysisDialogClose = () => {
    importAnalysisDialogDismissedRef.current = true;
    setImportAnalysisDialog((prev) => ({
      ...prev,
      open: false,
    }));
  };

  return (
    <>
      <div className="content-wide space-y-4">
        <ImportFilePanel
          fileName={importFileName}
          fileType={fileType}
          importBusy={importBusy}
          externalBusy={fileExternalBusy}
          onFileTypeChange={setFileType}
          onSelectFile={handleSelectImportFile}
          onImport={handleImport}
        />
        <input
          ref={importFileInputRef}
          type="file"
          accept=".txt,.md,.markdown,.epub,.docx,text/plain,text/markdown,application/epub+zip,application/vnd.openxmlformats-officedocument.wordprocessingml.document"
          className="hidden"
          onChange={handleImportFileChange}
        />
        <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-[minmax(0,1fr)_22rem]">
          <ImportReviewQueuePanel
            reviews={reviews}
            loading={loading}
            analysis={analysis}
            projectProgress={projectProgress}
            selected={selected}
            analyzeBusy={analyzeBusy}
            canCancelAnalysis={analysisInProgress}
            cancelBusy={cancelBusy}
            batchBusy={batchBusy}
            actBusy={actBusy}
            externalBusy={reviewExternalBusy}
            onStartAnalysis={handleStartAnalysis}
            onPreviewAnalysisRequest={handlePreviewImportAnalysisRequest}
            onCancelAnalysis={handleCancelAnalysis}
            onBatchAccept={handleBatchAccept}
            onToggleSelect={toggleSelect}
            onViewDetail={handleViewDetail}
            onAct={handleAct}
          />
          <aside className="space-y-4">
            <div ref={detailPanelRef} className="scroll-mt-28">
              <ImportReviewDetailPanel
                activeReview={activeReview}
                activeReviewJson={activeReviewJson}
                activeReviewJsonError={activeReviewJsonError}
                activeReviewJsonDirty={activeReviewJsonDirty}
                actBusy={actBusy}
                externalBusy={detailExternalBusy}
                onAnalysisJsonChange={handleAnalysisJsonChange}
                onModify={handleModify}
                onUpdateStatus={handleUpdateStatus}
              />
            </div>
            <ImportToolsPanel
              dedupText={dedupText}
              dedupResult={dedupResult}
              dedupBusy={dedupBusy}
              reverseType={reverseType}
              reverseSample={reverseSample}
              reverseAutoContext={reverseAutoContext}
              reverseResult={reverseResult}
              reverseBusy={reverseBusy}
              externalBusy={toolsExternalBusy}
              onDedupTextChange={handleDedupTextChange}
              onDedup={handleDedup}
              onReverseTypeChange={handleReverseTypeChange}
              onReverseSampleChange={handleReverseSampleChange}
              onReverseAutoContextChange={handleReverseAutoContextChange}
              onPreviewReverseRequest={handlePreviewReverseRequest}
              onReverseEngineer={handleReverseEngineer}
            />
          </aside>
        </div>
      </div>
      <ProgressDialog
        open={importAnalysisDialog.open}
        title="导入项目分析"
        status={importAnalysisDialog.status}
        current={importAnalysisDialog.current}
        total={importAnalysisDialog.total}
        message={importAnalysisDialog.message}
        error={importAnalysisDialog.error}
        onClose={handleImportAnalysisDialogClose}
        onCancel={importAnalysisDialog.status === "running" ? handleCancelAnalysis : undefined}
        cancelLabel="停止分析"
        cancelling={cancelBusy}
        runningHint="分析过程中已完成的章节和大纲会保留"
      />
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={requestPreviewTitle}
        onClose={handleRequestPreviewClose}
      />
    </>
  );
};
