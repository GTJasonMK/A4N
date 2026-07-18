import React, { useCallback, useEffect, useRef, useState } from "react";
import { Eye, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
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
  type ChapterOutlinePacing,
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
  return title ? `第 ${part.part_number} 部 · ${title}` : `第 ${part.part_number} 部`;
};

const CompletionRow: React.FC<{ label: string; current: number; total: number }> = ({
  label,
  current,
  total,
}) => {
  const percent = total > 0 ? Math.min(100, Math.round((current / total) * 100)) : 0;
  return (
    <div>
      <div className="mb-1 flex items-center gap-2 text-[11px]">
        <span className="font-medium text-book-text-sub">{label}</span>
        <span className="ml-auto tabular-nums text-book-text-muted">
          {current} / {total}
        </span>
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-book-bg-tertiary">
        <div
          className="h-full rounded-full bg-book-primary/75 transition-[width] duration-300"
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  );
};

const PacingSummary: React.FC<{ data: ChapterOutlinePacing }> = ({ data }) => (
  <div className="space-y-2">
    <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1 text-[11px]">
      <span className="font-medium text-book-text-main">{data.summary || "暂无节奏结论"}</span>
      <span className="text-book-text-muted">
        {data.chapter_count} 章 · 平均节奏 {data.pacing_score.toFixed(2)}
      </span>
    </div>
    {data.analysis.length > 0 ? (
      <div className="divide-y divide-book-border/35">
        {data.analysis.map((item) => (
          <div key={item.chapter_number} className="grid grid-cols-[minmax(0,1fr)_64px] items-center gap-2 py-1.5 text-[11px]">
            <span className="min-w-0 truncate text-book-text-sub">
              第 {item.chapter_number} 章{item.title ? ` · ${item.title}` : ""}
            </span>
            <div className="flex items-center gap-1.5">
              <div className="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full bg-book-bg-tertiary">
                <div
                  className="h-full rounded-full bg-book-primary/70"
                  style={{ width: `${Math.min(100, Math.max(0, item.pacing_score * 100))}%` }}
                />
              </div>
              <span className="w-6 text-right tabular-nums text-book-text-muted">
                {item.pacing_score.toFixed(1)}
              </span>
            </div>
          </div>
        ))}
      </div>
    ) : (
      <div className="text-[11px] text-book-text-muted">暂无章节节奏数据</div>
    )}
  </div>
);

function AdvancedActionRow({
  title,
  description,
  actionLabel,
  actionBusyLabel,
  busy,
  disabled,
  warning = false,
  standalone = false,
  onPreview,
  onAction,
}: {
  title: string;
  description: string;
  actionLabel: string;
  actionBusyLabel: string;
  busy: boolean;
  disabled: boolean;
  warning?: boolean;
  standalone?: boolean;
  onPreview: () => void;
  onAction: () => void;
}) {
  return (
    <div
      className={`${standalone ? "a4n-flat-surface " : ""}flex flex-col gap-3 px-3 py-3 sm:flex-row sm:items-center`}
    >
      <div className="min-w-0 flex-1">
        <h5 className="text-xs font-medium text-book-text-main">{title}</h5>
        <p className="mt-1 text-[11px] leading-relaxed text-book-text-muted">{description}</p>
      </div>
      <div className="flex items-center justify-end gap-2 sm:shrink-0">
        {!busy ? (
          <Dropdown
            items={[
              {
                label: "查看模型请求",
                icon: <Eye size={12} />,
                onClick: onPreview,
                disabled,
              },
            ]}
          />
        ) : null}
        <BookButton
          variant={warning ? "warning" : "secondary"}
          size="sm"
          onClick={onAction}
          disabled={disabled}
          className="min-w-0 flex-1 !text-xs sm:flex-none"
        >
          {busy ? (
            <span className="inline-flex items-center gap-1.5">
              <Loader2 size={11} className="animate-spin" /> {actionBusyLabel}
            </span>
          ) : (
            actionLabel
          )}
        </BookButton>
      </div>
    </div>
  );
}

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

  const [pacing, setPacing] = useState<ChapterOutlinePacing | null>(null);
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
          title: `生成第 ${requestPart.part_number} 部章纲`,
          startMessage: "准备调用模型生成本部章纲…",
          successMessage: `已为第 ${requestPart.part_number} 部生成章纲`,
          errorMessage: "生成失败",
          total: 3,
          taskId,
        },
        async ({ setProgress }) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress(`模型正在为第 ${requestPart.part_number} 部生成章纲…`, 1);
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
      addToast(`已为第 ${requestPart.part_number} 部生成章纲`, "success");
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
          title: `重新生成第 ${requestPart.part_number} 部`,
          startMessage: "准备调用模型重写部大纲…",
          successMessage: "已重新生成部大纲",
          errorMessage: "重新生成失败",
          total: 3,
          taskId,
        },
        async ({ setProgress }) => {
          ensureActionCurrent(requestProjectId, actionSeq);
          setProgress(`模型正在重写第 ${requestPart.part_number} 部大纲…`, 1);
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
      <div className="grid gap-3 lg:grid-cols-2">
        <section className="a4n-flat-surface p-3">
          <div className="mb-3 flex flex-wrap items-center gap-2">
            <h4 className="text-sm font-medium text-book-text-main">生成进度</h4>
            <button
              type="button"
              role="switch"
              aria-checked={autoPoll}
              onClick={() => setAutoPoll((current) => !current)}
              disabled={controlsDisabled}
              className="ml-auto inline-flex items-center gap-2 text-[11px] text-book-text-muted disabled:cursor-not-allowed disabled:opacity-50"
              title={autoPoll ? "关闭自动刷新" : "开启自动刷新"}
            >
              <span>自动刷新</span>
              <span
                className={`relative inline-flex h-[18px] w-8 shrink-0 rounded-full transition-colors ${
                  autoPoll ? "bg-book-primary" : "bg-book-text-muted/25"
                }`}
              >
                <span
                  className={`absolute top-[2px] h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform ${
                    autoPoll ? "translate-x-4" : "translate-x-[2px]"
                  }`}
                />
              </span>
            </button>
            <BookButton
              variant="ghost"
              size="sm"
              onClick={loadProgress}
              disabled={disabled || progressLoading}
              className="!px-2 !text-[11px]"
            >
              {progressLoading ? (
                <span className="inline-flex items-center gap-1.5">
                  <Loader2 size={11} className="animate-spin" /> 刷新中…
                </span>
              ) : (
                "刷新"
              )}
            </BookButton>
          </div>
          {progress ? (
            <div className="space-y-3">
              <CompletionRow label="分部大纲" current={progress.completed_parts} total={progress.total_parts} />
              <CompletionRow
                label="章节大纲"
                current={progress.completed_chapter_outlines}
                total={progress.total_chapter_outlines}
              />
            </div>
          ) : (
            <p className="text-[11px] text-book-text-muted">尚未查询生成进度。</p>
          )}
        </section>

        <section className="a4n-flat-surface p-3">
          <div className="mb-3 flex items-center gap-2">
            <div className="min-w-0 flex-1">
              <h4 className="text-sm font-medium text-book-text-main">章纲节奏分析</h4>
              {!pacing ? (
                <p className="mt-0.5 text-[11px] text-book-text-muted">检查高潮、平缓和转折的章节分布。</p>
              ) : null}
            </div>
            {pacingLoading ? (
              <span
                role="status"
                className="inline-flex min-h-8 shrink-0 items-center gap-1.5 text-[11px] text-book-text-muted"
                title="正在分析章纲节奏"
              >
                <Loader2 size={11} className="animate-spin" /> 分析中…
              </span>
            ) : pacing ? (
              <Dropdown
                items={[
                  {
                    label: "重新分析章纲节奏",
                    onClick: loadPacing,
                    disabled,
                  },
                ]}
              />
            ) : (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={loadPacing}
                disabled={disabled}
                className="!text-xs"
              >
                开始分析
              </BookButton>
            )}
          </div>
          {pacing ? <PacingSummary data={pacing} /> : null}
        </section>
      </div>

      <section className="space-y-2">
        <div>
          <h4 className="text-sm font-medium text-book-text-main">选中分部</h4>
          <p className="mt-0.5 text-[11px] text-book-text-muted">为一个分部补齐对应的章节大纲。</p>
        </div>
        <select
          value={selectedPartId}
          onChange={(event) => setSelectedPartId(event.target.value ? Number(event.target.value) : "")}
          disabled={controlsDisabled}
          className="pyqt-input w-full"
        >
          <option value="">选择分部</option>
          {parts.map((part) => (
            <option key={part.id} value={part.id}>
              {partOptionLabel(part)}
            </option>
          ))}
        </select>
        <div>
          <AdvancedActionRow
            title="生成本部章纲"
            description="根据选中分部的目标和章节范围，生成该部对应的章节大纲。"
            actionLabel="生成本部章纲"
            actionBusyLabel="生成中…"
            busy={partGenBusy}
            disabled={controlsDisabled || selectedPartId === ""}
            onPreview={handlePreviewGeneratePartChapters}
            onAction={handleGeneratePartChapters}
            standalone
          />
        </div>
      </section>

      <section className="space-y-2">
        <div>
          <h4 className="text-sm font-medium text-book-text-main">全部分部</h4>
          <p className="mt-0.5 text-[11px] text-book-text-muted">批量补齐现有章纲，或沿用当前结构续作下一部。</p>
        </div>
        <div className="grid gap-2">
          <AdvancedActionRow
            title="批量生成各部章纲"
            description="为当前所有分部补齐对应的章节大纲。"
            actionLabel="批量生成"
            actionBusyLabel="生成中…"
            busy={batchBusy}
            disabled={controlsDisabled || parts.length === 0}
            onPreview={handlePreviewBatchPartOutlines}
            onAction={handleBatchPartOutlines}
            standalone
          />
          <AdvancedActionRow
            title="续作下一部"
            description="沿用最后一部的章节跨度，继续规划新的故事阶段。"
            actionLabel="续作下一部"
            actionBusyLabel="续作中…"
            busy={continueBusy}
            disabled={controlsDisabled || parts.length === 0}
            onPreview={handlePreviewContinue}
            onAction={handleContinue}
            standalone
          />
        </div>
      </section>

      <details className="group">
        <summary className="a4n-flat-surface flex min-h-11 cursor-pointer list-none items-center gap-3 px-3 py-2.5 text-xs marker:content-none">
          <span className="font-medium text-book-text-main">覆盖性重写</span>
          <span className="min-w-0 flex-1 truncate text-right text-[11px] text-book-text-muted">
            重写选中分部或完整分部结构
          </span>
        </summary>
        <div className="mt-2 grid gap-2">
          <AdvancedActionRow
            title="重写选中分部"
            description="覆盖当前选择的阶段规划，其他分部保持不变。"
            actionLabel="重写本部"
            actionBusyLabel="重写中…"
            busy={partRegenBusy}
            disabled={controlsDisabled || selectedPartId === ""}
            warning
            onPreview={handlePreviewRegenPartOutline}
            onAction={handleRegenPartOutline}
            standalone
          />
          <AdvancedActionRow
            title="重新生成所有分部"
            description="覆盖全部分部规划，执行前会再次确认。"
            actionLabel="重新规划"
            actionBusyLabel="规划中…"
            busy={regenPartsBusy}
            disabled={controlsDisabled}
            warning
            onPreview={handlePreviewRegenAllParts}
            onAction={handleRegenAllParts}
            standalone
          />
        </div>
      </details>

      <details className="a4n-flat-surface overflow-hidden">
        <summary className="cursor-pointer px-3 py-3 text-sm font-medium text-book-text-main">
          节拍映射
        </summary>
        <div className="space-y-3 border-t border-book-border/40 p-3 text-xs">
          <p className="text-[11px] leading-relaxed text-book-text-muted">
            使用 JSON 描述故事节拍到章节的映射。该设置面向高级工作流，默认不影响普通大纲生成。
          </p>
          <textarea
            value={beatJson}
            onChange={(event) => setBeatJson(event.target.value)}
            disabled={controlsDisabled}
            className="pyqt-input h-24 w-full resize-y font-mono text-[11px]"
          />
          <div className="flex items-center justify-end gap-2">
            {!beatBusy ? (
              <Dropdown
                items={[
                  {
                    label: "查看模型请求",
                    icon: <Eye size={12} />,
                    onClick: handlePreviewBeatMap,
                    disabled: controlsDisabled || selectedPartId === "",
                  },
                ]}
              />
            ) : null}
            <BookButton
              variant="primary"
              size="sm"
              onClick={handleBeatMap}
              disabled={controlsDisabled || selectedPartId === ""}
              className="min-w-0 flex-1 !text-xs sm:flex-none"
            >
              {beatBusy ? (
                <span className="inline-flex items-center gap-1.5">
                  <Loader2 size={11} className="animate-spin" /> 应用中…
                </span>
              ) : (
                "应用节拍"
              )}
            </BookButton>
          </div>
        </div>
      </details>
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
