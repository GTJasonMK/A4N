import React, { useEffect, useRef, useState, type ChangeEvent } from "react";
import { useToast } from "../../components/feedback/Toast";
import { ProgressDialog } from "../../components/feedback/ProgressDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../api";
import {
  acceptImportReview,
  batchAcceptReviews,
  cancelImportAnalysis,
  checkImportDedup,
  confirmReverseEngineeringCandidate,
  deleteImportReview,
  getReverseEngineeringCandidate,
  getProjectImportAnalysisProgress,
  importProjectTextUpload,
  listImportQueue,
  MAX_PROJECT_TEXT_UPLOAD_BYTES,
  previewImportProjectAnalysisRequest,
  previewReverseEngineerRequest,
  rejectImportReview,
  REVERSE_ENGINEERING_PROGRESS_TOTAL,
  reverseEngineer,
  runImportProjectAnalysis,
  type DedupResult,
  type ImportReviewSummary,
  type ProjectImportAnalysisProgress,
  type ReverseEngineeringCandidate,
  type ReverseEngineeringTarget,
} from "../../api/import";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { cancelWorkflowTask } from "../../api/streaming";
import {
  createWorkflowTaskId,
  useProgressTask,
} from "../../hooks/useProgressTask";
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
  delay,
  formalReverseNotice,
  importAnalysisDialogFromProgress,
  type ImportAnalysisDialogState,
} from "./imports/importTabUtils";
import { readFileAsBase64 } from "../../utils/fileUpload";

interface ImportsTabProps {
  projectId: number;
  onBusyChange?: (busy: boolean) => void;
}

export const ImportsTab: React.FC<ImportsTabProps> = ({
  projectId,
  onBusyChange,
}) => {
  const { addToast } = useToast();
  const { runProgressTask: runReverseProgressTask } = useProgressTask();
  const [reviews, setReviews] = useState<ImportReviewSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [projectProgress, setProjectProgress] =
    useState<ProjectImportAnalysisProgress | null>(null);
  const [importBusy, setImportBusy] = useState(false);
  const [analyzeBusy, setAnalyzeBusy] = useState(false);
  const [cancelBusy, setCancelBusy] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [batchBusy, setBatchBusy] = useState(false);
  const [activeReview, setActiveReview] = useState<ImportReviewSummary | null>(null);
  const [activeReviewJson, setActiveReviewJson] = useState("");
  const [activeReviewJsonError, setActiveReviewJsonError] = useState<
    string | null
  >(null);
  const [activeReviewJsonDirty, setActiveReviewJsonDirty] = useState(false);
  const activeReviewRef = useRef<ImportReviewSummary | null>(null);
  const activeReviewJsonDirtyRef = useRef(false);
  const reviewsRef = useRef<ImportReviewSummary[]>([]);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const loadRequestRef = useRef(0);
  const importSeqRef = useRef(0);
  const analysisSeqRef = useRef(0);
  const cancelAnalysisSeqRef = useRef(0);
  const batchSeqRef = useRef(0);
  const actSeqRef = useRef(0);
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
  const reverseConfirmBusyRef = useRef(false);
  const requestPreviewBusyRef = useRef(false);
  const [actBusy, setActBusy] = useState<string | null>(null);
  const [dedupText, setDedupText] = useState("");
  const [dedupResult, setDedupResult] = useState<DedupResult | null>(null);
  const [dedupBusy, setDedupBusy] = useState(false);
  const [reverseType, setReverseType] =
    useState<ReverseEngineeringTarget>("blueprint");
  const [reverseSample, setReverseSample] = useState("");
  const [reverseAutoContext, setReverseAutoContext] = useState(true);
  const [reverseResult, setReverseResult] =
    useState<ReverseEngineeringCandidate | null>(null);
  const [reverseGenerating, setReverseGenerating] = useState(false);
  const [reverseConfirmBusy, setReverseConfirmBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [requestPreviewTitle, setRequestPreviewTitle] =
    useState("模型请求预览");
  const [importAnalysisDialog, setImportAnalysisDialog] =
    useState<ImportAnalysisDialogState | null>(null);
  const writeBusy =
    importBusy ||
    analyzeBusy ||
    cancelBusy ||
    batchBusy ||
    actBusy !== null ||
    reverseGenerating ||
    reverseConfirmBusy;

  // 导入新文件
  const [importFileName, setImportFileName] = useState("");
  const [importUploadFile, setImportUploadFile] = useState<File | null>(null);
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

  const setActiveReviewState = (review: ImportReviewSummary | null) => {
    activeReviewRef.current = review;
    setActiveReview(review);
  };

  const setActiveReviewJsonState = (value: string, dirty: boolean) => {
    activeReviewJsonDirtyRef.current = dirty;
    setActiveReviewJson(value);
    setActiveReviewJsonDirty(dirty);
  };

  const setProgressDialogFromProgress = (
    progress: ProjectImportAnalysisProgress,
  ) => {
    const dialog = importAnalysisDialogFromProgress(progress);
    setImportAnalysisDialog({
      ...dialog,
      open: !importAnalysisDialogDismissedRef.current,
    });
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

  const canCancelAnalysis = projectProgress?.status === "analyzing";
  const analysisInProgress = analyzeBusy || canCancelAnalysis;
  const fileExternalBusy =
    analysisInProgress ||
    batchBusy ||
    actBusy !== null ||
    dedupBusy ||
    reverseGenerating ||
    reverseConfirmBusy;
  const reviewExternalBusy =
    importBusy || dedupBusy || reverseGenerating || reverseConfirmBusy;
  const detailExternalBusy =
    importBusy ||
    analysisInProgress ||
    batchBusy ||
    dedupBusy ||
    reverseGenerating ||
    reverseConfirmBusy;
  const toolsExternalBusy =
    importBusy ||
    analysisInProgress ||
    batchBusy ||
    actBusy !== null ||
    reverseConfirmBusy;

  const invalidateProjectScopedRequests = () => {
    loadRequestRef.current += 1;
    importSeqRef.current += 1;
    analysisSeqRef.current += 1;
    cancelAnalysisSeqRef.current += 1;
    batchSeqRef.current += 1;
    actSeqRef.current += 1;
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
    reverseConfirmBusyRef.current = false;
    requestPreviewBusyRef.current = false;
  };

  const load = async (requestProjectId = projectId) => {
    const requestId = loadRequestRef.current + 1;
    loadRequestRef.current = requestId;
    setLoading(true);
    try {
      const [list, progress] = await Promise.all([
        listImportQueue(requestProjectId),
        getProjectImportAnalysisProgress(requestProjectId),
      ]);
      if (
        !isCurrentProject(requestProjectId) ||
        loadRequestRef.current !== requestId
      )
        return;
      reviewsRef.current = list;
      setReviews(list);
      setProjectProgress(progress);
      const reviewIds = new Set(
        list
          .filter((review) => review.review_status === "pending")
          .map((review) => review.id),
      );
      setSelected((current) => {
        const next = new Set([...current].filter((id) => reviewIds.has(id)));
        return next.size === current.size ? current : next;
      });
      const currentActiveReview = activeReviewRef.current;
      if (currentActiveReview) {
        const updatedActiveReview = list.find(
          (review) => review.id === currentActiveReview.id,
        );
        if (updatedActiveReview) {
          setActiveReviewState(updatedActiveReview);
          if (!activeReviewJsonDirtyRef.current) {
            setActiveReviewJsonState(
              JSON.stringify(
                updatedActiveReview.analysis_result,
                null,
                2,
              ),
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
      if (
        isCurrentProject(requestProjectId) &&
        loadRequestRef.current === requestId
      ) {
        addToast(extractErrorMessage(err, "加载导入队列失败"), "error");
      }
    } finally {
      if (
        isCurrentProject(requestProjectId) &&
        loadRequestRef.current === requestId
      ) {
        setLoading(false);
      }
    }
  };

  const restoreLatestReverseCandidate = async (
    requestProjectId: number,
    requestReverseType: ReverseEngineeringTarget,
  ) => {
    const requestSeq = nextSequence(reverseSeqRef);
    try {
      const restored = await getReverseEngineeringCandidate({
        project_id: requestProjectId,
        target: requestReverseType,
      });
      if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) return;
      setReverseResult(restored);
    } catch (err) {
      if (isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "恢复项目资料归纳结果失败"), "error");
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
    setReverseGenerating(false);
    setReverseConfirmBusy(false);
    setReverseResult(null);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setRequestPreviewTitle("模型请求预览");
    importAnalysisDialogDismissedRef.current = false;
    setImportAnalysisDialog(null);
    setSelected(new Set());
    setActiveReviewState(null);
    setActiveReviewJsonState("", false);
    setActiveReviewJsonError(null);
    setActiveReviewJsonDirty(false);
    void load(projectId);
    void restoreLatestReverseCandidate(projectId, reverseType);
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
    const requestProjectId = projectId;
    if (!requestFile || !requestFileName) {
      addToast("请选择要导入的文件", "error");
      return;
    }
    if (requestFile.size > MAX_PROJECT_TEXT_UPLOAD_BYTES) {
      addToast(
        `导入文件不能超过 ${MAX_PROJECT_TEXT_UPLOAD_BYTES / 1024 / 1024}MB`,
        "error",
      );
      return;
    }
    if (importBusyRef.current) return;
    importBusyRef.current = true;
    const requestSeq = nextSequence(importSeqRef);
    setImportBusy(true);
    try {
      const dataBase64 = await readFileAsBase64(requestFile);
      if (!isCurrentAction(requestProjectId, importSeqRef, requestSeq)) return;
      const reviews = await importProjectTextUpload({
        project_id: requestProjectId,
        file_name: requestFileName,
        data_base64: dataBase64,
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
    if (file.size > MAX_PROJECT_TEXT_UPLOAD_BYTES) {
      addToast(
        `导入文件不能超过 ${MAX_PROJECT_TEXT_UPLOAD_BYTES / 1024 / 1024}MB`,
        "error",
      );
      return;
    }
    setImportUploadFile(file);
    setImportFileName(file.name);
  };

  const handleStartAnalysis = async () => {
    if (analyzeBusyRef.current) return;
    analyzeBusyRef.current = true;
    const requestProjectId = projectId;
    const requestSeq = nextSequence(analysisSeqRef);
    setAnalyzeBusy(true);
    let polling = true;
    importAnalysisDialogDismissedRef.current = false;
    setImportAnalysisDialog(null);

    const pollProgress = async () => {
      while (
        polling &&
        isCurrentAction(requestProjectId, analysisSeqRef, requestSeq)
      ) {
        try {
          const progress =
            await getProjectImportAnalysisProgress(requestProjectId);
          if (!isCurrentAction(requestProjectId, analysisSeqRef, requestSeq))
            return;
          if (progress?.status === "analyzing") {
            setProjectProgress(progress);
            setProgressDialogFromProgress(progress);
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
      if (!isCurrentAction(requestProjectId, analysisSeqRef, requestSeq))
        return;
      setProjectProgress(result.progress);
      setProgressDialogFromProgress(result.progress);
      if (result.progress.status === "cancelled") {
        addToast("导入项目分析已停止，已完成部分会保留", "success");
      } else {
        addToast(
          `已完成 ${result.analyzed_chapters} 章的分析与资料同步`,
          "success",
        );
      }
      await load(requestProjectId);
    } catch (err) {
      if (!isCurrentAction(requestProjectId, analysisSeqRef, requestSeq))
        return;
      const message = extractErrorMessage(err, "启动失败");
      setImportAnalysisDialog((prev) =>
        prev
          ? {
              ...prev,
              open: !importAnalysisDialogDismissedRef.current,
              status: "error",
              message: "导入项目分析失败",
              error: message,
            }
          : null,
      );
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
      const preview =
        await previewImportProjectAnalysisRequest(requestProjectId);
      if (!isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq))
        return;
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
    if (!canCancelAnalysis) {
      addToast("当前没有正在运行的导入项目分析", "info");
      return;
    }
    cancelBusyRef.current = true;
    const requestProjectId = projectId;
    const requestSeq = nextSequence(cancelAnalysisSeqRef);
    setCancelBusy(true);
    try {
      await cancelImportAnalysis(requestProjectId);
      if (!isCurrentAction(requestProjectId, cancelAnalysisSeqRef, requestSeq))
        return;
      analysisSeqRef.current += 1;
      setAnalyzeBusy(false);
      importAnalysisDialogDismissedRef.current = false;
      addToast("导入项目分析已取消", "success");
      const progress = await getProjectImportAnalysisProgress(requestProjectId);
      if (!isCurrentAction(requestProjectId, cancelAnalysisSeqRef, requestSeq))
        return;
      if (progress) {
        setProjectProgress(progress);
        setProgressDialogFromProgress(progress);
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
      addToast(
        `已${action === "accept" ? "接受" : action === "reject" ? "拒绝" : "删除"}`,
        "success",
      );
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

  const handleViewDetail = (id: number) => {
    const detail = reviewsRef.current.find((review) => review.id === id);
    if (!detail || detail.project_id !== projectId) {
      addToast("审核项已不在当前项目队列中", "error");
      return;
    }
    setActiveReviewState(detail);
    setActiveReviewJsonState(JSON.stringify(detail.analysis_result, null, 2), false);
    setActiveReviewJsonError(null);
    if (window.matchMedia("(max-width: 1023px)").matches) {
      requestAnimationFrame(() => {
        detailPanelRef.current?.scrollIntoView({
          behavior: "smooth",
          block: "start",
        });
      });
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
      const updated = await acceptImportReview(requestReview.id, parsed);
      if (!isCurrentAction(requestProjectId, actSeqRef, requestSeq)) return;
      if (updated.project_id !== requestProjectId) {
        throw new Error("修改返回了其他项目的审核项");
      }
      setActiveReviewState(updated);
      setActiveReviewJsonState(
        JSON.stringify(updated.analysis_result, null, 2),
        false,
      );
      addToast("修改已保存并接受", "success");
      await load(requestProjectId);
    } catch (err) {
      if (isCurrentAction(requestProjectId, actSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "保存并接受失败"), "error");
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
      if (
        !isCurrentDedupRequest(
          requestProjectId,
          requestSeq,
          requestInputVersion,
        )
      ) {
        return;
      }
      setDedupResult(result);
      addToast("已完成去重检查", "success");
    } catch (err) {
      if (
        isCurrentDedupRequest(requestProjectId, requestSeq, requestInputVersion)
      ) {
        addToast(extractErrorMessage(err, "去重检查失败"), "error");
      }
    } finally {
      if (
        isCurrentDedupRequest(requestProjectId, requestSeq, requestInputVersion)
      ) {
        setDedupBusy(false);
      }
      if (dedupSeqRef.current === requestSeq) {
        dedupBusyRef.current = false;
      }
    }
  };

  const buildReverseInput = (
    requestProjectId: number,
    requestReverseType: ReverseEngineeringTarget,
    requestSample: string,
    requestAutoContext: boolean,
  ) => ({
    target: requestReverseType,
    project_id: requestProjectId,
    content_sample: requestSample.trim(),
    auto_context: requestAutoContext,
    context_limit: requestAutoContext
      ? REVERSE_CONTEXT_LIMITS[requestReverseType]
      : null,
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
    setRequestPreviewTitle("项目资料归纳请求预览");
    try {
      const preview = await previewReverseEngineerRequest(input);
      if (!isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq))
        return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "预览项目资料归纳请求失败"), "error");
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

  const handleReverseEngineer = async () => {
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
    const taskId = createWorkflowTaskId(
      `import-reverse-${requestReverseType}`,
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
    setReverseGenerating(true);
    setReverseResult(null);
    const writesFormalBlueprint = requestReverseType === "blueprint";
    try {
      const result = await runReverseProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: writesFormalBlueprint
            ? "依据当前资料归纳正式蓝图"
            : `生成${reverseLabel}候选`,
          startMessage: `正在整理${reverseLabel}所需资料`,
          successMessage: writesFormalBlueprint
            ? "蓝图已归纳并写入正式资料"
            : `${reverseLabel}候选已生成，尚未写入正式资料`,
          errorMessage: "项目资料归纳失败",
          total: REVERSE_ENGINEERING_PROGRESS_TOTAL,
          cancelLabel: "停止归纳",
          runningHint: writesFormalBlueprint
            ? "只有成功完成才会更新正式蓝图；失败或停止时保留原蓝图"
            : "归纳停止后，已生成的结果会保留；正式资料不会被自动修改",
        },
        () => {
          if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) {
            return Promise.reject(new Error("任务已停止"));
          }
          return reverseEngineer(input, taskId);
        },
      );
      if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) return;
      setReverseResult(result);
      const notice = formalReverseNotice(
        writesFormalBlueprint
          ? "蓝图已归纳并写入正式资料"
          : `${reverseLabel}候选已生成，尚未写入正式资料`,
        result.warnings,
      );
      addToast(notice.message, notice.tone);
      if (writesFormalBlueprint) await load(requestProjectId);
    } catch (err) {
      if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) return;
      const message = extractErrorMessage(err, "项目资料归纳失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) {
        setReverseGenerating(false);
      }
      if (reverseSeqRef.current === requestSeq) {
        reverseBusyRef.current = false;
      }
    }
  };

  const handleConfirmReverseCandidate = async () => {
    if (reverseConfirmBusyRef.current) return;
    if (reverseType === "blueprint") {
      addToast("蓝图归纳成功后会直接写入正式资料，无需再次确认", "info");
      return;
    }
    const candidate = reverseResult;
    if (!candidate || candidate.state !== "proposal") {
      addToast("当前没有可确认的反推候选", "error");
      return;
    }
    reverseConfirmBusyRef.current = true;
    const requestProjectId = projectId;
    let requestSeq: number | null = null;
    try {
      const reverseLabel =
        reverseType === "outline"
          ? "章节大纲"
          : "分部大纲";
      const confirmedByUser = await confirmDialog({
        title: `确认更新${reverseLabel}`,
        message:
          reverseType === "part_outline"
            ? "确认后会按分部编号更新候选覆盖的正式分部，并重新关联章节大纲；候选未覆盖的分部会保留。"
            : "确认后会按章节号更新候选覆盖的正式章纲；候选未覆盖的章纲会保留。",
        confirmText: `更新${reverseLabel}`,
        dialogType: "warning",
      });
      if (!confirmedByUser || !isCurrentProject(requestProjectId)) return;
      requestSeq = nextSequence(reverseSeqRef);
      setReverseConfirmBusy(true);
      const confirmed = await confirmReverseEngineeringCandidate({
        workflow_run_id: candidate.workflow_run_id,
      });
      if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) return;
      setReverseResult(confirmed);
      addToast(
        confirmed.warnings.length > 0
          ? `反推候选已确认并写入正式资料；${confirmed.warnings.join("；")}`
          : "反推候选已确认并写入正式资料",
        confirmed.warnings.length > 0 ? "info" : "success",
      );
      await load(requestProjectId);
    } catch (err) {
      if (
        requestSeq !== null &&
        isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)
      ) {
        addToast(extractErrorMessage(err, "确认项目资料候选失败"), "error");
      }
    } finally {
      if (
        requestSeq !== null &&
        isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)
      ) {
        setReverseConfirmBusy(false);
      }
      reverseConfirmBusyRef.current = false;
    }
  };

  const handleReverseTypeChange = (value: ReverseEngineeringTarget) => {
    setReverseType(value);
    setReverseResult(null);
    void restoreLatestReverseCandidate(projectId, value);
  };

  const handleReverseSampleChange = (value: string) => {
    reverseSeqRef.current += 1;
    setReverseSample(value);
    setReverseResult(null);
  };

  const handleReverseAutoContextChange = (value: boolean) => {
    reverseSeqRef.current += 1;
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
    setImportAnalysisDialog((prev) =>
      prev
        ? {
            ...prev,
            open: false,
          }
        : null,
    );
  };

  return (
    <>
      <div className="content-wide space-y-4">
        <ImportToolsPanel
          dedupText={dedupText}
          dedupResult={dedupResult}
          dedupBusy={dedupBusy}
          reverseType={reverseType}
          reverseSample={reverseSample}
          reverseAutoContext={reverseAutoContext}
          reverseResult={reverseResult}
          reverseGenerating={reverseGenerating}
          reverseConfirmBusy={reverseConfirmBusy}
          externalBusy={toolsExternalBusy}
          onDedupTextChange={handleDedupTextChange}
          onDedup={handleDedup}
          onReverseTypeChange={handleReverseTypeChange}
          onReverseSampleChange={handleReverseSampleChange}
          onReverseAutoContextChange={handleReverseAutoContextChange}
          onPreviewReverseRequest={handlePreviewReverseRequest}
          onReverseEngineer={handleReverseEngineer}
          onConfirmReverse={handleConfirmReverseCandidate}
        />
        <ImportFilePanel
          fileName={importFileName}
          importBusy={importBusy}
          externalBusy={fileExternalBusy}
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
          <div className="space-y-4">
            <ImportReviewQueuePanel
              reviews={reviews}
              loading={loading}
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
          </div>
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
              />
            </div>
          </aside>
        </div>
      </div>
      {importAnalysisDialog ? (
        <ProgressDialog
          open={importAnalysisDialog.open}
          title="导入项目分析"
          status={importAnalysisDialog.status}
          current={importAnalysisDialog.current}
          total={importAnalysisDialog.total}
          message={importAnalysisDialog.message}
          error={importAnalysisDialog.error}
          onClose={handleImportAnalysisDialogClose}
          onCancel={
            importAnalysisDialog.status === "running"
              ? handleCancelAnalysis
              : undefined
          }
          cancelLabel="停止分析"
          cancelling={cancelBusy}
          runningHint="分析过程中已完成的章节和大纲会保留"
        />
      ) : null}
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
