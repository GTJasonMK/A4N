import React, { useCallback, useEffect, useRef, useState } from "react";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../api";
import {
  deleteLatestChapterOutlines,
  deleteLatestParts,
  listChapterOutlines,
  listParts,
  previewGenerateChapterOutlinesByCountRequest,
  previewGeneratePartsRequest,
  previewRegenerateChapterOutlineRequest,
  previewRegenerateLastPartRequest,
  saveChapterOutline,
  savePartOutline,
  type ChapterOutline,
  type PartOutline,
} from "../../api/outline";
import type { ModelRequestPreview } from "../../api/requestPreview";
import { canEditProjectMaterials, type NovelProject } from "../../api/project";
import type { ChapterOutlineEditForm } from "./outlines/ChapterOutlineSection";
import type {
  ManualChapterOutlineForm,
  ManualPartOutlineForm,
} from "./outlines/OutlineDialogs";
import {
  DELETE_LATEST_CHAPTER_OUTLINES_ACTION,
  DELETE_LATEST_PARTS_ACTION,
  deleteChapterOutlinesFromAction,
  isPositiveInteger,
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
  const { run: runOutlineGeneration } = useOutlineGeneration();
  const [parts, setParts] = useState<PartOutline[]>([]);
  const [outlines, setOutlines] = useState<ChapterOutline[]>([]);
  const [loading, setLoading] = useState(true);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] = useState<ModelRequestPreview | null>(null);

  // 生成参数 + 状态
  const [partCount, setPartCount] =
    useState<NumberInputValue>(defaultPartCount);
  const [chaptersPerPart, setChaptersPerPart] =
    useState<NumberInputValue>(defaultChaptersPerPart);
  const [generatingParts, setGeneratingParts] = useState(false);
  const [chapterCount, setChapterCount] = useState(10);
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
  const [outlineDetail, setOutlineDetail] = useState<ChapterOutline | null>(null);
  const [outlineAction, setOutlineAction] = useState<ChapterOutline | null>(null);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  // 危险操作
  const [delPartCount, setDelPartCount] = useState(1);
  const [delChapterCount, setDelChapterCount] = useState(1);
  const [delActing, setDelActing] = useState<string | null>(null);
  const [regenLastPart, setRegenLastPart] = useState(false);

  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const partsRef = useRef<PartOutline[]>([]);
  const outlinesRef = useRef<ChapterOutline[]>([]);
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
        listChapterOutlines(requestProjectId),
      ]);
      if (!isLoadCurrent(requestProjectId, requestSeq)) return false;
      if (partsResult.status === "rejected") {
        addToast(extractErrorMessage(partsResult.reason, t("outline.loadFailed")), "error");
      }
      if (outlinesResult.status === "rejected") {
        addToast(extractErrorMessage(outlinesResult.reason, t("outline.loadFailed")), "error");
      }
      if (partsResult.status === "rejected" && outlinesResult.status === "rejected") {
        return false;
      }
      const pp = partsResult.status === "fulfilled" ? partsResult.value : partsRef.current;
      const oo = outlinesResult.status === "fulfilled" ? outlinesResult.value : outlinesRef.current;
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
      return true;
    } catch (err) {
      if (isLoadCurrent(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, t("outline.loadFailed")), "error");
      }
      return false;
    } finally {
      if (showLoading && isLoadCurrent(requestProjectId, requestSeq)) setLoading(false);
    }
  }, [addToast, isLoadCurrent, projectId]);

  useEffect(() => {
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

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    loadSeqRef.current += 1;
    actionSeqRef.current += 1;
    previewSeqRef.current += 1;
    actionBusyRef.current = false;
    previewBusyRef.current = false;
    confirmBusyRef.current = false;
    partsRef.current = [];
    outlinesRef.current = [];
    setParts([]);
    setOutlines([]);
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

  useEffect(() => {
    setPartCount(defaultPartCount);
    setChaptersPerPart(defaultChaptersPerPart);
  }, [defaultChaptersPerPart, defaultPartCount, projectId]);

  useEffect(() => {
    if (canEditOutlines) return;
    setEditingId(null);
    setEditForm({ title: "", summary: "" });
    setManualChapterInitial(null);
    setManualPartEditor(null);
    setOutlineAction(null);
    setAdvancedOpen(false);
  }, [canEditOutlines]);

  useEffect(() => {
    setPartDetail((current) => {
      if (!current) return null;
      return parts.find((part) => part.id === current.id) ?? null;
    });
    partsRef.current = parts;
  }, [parts]);

  useEffect(() => {
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
          await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
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
    const requestedChapterCount = chapterCount;
    if (chapterCount <= 0) {
      addToast(t("outline.chapterCountPositive"), "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setGeneratingOutlines(true);
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
          await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
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
    const requestedChapterCount = chapterCount;
    if (requestedChapterCount <= 0) {
      addToast(t("outline.chapterCountPositive"), "error");
      return;
    }
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

  const startEdit = (o: ChapterOutline) => {
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

  const handleSaveOutline = async (o: ChapterOutline) => {
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
      addOutlineMutationToast(t("outline.chapterSaved", { chapter: o.chapter_number }), result.warnings);
      cancelEdit();
      await load();
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
    if (!Number.isInteger(form.chapterNumber) || form.chapterNumber <= 0) {
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
      addOutlineMutationToast(t("outline.chapterCreated", { chapter: form.chapterNumber }), result.warnings);
      setManualChapterInitial(null);
      await load({ showLoading: false });
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
      !Number.isInteger(form.partNumber) ||
      !Number.isInteger(form.startChapter) ||
      !Number.isInteger(form.endChapter) ||
      form.partNumber <= 0 ||
      form.startChapter <= 0 ||
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
      addOutlineMutationToast(t("outline.partSaved", { part: form.partNumber }), result.warnings);
      setManualPartEditor(null);
      await load({ showLoading: false });
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, t("outline.savePartFailed")), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setManualSaving(null);
      finishAction(actionSeq);
    }
  };

  const handleRegenOutline = async (o: ChapterOutline) => {
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
          await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
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
          await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
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

  const handlePreviewRegenOutline = async (o: ChapterOutline) => {
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

  const handleDeleteOutline = async (o: ChapterOutline) => {
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
      addOutlineMutationToast(t("outline.deletedChapters", { count }), result.warnings);
      await load();
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
    if (requestPartCount <= 0 || requestPartCount > availableParts) {
      addToast(t("outline.partCountRange", { count: availableParts }), "error");
      return;
    }
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
      addOutlineMutationToast(t("outline.deletedParts", { count: requestPartCount }), result.warnings);
      await load();
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
    if (requestChapterCount <= 0 || requestChapterCount > availableOutlines) {
      addToast(t("outline.outlineCountRange", { count: availableOutlines }), "error");
      return;
    }
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
      addOutlineMutationToast(
        t("outline.deletedOutlines", { count: requestChapterCount }),
        result.warnings,
      );
      await load();
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
          await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
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

  const mutationBusy =
    generatingParts ||
    generatingOutlines ||
    savingId != null ||
    manualSaving != null ||
    regenId != null ||
    delActing != null ||
    regenLastPart ||
    partRegenId != null;
  const interactionBusy = mutationBusy || requestPreviewLoading;

  return (
    <OutlinesTabView
      projectId={projectId}
      loading={loading}
      activeOutlineView={activeOutlineView}
      parts={parts}
      outlines={outlines}
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
      onCloseAdvanced={() => setAdvancedOpen(false)}
      onDelPartCountChange={setDelPartCount}
      onDelChapterCountChange={setDelChapterCount}
      onRegenLastPart={() => void handleRegenLastPart()}
      onPreviewRegenLastPart={() => void handlePreviewRegenLastPart()}
      onDeleteLatestParts={() => void handleDeleteLatestParts()}
      onDeleteLatestOutlines={() => void handleDeleteLatestOutlines()}
      onChanged={async () => {
        await load({ showLoading: false });
      }}
      onCloseRequestPreview={closeRequestPreview}
    />
  );
};
