import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { useToast } from "../../../components/feedback/Toast";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { extractErrorMessage } from "../../../api";
import {
  applyBeatMapping,
  batchGeneratePartOutlines,
  continuePartOutlines,
  generatePartChapterOutlines,
  getChapterOutlinesPacing,
  getGenerationProgress,
  previewApplyBeatMappingRequest,
  previewBatchGeneratePartOutlinesRequest,
  previewContinuePartOutlinesRequest,
  previewGeneratePartChapterOutlinesRequest,
  previewRegeneratePartOutlineRequest,
  previewRegeneratePartsRequest,
  regeneratePartOutline,
  regenerateParts,
  type GenerationProgress,
  type PartOutline,
} from "../../../api/outline";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import {
  createProgressTaskId,
  type RunProgressTask,
} from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { nonEmptyText } from "../../../utils/text";

interface OutlineAdvancedPanelProps {
  projectId: number;
  parts: PartOutline[];
  partCount: number | null;
  chaptersPerPart: number | null;
  runProgressTask: RunProgressTask;
  disabled?: boolean;
  onChanged: () => void | Promise<void>;
  onBusyChange?: (busy: boolean) => void;
}

const partOptionLabel = (part: PartOutline): string => {
  const title = nonEmptyText(part.title);
  return title ? `#${part.part_number} ${title}` : `#${part.part_number}`;
};

export const OutlineAdvancedPanel: React.FC<OutlineAdvancedPanelProps> = ({
  projectId,
  parts,
  partCount,
  chaptersPerPart,
  runProgressTask,
  disabled = false,
  onChanged,
  onBusyChange,
}) => {
  const { addToast } = useToast();

  const [progress, setProgress] = useState<GenerationProgress | null>(null);
  const [progressLoading, setProgressLoading] = useState(false);
  const [autoPoll, setAutoPoll] = useState(false);

  const [pacing, setPacing] = useState<unknown>(null);
  const [pacingLoading, setPacingLoading] = useState(false);

  const [beatJson, setBeatJson] = useState<string>(
    '{"beats": [{"chapter": 1, "beat": "起"}, {"chapter": 5, "beat": "承"}]}',
  );
  const [beatBusy, setBeatBusy] = useState(false);

  const [selectedPartId, setSelectedPartId] = useState<number | "">("");
  const [partGenBusy, setPartGenBusy] = useState(false);
  const [partRegenBusy, setPartRegenBusy] = useState(false);

  const [continueBusy, setContinueBusy] = useState(false);
  const [regenPartsBusy, setRegenPartsBusy] = useState(false);
  const [batchBusy, setBatchBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] = useState<ModelRequestPreview | null>(null);

  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const disabledRef = useRef(disabled);
  const partsRef = useRef<PartOutline[]>(parts);
  const progressSeqRef = useRef(0);
  const pacingSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const actionBusyRef = useRef(false);
  const previewBusyRef = useRef(false);
  const confirmBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;
  disabledRef.current = disabled;
  partsRef.current = parts;

  const mutationBusy =
    beatBusy ||
    partGenBusy ||
    partRegenBusy ||
    continueBusy ||
    regenPartsBusy ||
    batchBusy;
  const interactionBusy = mutationBusy || requestPreviewLoading;
  const controlsDisabled = disabled || interactionBusy;

  useEffect(() => {
    onBusyChange?.(interactionBusy);
  }, [interactionBusy, onBusyChange]);

  useEffect(() => {
    return () => {
      if (!actionBusyRef.current) {
        onBusyChange?.(false);
      }
    };
  }, [onBusyChange]);

  const showGenerationError = useCallback((err: unknown, fallback: string) => {
    const message = extractErrorMessage(err, fallback);
    if (!isCancelledOperation(message)) addToast(message, "error");
  }, [addToast]);

  const isProjectCurrent = useCallback(
    (requestProjectId: number) =>
      mountedRef.current && activeProjectIdRef.current === requestProjectId,
    [],
  );

  const isProgressCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && progressSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isPacingCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && pacingSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isPreviewCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && previewSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isActionCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && actionSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const ensureActionCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) => {
      if (!isActionCurrent(requestProjectId, requestSeq)) {
        throw new Error("任务已停止");
      }
    },
    [isActionCurrent],
  );

  const beginAction = useCallback(
    (requestProjectId: number): number | null => {
      if (!isProjectCurrent(requestProjectId)) return null;
      if (disabledRef.current) {
        addToast("已有大纲操作进行中，请等待当前操作完成", "error");
        return null;
      }
      if (actionBusyRef.current) {
        addToast("已有大纲高级操作进行中，请等待当前操作完成", "error");
        return null;
      }
      actionBusyRef.current = true;
      actionSeqRef.current += 1;
      onBusyChange?.(true);
      return actionSeqRef.current;
    },
    [addToast, isProjectCurrent, onBusyChange],
  );

  const finishAction = useCallback((requestSeq: number) => {
    if (actionSeqRef.current === requestSeq) {
      actionBusyRef.current = false;
      onBusyChange?.(false);
    }
  }, [onBusyChange]);

  const beginPreview = useCallback(
    (requestProjectId: number): number | null => {
      if (!isProjectCurrent(requestProjectId)) return null;
      if (previewBusyRef.current) return null;
      previewBusyRef.current = true;
      previewSeqRef.current += 1;
      return previewSeqRef.current;
    },
    [isProjectCurrent],
  );

  const finishPreview = useCallback((requestSeq: number) => {
    if (previewSeqRef.current === requestSeq) {
      previewBusyRef.current = false;
    }
  }, []);

  const beginConfirmAction = useCallback(
    (requestProjectId: number): boolean => {
      if (!isProjectCurrent(requestProjectId)) return false;
      if (disabledRef.current || actionBusyRef.current || confirmBusyRef.current) {
        addToast("已有大纲高级操作进行中，请等待当前操作完成", "error");
        return false;
      }
      confirmBusyRef.current = true;
      return true;
    },
    [addToast, isProjectCurrent],
  );

  const finishConfirmAction = useCallback(() => {
    confirmBusyRef.current = false;
  }, []);

  const selectedPart = useCallback(
    (requestProjectId: number): PartOutline | null => {
      if (typeof selectedPartId !== "number") {
        addToast("请选择部", "error");
        return null;
      }
      const part = partsRef.current.find((item) => item.id === selectedPartId) ?? null;
      if (!part || part.project_id !== requestProjectId) {
        addToast("分部列表已变化，请重新选择", "error");
        return null;
      }
      return part;
    },
    [addToast, selectedPartId],
  );

  const closeRequestPreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyRef.current = false;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, []);

  const openPreview = async (
    requestProjectId: number,
    loadPreview: () => Promise<ModelRequestPreview>,
    fallback: string,
  ) => {
    const requestSeq = beginPreview(requestProjectId);
    if (requestSeq == null) return;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await loadPreview();
      if (!isPreviewCurrent(requestProjectId, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, fallback), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, requestSeq)) setRequestPreviewLoading(false);
      finishPreview(requestSeq);
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      progressSeqRef.current += 1;
      pacingSeqRef.current += 1;
      previewSeqRef.current += 1;
      actionSeqRef.current += 1;
      actionBusyRef.current = false;
      previewBusyRef.current = false;
      confirmBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    partsRef.current = parts;
    progressSeqRef.current += 1;
    pacingSeqRef.current += 1;
    previewSeqRef.current += 1;
    actionSeqRef.current += 1;
    actionBusyRef.current = false;
    previewBusyRef.current = false;
    confirmBusyRef.current = false;
    onBusyChange?.(false);
    setProgress(null);
    setProgressLoading(false);
    setPacing(null);
    setPacingLoading(false);
    setAutoPoll(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setBeatBusy(false);
    setPartGenBusy(false);
    setPartRegenBusy(false);
    setContinueBusy(false);
    setRegenPartsBusy(false);
    setBatchBusy(false);
    setSelectedPartId("");
  }, [onBusyChange, projectId]);

  useEffect(() => {
    partsRef.current = parts;
    setSelectedPartId((current) => {
      if (current === "") return current;
      return parts.some((part) => part.id === current && part.project_id === projectId)
        ? current
        : "";
    });
  }, [parts, projectId]);

  const loadProgress = useCallback(async () => {
    const requestProjectId = projectId;
    const requestSeq = progressSeqRef.current + 1;
    progressSeqRef.current = requestSeq;
    setProgressLoading(true);
    try {
      const result = await getGenerationProgress(requestProjectId);
      if (!isProgressCurrent(requestProjectId, requestSeq)) return;
      setProgress(result);
    } catch (err) {
      if (isProgressCurrent(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, "查询进度失败"), "error");
      }
    } finally {
      if (isProgressCurrent(requestProjectId, requestSeq)) setProgressLoading(false);
    }
  }, [addToast, isProgressCurrent, projectId]);

  useEffect(() => {
    if (!autoPoll || disabled) return;
    void loadProgress();
    const timer = setInterval(() => void loadProgress(), 3000);
    return () => clearInterval(timer);
  }, [autoPoll, disabled, loadProgress]);

  const loadPacing = async () => {
    const requestProjectId = projectId;
    const requestSeq = pacingSeqRef.current + 1;
    pacingSeqRef.current = requestSeq;
    setPacingLoading(true);
    try {
      const result = await getChapterOutlinesPacing(requestProjectId);
      if (!isPacingCurrent(requestProjectId, requestSeq)) return;
      setPacing(result);
    } catch (err) {
      if (isPacingCurrent(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, "节奏分析失败"), "error");
      }
    } finally {
      if (isPacingCurrent(requestProjectId, requestSeq)) setPacingLoading(false);
    }
  };

  const handleBeatMap = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    let payload: Record<string, unknown>;
    try {
      const value = JSON.parse(beatJson);
      if (typeof value !== "object" || value === null) {
        throw new Error("必须是对象");
      }
      payload = value as Record<string, unknown>;
    } catch (err) {
      addToast(`JSON 解析失败：${extractErrorMessage(err, "")}`, "error");
      return;
    }
    const beatSheet = JSON.stringify(payload);
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setBeatBusy(true);
    try {
      const taskId = createProgressTaskId("outline-beat-mapping", requestProjectId);
      await runProgressTask(
        {
          title: "应用节拍映射",
          startMessage: "准备调用模型应用节拍映射…",
          successMessage: "节拍映射已应用",
          errorMessage: "应用节拍失败",
          total: 3,
          taskId,
        },
        async ({ setProgress }) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress("模型正在把节拍映射到章节大纲…", 1);
          await applyBeatMapping({
            project_id: requestProjectId,
            part_id: requestPart.id,
            beat_sheet: beatSheet,
            task_id: taskId,
          });
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress("正在刷新章纲列表…", 2);
          await onChanged();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast("节拍映射已应用", "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        await onChanged();
        showGenerationError(err, "应用节拍失败");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setBeatBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewBeatMap = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    let payload: Record<string, unknown>;
    try {
      const value = JSON.parse(beatJson);
      if (typeof value !== "object" || value === null) throw new Error("必须是对象");
      payload = value as Record<string, unknown>;
    } catch (err) {
      addToast(`JSON 解析失败：${extractErrorMessage(err, "")}`, "error");
      return;
    }
    const beatSheet = JSON.stringify(payload);
    await openPreview(
      requestProjectId,
      () =>
        previewApplyBeatMappingRequest({
          project_id: requestProjectId,
          part_id: requestPart.id,
          beat_sheet: beatSheet,
        }),
      "预览节拍映射提示词失败",
    );
  };

  const handleGeneratePartChapters = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setPartGenBusy(true);
    try {
      const taskId = createProgressTaskId("outline-part-chapters", requestProjectId);
      await runProgressTask(
        {
          title: `生成部 #${requestPart.part_number} 章纲`,
          startMessage: "准备调用模型生成本部章纲…",
          successMessage: `已为部 #${requestPart.part_number} 生成章纲`,
          errorMessage: "生成失败",
          total: 3,
          taskId,
        },
        async ({ setProgress }) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress(`模型正在为部 #${requestPart.part_number} 生成章纲…`, 1);
          await generatePartChapterOutlines({
            project_id: requestProjectId,
            part_id: requestPart.id,
            task_id: taskId,
          });
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress("正在刷新章纲列表…", 2);
          await onChanged();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(`已为部 #${requestPart.part_number} 生成章纲`, "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        await onChanged();
        showGenerationError(err, "生成失败");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setPartGenBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewGeneratePartChapters = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    await openPreview(
      requestProjectId,
      () =>
        previewGeneratePartChapterOutlinesRequest({
          project_id: requestProjectId,
          part_id: requestPart.id,
        }),
      "预览本部章纲提示词失败",
    );
  };

  const handleBatchPartOutlines = async () => {
    const requestProjectId = projectId;
    const requestParts = partsRef.current;
    if (requestParts.length === 0) {
      addToast("没有可生成章纲的分部", "error");
      return;
    }
    const foreignPart = requestParts.find((part) => part.project_id !== requestProjectId);
    if (foreignPart) {
      addToast("分部列表包含其他项目的数据，无法批量生成", "error");
      return;
    }
    const partIds = requestParts.map((part) => part.id);
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setBatchBusy(true);
    try {
      const taskId = createProgressTaskId("outline-batch-part-chapters", requestProjectId);
      await runProgressTask(
        {
          title: "批量生成各部章纲",
          startMessage: "准备批量生成各部章纲…",
          successMessage: "批量生成已触发",
          errorMessage: "批量生成失败",
          total: 3,
          taskId,
        },
        async ({ setProgress }) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress(`正在提交 ${partIds.length} 个分部的章纲生成任务…`, 1);
          await batchGeneratePartOutlines({
            project_id: requestProjectId,
            part_ids: partIds,
            task_id: taskId,
          });
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress("正在刷新章纲列表…", 2);
          await onChanged();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast("批量生成已触发", "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        await onChanged();
        showGenerationError(err, "批量生成失败");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setBatchBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewBatchPartOutlines = async () => {
    const requestProjectId = projectId;
    const requestParts = partsRef.current;
    if (requestParts.length === 0) {
      addToast("没有可生成章纲的分部", "error");
      return;
    }
    const foreignPart = requestParts.find((part) => part.project_id !== requestProjectId);
    if (foreignPart) {
      addToast("分部列表包含其他项目的数据，无法预览", "error");
      return;
    }
    const partIds = requestParts.map((part) => part.id);
    await openPreview(
      requestProjectId,
      () =>
        previewBatchGeneratePartOutlinesRequest({
          project_id: requestProjectId,
          part_ids: partIds,
        }),
      "预览批量章纲提示词失败",
    );
  };

  const handleRegenPartOutline = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setPartRegenBusy(true);
    try {
      const taskId = createProgressTaskId("outline-regenerate-part", requestProjectId);
      await runProgressTask(
        {
          title: `重新生成部 #${requestPart.part_number}`,
          startMessage: "准备调用模型重写部大纲…",
          successMessage: "已重新生成部大纲",
          errorMessage: "重新生成失败",
          total: 3,
          taskId,
        },
        async ({ setProgress }) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress(`模型正在重写部 #${requestPart.part_number} 大纲…`, 1);
          await regeneratePartOutline({
            project_id: requestProjectId,
            part_id: requestPart.id,
            task_id: taskId,
          });
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress("正在刷新分部列表…", 2);
          await onChanged();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast("已重新生成部大纲", "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        showGenerationError(err, "重新生成失败");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setPartRegenBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewRegenPartOutline = async () => {
    const requestProjectId = projectId;
    const requestPart = selectedPart(requestProjectId);
    if (!requestPart) return;
    await openPreview(
      requestProjectId,
      () =>
        previewRegeneratePartOutlineRequest({
          project_id: requestProjectId,
          part_id: requestPart.id,
        }),
      "预览重生成本部提示词失败",
    );
  };

  const handleRegenAllParts = async () => {
    const requestProjectId = projectId;
    const requestPartCount = partCount;
    const requestChaptersPerPart = chaptersPerPart;
    if (!beginConfirmAction(requestProjectId)) return;
    const ok = await confirmDialog({
      title: "重新生成分部",
      message: "重新生成所有部大纲？已有内容会被覆盖。",
      confirmText: "重新生成",
      dialogType: "warning",
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
    setRegenPartsBusy(true);
    try {
      const taskId = createProgressTaskId("outline-regenerate-parts", requestProjectId);
      await runProgressTask(
        {
          title: "重新生成所有部",
          startMessage: "准备调用模型重新生成所有分部…",
          successMessage: "已重新生成所有部",
          errorMessage: "重新生成失败",
          total: 3,
          taskId,
        },
        async ({ setProgress }) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress("模型正在重新生成所有分部大纲，耗时可能较长…", 1);
          await regenerateParts({
            project_id: requestProjectId,
            part_count: requestPartCount,
            chapters_per_part: requestChaptersPerPart,
            task_id: taskId,
          });
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress("正在刷新分部列表…", 2);
          await onChanged();
          ensureActionCurrent(requestProjectId, actionSeq);
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast("已重新生成所有部", "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        await onChanged();
        showGenerationError(err, "重新生成失败");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setRegenPartsBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewRegenAllParts = async () => {
    const requestProjectId = projectId;
    const requestPartCount = partCount;
    const requestChaptersPerPart = chaptersPerPart;
    await openPreview(
      requestProjectId,
      () =>
        previewRegeneratePartsRequest({
          project_id: requestProjectId,
          part_count: requestPartCount,
          chapters_per_part: requestChaptersPerPart,
        }),
      "预览重生成所有部提示词失败",
    );
  };

  const handleContinue = async () => {
    const requestProjectId = projectId;
    const requestParts = partsRef.current;
    if (requestParts.length === 0) {
      addToast("请先生成至少一个分部，再续作下一部", "error");
      return;
    }
    const foreignPart = requestParts.find((part) => part.project_id !== requestProjectId);
    if (foreignPart) {
      addToast("分部列表包含其他项目的数据，无法续作", "error");
      return;
    }
    const lastPart = requestParts.reduce((latest, part) =>
      part.part_number > latest.part_number ? part : latest,
    );
    const chapterSpan = lastPart.end_chapter - lastPart.start_chapter + 1;
    if (chapterSpan <= 0) {
      addToast("最后一个分部的章节范围无效", "error");
      return;
    }
    const startChapter = lastPart.end_chapter + 1;
    const endChapter = startChapter + chapterSpan - 1;
    const actionSeq = beginAction(requestProjectId);
    if (actionSeq == null) return;
    setContinueBusy(true);
    try {
      const taskId = createProgressTaskId("outline-continue-part", requestProjectId);
      const created = await runProgressTask(
        {
          title: `续作第 ${lastPart.part_number + 1} 部`,
          startMessage: "准备调用模型续作分部…",
          successMessage: `已续作第 ${lastPart.part_number + 1} 部`,
          errorMessage: "续作失败",
          total: 3,
          taskId,
        },
        async ({ setProgress }) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress(
            `模型正在生成第 ${lastPart.part_number + 1} 部（${startChapter}-${endChapter} 章）…`,
            1,
          );
          const createdPart = await continuePartOutlines({
            project_id: requestProjectId,
            part_number: lastPart.part_number + 1,
            start_chapter: startChapter,
            end_chapter: endChapter,
            task_id: taskId,
          });
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress("正在刷新分部列表…", 2);
          await onChanged();
          ensureActionCurrent(requestProjectId, actionSeq);
          return createdPart;
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(`已续作第 ${created.part_number} 部`, "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        await onChanged();
        showGenerationError(err, "续作失败");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setContinueBusy(false);
      finishAction(actionSeq);
    }
  };

  const handlePreviewContinue = async () => {
    const requestProjectId = projectId;
    const requestParts = partsRef.current;
    if (requestParts.length === 0) {
      addToast("请先生成至少一个分部，再续作下一部", "error");
      return;
    }
    const foreignPart = requestParts.find((part) => part.project_id !== requestProjectId);
    if (foreignPart) {
      addToast("分部列表包含其他项目的数据，无法预览", "error");
      return;
    }
    const lastPart = requestParts.reduce((latest, part) =>
      part.part_number > latest.part_number ? part : latest,
    );
    const chapterSpan = lastPart.end_chapter - lastPart.start_chapter + 1;
    if (chapterSpan <= 0) {
      addToast("最后一个分部的章节范围无效", "error");
      return;
    }
    const startChapter = lastPart.end_chapter + 1;
    const endChapter = startChapter + chapterSpan - 1;
    await openPreview(
      requestProjectId,
      () =>
        previewContinuePartOutlinesRequest({
          project_id: requestProjectId,
          part_number: lastPart.part_number + 1,
          start_chapter: startChapter,
          end_chapter: endChapter,
        }),
      "预览续作分部提示词失败",
    );
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <section className="rounded-sm border border-book-border/40 bg-book-bg-paper/25 p-3">
          <div className="flex items-center justify-between mb-2">
            <h4 className="text-xs font-serif text-book-text-main">生成进度</h4>
            <div className="inline-flex items-center gap-2 text-[10px] text-book-text-muted">
              <label className="inline-flex items-center gap-1 cursor-pointer">
                <input
                  type="checkbox"
                  checked={autoPoll}
                  disabled={disabled}
                  onChange={(e) => setAutoPoll(e.target.checked)}
                  className="accent-book-primary"
                />
                每 3s 刷新
              </label>
              <BookButton
                variant="secondary"
                onClick={loadProgress}
                disabled={disabled || progressLoading}
              >
                {progressLoading ? <Loader2 size={11} className="animate-spin" /> : "查询"}
              </BookButton>
            </div>
          </div>
          {progress ? (
            <pre className="text-[10px] font-mono whitespace-pre-wrap break-words max-h-32 overflow-y-auto custom-scrollbar bg-book-bg-tertiary/40 p-2 rounded">
              {JSON.stringify(progress, null, 2)}
            </pre>
          ) : (
            <div className="text-[10px] text-book-text-muted italic">点击"查询"加载进度</div>
          )}
        </section>

        <section className="rounded-sm border border-book-border/40 bg-book-bg-paper/25 p-3">
          <div className="flex items-center justify-between mb-2">
            <h4 className="text-xs font-serif text-book-text-main">章纲节奏分析</h4>
            <BookButton
              variant="secondary"
              onClick={loadPacing}
              disabled={disabled || pacingLoading}
            >
              {pacingLoading ? <Loader2 size={11} className="animate-spin" /> : "分析"}
            </BookButton>
          </div>
          {pacing ? (
            <pre className="text-[10px] font-mono whitespace-pre-wrap break-words max-h-32 overflow-y-auto custom-scrollbar bg-book-bg-tertiary/40 p-2 rounded">
              {JSON.stringify(pacing, null, 2)}
            </pre>
          ) : (
            <div className="text-[10px] text-book-text-muted italic">
              统计每章节奏（高潮/平缓/转折）的分布
            </div>
          )}
        </section>
      </div>

      <section className="border-t border-book-border/40 pt-4 space-y-2 text-xs">
        <h4 className="font-serif text-sm text-book-text-main">部级章纲生成</h4>
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-book-text-muted shrink-0">选择部</span>
          <select
            value={selectedPartId}
            onChange={(e) => setSelectedPartId(e.target.value ? Number(e.target.value) : "")}
            disabled={controlsDisabled}
            className="pyqt-input !py-1 !px-2"
          >
            <option value="">选择部</option>
            {parts.map((part) => (
              <option key={part.id} value={part.id}>
                {partOptionLabel(part)}
              </option>
            ))}
          </select>
          <BookButton
            variant="secondary"
            onClick={handlePreviewGeneratePartChapters}
            disabled={controlsDisabled || selectedPartId === ""}
          >
            <span className="inline-flex items-center gap-1.5">
              <Eye size={12} /> 预览本部章纲
            </span>
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={handleGeneratePartChapters}
            disabled={controlsDisabled || selectedPartId === ""}
          >
            {partGenBusy ? <Loader2 size={11} className="animate-spin" /> : "生成本部章纲"}
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={handlePreviewRegenPartOutline}
            disabled={controlsDisabled || selectedPartId === ""}
          >
            <span className="inline-flex items-center gap-1.5">
              <Eye size={12} /> 预览重生成本部
            </span>
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={handleRegenPartOutline}
            disabled={controlsDisabled || selectedPartId === ""}
          >
            {partRegenBusy ? <Loader2 size={11} className="animate-spin" /> : "重新生成本部"}
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={handlePreviewBatchPartOutlines}
            disabled={controlsDisabled}
          >
            <span className="inline-flex items-center gap-1.5">
              <Eye size={12} /> 预览批量章纲
            </span>
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={handleBatchPartOutlines}
            disabled={controlsDisabled}
          >
            {batchBusy ? <Loader2 size={11} className="animate-spin" /> : "批量生成各部章纲"}
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={handlePreviewContinue}
            disabled={controlsDisabled}
          >
            <span className="inline-flex items-center gap-1.5">
              <Eye size={12} /> 预览续作
            </span>
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={handleContinue}
            disabled={controlsDisabled}
          >
            {continueBusy ? <Loader2 size={11} className="animate-spin" /> : "续作分部"}
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={handlePreviewRegenAllParts}
            disabled={controlsDisabled}
          >
            <span className="inline-flex items-center gap-1.5">
              <Eye size={12} /> 预览重生成所有部
            </span>
          </BookButton>
          <BookButton
            variant="secondary"
            onClick={handleRegenAllParts}
            disabled={controlsDisabled}
          >
            {regenPartsBusy ? <Loader2 size={11} className="animate-spin" /> : "重新生成所有部"}
          </BookButton>
        </div>
      </section>

      <section className="border-t border-book-border/40 pt-4 space-y-2 text-xs">
        <h4 className="font-serif text-sm text-book-text-main">节拍映射（Beat → Chapter）</h4>
        <div className="text-book-text-muted">
          通过 JSON 描述节拍点（起承转合 / 三幕结构）到章节的映射，应用后用于生成时引导。
        </div>
        <textarea
          value={beatJson}
          onChange={(e) => setBeatJson(e.target.value)}
          disabled={controlsDisabled}
          className="pyqt-input w-full font-mono text-[11px] h-24 resize-y"
        />
        <div className="flex justify-end gap-2">
          <BookButton
            variant="secondary"
            onClick={handlePreviewBeatMap}
            disabled={controlsDisabled || selectedPartId === ""}
          >
            <span className="inline-flex items-center gap-1.5">
              <Eye size={12} /> 预览提示词
            </span>
          </BookButton>
          <BookButton
            variant="primary"
            onClick={handleBeatMap}
            disabled={controlsDisabled || selectedPartId === ""}
          >
            {beatBusy ? <Loader2 size={11} className="animate-spin" /> : "应用节拍"}
          </BookButton>
        </div>
      </section>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="大纲高级请求预览"
        onClose={closeRequestPreview}
      />
    </div>
  );
};
