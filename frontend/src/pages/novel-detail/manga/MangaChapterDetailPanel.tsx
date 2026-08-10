import { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle,
  CheckCircle2,
  CircleDashed,
  Eye,
  ImagePlus,
  PauseCircle,
  Play,
  RefreshCw,
  Trash2,
  XCircle, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import type { CharacterProfile } from "../../../api/character";
import type { DownstreamConflictStrategy } from "../../../api/generated/host";
import {
  previewPageImageRequest,
  startImageGeneration,
  verifyImageGenerationResult,
} from "../../../api/image";
import type { GeneratedImage } from "../../../api/image";
import { runGenerationProgressTaskThen } from "../../../hooks/runGenerationProgressTask";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import type { Portrait } from "../../../api/portrait";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../../components/feedback/PromptDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown, type DropdownItem } from "../../../components/ui/Dropdown";
import {
  bindMangaReferenceImage,
  deleteMangaPrompt,
  optimizeMangaPrompt,
  type BindMangaReferenceImageInput,
  type MangaPipelineStatusSummary,
  type MangaPromptSummary,
  updateMangaPrompt,
} from "../../../api/manga";
import { useProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { chooseDownstreamConflictStrategy } from "../dependencyConflict";
import { isChapterGenerating, type ChapterRow } from "../shared";
import { MangaPanelList } from "./MangaPanelList";
import { explainMangaPipelineAiConfigError } from "./mangaPipelineErrors";
import {
  chapterHeading,
  MANGA_PIPELINE_START_STAGE_OPTIONS,
  mangaStageLabel,
  summarizePanelSources,
  type MangaPipelineStartStageOption,
} from "./mangaTabUtils";

interface MangaChapterDetailPanelProps {
  projectId: number;
  isSingleMode: boolean;
  selectedChapter: ChapterRow | null;
  selectedStatus: MangaPipelineStatusSummary | null | undefined;
  panels: MangaPromptSummary[];
  panelsLoading: boolean;
  currentContentHash: string | null;
  characterAssetsLoading: boolean;
  projectImagesLoading: boolean;
  profiles: CharacterProfile[];
  portraits: Portrait[];
  projectImages: GeneratedImage[];
  primaryPortraitCount: number;
  characterProfileCount: number;
  portraitCount: number;
  missingPortraitCount: number;
  pipelineStartStage: MangaPipelineStartStageOption;
  acting: string | null;
  canEditManga: boolean;
  onPipelineStartStageChange: (value: MangaPipelineStartStageOption) => void;
  onPreviewPipeline: (mode: "start" | "resume") => void;
  onRunPipeline: (mode: "start" | "resume") => void;
  onGenerateMissingPortraits: () => void;
  onCancelPipeline: () => void;
  onDeleteManga: () => void;
  onPanelsChanged: () => Promise<void>;
}

type NoticeTone = "ok" | "warning" | "danger" | "muted";

const noticeClass: Record<NoticeTone, string> = {
  ok: "border-success/40 text-success",
  warning: "border-warning/50 text-warning",
  danger: "border-error/50 text-error",
  muted: "border-book-border/50 text-book-text-sub",
};

const noticeIcon = (tone: NoticeTone) => {
  if (tone === "ok") return <CheckCircle2 size={13} />;
  if (tone === "danger") return <XCircle size={13} />;
  if (tone === "warning") return <AlertTriangle size={13} />;
  return <CircleDashed size={13} />;
};

export function MangaChapterDetailPanel({
  projectId,
  isSingleMode,
  selectedChapter,
  selectedStatus,
  panels,
  panelsLoading,
  currentContentHash,
  characterAssetsLoading,
  projectImagesLoading,
  profiles,
  portraits,
  projectImages,
  primaryPortraitCount,
  characterProfileCount,
  portraitCount,
  missingPortraitCount,
  pipelineStartStage,
  acting,
  canEditManga,
  onPipelineStartStageChange,
  onPreviewPipeline,
  onRunPipeline,
  onGenerateMissingPortraits,
  onCancelPipeline,
  onDeleteManga,
  onPanelsChanged,
}: MangaChapterDetailPanelProps) {
  const addToast = useToast((state) => state.addToast);
  const { runProgressTask: runPageImageProgressTask } = useProgressTask();
  const [generatingPageNumber, setGeneratingPageNumber] = useState<number | null>(null);
  const [panelActionBusy, setPanelActionBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [optimizingPromptId, setOptimizingPromptId] = useState<number | null>(null);
  const selectedChapterId = selectedChapter?.id ?? null;
  const scopeKey = `${projectId}:${selectedChapterId ?? "none"}`;
  const mountedRef = useRef(false);
  const activeScopeRef = useRef(scopeKey);
  const panelsRef = useRef<MangaPromptSummary[]>(panels);
  const actionSeqRef = useRef(0);
  const actionBusyRef = useRef(false);
  const previewSeqRef = useRef(0);
  const previewBusyRef = useRef(false);
  const selectedChapterReady = Boolean(selectedChapter?.content?.trim());
  const selectedChapterGenerating = isChapterGenerating(selectedChapter);
  const detailBusy = panelActionBusy || requestPreviewLoading;

  activeScopeRef.current = scopeKey;
  panelsRef.current = panels;

  const isCurrentScope = useCallback(
    (requestScope: string) => mountedRef.current && activeScopeRef.current === requestScope,
    [],
  );

  const isCurrentAction = useCallback(
    (requestScope: string, requestSeq: number) =>
      isCurrentScope(requestScope) && actionSeqRef.current === requestSeq,
    [isCurrentScope],
  );

  const isCurrentPreview = useCallback(
    (requestScope: string, requestSeq: number) =>
      isCurrentScope(requestScope) && previewSeqRef.current === requestSeq,
    [isCurrentScope],
  );

  const ensureCurrentAction = useCallback(
    (requestScope: string, requestSeq: number) => {
      if (!isCurrentAction(requestScope, requestSeq)) {
        throw new Error("任务已停止");
      }
    },
    [isCurrentAction],
  );

  const beginAction = useCallback(
    (requestScope: string): number | null => {
      if (!isCurrentScope(requestScope)) return null;
      if (actionBusyRef.current) {
        addToast("已有漫画页操作进行中，请等待当前操作完成", "error");
        return null;
      }
      actionBusyRef.current = true;
      actionSeqRef.current += 1;
      setPanelActionBusy(true);
      return actionSeqRef.current;
    },
    [addToast, isCurrentScope],
  );

  const finishAction = useCallback(
    (requestSeq: number) => {
      if (actionSeqRef.current === requestSeq) {
        actionBusyRef.current = false;
        setPanelActionBusy(false);
      }
    },
    [],
  );

  const closeRequestPreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyRef.current = false;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      actionSeqRef.current += 1;
      previewSeqRef.current += 1;
      actionBusyRef.current = false;
      previewBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    activeScopeRef.current = scopeKey;
    actionSeqRef.current += 1;
    previewSeqRef.current += 1;
    actionBusyRef.current = false;
    previewBusyRef.current = false;
    panelsRef.current = panels;
    setGeneratingPageNumber(null);
    setPanelActionBusy(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setOptimizingPromptId(null);
  }, [scopeKey]);

  useEffect(() => {
    panelsRef.current = panels;
  }, [panels]);
  const panelSourceStatus =
    selectedChapter != null
      ? summarizePanelSources(panels, currentContentHash)
      : null;

  const stageText = selectedStatus?.stage
    ? mangaStageLabel(selectedStatus.stage)
    : selectedStatus?.pipeline_status
      ? mangaStageLabel(selectedStatus.pipeline_status)
      : "未开始";
  const statusTone: NoticeTone =
    selectedStatus?.pipeline_status === "failed"
      ? "danger"
      : selectedStatus?.pipeline_status === "running"
        ? "warning"
        : selectedStatus?.pipeline_status === "completed"
          ? "ok"
          : "muted";
  const chapterMeta = [
    `正文 ${selectedChapter?.content?.length ?? 0} 字`,
    characterProfileCount > 0 || portraitCount > 0
      ? `立绘 ${primaryPortraitCount}/${characterProfileCount}`
      : null,
    portraitCount > 0 ? `素材 ${portraitCount} 张` : null,
    selectedStatus?.current != null && selectedStatus.total != null
      ? `流水线 ${selectedStatus.current}/${selectedStatus.total}`
      : null,
  ].filter((item): item is string => item != null);

  const notice = (() => {
    if (!selectedChapterReady) {
      return {
        tone: "warning" as NoticeTone,
        text: "当前章节正文为空，需先完成正文后再生成整页漫画提示词。",
      };
    }
    if (selectedChapterGenerating) {
      return {
        tone: "warning" as NoticeTone,
        text: "当前章节正文仍在生成中，漫画操作已锁定。",
      };
    }
    if (selectedStatus?.pipeline_status === "failed") {
      return {
        tone: "danger" as NoticeTone,
        text: selectedStatus.message
          ? explainMangaPipelineAiConfigError(selectedStatus.message)
          : "漫画流水线失败，可使用续传保留已完成断点。",
      };
    }
    if (selectedStatus?.can_resume) {
      return {
        tone: "warning" as NoticeTone,
        text: selectedStatus.checkpoint_valid
          ? "检测到可续传断点，优先续传可以复用已完成阶段。"
          : "检测到断点但正文已变化，建议重新运行流水线。",
      };
    }
    if (missingPortraitCount > 0) {
      return {
        tone: "warning" as NoticeTone,
        text: `有 ${missingPortraitCount} 位角色缺少立绘；生成时会退回文字档案描述。`,
      };
    }
    if (panelSourceStatus && panelSourceStatus.staleCount > 0) {
      return {
        tone: "warning" as NoticeTone,
        text: `有 ${panelSourceStatus.staleCount} 页提示词基于旧正文，建议从受影响阶段重新生成。`,
      };
    }
    if (panels.length > 0) {
      return {
        tone: "ok" as NoticeTone,
        text: "当前整页提示词可继续绑定参考图并生成图片。",
      };
    }
    return {
      tone: "muted" as NoticeTone,
      text: "章节正文已就绪，可以启动漫画流水线。",
    };
  })();

  const handleGeneratePageImage = async (pageNumber: number) => {
    if (!canEditManga) {
      addToast("当前账号没有编辑此漫画项目的权限", "error");
      return;
    }
    const requestScope = scopeKey;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (!selectedChapterReady) {
      addToast("当前章节正文为空，无法生成整页漫画图片", "error");
      return;
    }
    if (selectedChapterGenerating) {
      addToast("当前章节正文仍在生成中，无法生成整页漫画图片", "error");
      return;
    }

    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setGeneratingPageNumber(pageNumber);
    try {
      const handle = await startImageGeneration({
        project_id: projectId,
        operation: {
          kind: "page",
          chapter_number: chapter.chapter_number,
          page_number: pageNumber,
        },
      });
      await runGenerationProgressTaskThen(
        runPageImageProgressTask,
        handle,
        {
          title: "生成整页漫画",
          startMessage: `正在准备第 ${pageNumber} 页图片生成`,
          successMessage: "整页漫画生成完成",
          errorMessage: "整页漫画生成失败",
          total: 4,
          cancelLabel: "停止生成",
          runningHint: "使用当前页 page_generation 提示词直接生成完整漫画页",
        },
        async (task) => {
          ensureCurrentAction(requestScope, actionSeq);
          await verifyImageGenerationResult(task, projectId);
        },
      );
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast(`已生成第 ${pageNumber} 页整页漫画`, "success");
      await onPanelsChanged();
    } catch (err) {
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const message = extractErrorMessage(err, "整页漫画生成失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setGeneratingPageNumber(null);
      finishAction(actionSeq);
    }
  };

  const handlePreviewPageImage = async (pageNumber: number) => {
    if (!canEditManga) {
      addToast("当前账号没有编辑此漫画项目的权限", "error");
      return;
    }
    const requestScope = scopeKey;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (!selectedChapterReady) {
      addToast("当前章节正文为空，无法预览整页漫画图片请求", "error");
      return;
    }
    if (selectedChapterGenerating) {
      addToast("当前章节正文仍在生成中，无法预览整页漫画图片请求", "error");
      return;
    }
    if (previewBusyRef.current) return;
    const requestSeq = previewSeqRef.current + 1;
    previewSeqRef.current = requestSeq;
    previewBusyRef.current = true;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview = await previewPageImageRequest(
        projectId,
        chapter.chapter_number,
        pageNumber,
      );
      if (!isCurrentPreview(requestScope, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentPreview(requestScope, requestSeq)) {
        addToast(extractErrorMessage(err, "预览整页漫画图片请求失败"), "error");
        setRequestPreviewOpen(false);
      }
    } finally {
      if (isCurrentPreview(requestScope, requestSeq)) {
        previewBusyRef.current = false;
        setRequestPreviewLoading(false);
      }
    }
  };

  const handleBindReferenceImage = async (
    input: Omit<BindMangaReferenceImageInput, "downstream_strategy">,
  ) => {
    if (!canEditManga) {
      addToast("当前账号没有编辑此漫画项目的权限", "error");
      return;
    }
    if (selectedChapterGenerating) {
      addToast("当前章节正文仍在生成中，无法绑定参考图", "error");
      return;
    }
    const requestScope = scopeKey;
    const prompt = panelsRef.current.find((panel) => panel.id === input.prompt_id) ?? null;
    if (!prompt) {
      addToast("漫画提示词已不在当前章节中", "error");
      return;
    }
    if (actionBusyRef.current) {
      addToast("已有漫画页操作进行中，请等待当前操作完成", "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    try {
      let downstreamStrategy: DownstreamConflictStrategy = "reject";
      if (prompt?.generated_image_id != null) {
        const strategy = await chooseDownstreamConflictStrategy({
          title: "漫画提示词存在下游图片",
          assetLabel: `第 ${prompt.page_number} 页提示词`,
          downstreamLabel: "漫画图片",
          downstreamCount: 1,
          operation: "修改参考图绑定",
        });
        if (!strategy) return;
        downstreamStrategy = strategy;
      }
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const currentPrompt = panelsRef.current.find((panel) => panel.id === input.prompt_id) ?? null;
      if (!currentPrompt) {
        addToast("漫画提示词已不在当前章节中", "error");
        return;
      }
      await bindMangaReferenceImage({
        ...input,
        downstream_strategy: downstreamStrategy,
      });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast("已绑定参考图", "success");
      await onPanelsChanged();
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "绑定参考图失败"), "error");
      }
    } finally {
      finishAction(actionSeq);
    }
  };

  const handleUpdatePrompt = async (promptId: number, imagePrompt: string) => {
    if (!canEditManga) {
      const message = "当前账号没有编辑此漫画项目的权限";
      addToast(message, "error");
      throw new Error(message);
    }
    if (selectedChapterGenerating) {
      const message = "当前章节正文仍在生成中，无法修改提示词";
      addToast(message, "error");
      throw new Error(message);
    }
    const requestScope = scopeKey;
    const prompt = panelsRef.current.find((panel) => panel.id === promptId) ?? null;
    if (!prompt) {
      const message = "漫画提示词已不在当前章节中";
      addToast(message, "error");
      throw new Error(message);
    }
    if (actionBusyRef.current) {
      const message = "已有漫画页操作进行中，请等待当前操作完成";
      addToast(message, "error");
      throw new Error(message);
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) throw new Error("任务已停止");
    try {
      let downstreamStrategy: DownstreamConflictStrategy = "reject";
      if (prompt?.generated_image_id != null) {
        const strategy = await chooseDownstreamConflictStrategy({
          title: "漫画提示词存在下游图片",
          assetLabel: `第 ${prompt.page_number} 页提示词`,
          downstreamLabel: "漫画图片",
          downstreamCount: 1,
          operation: "修改",
        });
        if (!strategy) {
          throw new Error("已取消漫画提示词下游冲突处理");
        }
        downstreamStrategy = strategy;
      }
      if (!isCurrentAction(requestScope, actionSeq)) throw new Error("任务已停止");
      const currentPrompt = panelsRef.current.find((panel) => panel.id === promptId) ?? null;
      if (!currentPrompt) {
        const message = "漫画提示词已不在当前章节中";
        addToast(message, "error");
        throw new Error(message);
      }
      await updateMangaPrompt({
        prompt_id: promptId,
        image_prompt: imagePrompt,
        downstream_strategy: downstreamStrategy,
      });
      if (!isCurrentAction(requestScope, actionSeq)) throw new Error("任务已停止");
      addToast("已保存提示词", "success");
      await onPanelsChanged();
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        const message = extractErrorMessage(err, "保存提示词失败");
        if (
          !isCancelledOperation(message) &&
          !message.includes("任务已停止") &&
          !message.includes("漫画提示词已不在当前章节中")
        ) {
          addToast(message, "error");
        }
      }
      throw err;
    } finally {
      finishAction(actionSeq);
    }
  };

  const handleOptimizePrompt = async (prompt: MangaPromptSummary) => {
    if (!canEditManga) {
      addToast("当前账号没有编辑此漫画项目的权限", "error");
      return;
    }
    if (selectedChapterGenerating) {
      addToast("当前章节正文仍在生成中，无法优化提示词", "error");
      return;
    }
    const requestScope = scopeKey;
    const requestPrompt = panelsRef.current.find((panel) => panel.id === prompt.id) ?? null;
    if (!requestPrompt) {
      addToast("漫画提示词已不在当前章节中", "error");
      return;
    }
    if (actionBusyRef.current) {
      addToast("已有漫画页操作进行中，请等待当前操作完成", "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    try {
      let downstreamStrategy: DownstreamConflictStrategy = "reject";
      if (requestPrompt.generated_image_id != null) {
        const strategy = await chooseDownstreamConflictStrategy({
          title: "漫画提示词存在下游图片",
          assetLabel: `第 ${requestPrompt.page_number} 页提示词`,
          downstreamLabel: "漫画图片",
          downstreamCount: 1,
          operation: "优化",
        });
        if (!strategy) return;
        downstreamStrategy = strategy;
      }
      const instruction = await promptDialog({
        title: "优化文字资产",
        message: "输入你希望如何优化这条漫画整页提示词。有下游图片时会按刚才选择的处理方式继续。",
        placeholder: "例如：保留角色和参考图约束，增强构图关系，压缩重复形容词…",
        multiline: true,
        required: true,
        confirmText: "开始优化",
      });
      if (instruction == null) return;
      const trimmed = instruction.trim();
      if (!trimmed) return;
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const currentPrompt =
        panelsRef.current.find((panel) => panel.id === requestPrompt.id) ?? null;
      if (!currentPrompt) {
        addToast("漫画提示词已不在当前章节中", "error");
        return;
      }
      setOptimizingPromptId(requestPrompt.id);
      await optimizeMangaPrompt({
        prompt_id: requestPrompt.id,
        instruction: trimmed,
        downstream_strategy: downstreamStrategy,
      });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast("漫画提示词已优化", "success");
      await onPanelsChanged();
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "优化漫画提示词失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setOptimizingPromptId(null);
      finishAction(actionSeq);
    }
  };

  const handleDeletePrompt = async (prompt: MangaPromptSummary) => {
    if (!canEditManga) {
      addToast("当前账号没有编辑此漫画项目的权限", "error");
      return;
    }
    if (selectedChapterGenerating) {
      addToast("当前章节正文仍在生成中，无法删除漫画提示词", "error");
      return;
    }
    const requestScope = scopeKey;
    const requestPrompt = panelsRef.current.find((panel) => panel.id === prompt.id) ?? null;
    if (!requestPrompt) {
      addToast("漫画提示词已不在当前章节中", "error");
      return;
    }
    if (actionBusyRef.current) {
      addToast("已有漫画页操作进行中，请等待当前操作完成", "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    try {
      let downstreamStrategy: DownstreamConflictStrategy = "reject";
      if (requestPrompt.generated_image_id != null) {
        const strategy = await chooseDownstreamConflictStrategy({
          title: "漫画提示词存在下游图片",
          assetLabel: `第 ${requestPrompt.page_number} 页提示词`,
          downstreamLabel: "漫画图片",
          downstreamCount: 1,
          operation: "删除",
        });
        if (!strategy) return;
        downstreamStrategy = strategy;
      } else {
        const ok = await confirmDialog({
          title: "删除漫画提示词",
          message: `删除第 ${requestPrompt.page_number} 页提示词？`,
          confirmText: "删除",
          dialogType: "danger",
        });
        if (!ok) return;
      }
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const currentPrompt =
        panelsRef.current.find((panel) => panel.id === requestPrompt.id) ?? null;
      if (!currentPrompt) {
        addToast("漫画提示词已不在当前章节中", "error");
        return;
      }
      await deleteMangaPrompt(requestPrompt.id, downstreamStrategy);
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast("漫画提示词已删除", "success");
      await onPanelsChanged();
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "删除漫画提示词失败"), "error");
      }
    } finally {
      finishAction(actionSeq);
    }
  };

  const pipelineRunning = selectedStatus?.can_cancel === true;
  const pipelineCanResume = selectedStatus?.can_resume === true && !pipelineRunning;
  const hasMangaResult = panels.length > 0;
  const mangaResultStale = (panelSourceStatus?.staleCount ?? 0) > 0;
  const pipelineFailed =
    selectedStatus?.pipeline_status === "failed" && !pipelineCanResume;
  const showPipelineDirectAction =
    !hasMangaResult || pipelineCanResume || pipelineFailed || mangaResultStale;
  const pipelineDirectVariant =
    mangaResultStale && !pipelineCanResume && !pipelineFailed
      ? "secondary"
      : "primary";
  const pipelineDirectLabel = pipelineCanResume
    ? "继续生成"
    : pipelineFailed
      ? "重新生成本章漫画"
      : mangaResultStale
        ? "更新本章漫画"
        : "生成本章漫画";
  const pipelinePreviewLabel = pipelineCanResume
    ? "查看继续生成请求"
    : pipelineFailed
      ? "查看重新生成请求"
      : mangaResultStale
        ? "查看更新请求"
        : hasMangaResult
          ? "查看重新生成请求"
          : "查看生成请求";
  const pipelineStartBusy =
    selectedChapter != null &&
    acting === `start-${selectedChapter.chapter_number}`;
  const pipelineResumeBusy =
    selectedChapter != null &&
    acting === `resume-${selectedChapter.chapter_number}`;
  const pipelineMainDisabled =
    selectedChapter == null ||
    detailBusy ||
    !selectedChapterReady ||
    selectedChapterGenerating;
  const pipelineMenuItems: DropdownItem[] = [];
  if (canEditManga && selectedChapter && !pipelineRunning) {
    if (showPipelineDirectAction) {
      pipelineMenuItems.push({
        label: pipelinePreviewLabel,
        icon: <Eye size={12} />,
        disabled: pipelineMainDisabled || pipelineResumeBusy || pipelineStartBusy,
        onClick: () => onPreviewPipeline(pipelineCanResume ? "resume" : "start"),
      });
    }
    if (pipelineCanResume || (hasMangaResult && !mangaResultStale && !pipelineFailed)) {
      pipelineMenuItems.push({
        label: "从所选阶段重新生成",
        icon: <Play size={12} />,
        disabled: pipelineMainDisabled || pipelineStartBusy,
        onClick: () => onRunPipeline("start"),
      });
      pipelineMenuItems.push({
        label: "查看重新生成请求",
        icon: <Eye size={12} />,
        disabled: pipelineMainDisabled || pipelineStartBusy,
        onClick: () => onPreviewPipeline("start"),
      });
    }
    if (missingPortraitCount > 0) {
      pipelineMenuItems.push({ type: "divider" });
      pipelineMenuItems.push({
        label: `补齐 ${missingPortraitCount} 张立绘`,
        icon: <ImagePlus size={12} />,
        disabled:
          detailBusy ||
          acting === "generate-missing-portraits" ||
          characterAssetsLoading,
        onClick: onGenerateMissingPortraits,
      });
    }
    if (!detailBusy && acting !== `delete-${selectedChapter.id}` && !selectedChapterGenerating) {
      pipelineMenuItems.push({ type: "divider" });
      pipelineMenuItems.push({
        label: "删除本章漫画",
        icon: <Trash2 size={12} />,
        danger: true,
        onClick: onDeleteManga,
      });
    }
  }
  const selectedStartStageLabel =
    MANGA_PIPELINE_START_STAGE_OPTIONS.find(
      (option) => option.value === pipelineStartStage,
    )?.label ?? "从头开始";

  return (
    <>
      <section
        className={
          isSingleMode
            ? "flex h-full min-h-0 min-w-0 flex-col"
            : "flex min-h-[60vh] min-w-0 flex-col lg:h-full lg:min-h-0"
        }
      >
      {!selectedChapter ? (
        <div className="m-auto text-sm text-book-text-muted">
          {isSingleMode ? "请先在写作桌选择章节" : "从左侧选择一章查看漫画详情"}
        </div>
      ) : (
        <>
          <header className="shrink-0 border-b border-book-border/55 pb-3">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0">
                <h2 className="truncate text-sm font-semibold text-book-text-main">
                  {chapterHeading(selectedChapter)}
                </h2>
                <p className={`mt-1 text-small ${noticeClass[statusTone]}`}>
                  {[...chapterMeta, stageText].filter(Boolean).join(" · ")}
                </p>
              </div>
              {canEditManga ? (
                <div className="flex items-center justify-end gap-2">
                  {pipelineMenuItems.length > 0 ? <Dropdown items={pipelineMenuItems} /> : null}
                  {pipelineRunning ? (
                    <BookButton
                      size="sm"
                      variant="secondary"
                      onClick={onCancelPipeline}
                      disabled={detailBusy || acting === `cancel-${selectedChapter.id}`}
                    >
                      {acting === `cancel-${selectedChapter.id}` ? (
                        <LoadingSpinner size="sm" />
                      ) : (
                        <PauseCircle size={13} />
                      )}
                      停止生成
                    </BookButton>
                  ) : showPipelineDirectAction ? (
                    <BookButton
                      size="sm"
                      variant={pipelineDirectVariant}
                      onClick={() => onRunPipeline(pipelineCanResume ? "resume" : "start")}
                      disabled={pipelineMainDisabled || pipelineResumeBusy || pipelineStartBusy}
                    >
                      {pipelineResumeBusy || pipelineStartBusy ? (
                        <LoadingSpinner size="sm" />
                      ) : pipelineCanResume || pipelineFailed || mangaResultStale ? (
                        <RefreshCw size={13} />
                      ) : (
                        <Play size={13} />
                      )}
                      {pipelineDirectLabel}
                    </BookButton>
                  ) : null}
                </div>
              ) : null}
            </div>

            {canEditManga && !pipelineRunning ? (
              <details className="mt-2 text-small text-book-text-muted">
                <summary className="w-fit cursor-pointer hover:text-book-text-main">
                  生成选项 · {selectedStartStageLabel}
                </summary>
                <label className="mt-2 flex max-w-sm flex-col gap-1">
                  <span>开始阶段</span>
                  <select
                    value={pipelineStartStage}
                    onChange={(event) =>
                      onPipelineStartStageChange(
                        event.target.value as MangaPipelineStartStageOption,
                      )
                    }
                    className="pyqt-input h-9 !px-2 !py-1 text-xs"
                    disabled={detailBusy || pipelineStartBusy || selectedChapterGenerating}
                  >
                    {MANGA_PIPELINE_START_STAGE_OPTIONS.map((option) => (
                      <option key={option.value || "fresh"} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </label>
              </details>
            ) : null}

            <div
              className={`mt-3 flex items-start gap-2 rounded-xl border bg-book-bg-paper/50 px-2.5 py-2 text-xs ${noticeClass[notice.tone]}`}
            >
              <span className="mt-0.5 shrink-0">{noticeIcon(notice.tone)}</span>
              <span className="min-w-0 flex-1 whitespace-pre-wrap break-words leading-relaxed">
                {notice.text}
              </span>
            </div>
          </header>

          <div className="mt-3 min-h-0 flex-1">
            <MangaPanelList
              panels={panels}
              panelsLoading={panelsLoading}
              currentContentHash={currentContentHash}
              chapterLocked={selectedChapterGenerating || !selectedChapterReady}
              generatingPageNumber={generatingPageNumber}
              profiles={profiles}
              portraits={portraits}
              projectImages={projectImages}
              assetsLoading={characterAssetsLoading || projectImagesLoading}
              pipelineStatus={selectedStatus}
              optimizingPromptId={optimizingPromptId}
              actionsDisabled={detailBusy}
              canEditManga={canEditManga}
              onPreviewPageImage={handlePreviewPageImage}
              onGeneratePageImage={handleGeneratePageImage}
              onBindReferenceImage={handleBindReferenceImage}
              onUpdatePrompt={handleUpdatePrompt}
              onOptimizePrompt={handleOptimizePrompt}
              onDeletePrompt={handleDeletePrompt}
            />
          </div>
        </>
      )}
      </section>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="漫画页图片请求预览"
        onClose={closeRequestPreview}
      />
    </>
  );
}
