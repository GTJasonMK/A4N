import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { useToast } from "../../components/feedback/Toast";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
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
  type ChapterOutline,
  type PartOutline,
  updateChapterOutline,
} from "../../api/outline";
import type { ModelRequestPreview } from "../../api/requestPreview";
import type { NovelProject } from "../../api/project";
import {
  ChapterOutlineSection,
  type ChapterOutlineEditForm,
} from "./outlines/ChapterOutlineSection";
import { OutlineAdvancedToolsDialog } from "./outlines/OutlineAdvancedToolsDialog";
import {
  ChapterOutlineActionDialog,
  ChapterOutlineDetailDialog,
  PartDetailDialog,
} from "./outlines/OutlineDialogs";
import { PartOutlineSection } from "./outlines/PartOutlineSection";
import {
  DELETE_LATEST_CHAPTER_OUTLINES_ACTION,
  DELETE_LATEST_PARTS_ACTION,
  deleteChapterOutlinesFromAction,
  isPositiveInteger,
  type NumberInputValue,
} from "./outlines/outlineTabUtils";
import { isCancelledOperation } from "../../utils/cancellation";
import { useOutlineGeneration } from "./outlines/useOutlineGeneration";

export const OutlinesTab: React.FC<{ project: NovelProject }> = ({ project }) => {
  const projectId = project.id;
  const canEditOutlines =
    project.my_role === "owner" ||
    project.my_role === "manager" ||
    project.my_role === "editor";
  const defaultPartCount: NumberInputValue = 1;
  const defaultChaptersPerPart: NumberInputValue =
    project.chapters_per_part != null && project.chapters_per_part > 0
      ? project.chapters_per_part
      : "";
  const { addToast } = useToast();
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

  // 章纲编辑 / 操作
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editForm, setEditForm] = useState<ChapterOutlineEditForm>({
    title: "",
    summary: "",
  });
  const [savingId, setSavingId] = useState<number | null>(null);
  const [regenId, setRegenId] = useState<number | null>(null);
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

  const isProjectCurrent = useCallback(
    (requestProjectId: number) =>
      mountedRef.current && activeProjectIdRef.current === requestProjectId,
    [],
  );

  const isLoadCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && loadSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isActionCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && actionSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isPreviewCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && previewSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const beginAction = useCallback(
    (requestProjectId: number): number | null => {
      if (!isProjectCurrent(requestProjectId)) return null;
      if (!canEditOutlines) {
        addToast("只读协作者不能修改章节大纲", "error");
        return null;
      }
      if (actionBusyRef.current) {
        addToast("已有大纲操作进行中，请等待当前操作完成", "error");
        return null;
      }
      actionBusyRef.current = true;
      actionSeqRef.current += 1;
      return actionSeqRef.current;
    },
    [addToast, canEditOutlines, isProjectCurrent],
  );

  const finishAction = useCallback((requestSeq: number) => {
    if (actionSeqRef.current === requestSeq) {
      actionBusyRef.current = false;
    }
  }, []);

  const invalidatePreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyRef.current = false;
  }, []);

  const beginPreview = useCallback(
    (requestProjectId: number): number | null => {
      if (!isProjectCurrent(requestProjectId)) return null;
      if (!canEditOutlines) {
        addToast("只读协作者不能预览大纲生成请求", "error");
        return null;
      }
      if (previewBusyRef.current) return null;
      previewBusyRef.current = true;
      previewSeqRef.current += 1;
      return previewSeqRef.current;
    },
    [addToast, canEditOutlines, isProjectCurrent],
  );

  const finishPreview = useCallback((requestSeq: number) => {
    if (previewSeqRef.current === requestSeq) {
      previewBusyRef.current = false;
    }
  }, []);

  const beginConfirmAction = useCallback(
    (requestProjectId: number): boolean => {
      if (!isProjectCurrent(requestProjectId)) return false;
      if (!canEditOutlines) {
        addToast("只读协作者不能修改章节大纲", "error");
        return false;
      }
      if (actionBusyRef.current || confirmBusyRef.current) {
        addToast("已有大纲操作进行中，请等待当前操作完成", "error");
        return false;
      }
      confirmBusyRef.current = true;
      return true;
    },
    [addToast, canEditOutlines, isProjectCurrent],
  );

  const finishConfirmAction = useCallback(() => {
    confirmBusyRef.current = false;
  }, []);

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
      const [pp, oo] = await Promise.all([
        listParts(requestProjectId),
        listChapterOutlines(requestProjectId),
      ]);
      if (!isLoadCurrent(requestProjectId, requestSeq)) return false;
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
        addToast(extractErrorMessage(err, "加载大纲失败"), "error");
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
    setEditingId(null);
    setEditForm({ title: "", summary: "" });
    setSavingId(null);
    setRegenId(null);
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
      addToast("只读协作者不能生成分部大纲", "error");
      return;
    }
    const requestedPartCount = typeof partCount === "number" ? partCount : null;
    const requestedChaptersPerPart =
      typeof chaptersPerPart === "number" ? chaptersPerPart : null;

    if (!isPositiveInteger(requestedPartCount)) {
      addToast("生成部数必须为正整数", "error");
      return;
    }
    if (
      requestedChaptersPerPart !== null &&
      !isPositiveInteger(requestedChaptersPerPart)
    ) {
      addToast("每部章数必须为正整数", "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    const extendingParts = partsRef.current.length > 0;
    const progressActionLabel = extendingParts ? "追加分部大纲" : "生成分部大纲";
    const successMessage = extendingParts
      ? `已追加 ${requestedPartCount} 个分部`
      : `已生成 ${requestedPartCount} 个分部`;
    setGeneratingParts(true);
    try {
      await runOutlineGeneration(
        {
          title: progressActionLabel,
          startMessage: `模型正在${progressActionLabel}，耗时可能较长…`,
          successMessage,
          errorMessage: extendingParts ? "追加分部失败" : "生成分部失败",
          total: 3,
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
          control.setProgress("正在刷新…", 2);
          await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(successMessage, "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        await load({ showLoading: false });
        const message = extractErrorMessage(
          err,
          extendingParts ? "追加分部失败" : "生成分部失败",
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
      addToast("只读协作者不能预览大纲生成请求", "error");
      return;
    }
    const requestedPartCount = typeof partCount === "number" ? partCount : null;
    const requestedChaptersPerPart =
      typeof chaptersPerPart === "number" ? chaptersPerPart : null;
    if (!isPositiveInteger(requestedPartCount)) {
      addToast("生成部数必须为正整数", "error");
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
        addToast(extractErrorMessage(err, "预览分部提示词失败"), "error");
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
      addToast("只读协作者不能生成章节大纲", "error");
      return;
    }
    const requestedChapterCount = chapterCount;
    if (chapterCount <= 0) {
      addToast("章数必须大于 0", "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setGeneratingOutlines(true);
    try {
      const startChapter =
        outlinesRef.current.reduce((max, outline) => Math.max(max, outline.chapter_number), 0) + 1;
      const existingOutlineCount = outlinesRef.current.length;
      await runOutlineGeneration(
        {
          title: existingOutlineCount > 0 ? "继续生成章纲" : "生成章节大纲",
          startMessage: `模型正在生成第 ${startChapter} 章起的 ${requestedChapterCount} 个章纲…`,
          successMessage: `已追加生成 ${requestedChapterCount} 个章纲`,
          errorMessage: "生成章纲失败",
          total: 3,
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
          control.setProgress("正在刷新…", 2);
          await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(`已追加生成 ${requestedChapterCount} 个章纲`, "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        await load({ showLoading: false });
        const message = extractErrorMessage(err, "生成章纲失败");
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
      addToast("只读协作者不能预览大纲生成请求", "error");
      return;
    }
    const requestedChapterCount = chapterCount;
    if (requestedChapterCount <= 0) {
      addToast("章数必须大于 0", "error");
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
        addToast(extractErrorMessage(err, "预览章纲提示词失败"), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, previewSeq)) setRequestPreviewLoading(false);
      finishPreview(previewSeq);
    }
  };

  const startEdit = (o: ChapterOutline) => {
    if (!canEditOutlines) {
      addToast("只读协作者不能编辑章节大纲", "error");
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
      addToast("只读协作者不能编辑章节大纲", "error");
      return;
    }
    if (o.project_id !== requestProjectId) {
      addToast("章纲不属于当前项目，无法保存", "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    const formSnapshot = { ...editForm };
    setSavingId(o.id);
    try {
      await updateChapterOutline({
        project_id: requestProjectId,
        outline_id: o.id,
        title: formSnapshot.title,
        summary: formSnapshot.summary,
      });
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(`已保存第 ${o.chapter_number} 章章纲`, "success");
      cancelEdit();
      await load();
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, "保存失败"), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setSavingId(null);
      finishAction(actionSeq);
    }
  };

  const handleRegenOutline = async (o: ChapterOutline) => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast("只读协作者不能重新生成章节大纲", "error");
      return;
    }
    if (o.project_id !== requestProjectId) {
      addToast("章纲不属于当前项目，无法重新生成", "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setRegenId(o.id);
    try {
      await runOutlineGeneration(
        {
          title: `重新生成第 ${o.chapter_number} 章章纲`,
          startMessage: `模型正在重写第 ${o.chapter_number} 章章纲…`,
          successMessage: `已重新生成第 ${o.chapter_number} 章章纲`,
          errorMessage: "重新生成失败",
          total: 3,
        },
        {
          project_id: requestProjectId,
          operation: {
            kind: "regenerate_chapter_outline",
            outline_id: o.id,
            variables: null,
          },
        },
        async (control) => {
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          control.setProgress("正在刷新…", 2);
          await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(`已重新生成第 ${o.chapter_number} 章章纲`, "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        const message = extractErrorMessage(err, "重新生成失败");
        if (!isCancelledOperation(message)) addToast(message, "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setRegenId(null);
      finishAction(actionSeq);
    }
  };

  const handlePreviewRegenOutline = async (o: ChapterOutline) => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast("只读协作者不能预览大纲生成请求", "error");
      return;
    }
    if (o.project_id !== requestProjectId) {
      addToast("章纲不属于当前项目，无法预览", "error");
      return;
    }
    const previewSeq = beginPreview(requestProjectId);
    if (previewSeq == null) return;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewRegenerateChapterOutlineRequest({
        project_id: requestProjectId,
        outline_id: o.id,
        variables: null,
      });
      if (!isPreviewCurrent(requestProjectId, previewSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, previewSeq)) {
        addToast(extractErrorMessage(err, "预览重生成提示词失败"), "error");
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
      addToast("只读协作者不能删除章节大纲", "error");
      return;
    }
    if (o.project_id !== requestProjectId) {
      addToast("章纲不属于当前项目，无法删除", "error");
      return;
    }
    if (!beginConfirmAction(requestProjectId)) return;
    const requestOutlines = outlinesRef.current;
    const outlineIndex = requestOutlines.findIndex((x) => x.id === o.id);
    if (outlineIndex < 0) {
      finishConfirmAction();
      addToast("章纲列表已变化，请重新选择", "error");
      return;
    }
    const count = requestOutlines.length - outlineIndex;
    const lastChapterNumber = requestOutlines[requestOutlines.length - 1]?.chapter_number;
    const ok = await confirmDialog({
      title: "删除后续章纲",
      message:
        `确认删除第 ${o.chapter_number} 章及其之后所有章纲？` +
        `\n\n将删除第 ${o.chapter_number} 章到第 ${lastChapterNumber} 章，共 ${count} 个章纲。`,
      confirmText: "删除",
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
    const currentOutlines = outlinesRef.current;
    const currentIndex = currentOutlines.findIndex((x) => x.id === o.id);
    const currentCount = currentIndex < 0 ? 0 : currentOutlines.length - currentIndex;
    const currentLastChapterNumber = currentOutlines[currentOutlines.length - 1]?.chapter_number;
    if (
      currentIndex < 0 ||
      currentCount !== count ||
      currentLastChapterNumber !== lastChapterNumber
    ) {
      finishConfirmAction();
      addToast("章纲列表已变化，请重新确认删除范围", "error");
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
      await deleteLatestChapterOutlines(requestProjectId, count);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(`已删除最近 ${count} 个章纲`, "success");
      await load();
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, "删除失败"), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setDelActing(null);
      finishAction(actionSeq);
    }
  };

  const handleDeleteLatestParts = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast("只读协作者不能删除分部大纲", "error");
      return;
    }
    const requestPartCount = delPartCount;
    const availableParts = partsRef.current.length;
    if (requestPartCount <= 0 || requestPartCount > availableParts) {
      addToast(`输入 1 ~ ${availableParts} 之间的数字`, "error");
      return;
    }
    if (!beginConfirmAction(requestProjectId)) return;
    const ok = await confirmDialog({
      title: "删除分部大纲",
      message: `删除最近 ${requestPartCount} 个分部？不可撤销。`,
      confirmText: "删除",
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
    if (requestPartCount > partsRef.current.length) {
      finishConfirmAction();
      addToast("分部列表已变化，请重新确认删除数量", "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) {
      finishConfirmAction();
      return;
    }
    finishConfirmAction();
    setDelActing(DELETE_LATEST_PARTS_ACTION);
    try {
      await deleteLatestParts(requestProjectId, requestPartCount);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(`已删除最近 ${requestPartCount} 个分部`, "success");
      await load();
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, "删除失败"), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setDelActing(null);
      finishAction(actionSeq);
    }
  };

  const handleDeleteLatestOutlines = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast("只读协作者不能删除章节大纲", "error");
      return;
    }
    const requestChapterCount = delChapterCount;
    const availableOutlines = outlinesRef.current.length;
    if (requestChapterCount <= 0 || requestChapterCount > availableOutlines) {
      addToast(`输入 1 ~ ${availableOutlines} 之间的数字`, "error");
      return;
    }
    if (!beginConfirmAction(requestProjectId)) return;
    const ok = await confirmDialog({
      title: "删除章纲",
      message: `删除最近 ${requestChapterCount} 个章纲？不可撤销。`,
      confirmText: "删除",
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
    if (requestChapterCount > outlinesRef.current.length) {
      finishConfirmAction();
      addToast("章纲列表已变化，请重新确认删除数量", "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) {
      finishConfirmAction();
      return;
    }
    finishConfirmAction();
    setDelActing(DELETE_LATEST_CHAPTER_OUTLINES_ACTION);
    try {
      await deleteLatestChapterOutlines(requestProjectId, requestChapterCount);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(`已删除最近 ${requestChapterCount} 个章纲`, "success");
      await load();
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, "删除失败"), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setDelActing(null);
      finishAction(actionSeq);
    }
  };

  const handleRegenLastPart = async () => {
    const requestProjectId = projectId;
    if (!canEditOutlines) {
      addToast("只读协作者不能重新生成分部大纲", "error");
      return;
    }
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setRegenLastPart(true);
    try {
      await runOutlineGeneration(
        {
          title: "重新生成最后一部",
          startMessage: "模型正在重写最后一部大纲…",
          successMessage: "最后一部已重新生成",
          errorMessage: "重新生成失败",
          total: 3,
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
          control.setProgress("正在刷新…", 2);
          await load({ showLoading: false });
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast("最后一部已重新生成", "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        const message = extractErrorMessage(err, "重新生成失败");
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
      addToast("只读协作者不能预览大纲生成请求", "error");
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
        addToast(extractErrorMessage(err, "预览最后一部重生成提示词失败"), "error");
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
    regenId != null ||
    delActing != null ||
    regenLastPart;
  const interactionBusy = mutationBusy || requestPreviewLoading;

  if (loading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <Loader2 size={12} className="animate-spin" /> 加载大纲…
      </div>
    );
  }

  return (
    <div className="content-main flex flex-col gap-5">
      <PartOutlineSection
        parts={parts}
        canEdit={canEditOutlines}
        partCount={partCount}
        chaptersPerPart={chaptersPerPart}
        defaultChaptersPerPart={defaultChaptersPerPart}
        generating={generatingParts}
        disabled={interactionBusy}
        onPartCountChange={setPartCount}
        onChaptersPerPartChange={setChaptersPerPart}
        onGenerate={() => void handleGenerateParts()}
        onPreviewGenerate={() => void handlePreviewGenerateParts()}
        onOpenAdvanced={() => {
          if (!canEditOutlines) {
            addToast("只读协作者不能使用大纲高级工具", "error");
            return;
          }
          setAdvancedOpen(true);
        }}
        onOpenPart={setPartDetail}
      />

      <ChapterOutlineSection
        outlines={outlines}
        canEdit={canEditOutlines}
        chapterCount={chapterCount}
        generating={generatingOutlines}
        editingId={editingId}
        editForm={editForm}
        savingId={savingId}
        regenId={regenId}
        delActing={delActing}
        disabled={interactionBusy}
        onChapterCountChange={setChapterCount}
        onGenerate={() => void handleGenerateChapterOutlines()}
        onPreviewGenerate={() => void handlePreviewGenerateChapterOutlines()}
        onOpenDetail={setOutlineDetail}
        onCancelEdit={cancelEdit}
        onEditFormChange={setEditForm}
        onSave={(outline) => void handleSaveOutline(outline)}
        onOpenAction={(outline) => {
          if (!canEditOutlines) {
            addToast("只读协作者不能修改章节大纲", "error");
            return;
          }
          setOutlineAction(outline);
        }}
      />

      {partDetail && (
        <PartDetailDialog part={partDetail} onClose={() => setPartDetail(null)} />
      )}
      {outlineDetail && (
        <ChapterOutlineDetailDialog
          outline={outlineDetail}
          onClose={() => setOutlineDetail(null)}
        />
      )}
      {canEditOutlines && (
        <ChapterOutlineActionDialog
          outline={outlineAction}
          regenId={regenId}
          delActing={delActing}
          disabled={interactionBusy}
          onClose={() => setOutlineAction(null)}
          onEdit={startEdit}
          onRegenerate={handleRegenOutline}
          onPreviewRegenerate={(outline) => void handlePreviewRegenOutline(outline)}
          onDelete={handleDeleteOutline}
        />
      )}

      {canEditOutlines && (
        <OutlineAdvancedToolsDialog
          open={advancedOpen}
          projectId={projectId}
          parts={parts}
          outlines={outlines}
          partCount={partCount}
          chaptersPerPart={chaptersPerPart}
          delPartCount={delPartCount}
          delChapterCount={delChapterCount}
          delActing={delActing}
          regenLastPart={regenLastPart}
          runGeneration={runOutlineGeneration}
          disabled={interactionBusy}
          onClose={() => setAdvancedOpen(false)}
          onDelPartCountChange={setDelPartCount}
          onDelChapterCountChange={setDelChapterCount}
          onRegenLastPart={() => void handleRegenLastPart()}
          onPreviewRegenLastPart={() => void handlePreviewRegenLastPart()}
          onDeleteLatestParts={() => void handleDeleteLatestParts()}
          onDeleteLatestOutlines={() => void handleDeleteLatestOutlines()}
          onChanged={async () => {
            await load({ showLoading: false });
          }}
        />
      )}
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="大纲请求预览"
        onClose={closeRequestPreview}
      />
    </div>
  );
};
