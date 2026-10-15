import React, { useEffect, useLayoutEffect, useRef, useState, type ChangeEvent } from "react";
import { useToast } from "../../components/feedback/Toast";
import { ProgressDialog } from "../../components/feedback/ProgressDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../api";
import { getChapter, type ChapterSummary } from "../../api/chapter";
import { Modal } from "../../components/ui/Modal";
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
import { resolveScrollBehavior } from "../../utils/scrollRail";
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
import { openFilePicker, readFileAsBase64 } from "../../utils/fileUpload";
import { formatNumber, useI18n } from "../../i18n";
import { useNovelDetailDirty } from "./novelDetailDirty";

interface ImportsTabProps {
  projectId: number;
  onBusyChange?: (busy: boolean) => void;
}

export const ImportsTab: React.FC<ImportsTabProps> = ({
  projectId,
  onBusyChange,
}) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const tRef = useRef(t);
  tRef.current = t;
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
  const [previewChapter, setPreviewChapter] = useState<ChapterSummary | null>(null);
  const [previewChapterLoading, setPreviewChapterLoading] = useState(false);
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
    useState(() => t("novel.importReview.requestPreview"));
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
  useNovelDetailDirty(
    `imports:${projectId}`,
    activeReviewJsonDirty || Boolean(importFileName.trim() || dedupText.trim() || reverseSample.trim()),
    undefined,
    () => !writeBusy && !dedupBusy && !requestPreviewLoading,
  );

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

  const load = async (requestProjectId = projectId): Promise<boolean> => {
    const requestId = loadRequestRef.current + 1;
    loadRequestRef.current = requestId;
    setLoading(true);
    try {
      const [listResult, progressResult] = await Promise.allSettled([
        listImportQueue(requestProjectId),
        getProjectImportAnalysisProgress(requestProjectId),
      ]);
      if (
        !isCurrentProject(requestProjectId) ||
        loadRequestRef.current !== requestId
      )
        return false;
      if (listResult.status === "fulfilled") {
        const list = listResult.value;
        reviewsRef.current = list;
        setReviews(list);
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
                JSON.stringify(updatedActiveReview.analysis_result, null, 2),
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
      } else {
        addToast(extractErrorMessage(listResult.reason, t("novel.importReview.loadQueueFailed")), "error");
      }
      if (progressResult.status === "fulfilled") {
        setProjectProgress(progressResult.value);
      } else {
        addToast(
          extractErrorMessage(progressResult.reason, t("novel.importReview.analysisProgressLoadFailed")),
          "error",
        );
      }
      return listResult.status === "fulfilled" && progressResult.status === "fulfilled";
    } catch (err) {
      if (
        isCurrentProject(requestProjectId) &&
        loadRequestRef.current === requestId
      ) {
        addToast(extractErrorMessage(err, t("novel.importReview.loadQueueFailed")), "error");
      }
      return false;
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
        addToast(extractErrorMessage(err, t("novel.importReview.restoreCandidateFailed")), "error");
      }
    }
  };

  useLayoutEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      invalidateProjectScopedRequests();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useLayoutEffect(() => {
    invalidateProjectScopedRequests();
    activeProjectIdRef.current = projectId;
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
    setPreviewChapter(null);
    setPreviewChapterLoading(false);
    setReverseGenerating(false);
    setReverseConfirmBusy(false);
    setReverseResult(null);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setRequestPreviewTitle(t("novel.importReview.requestPreview"));
    importAnalysisDialogDismissedRef.current = false;
    setImportAnalysisDialog(null);
    setSelected(new Set());
    setActiveReviewState(null);
    setActiveReviewJsonState("", false);
    setActiveReviewJsonError(null);
    setActiveReviewJsonDirty(false);
    void load(projectId);
    void restoreLatestReverseCandidate(projectId, reverseType);
    const refreshTimer = setInterval(() => {
      void load(projectId);
    }, 5000);
    return () => {
      loadRequestRef.current += 1;
      clearInterval(refreshTimer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  const handleImport = async () => {
    const requestFile = importUploadFile;
    const requestFileName = importFileName.trim();
    const requestProjectId = projectId;
    if (!requestFile || !requestFileName) {
      addToast(t("novel.importReview.chooseFile"), "error");
      return;
    }
    if (requestFile.size > MAX_PROJECT_TEXT_UPLOAD_BYTES) {
      addToast(
        t("novel.importReview.fileTooLarge", {
          size: formatNumber(MAX_PROJECT_TEXT_UPLOAD_BYTES / 1024 / 1024, locale),
        }),
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
      const loaded = await load(requestProjectId);
      if (!isCurrentAction(requestProjectId, importSeqRef, requestSeq)) return;
      if (!loaded) return;
      addToast(tRef.current("novel.importReview.reviewsCreated", { count: reviews.length }), "success");
      setImportFileName("");
      setImportUploadFile(null);
    } catch (err) {
      if (isCurrentAction(requestProjectId, importSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, tRef.current("novel.importReview.importFailed")), "error");
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
    openFilePicker(importFileInputRef.current);
  };

  const handleImportFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.currentTarget.files?.[0] ?? null;
    event.currentTarget.value = "";
    if (!file) return;
    if (file.size > MAX_PROJECT_TEXT_UPLOAD_BYTES) {
      addToast(
        t("novel.importReview.fileTooLarge", {
          size: formatNumber(MAX_PROJECT_TEXT_UPLOAD_BYTES / 1024 / 1024, locale),
        }),
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
      const loaded = await load(requestProjectId);
      if (!isCurrentAction(requestProjectId, analysisSeqRef, requestSeq)) return;
      if (!loaded) return;
      if (result.progress.status === "cancelled") {
        addToast(tRef.current("novel.importReview.analysisStopped"), "success");
      } else {
        addToast(
          tRef.current("novel.importReview.analysisCompleted", { count: result.analyzed_chapters }),
          "success",
        );
      }
    } catch (err) {
      if (!isCurrentAction(requestProjectId, analysisSeqRef, requestSeq))
        return;
      const message = extractErrorMessage(err, tRef.current("novel.importReview.startFailed"));
      setImportAnalysisDialog((prev) =>
        prev
          ? {
              ...prev,
              open: !importAnalysisDialogDismissedRef.current,
              status: "error",
              message: tRef.current("novel.importReview.analysisFailed"),
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
    setRequestPreviewTitle(t("novel.importReview.analysisRequestPreview"));
    try {
      const preview =
        await previewImportProjectAnalysisRequest(requestProjectId);
      if (!isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq))
        return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.importReview.analysisPreviewFailed")), "error");
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
      addToast(t("novel.importReview.noAnalysisRunning"), "info");
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
      const progress = await getProjectImportAnalysisProgress(requestProjectId);
      if (!isCurrentAction(requestProjectId, cancelAnalysisSeqRef, requestSeq))
        return;
      if (progress) {
        setProjectProgress(progress);
        setProgressDialogFromProgress(progress);
      }
      const loaded = await load(requestProjectId);
      if (!loaded) return;
      if (!isCurrentAction(requestProjectId, cancelAnalysisSeqRef, requestSeq)) return;
      addToast(tRef.current("novel.importReview.analysisCancelled"), "success");
    } catch (err) {
      if (isCurrentAction(requestProjectId, cancelAnalysisSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, tRef.current("novel.importReview.cancelFailed")), "error");
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
      addToast(t("novel.importReview.selectReviews"), "error");
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
      const loaded = await load(requestProjectId);
      if (!isCurrentAction(requestProjectId, batchSeqRef, requestSeq)) return;
      if (!loaded) return;
      addToast(tRef.current("novel.importReview.batchAccepted", { count: result.length }), "success");
      setSelected(new Set());
    } catch (err) {
      if (isCurrentAction(requestProjectId, batchSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, tRef.current("novel.importReview.operationFailed")), "error");
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
      addToast(t("novel.importReview.reviewNotInProject"), "error");
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
      const loaded = await load(requestProjectId);
      if (!loaded) return;
      if (!isCurrentAction(requestProjectId, actSeqRef, requestSeq)) return;
      addToast(
        action === "accept"
          ? tRef.current("novel.importReview.accepted")
          : action === "reject"
            ? tRef.current("novel.importReview.rejected")
            : tRef.current("novel.importReview.deleted"),
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
    } catch (err) {
      if (isCurrentAction(requestProjectId, actSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, tRef.current("novel.importReview.operationFailed")), "error");
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
      addToast(t("novel.importReview.reviewNotInProject"), "error");
      return;
    }
    setActiveReviewState(detail);
    setActiveReviewJsonState(JSON.stringify(detail.analysis_result, null, 2), false);
    setActiveReviewJsonError(null);
    if (window.matchMedia("(max-width: 1023px)").matches) {
      requestAnimationFrame(() => {
        detailPanelRef.current?.scrollIntoView({
          behavior: resolveScrollBehavior("smooth"),
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
      setActiveReviewJsonError(t("novel.importReview.invalidJson"));
    }
  };

  const handleModify = async () => {
    if (batchBusyRef.current || actBusyRef.current != null) return;
    if (!activeReviewJsonDirtyRef.current) return;
    const requestReview = activeReviewRef.current;
    if (!requestReview) return;
    const requestProjectId = projectId;
    if (requestReview.project_id !== requestProjectId) {
      addToast(t("novel.importReview.reviewNotInProject"), "error");
      return;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(activeReviewJson);
      setActiveReviewJsonError(null);
    } catch {
      setActiveReviewJsonError(t("novel.importReview.invalidJson"));
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
      const loaded = await load(requestProjectId);
      if (!isCurrentAction(requestProjectId, actSeqRef, requestSeq)) return;
      if (!loaded) return;
    } catch (err) {
      if (isCurrentAction(requestProjectId, actSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, tRef.current("novel.importReview.saveAndAcceptFailed")), "error");
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
    setPreviewChapter(null);
    setPreviewChapterLoading(false);
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
      addToast(t("novel.importReview.dedupSampleRequired"), "error");
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
      addToast(t("novel.importReview.dedupCompleted"), "success");
    } catch (err) {
      if (
        isCurrentDedupRequest(requestProjectId, requestSeq, requestInputVersion)
      ) {
        addToast(extractErrorMessage(err, t("novel.importReview.dedupFailed")), "error");
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

  const handleViewMatchedChapter = async () => {
    const chapterId = dedupResult?.matched_chapter_id;
    if (chapterId == null || previewChapterLoading) return;
    const requestProjectId = projectId;
    const sequence = dedupSeqRef.current;
    const inputVersion = dedupInputVersionRef.current;
    setPreviewChapterLoading(true);
    try {
      const chapter = await getChapter(requestProjectId, chapterId);
      if (!isCurrentDedupRequest(requestProjectId, sequence, inputVersion)) return;
      setPreviewChapter(chapter);
    } catch (error) {
      if (isCurrentDedupRequest(requestProjectId, sequence, inputVersion)) {
        addToast(extractErrorMessage(error, t("novel.loadChaptersFailed")), "error");
      }
    } finally {
      if (isCurrentDedupRequest(requestProjectId, sequence, inputVersion)) setPreviewChapterLoading(false);
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
      addToast(t("novel.importReview.reverseSampleRequired"), "error");
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
    setRequestPreviewTitle(t("novel.importReview.reverseRequestPreview"));
    try {
      const preview = await previewReverseEngineerRequest(input);
      if (!isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq))
        return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentAction(requestProjectId, requestPreviewSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.importReview.reversePreviewFailed")), "error");
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
      addToast(t("novel.importReview.reverseSampleRequired"), "error");
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
        ? t("novel.importReview.blueprint")
        : requestReverseType === "outline"
          ? t("novel.importReview.chapterOutline")
          : t("novel.importReview.partOutline");
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
            ? t("novel.importReview.tools.generateBlueprint")
            : t("novel.importReview.tools.generateCandidate", { label: reverseLabel }),
          startMessage: t("novel.importReview.tools.generatingResult"),
          successMessage: writesFormalBlueprint
            ? t("novel.importReview.reverseSucceeded")
            : t("novel.importReview.tools.candidatePending", { label: reverseLabel }),
          errorMessage: t("novel.importReview.reverseFailed"),
          total: REVERSE_ENGINEERING_PROGRESS_TOTAL,
          cancelLabel: t("workflow.stop"),
          runningHint: writesFormalBlueprint
            ? t("novel.importReview.reverseBlueprintHint")
            : t("novel.importReview.reverseStopped"),
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
          ? t("novel.importReview.reverseSucceeded")
          : t("novel.importReview.tools.candidatePending", { label: reverseLabel }),
        result.warnings,
        t("common.summarySeparator"),
      );
      if (writesFormalBlueprint && !await load(requestProjectId)) return;
      addToast(notice.message, notice.tone);
    } catch (err) {
      if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) return;
      const message = extractErrorMessage(err, t("novel.importReview.reverseFailed"));
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
      addToast(t("novel.importReview.reverseDirectNotice"), "info");
      return;
    }
    const candidate = reverseResult;
    if (!candidate || candidate.state !== "proposal") {
      addToast(t("novel.importReview.noCandidate"), "error");
      return;
    }
    reverseConfirmBusyRef.current = true;
    const requestProjectId = projectId;
    let requestSeq: number | null = null;
    try {
      const reverseLabel =
        reverseType === "outline"
          ? t("novel.importReview.chapterOutline")
          : t("novel.importReview.partOutline");
      const confirmedByUser = await confirmDialog({
        title: t("novel.importReview.confirmTitle", { label: reverseLabel }),
        message:
          reverseType === "part_outline"
            ? t("novel.importReview.confirmPartMessage")
            : t("novel.importReview.confirmOutlineMessage"),
        confirmText: t("novel.importReview.confirmButton", { label: reverseLabel }),
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
      const loaded = await load(requestProjectId);
      if (!isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)) return;
      if (!loaded) return;
      addToast(
        confirmed.warnings.length > 0
          ? tRef.current("novel.importReview.candidateConfirmedWithWarnings", {
              warnings: confirmed.warnings.join(tRef.current("common.summarySeparator")),
            })
          : tRef.current("novel.importReview.candidateConfirmed"),
        confirmed.warnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      if (
        requestSeq !== null &&
        isCurrentAction(requestProjectId, reverseSeqRef, requestSeq)
      ) {
        addToast(extractErrorMessage(err, tRef.current("novel.importReview.confirmFailed")), "error");
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
          previewChapterLoading={previewChapterLoading}
          onViewMatchedChapter={handleViewMatchedChapter}
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
          aria-label={t("novel.importReview.file.choose")}
          tabIndex={-1}
          type="file"
          accept=".txt,.md,.markdown,.epub,.docx,text/plain,text/markdown,application/epub+zip,application/vnd.openxmlformats-officedocument.wordprocessingml.document"
          className="sr-only"
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
      {previewChapter && (
        <Modal isOpen onClose={() => setPreviewChapter(null)} title={`${t("novel.chapterRangeSingle", { number: previewChapter.chapter_number })}${previewChapter.title ? ` · ${previewChapter.title}` : ""}`} maxWidthClassName="max-w-3xl" className="[--a4n-dialog-opacity:1]">
          <article aria-label={t("novel.chapterBody")} className="whitespace-pre-wrap break-words font-serif text-sm leading-relaxed text-book-text-main">
            {previewChapter.content}
          </article>
        </Modal>
      )}
      {importAnalysisDialog ? (
        <ProgressDialog
          open={importAnalysisDialog.open}
          title={t("novel.importReview.analysisTitle")}
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
          cancelLabel={t("novel.importReview.stopAnalysis")}
          cancelling={cancelBusy}
          runningHint={t("novel.importReview.analysisRunningHint")}
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
