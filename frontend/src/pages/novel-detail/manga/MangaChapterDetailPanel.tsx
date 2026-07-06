import { useCallback, useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  CircleDashed,
  Eye,
  ImagePlus,
  Loader2,
  PauseCircle,
  Play,
  Plus,
  RefreshCw,
  Trash2,
  XCircle,
} from "lucide-react";
import { extractErrorMessage } from "../../../api";
import type { CharacterProfile } from "../../../api/character";
import type { DownstreamConflictStrategy } from "../../../api/dependencyConflict";
import { generatePageImage, previewPageImageRequest } from "../../../api/image";
import type { GeneratedImage } from "../../../api/image";
import type { ModelRequestPreview } from "../../../api/requestPreview";
import type { Portrait } from "../../../api/portrait";
import { RequestPreviewModal } from "../../../components/feedback/RequestPreviewModal";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { promptDialog } from "../../../components/feedback/PromptDialog";
import { useToast } from "../../../components/feedback/Toast";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import { Modal } from "../../../components/ui/Modal";
import {
  bindMangaReferenceImage,
  createMangaPrompt,
  deleteMangaPrompt,
  optimizeMangaPrompt,
  type BindMangaReferenceImageInput,
  type MangaPanel,
  type PipelineStatus,
  updateMangaPrompt,
} from "../../../api/manga";
import { createProgressTaskId, useProgressTask } from "../../../hooks/useProgressTask";
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
  selectedStatus: PipelineStatus | null | undefined;
  panels: MangaPanel[];
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
  onPanelsChanged: () => void;
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
  const { addToast } = useToast();
  const { runProgressTask: runPageImageProgressTask } = useProgressTask();
  const [generatingPageNumber, setGeneratingPageNumber] = useState<number | null>(null);
  const [panelActionBusy, setPanelActionBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [createPanelOpen, setCreatePanelOpen] = useState(false);
  const [createPageNumber, setCreatePageNumber] = useState("1");
  const [createPrompt, setCreatePrompt] = useState("");
  const [creatingPanel, setCreatingPanel] = useState(false);
  const [optimizingPromptId, setOptimizingPromptId] = useState<number | null>(null);
  const selectedChapterId = selectedChapter?.id ?? null;
  const scopeKey = `${projectId}:${selectedChapterId ?? "none"}`;
  const mountedRef = useRef(false);
  const activeScopeRef = useRef(scopeKey);
  const panelsRef = useRef<MangaPanel[]>(panels);
  const actionSeqRef = useRef(0);
  const actionBusyRef = useRef(false);
  const previewSeqRef = useRef(0);
  const previewBusyRef = useRef(false);
  const selectedChapterReady = Boolean(selectedChapter?.content?.trim());
  const selectedChapterGenerating = isChapterGenerating(selectedChapter);
  const detailBusy = panelActionBusy || requestPreviewLoading || creatingPanel;

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
    setCreatePanelOpen(false);
    setCreatePageNumber("1");
    setCreatePrompt("");
    setCreatingPanel(false);
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
    const taskId = createProgressTaskId("manga-page-image", projectId);
    setGeneratingPageNumber(pageNumber);
    try {
      await runPageImageProgressTask(
        {
          title: "生成整页漫画",
          startMessage: `正在准备第 ${pageNumber} 页图片生成`,
          successMessage: "整页漫画生成完成",
          errorMessage: "整页漫画生成失败",
          taskId,
          total: 4,
          cancelLabel: "停止生成",
          runningHint: "使用当前页 page_generation 提示词直接生成完整漫画页",
        },
        async () => {
          ensureCurrentAction(requestScope, actionSeq);
          await generatePageImage(projectId, chapter.chapter_number, pageNumber, taskId);
          ensureCurrentAction(requestScope, actionSeq);
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

  const handleBindReferenceImage = async (input: BindMangaReferenceImageInput) => {
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
          assetLabel: `第 ${prompt.page_number} 页提示词 #${prompt.id}`,
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
          assetLabel: `第 ${prompt.page_number} 页提示词 #${prompt.id}`,
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

  const handleOptimizePrompt = async (prompt: MangaPanel) => {
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
          assetLabel: `第 ${requestPrompt.page_number} 页提示词 #${requestPrompt.id}`,
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

  const handleCreatePanel = async () => {
    if (!canEditManga) {
      addToast("当前账号没有编辑此漫画项目的权限", "error");
      return;
    }
    const requestScope = scopeKey;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (selectedChapterGenerating) {
      addToast("当前章节正文仍在生成中，无法新增漫画提示词", "error");
      return;
    }
    const pageNumber = Number(createPageNumber);
    if (!Number.isInteger(pageNumber) || pageNumber <= 0) {
      addToast("页码必须是正整数", "error");
      return;
    }
    const imagePrompt = createPrompt.trim();
    if (!imagePrompt) {
      addToast("整页漫画提示词不能为空", "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    setCreatingPanel(true);
    try {
      await createMangaPrompt(projectId, chapter.id, {
        page_number: pageNumber,
        image_prompt: imagePrompt,
      });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast("漫画提示词已新增", "success");
      setCreatePanelOpen(false);
      setCreatePrompt("");
      await onPanelsChanged();
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, "新增漫画提示词失败"), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setCreatingPanel(false);
      finishAction(actionSeq);
    }
  };

  const handleDeletePrompt = async (prompt: MangaPanel) => {
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
          assetLabel: `第 ${requestPrompt.page_number} 页提示词 #${requestPrompt.id}`,
          downstreamLabel: "漫画图片",
          downstreamCount: 1,
          operation: "删除",
        });
        if (!strategy) return;
        downstreamStrategy = strategy;
      } else {
        const ok = await confirmDialog({
          title: "删除漫画提示词",
          message: `删除第 ${requestPrompt.page_number} 页提示词 #${requestPrompt.id}？`,
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

  return (
    <>
      <BookCard
        className={
          isSingleMode
            ? "flex h-full min-h-0 flex-col p-3"
            : "flex min-h-[60vh] flex-col p-3 lg:h-[calc(100vh-260px)] lg:min-h-0 lg:p-4"
        }
      >
      {!selectedChapter ? (
        <div className="m-auto text-sm text-book-text-muted">
          {isSingleMode ? "请先在写作桌选择章节" : "从左侧选择一章查看漫画详情"}
        </div>
      ) : (
        <>
          <header className="shrink-0 border-b border-book-border/55 pb-3">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="flex min-w-0 flex-wrap items-center gap-2">
                  <h3 className="truncate font-serif text-sm text-book-text-main">
                    {chapterHeading(selectedChapter)}
                  </h3>
                  <span className={`pyqt-pill !text-[10px] ${noticeClass[statusTone]}`}>
                    {stageText}
                  </span>
                </div>
                <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-book-text-muted">
                  <span>正文 {selectedChapter.content?.length ?? 0} 字符</span>
                  <span>立绘 {primaryPortraitCount}/{characterProfileCount} 主立绘</span>
                  <span>素材 {portraitCount} 张</span>
                  {selectedStatus?.current != null && selectedStatus.total != null ? (
                    <span>
                      流水线 {selectedStatus.current}/{selectedStatus.total}
                    </span>
                  ) : null}
                </div>
              </div>

              {canEditManga ? (
                <div className="flex max-w-full flex-wrap items-center justify-end gap-2">
                  <select
                    value={pipelineStartStage}
                    onChange={(event) =>
                      onPipelineStartStageChange(
                        event.target.value as MangaPipelineStartStageOption,
                      )
                    }
                    className="pyqt-input h-8 min-w-[136px] !px-2 !py-1 text-xs"
                    disabled={
                      detailBusy ||
                      acting === `start-${selectedChapter.chapter_number}` ||
                      selectedStatus?.can_cancel === true ||
                      selectedChapterGenerating
                    }
                  >
                    {MANGA_PIPELINE_START_STAGE_OPTIONS.map((option) => (
                      <option key={option.value || "fresh"} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                  <BookButton
                    size="sm"
                    variant="secondary"
                    onClick={() => onPreviewPipeline("start")}
                    disabled={
                      detailBusy ||
                      acting === `start-${selectedChapter.chapter_number}` ||
                      !selectedChapterReady ||
                      selectedStatus?.can_cancel === true ||
                      selectedChapterGenerating
                    }
                  >
                    <Eye size={13} />
                    预览启动
                  </BookButton>
                  <BookButton
                    size="sm"
                    variant="primary"
                    onClick={() => onRunPipeline("start")}
                    disabled={
                      detailBusy ||
                      acting === `start-${selectedChapter.chapter_number}` ||
                      !selectedChapterReady ||
                      selectedStatus?.can_cancel === true ||
                      selectedChapterGenerating
                    }
                  >
                    {acting === `start-${selectedChapter.chapter_number}` ? (
                      <Loader2 size={13} className="animate-spin" />
                    ) : (
                      <Play size={13} />
                    )}
                    启动
                  </BookButton>
                  <BookButton
                    size="sm"
                    variant="secondary"
                    onClick={() => onPreviewPipeline("resume")}
                    disabled={
                      detailBusy ||
                      acting === `resume-${selectedChapter.chapter_number}` ||
                      !selectedChapterReady ||
                      selectedStatus?.can_resume !== true ||
                      selectedChapterGenerating
                    }
                  >
                    <Eye size={13} />
                    预览续传
                  </BookButton>
                  <BookButton
                    size="sm"
                    variant="secondary"
                    onClick={() => onRunPipeline("resume")}
                    disabled={
                      detailBusy ||
                      acting === `resume-${selectedChapter.chapter_number}` ||
                      !selectedChapterReady ||
                      selectedStatus?.can_resume !== true ||
                      selectedChapterGenerating
                    }
                  >
                    <RefreshCw size={13} />
                    续传
                  </BookButton>
                  <BookButton
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      const nextPage = panels.reduce((max, panel) => Math.max(max, panel.page_number), 0) + 1;
                      setCreatePageNumber(String(nextPage));
                      setCreatePrompt("");
                      setCreatePanelOpen(true);
                    }}
                    disabled={detailBusy || selectedChapterGenerating}
                  >
                    <Plus size={13} />
                    新增页
                  </BookButton>
                  <BookButton
                    size="sm"
                    variant="secondary"
                    onClick={onGenerateMissingPortraits}
                    disabled={
                      detailBusy ||
                      acting === "generate-missing-portraits" ||
                      characterAssetsLoading ||
                      missingPortraitCount === 0
                    }
                  >
                    {acting === "generate-missing-portraits" ? (
                      <Loader2 size={13} className="animate-spin" />
                    ) : (
                      <ImagePlus size={13} />
                    )}
                    {missingPortraitCount > 0 ? missingPortraitCount : "立绘"}
                  </BookButton>
                  <BookButton
                    size="sm"
                    variant="secondary"
                    onClick={onCancelPipeline}
                    disabled={
                      detailBusy ||
                      acting === `cancel-${selectedChapter.id}` ||
                      selectedStatus?.can_cancel !== true
                    }
                  >
                    <PauseCircle size={13} />
                    取消
                  </BookButton>
                  <BookButton
                    size="sm"
                    variant="danger"
                    onClick={onDeleteManga}
                    disabled={
                      detailBusy ||
                      acting === `delete-${selectedChapter.id}` ||
                      selectedChapterGenerating
                    }
                  >
                    <Trash2 size={13} />
                    删除
                  </BookButton>
                </div>
              ) : null}
            </div>

            <div
              className={`mt-3 flex items-start gap-2 rounded border bg-book-bg-paper/50 px-2.5 py-2 text-xs ${noticeClass[notice.tone]}`}
            >
              <span className="mt-0.5 shrink-0">{noticeIcon(notice.tone)}</span>
              <span className="min-w-0 flex-1 whitespace-pre-wrap break-words leading-relaxed">
                {notice.text}
              </span>
              {selectedStatus?.task_id ? (
                <span className="hidden max-w-[220px] truncate font-mono text-[10px] text-book-text-muted lg:inline">
                  {selectedStatus.task_id}
                </span>
              ) : null}
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
      </BookCard>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="漫画页图片请求预览"
        onClose={closeRequestPreview}
      />
      <Modal
        isOpen={createPanelOpen}
        onClose={() => {
          if (!creatingPanel && !panelActionBusy) setCreatePanelOpen(false);
        }}
        title="新增整页漫画提示词"
        icon={<Plus size={15} />}
        maxWidthClassName="max-w-2xl"
        footer={
          <>
            <BookButton
              variant="secondary"
              onClick={() => setCreatePanelOpen(false)}
              disabled={creatingPanel || panelActionBusy}
            >
              取消
            </BookButton>
            <BookButton
              variant="primary"
              onClick={() => void handleCreatePanel()}
              disabled={creatingPanel || panelActionBusy || !createPrompt.trim()}
            >
              {creatingPanel ? <Loader2 size={12} className="animate-spin" /> : null}
              新增
            </BookButton>
          </>
        }
      >
        <div className="grid gap-3 text-xs">
          <label className="grid gap-1 text-[11px] text-book-text-muted">
            页码
            <input
              type="number"
              min={1}
              value={createPageNumber}
              disabled={creatingPanel || panelActionBusy}
              onChange={(event) => setCreatePageNumber(event.target.value)}
              className="pyqt-input !px-2 !py-1 text-xs"
            />
          </label>
          <label className="grid gap-1 text-[11px] text-book-text-muted">
            整页漫画提示词
            <textarea
              value={createPrompt}
              disabled={creatingPanel || panelActionBusy}
              onChange={(event) => setCreatePrompt(event.target.value)}
              className="pyqt-input min-h-[220px] resize-y !px-2 !py-1 font-mono text-xs"
            />
          </label>
        </div>
      </Modal>
    </>
  );
}
