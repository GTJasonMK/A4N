import React, { useCallback, useLayoutEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { BookButton } from "../../components/ui/BookButton";
import { extractErrorMessage } from "../../api";
import {
  deleteLatestChapterOutlines,
  deleteLatestParts,
  listChapterOutlineIndex,
  listParts,
  previewGenerateChapterOutlinesByCountRequest,
  previewGeneratePartsRequest,
  previewRegenerateChapterOutlineRequest,
  previewRegenerateLastPartRequest,
  saveChapterOutline,
  savePartOutline,
  type ChapterOutlineListEntry,
  type PartOutline,
} from "../../api/outline";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { canEditProjectMaterials, type NovelProject } from "../../api/project";
import { isPositiveSafeInteger } from "../../utils/numberRange";
import type { ChapterOutlineEditForm } from "./outlines/ChapterOutlineSection";
import type {
  ManualChapterOutlineForm,
  ManualPartOutlineForm,
} from "./outlines/OutlineDialogs";
import {
  DELETE_LATEST_CHAPTER_OUTLINES_ACTION,
  DELETE_LATEST_PARTS_ACTION,
  deleteChapterOutlinesFromAction,
  isValidDeletionCount,
  isPositiveInteger,
  isValidChapterOutlineCount,
  outlineOptimizationVariables,
  type NumberInputValue,
} from "./outlines/outlineTabUtils";
import { isCancelledOperation } from "../../utils/cancellation";
import { useOutlineGeneration } from "./outlines/useOutlineGeneration";
import {
  requestOptimizationDirection,
  useOutlineTabGuards,
} from "./outlines/useOutlineTabGuards";
import { useI18n } from "../../i18n";
import { OutlinesTabView } from "./outlines/OutlinesTabView";
import { useNovelDetailDirty } from "./novelDetailDirty";

type OutlineView = "parts" | "chapters";

export const OutlinesTab: React.FC<{ project: NovelProject }> = ({ project }) => {
  const projectId = project.id;
  const canEditOutlines = canEditProjectMaterials(project);
  const defaultPartCount: NumberInputValue = 1;
  const defaultChaptersPerPart: NumberInputValue =
    project.chapters_per_part != null && project.chapters_per_part > 0
      ? project.chapters_per_part
      : "";
  const { addToast } = useToast();
  const { t } = useI18n();
  const addOutlineMutationToast = (message: string, warnings: string[]) => {
    addToast(
      warnings.length > 0 ? `${message}${t("common.summarySeparator")}${warnings.join(t("common.summarySeparator"))}` : message,
      warnings.length > 0 ? "info" : "success",
    );
  };
  const reportRefreshFailure = (refreshed: boolean) => {
    if (!refreshed) addToast(t("novel.refreshFailed"), "error");
  };
  const { run: runOutlineGeneration } = useOutlineGeneration();
  const [parts, setParts] = useState<PartOutline[]>([]);
  const [outlines, setOutlines] = useState<ChapterOutlineListEntry[]>([]);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const [partsLoaded, setPartsLoaded] = useState(false);
  const [outlinesLoaded, setOutlinesLoaded] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<{ projectId: number; message: string } | null>(null);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] = useState<ModelRequestPreview | null>(null);

  // 生成参数 + 状态
  const [partCount, setPartCount] =
    useState<NumberInputValue>(defaultPartCount);
  const [chaptersPerPart, setChaptersPerPart] =
    useState<NumberInputValue>(defaultChaptersPerPart);
  const [generatingParts, setGeneratingParts] = useState(false);
  const [chapterCount, setChapterCount] = useState<NumberInputValue>(10);
  const [generatingOutlines, setGeneratingOutlines] = useState(false);
  const [activeOutlineView, setActiveOutlineView] = useState<OutlineView>("parts");

  // 章纲编辑 / 操作
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editForm, setEditForm] = useState<ChapterOutlineEditForm>({
    title: "",
    summary: "",
  });
  const [savingId, setSavingId] = useState<number | null>(null);
  const [regenId, setRegenId] = useState<number | null>(null);
  const [partRegenId, setPartRegenId] = useState<number | null>(null);
  const [manualChapterInitial, setManualChapterInitial] =
    useState<ManualChapterOutlineForm | null>(null);
  const [manualPartEditor, setManualPartEditor] = useState<{
    part: PartOutline | null;
    initial: ManualPartOutlineForm;
  } | null>(null);
  const [manualSaving, setManualSaving] = useState<"chapter" | "part" | null>(null);
  const [partDetail, setPartDetail] = useState<PartOutline | null>(null);
  const [outlineDetail, setOutlineDetail] = useState<ChapterOutlineListEntry | null>(null);
  const [outlineAction, setOutlineAction] = useState<ChapterOutlineListEntry | null>(null);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  // 危险操作
  const [delPartCount, setDelPartCount] = useState<NumberInputValue>(1);
  const [delChapterCount, setDelChapterCount] = useState<NumberInputValue>(1);
  const [delPartCountError, setDelPartCountError] = useState<string | null>(null);
  const [delChapterCountError, setDelChapterCountError] = useState<string | null>(null);
  const [delActing, setDelActing] = useState<string | null>(null);
  const [regenLastPart, setRegenLastPart] = useState(false);

  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const partsRef = useRef<PartOutline[]>([]);
  const outlinesRef = useRef<ChapterOutlineListEntry[]>([]);
  const loadSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const actionBusyRef = useRef(false);
  const previewBusyRef = useRef(false);
  const confirmBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;

  const {
    beginAction,
    beginConfirmAction,
    beginPreview,
    finishAction,
    finishConfirmAction,
    finishPreview,
    invalidatePreview,
    isActionCurrent,
    isLoadCurrent,
    isPreviewCurrent,
    isProjectCurrent,
  } = useOutlineTabGuards({
    canEditOutlines,
    addToast,
    t,
    mountedRef,
    activeProjectIdRef,
    loadSeqRef,
    actionSeqRef,
    previewSeqRef,
    actionBusyRef,
    previewBusyRef,
    confirmBusyRef,
  });
  const editingOutline = editingId == null
    ? null
    : outlines.find((outline) => outline.id === editingId) ?? null;
  const mutationBusy = generatingParts || generatingOutlines || savingId != null
    || manualSaving != null || regenId != null || delActing != null
    || regenLastPart || partRegenId != null;
  useNovelDetailDirty(
    `outlines:${projectId}`,
    editingOutline != null && (
      editForm.title !== editingOutline.title ||
      editForm.summary !== editingOutline.summary
    ),
    undefined,
    () => !mutationBusy && !requestPreviewLoading,
  );

  const closeRequestPreview = useCallback(() => {
    invalidatePreview();
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, [invalidatePreview]);

  const load = useCallback(async (options: { showLoading?: boolean } = {}) => {
    const requestProjectId = projectId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    const showLoading = options.showLoading ?? true;
    if (showLoading && isLoadCurrent(requestProjectId, requestSeq)) setLoading(true);
    try {
      const [partsResult, outlinesResult] = await Promise.allSettled([
        listParts(requestProjectId),
        listChapterOutlineIndex(requestProjectId),
      ]);
      if (!isLoadCurrent(requestProjectId, requestSeq)) return false;
      const errors: string[] = [];
      if (partsResult.status === "rejected") {
        const message = extractErrorMessage(partsResult.reason, t("outline.loadFailed"));
        errors.push(message);
      }
      if (outlinesResult.status === "rejected") {
        const message = extractErrorMessage(outlinesResult.reason, t("outline.loadFailed"));
        errors.push(message);
      }
      setLoadError(
        errors.length > 0
          ? { projectId: requestProjectId, message: errors.join(t("common.summarySeparator")) }
          : null,
      );
      const pp = partsResult.status === "fulfilled" ? partsResult.value : [];
      const oo = outlinesResult.status === "fulfilled" ? outlinesResult.value : [];
      const foreignPart = pp.find((part) => part.project_id !== requestProjectId);
      if (foreignPart) {
        throw new Error(`分部大纲接口返回了其他项目的数据：part_id=${foreignPart.id}`);
      }
      const foreignOutline = oo.find((outline) => outline.project_id !== requestProjectId);
      if (foreignOutline) {
        throw new Error(`章节大纲接口返回了其他项目的数据：outline_id=${foreignOutline.id}`);
      }
      partsRef.current = pp;
      outlinesRef.current = oo;
      setParts(pp);
      setOutlines(oo);
      setLoadedProjectId(requestProjectId);
      setPartsLoaded(partsResult.status === "fulfilled");
      setOutlinesLoaded(outlinesResult.status === "fulfilled");
      return partsResult.status === "fulfilled" && outlinesResult.status === "fulfilled";
    } catch (err) {
      if (isLoadCurrent(requestProjectId, requestSeq)) {
        const message = extractErrorMessage(err, t("outline.loadFailed"));
        partsRef.current = [];
        outlinesRef.current = [];
        setParts([]);
        setOutlines([]);
        setPartsLoaded(false);
        setOutlinesLoaded(false);
        setLoadedProjectId(requestProjectId);
        setLoadError({ projectId: requestProjectId, message });
      }
      return false;
    } finally {
      if (showLoading && isLoadCurrent(requestProjectId, requestSeq)) setLoading(false);
    }
  }, [addToast, isLoadCurrent, projectId]);

  useLayoutEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      actionSeqRef.current += 1;
      previewSeqRef.current += 1;
      actionBusyRef.current = false;
      previewBusyRef.current = false;
      confirmBusyRef.current = false;
    };
  }, []);

  useLayoutEffect(() => {
    activeProjectIdRef.current = projectId;
    loadSeqRef.current += 1;
    actionSeqRef.current += 1;
    previewSeqRef.current += 1;
    actionBusyRef.current = false;
    previewBusyRef.current = false;
    confirmBusyRef.current = false;
    partsRef.current = [];
    outlinesRef.current = [];
    setLoadedProjectId(null);
    setParts([]);
    setOutlines([]);
    setPartsLoaded(false);
    setOutlinesLoaded(false);
    setLoadError(null);
    setLoading(true);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setGeneratingParts(false);
    setGeneratingOutlines(false);
    setActiveOutlineView("parts");
    setEditingId(null);
    setEditForm({ title: "", summary: "" });
    setSavingId(null);
    setRegenId(null);
    setPartRegenId(null);
    setManualChapterInitial(null);
    setManualPartEditor(null);
    setManualSaving(null);
    setPartDetail(null);
    setOutlineDetail(null);
    setOutlineAction(null);
    setAdvancedOpen(false);
    setDelActing(null);
    setRegenLastPart(false);
    void load();
  }, [load, projectId]);

  useLayoutEffect(() => {
    setPartCount(defaultPartCount);
    setChaptersPerPart(defaultChaptersPerPart);
  }, [defaultChaptersPerPart, defaultPartCount, projectId]);

  useLayoutEffect(() => {
    if (canEditOutlines) return;
    setEditingId(null);
    setEditForm({ title: "", summary: "" });
    setManualChapterInitial(null);
    setManualPartEditor(null);
    setOutlineAction(null);
    setAdvancedOpen(false);
  }, [canEditOutlines]);

  useLayoutEffect(() => {
    setPartDetail((current) => {
      if (!current) return null;
      return parts.find((part) => part.id === current.id) ?? null;
    });
    partsRef.current = parts;
  }, [parts]);

  useLayoutEffect(() => {
    outlinesRef.current = outlines;
    setOutlineDetail((current) => {
      if (!current) return null;
      return outlines.find((outline) => outline.id === current.id) ?? null;
    });
    setOutlineAction((current) => {
      if (!current) return null;
      return outlines.find((outline) => outline.id === current.id) ?? null;
    });
    if (editingId != null && !outlines.some((outline) => outline.id === editingId)) {
      setEditingId(null);
      setEditForm({ title: "", summary: "" });
    }
  }, [editingId, outlines]);

  const handleGenerateParts = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyGenerateParts"), "error");
      return;
    }
    const requestedPartCount = typeof partCount === "number" ? partCount : null;
    const requestedChaptersPerPart =
      typeof chaptersPerPart === "number" ? chaptersPerPart : null;

    if (!isPositiveInteger(requestedPartCount)) {
      addToast(t("outline.partCountPositive"), "error");
      return;
    }
    if (
      requestedChaptersPerPart !== null &&
      !isPositiveInteger(requestedChaptersPerPart)
    ) {
      addToast(t("outline.chaptersPerPartPositive"), "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    const extendingParts = partsRef.current.length > 0;
    const progressActionLabel = extendingParts
      ? t("outline.appendParts")
      : t("outline.generateParts");
    const successMessage = extendingParts
      ? t("outline.partsAppended", { count: requestedPartCount })
      : t("outline.partsGenerated", { count: requestedPartCount });
    setGeneratingParts(true);
    let refreshOk = true;
    try {
      const warnings = await runOutlineGeneration(
        {
          title: progressActionLabel,
          startMessage: t("outline.modelGeneratingParts", { action: progressActionLabel }),
          successMessage,
          errorMessage: extendingParts
            ? t("outline.partsAppendFailed")
            : t("outline.partsGenerationFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "generate_parts",
            part_count: requestedPartCount,
            chapters_per_part: requestedChaptersPerPart,
            variables: null,
          },
        },
        async (control) => {
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          control.setProgress(t("outline.refreshing"), 2);
          refreshOk = await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (!refreshOk) return;
      addOutlineMutationToast(successMessage, warnings);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        await load({ showLoading: false });
        const message = extractErrorMessage(
          err,
          extendingParts
            ? t("outline.partsAppendFailed")
            : t("outline.partsGenerationFailed"),
        );
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setGeneratingParts(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewGenerateParts = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyPreview"), "error");
      return;
    }
    const requestedPartCount = typeof partCount === "number" ? partCount : null;
    const requestedChaptersPerPart =
      typeof chaptersPerPart === "number" ? chaptersPerPart : null;
    if (!isPositiveInteger(requestedPartCount)) {
      addToast(t("outline.partCountPositive"), "error");
      return;
    }
    const previewSeq = beginPreview(requestProjectId);
    if (previewSeq == null) return;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGeneratePartsRequest({
        project_id: requestProjectId,
        part_count: requestedPartCount,
        chapters_per_part: requestedChaptersPerPart,
        variables: null,
      });
      if (!isPreviewCurrent(requestProjectId, previewSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, previewSeq)) {
        addToast(extractErrorMessage(err, t("outline.previewPartsFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, previewSeq)) setRequestPreviewLoading(false);
      finishPreview(previewSeq);
    }
  };

  const handleGenerateChapterOutlines = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyGenerateChapters"), "error");
      return;
    }
    if (!isValidChapterOutlineCount(chapterCount)) {
      addToast(t("outline.chapterCountPositive"), "error");
      return;
    }
    const requestedChapterCount = chapterCount;
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setGeneratingOutlines(true);
    let refreshOk = true;
    try {
      const startChapter =
        outlinesRef.current.reduce((max, outline) => Math.max(max, outline.chapter_number), 0) + 1;
      const existingOutlineCount = outlinesRef.current.length;
      const warnings = await runOutlineGeneration(
        {
          title:
            existingOutlineCount > 0
              ? t("outline.continueChaptersTitle")
              : t("outline.generateChaptersTitle"),
          startMessage: t("outline.generatingChapters", {
            start: startChapter,
            count: requestedChapterCount,
          }),
          successMessage: t("outline.chaptersGenerated", { count: requestedChapterCount }),
          errorMessage: t("outline.chaptersGenerationFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "generate_chapter_outlines_by_count",
            start_chapter: startChapter,
            count: requestedChapterCount,
          },
        },
        async (control) => {
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          control.setProgress(t("outline.refreshing"), 2);
          refreshOk = await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (!refreshOk) return;
      addOutlineMutationToast(t("outline.chaptersGenerated", { count: requestedChapterCount }), warnings);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        await load({ showLoading: false });
        const message = extractErrorMessage(err, t("outline.chaptersGenerationFailed"));
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setGeneratingOutlines(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewGenerateChapterOutlines = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyPreview"), "error");
      return;
    }
    if (!isValidChapterOutlineCount(chapterCount)) {
      addToast(t("outline.chapterCountPositive"), "error");
      return;
    }
    const requestedChapterCount = chapterCount;
    const previewSeq = beginPreview(requestProjectId);
    if (previewSeq == null) return;
    const startChapter =
      outlinesRef.current.reduce((max, outline) => Math.max(max, outline.chapter_number), 0) + 1;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewGenerateChapterOutlinesByCountRequest({
        project_id: requestProjectId,
        start_chapter: startChapter,
        count: requestedChapterCount,
      });
      if (!isPreviewCurrent(requestProjectId, previewSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, previewSeq)) {
        addToast(extractErrorMessage(err, t("outline.previewChaptersFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, previewSeq)) setRequestPreviewLoading(false);
      finishPreview(previewSeq);
    }
  };

  const startEdit = (o: ChapterOutlineListEntry) => {
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyEdit"), "error");
      return;
    }
    setEditingId(o.id);
    setEditForm({
      title: o.title == null ? "" : o.title,
      summary: o.summary == null ? "" : o.summary,
    });
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditForm({ title: "", summary: "" });
  };

  const handleRetry = async () => {
    if (interactionBusy) return;
    const draftDirty = editingOutline != null && (
      editForm.title !== editingOutline.title || editForm.summary !== editingOutline.summary
    );
    if (draftDirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("outline.chapterSection.title"), action: t("common.retry") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    if (editingId != null) cancelEdit();
    await load();
  };

  const handleSaveOutline = async (o: ChapterOutlineListEntry) => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyEdit"), "error");
      return;
    }
    if (o.project_id !== requestProjectId) {
      addToast(t("outline.outlineNotInProject"), "error");
      return;
    }
    if (!editForm.title.trim() && !editForm.summary.trim()) {
      addToast(t("outline.titleSummaryRequired"), "error");
      return;
    }
    if (!o.updated_at) {
      addToast(t("outline.versionMissing"), "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    const formSnapshot = { ...editForm };
    setSavingId(o.id);
    try {
      const result = await saveChapterOutline({
        project_id: requestProjectId,
        outline_id: o.id,
        expected_updated_at: o.updated_at,
        chapter_number: o.chapter_number,
        title: formSnapshot.title,
        summary: formSnapshot.summary,
      });
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const refreshed = await load();
      reportRefreshFailure(refreshed);
      addOutlineMutationToast(t("outline.chapterSaved", { chapter: o.chapter_number }), result.warnings);
      cancelEdit();
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, t("outline.saveFailed")), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setSavingId(null);
      finishAction(actionSeq);
    }
  };

  const openManualChapter = () => {
    const nextChapter =
      outlinesRef.current.reduce(
        (max, outline) => Math.max(max, outline.chapter_number),
        0,
      ) + 1;
    setManualChapterInitial({ chapterNumber: nextChapter, title: "", summary: "" });
  };

  const handleSaveManualChapter = async (form: ManualChapterOutlineForm) => {
    const requestProjectId = projectId;
    if (!isPositiveSafeInteger(form.chapterNumber)) {
      addToast(t("outline.chapterNumberPositive"), "error");
      return;
    }
    if (!form.title.trim() && !form.summary.trim()) {
      addToast(t("outline.titleSummaryRequired"), "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setManualSaving("chapter");
    try {
      const result = await saveChapterOutline({
        project_id: requestProjectId,
        outline_id: null,
        expected_updated_at: null,
        chapter_number: form.chapterNumber,
        title: form.title,
        summary: form.summary,
      });
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const refreshed = await load({ showLoading: false });
      reportRefreshFailure(refreshed);
      addOutlineMutationToast(t("outline.chapterCreated", { chapter: form.chapterNumber }), result.warnings);
      setManualChapterInitial(null);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, t("outline.saveChapterFailed")), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setManualSaving(null);
      finishAction(actionSeq);
    }
  };

  const openManualPart = (part: PartOutline | null) => {
    if (part) {
      setManualPartEditor({
        part,
        initial: {
          partNumber: part.part_number,
          startChapter: part.start_chapter,
          endChapter: part.end_chapter,
          title: part.title,
          summary: part.summary ?? "",
          theme: part.theme ?? "",
        },
      });
      return;
    }
    const nextPart =
      partsRef.current.reduce((max, item) => Math.max(max, item.part_number), 0) + 1;
    const startChapter =
      partsRef.current.reduce((max, item) => Math.max(max, item.end_chapter), 0) + 1;
    const requestedSize =
      typeof chaptersPerPart === "number" && chaptersPerPart > 0 ? chaptersPerPart : 1;
    setManualPartEditor({
      part: null,
      initial: {
        partNumber: nextPart,
        startChapter,
        endChapter: startChapter + requestedSize - 1,
        title: "",
        summary: "",
        theme: "",
      },
    });
  };

  const handleSaveManualPart = async (form: ManualPartOutlineForm) => {
    const requestProjectId = projectId;
    if (
      !isPositiveSafeInteger(form.partNumber) ||
      !isPositiveSafeInteger(form.startChapter) ||
      !isPositiveSafeInteger(form.endChapter) ||
      form.endChapter < form.startChapter
    ) {
      addToast(t("outline.validPartRange"), "error");
      return;
    }
    const editor = manualPartEditor;
    if (!editor) return;
    if (editor.part && editor.part.project_id !== requestProjectId) {
      addToast(t("outline.partNotInProject"), "error");
      return;
    }
    if (editor.part && !editor.part.updated_at) {
      addToast(t("outline.partVersionMissing"), "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setManualSaving("part");
    try {
      const result = await savePartOutline({
        project_id: requestProjectId,
        part_id: editor.part?.id ?? null,
        expected_updated_at: editor.part?.updated_at ?? null,
        part_number: form.partNumber,
        start_chapter: form.startChapter,
        end_chapter: form.endChapter,
        title: form.title,
        summary: form.summary,
        theme: form.theme,
      });
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const refreshed = await load({ showLoading: false });
      reportRefreshFailure(refreshed);
      addOutlineMutationToast(t("outline.partSaved", { part: form.partNumber }), result.warnings);
      setManualPartEditor(null);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, t("outline.savePartFailed")), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setManualSaving(null);
      finishAction(actionSeq);
    }
  };

  const handleRegenOutline = async (o: ChapterOutlineListEntry) => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyRegenerateChapter"), "error");
      return;
    }
    if (o.project_id !== requestProjectId) {
      addToast(t("outline.outlineNotInProjectRegenerate"), "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setRegenId(o.id);
    let refreshOk = true;
    try {
      const direction = await requestOptimizationDirection(
        t("outline.dialogs.refineChapter", { chapter: o.chapter_number }),
        t,
      );
      if (direction === null || !isActionCurrent(requestProjectId, actionSeq)) return;
      const variables = outlineOptimizationVariables(direction);
      const warnings = await runOutlineGeneration(
        {
          title: t("outline.regenerateChapterTitle", { chapter: o.chapter_number }),
          startMessage: t("outline.rewritingChapter", { chapter: o.chapter_number }),
          successMessage: t("outline.regeneratedChapter", { chapter: o.chapter_number }),
          errorMessage: t("outline.regenerateFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "regenerate_chapter_outline",
            outline_id: o.id,
            variables,
          },
        },
        async (control) => {
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          control.setProgress(t("outline.refreshing"), 2);
          refreshOk = await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (!refreshOk) return;
      addOutlineMutationToast(t("outline.regeneratedChapter", { chapter: o.chapter_number }), warnings);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        const message = extractErrorMessage(err, t("outline.regenerateFailed"));
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setRegenId(null);
      finishAction(actionSeq);
    }
  };

  const handleRefinePart = async (part: PartOutline) => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyRefinePart"), "error");
      return;
    }
    if (part.project_id !== requestProjectId) {
      addToast(t("outline.partNotInProjectRefine"), "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setPartRegenId(part.id);
    let refreshOk = true;
    try {
      const direction = await requestOptimizationDirection(
        t("outline.dialogs.refinePart", { part: part.part_number }),
        t,
      );
      if (direction === null || !isActionCurrent(requestProjectId, actionSeq)) return;
      const warnings = await runOutlineGeneration(
        {
          title: t("outline.refinePartTitle", { part: part.part_number }),
          startMessage: t("outline.refiningPart", { part: part.part_number }),
          successMessage: t("outline.refinedPart", { part: part.part_number }),
          errorMessage: t("outline.regenerateFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "regenerate_part_outline",
            part_id: part.id,
            variables: outlineOptimizationVariables(direction),
          },
        },
        async (control) => {
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          control.setProgress(t("outline.refreshing"), 2);
          refreshOk = await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (!refreshOk) return;
      addOutlineMutationToast(t("outline.refinedPart", { part: part.part_number }), warnings);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        const message = extractErrorMessage(err, t("outline.regenerateFailed"));
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setPartRegenId(null);
      finishAction(actionSeq);
    }
  };

  const handlePreviewRegenOutline = async (o: ChapterOutlineListEntry) => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyPreview"), "error");
      return;
    }
    if (o.project_id !== requestProjectId) {
      addToast(t("outline.outlineNotInProjectDelete"), "error");
      return;
    }
    const previewSeq = beginPreview(requestProjectId);
    if (previewSeq == null) return;
    try {
      const direction = await requestOptimizationDirection(
        t("outline.dialogs.refineChapter", { chapter: o.chapter_number }),
        t,
      );
      if (direction === null || !isPreviewCurrent(requestProjectId, previewSeq)) return;
      setRequestPreviewOpen(true);
      setRequestPreviewLoading(true);
      setRequestPreviewData(null);
      const preview = await previewRegenerateChapterOutlineRequest({
        project_id: requestProjectId,
        outline_id: o.id,
        variables: outlineOptimizationVariables(direction),
      });
      if (!isPreviewCurrent(requestProjectId, previewSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, previewSeq)) {
        addToast(extractErrorMessage(err, t("outline.previewRegenerateFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, previewSeq)) setRequestPreviewLoading(false);
      finishPreview(previewSeq);
    }
  };

  const handleDeleteOutline = async (o: ChapterOutlineListEntry) => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyDeleteChapter"), "error");
      return;
    }
    if (o.project_id !== requestProjectId) {
      addToast(t("outline.outlineNotInProjectDelete"), "error");
      return;
    }
    if (!beginConfirmAction(requestProjectId)) return;
    const requestOutlines = outlinesRef.current;
    const outlineIndex = requestOutlines.findIndex((x) => x.id === o.id);
    if (outlineIndex < 0) {
      finishConfirmAction();
      addToast(t("outline.outlineListChanged"), "error");
      return;
    }
    const targets = requestOutlines.slice(outlineIndex);
    const count = targets.length;
    const lastChapterNumber = requestOutlines[requestOutlines.length - 1]?.chapter_number;
    const ok = await confirmDialog({
      title: t("outline.deleteFollowingTitle"),
      message: t("outline.deleteFollowingMessage", {
        chapter: o.chapter_number,
        lastChapter: lastChapterNumber,
        count,
      }),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!ok) {
      finishConfirmAction();
      return;
    }
    if (!isProjectCurrent(requestProjectId)) {
      finishConfirmAction();
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) {
      finishConfirmAction();
      return;
    }
    finishConfirmAction();
    setDelActing(deleteChapterOutlinesFromAction(o.id));
    try {
      const result = await deleteLatestChapterOutlines(requestProjectId, targets);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const refreshed = await load();
      reportRefreshFailure(refreshed);
      addOutlineMutationToast(t("outline.deletedChapters", { count }), result.warnings);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, t("outline.saveFailed")), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setDelActing(null);
      finishAction(actionSeq);
    }
  };

  const handleDeleteLatestParts = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyDeleteParts"), "error");
      return;
    }
    const requestPartCount = delPartCount;
    const requestParts = partsRef.current;
    const availableParts = requestParts.length;
    if (!isValidDeletionCount(requestPartCount, availableParts)) {
      setDelPartCountError(t("outline.partCountRange", { count: availableParts }));
      return;
    }
    setDelPartCountError(null);
    if (!beginConfirmAction(requestProjectId)) return;
    const ok = await confirmDialog({
      title: t("outline.deletePartsTitle"),
      message: t("outline.deletePartsMessage", { count: requestPartCount }),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!ok) {
      finishConfirmAction();
      return;
    }
    if (!isProjectCurrent(requestProjectId)) {
      finishConfirmAction();
      return;
    }
    const targets = requestParts.slice(-requestPartCount);
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) {
      finishConfirmAction();
      return;
    }
    finishConfirmAction();
    setDelActing(DELETE_LATEST_PARTS_ACTION);
    try {
      const result = await deleteLatestParts(requestProjectId, targets);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const refreshed = await load();
      reportRefreshFailure(refreshed);
      addOutlineMutationToast(t("outline.deletedParts", { count: requestPartCount }), result.warnings);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, t("outline.saveFailed")), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setDelActing(null);
      finishAction(actionSeq);
    }
  };

  const handleDeleteLatestOutlines = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyDeleteOutlines"), "error");
      return;
    }
    const requestChapterCount = delChapterCount;
    const requestOutlines = outlinesRef.current;
    const availableOutlines = requestOutlines.length;
    if (!isValidDeletionCount(requestChapterCount, availableOutlines)) {
      setDelChapterCountError(t("outline.outlineCountRange", { count: availableOutlines }));
      return;
    }
    setDelChapterCountError(null);
    if (!beginConfirmAction(requestProjectId)) return;
    const ok = await confirmDialog({
      title: t("outline.deleteOutlinesTitle"),
      message: t("outline.deleteOutlinesMessage", { count: requestChapterCount }),
      confirmText: t("common.delete"),
      dialogType: "danger",
    });
    if (!ok) {
      finishConfirmAction();
      return;
    }
    if (!isProjectCurrent(requestProjectId)) {
      finishConfirmAction();
      return;
    }
    const targets = requestOutlines.slice(-requestChapterCount);
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) {
      finishConfirmAction();
      return;
    }
    finishConfirmAction();
    setDelActing(DELETE_LATEST_CHAPTER_OUTLINES_ACTION);
    try {
      const result = await deleteLatestChapterOutlines(requestProjectId, targets);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const refreshed = await load();
      reportRefreshFailure(refreshed);
      addOutlineMutationToast(
        t("outline.deletedOutlines", { count: requestChapterCount }),
        result.warnings,
      );
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, t("outline.saveFailed")), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setDelActing(null);
      finishAction(actionSeq);
    }
  };

  const handleRegenLastPart = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyRegenerateParts"), "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setRegenLastPart(true);
    let refreshOk = true;
    try {
      const warnings = await runOutlineGeneration(
        {
          title: t("outline.regenerateLastPartTitle"),
          startMessage: t("outline.regenerateLastPartStart"),
          successMessage: t("outline.regeneratedLastPart"),
          errorMessage: t("outline.regenerateFailed"),
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "regenerate_last_part",
            variables: null,
          },
        },
        async (control) => {
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          control.setProgress(t("outline.refreshing"), 2);
          refreshOk = await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (!refreshOk) return;
      addOutlineMutationToast(t("outline.regeneratedLastPart"), warnings);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        const message = extractErrorMessage(err, t("outline.regenerateFailed"));
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setRegenLastPart(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewRegenLastPart = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast(t("outline.readOnlyPreview"), "error");
      return;
    }
    const previewSeq = beginPreview(requestProjectId);
    if (previewSeq == null) return;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewRegenerateLastPartRequest({
        project_id: requestProjectId,
        variables: null,
      });
      if (!isPreviewCurrent(requestProjectId, previewSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, previewSeq)) {
        addToast(extractErrorMessage(err, t("outline.previewLastPartFailed")), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, previewSeq)) setRequestPreviewLoading(false);
      finishPreview(previewSeq);
    }
  };

  const interactionBusy = mutationBusy || requestPreviewLoading;
  const currentProjectLoaded = loadedProjectId === projectId;
  const currentLoadError = loadError?.projectId === projectId ? loadError.message : null;

  return (
    <>
      {currentLoadError ? (
        <div role="alert" className="mb-3 flex flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <p className="min-w-0 flex-1">{currentLoadError}</p>
          <BookButton variant="secondary" size="sm" onClick={() => void handleRetry()} disabled={interactionBusy}>
            <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
            {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      <OutlinesTabView
      projectId={projectId}
      loading={loading || !currentProjectLoaded}
      partsLoaded={currentProjectLoaded && partsLoaded}
      outlinesLoaded={currentProjectLoaded && outlinesLoaded}
      activeOutlineView={activeOutlineView}
      parts={currentProjectLoaded ? parts : []}
      outlines={currentProjectLoaded ? outlines : []}
      canEditOutlines={canEditOutlines}
      partCount={partCount}
      chaptersPerPart={chaptersPerPart}
      defaultChaptersPerPart={defaultChaptersPerPart}
      generatingParts={generatingParts}
      chapterCount={chapterCount}
      generatingOutlines={generatingOutlines}
      editingId={editingId}
      editForm={editForm}
      savingId={savingId}
      regenId={regenId}
      manualChapterInitial={manualChapterInitial}
      manualPartEditor={manualPartEditor}
      manualSaving={manualSaving}
      partDetail={partDetail}
      outlineDetail={outlineDetail}
      outlineAction={outlineAction}
      advancedOpen={advancedOpen}
      delPartCount={delPartCount}
      delChapterCount={delChapterCount}
      delPartCountError={delPartCountError}
      delChapterCountError={delChapterCountError}
      delActing={delActing}
      regenLastPart={regenLastPart}
      interactionBusy={interactionBusy}
      requestPreviewOpen={requestPreviewOpen}
      requestPreviewLoading={requestPreviewLoading}
      requestPreviewData={requestPreviewData}
      runOutlineGeneration={runOutlineGeneration}
      onSetOutlineView={setActiveOutlineView}
      onPartCountChange={setPartCount}
      onChaptersPerPartChange={setChaptersPerPart}
      onGenerateParts={() => void handleGenerateParts()}
      onPreviewGenerateParts={() => void handlePreviewGenerateParts()}
      onCreatePart={() => openManualPart(null)}
      onOpenAdvanced={() => {
        if (!canEditOutlines) {
          addToast(t("outline.readOnlyTools"), "error");
          return;
        }
        setDelPartCountError(null);
        setDelChapterCountError(null);
        setAdvancedOpen(true);
      }}
      onOpenPart={setPartDetail}
      onChapterCountChange={setChapterCount}
      onGenerateChapterOutlines={() => void handleGenerateChapterOutlines()}
      onPreviewGenerateChapterOutlines={() => void handlePreviewGenerateChapterOutlines()}
      onCreateChapter={openManualChapter}
      onOpenOutlineDetail={setOutlineDetail}
      onCancelEdit={cancelEdit}
      onEditFormChange={setEditForm}
      onSaveOutline={(outline) => void handleSaveOutline(outline)}
      onOpenOutlineAction={(outline) => {
        if (!canEditOutlines) {
          addToast(t("outline.readOnlyEdit"), "error");
          return;
        }
        setOutlineAction(outline);
      }}
      onClosePartDetail={() => setPartDetail(null)}
      onEditPart={openManualPart}
      onRefinePart={(part) => void handleRefinePart(part)}
      onCloseManualChapter={() => setManualChapterInitial(null)}
      onSaveManualChapter={(form) => void handleSaveManualChapter(form)}
      onCloseManualPart={() => setManualPartEditor(null)}
      onSaveManualPart={(form) => void handleSaveManualPart(form)}
      onCloseOutlineDetail={() => setOutlineDetail(null)}
      onCloseOutlineAction={() => setOutlineAction(null)}
      onStartEdit={startEdit}
      onRegenerateOutline={(outline) => void handleRegenOutline(outline)}
      onPreviewRegenerateOutline={(outline) => void handlePreviewRegenOutline(outline)}
      onDeleteOutline={(outline) => void handleDeleteOutline(outline)}
      onCloseAdvanced={() => {
        setAdvancedOpen(false);
        setDelPartCountError(null);
        setDelChapterCountError(null);
      }}
      onDelPartCountChange={(value) => {
        setDelPartCount(value);
        setDelPartCountError(null);
      }}
      onDelChapterCountChange={(value) => {
        setDelChapterCount(value);
        setDelChapterCountError(null);
      }}
      onRegenLastPart={() => void handleRegenLastPart()}
      onPreviewRegenLastPart={() => void handlePreviewRegenLastPart()}
      onDeleteLatestParts={() => void handleDeleteLatestParts()}
      onDeleteLatestOutlines={() => void handleDeleteLatestOutlines()}
      onChanged={() => load({ showLoading: false })}
      onCloseRequestPreview={closeRequestPreview}
      />
    </>
  );
};
