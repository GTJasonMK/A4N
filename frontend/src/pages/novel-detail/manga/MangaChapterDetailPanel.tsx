import { useCallback, useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  CircleDashed,
  Eye,
  ImagePlus,
  PauseCircle,
  Play,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ImageSizeSelect } from "../../../components/image/ImageSizeSelect";
import { extractErrorMessage } from "../../../api";
import type { CharacterProfile } from "../../../api/character";
import type {
  MangaPipelineArtifactDeletionItem,
  MangaPipelineArtifactRecord,
  MangaPipelineArtifactStage,
  MangaPipelineStopAfter,
  MangaReferenceAssetKind,
  MangaPageReferenceTargetKind,
} from "../../../api/generated/host";
import {
  previewPageImageRequest,
  startImageGeneration,
  uploadImage,
  verifyImageGenerationResult,
} from "../../../api/image";
import type { GeneratedImage, ImageSize } from "../../../api/image";
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
  deleteMangaPipelineArtifact,
  deleteMangaPage,
  optimizeMangaPrompt,
  saveMangaPipelineArtifact,
  type BindMangaReferenceImageInput,
  type MangaPipelineStatusSummary,
  type MangaPagePromptSummary,
  updateMangaPrompt,
} from "../../../api/manga";
import { previewDeleteMangaPipelineArtifact } from "../../../api/manga/pipelineArtifacts";
import { useProgressTask } from "../../../hooks/useProgressTask";
import { isCancelledOperation } from "../../../utils/cancellation";
import { EmptyHint, isChapterGenerating, type ChapterRow } from "../shared";
import { MangaPagePromptList } from "./MangaPagePromptList";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { MAX_UPLOAD_IMAGE_BYTES } from "../../../api/generated/config";
import { isSupportedImageMime } from "../../../utils/imageUpload";
import {
  chapterHeading,
  countStaleMangaPagePrompts,
  mangaDeletionDownstreamSummary,
  mangaPageImageBlockReason,
  mangaPagePipelineStageHasOutput,
  mangaStageLabel,
  mangaImageStatusMatchesWorkspaceStage,
  mangaPageNumberForPipelineStage,
  mangaPageStageNeedsGeneration,
  mangaPipelineActionIsReady,
  mangaPipelineActionsForWorkspaceStage,
  mangaPipelineStageMatchesWorkspaceStage,
  mangaPipelineTargetForWorkspaceStage,
  mangaStageAllowsReferenceBinding,
  type MangaPageWorkspaceStage,
} from "./mangaTabUtils";
import { useI18n } from "../../../i18n";

interface MangaChapterDetailPanelProps {
  stageMode?: MangaPageWorkspaceStage;
  onDirtyChange?: (dirty: boolean) => void;
  projectId: number;
  isSingleMode: boolean;
  selectedChapter: ChapterRow | null;
  selectedChapterLoading: boolean;
  selectedChapterLoadError: string | null;
  onRetryChapter: () => void;
  selectedStatus: MangaPipelineStatusSummary | null | undefined;
  pagePrompts: MangaPagePromptSummary[];
  pageSlotsNeedsGeneration?: boolean;
  pageSlotsArtifactStale?: boolean;
  preprocessingArtifactStale?: boolean;
  paginationActions?: React.ReactNode;
  selectedPageNumber: number | null;
  onSelectedPageChange: (pageNumber: number | null) => void;
  pagePromptsLoading: boolean;
  pagePromptsError: string | null;
  currentContentHash: string | null;
  characterAssetsLoading: boolean;
  characterAssetsError: string | null;
  projectImagesLoading: boolean;
  profiles: CharacterProfile[];
  portraits: Portrait[];
  projectImages: GeneratedImage[];
  imageSize: ImageSize | null;
  imageSizes: readonly ImageSize[];
  imageSizeLoading: boolean;
  imageSizeError: Error | null;
  onImageSizeChange: (size: ImageSize) => void;
  primaryPortraitCount: number;
  characterProfileCount: number;
  portraitCount: number;
  missingPortraitCount: number;
  acting: string | null;
  canEditManga: boolean;
  onPreviewPipeline: (
    mode: "start" | "resume",
    stopAfter: MangaPipelineStopAfter | null,
    pageNumber: number | null,
    runStage: MangaPipelineArtifactStage | null,
  ) => void;
  onRunPipeline: (
    mode: "start" | "resume",
    stopAfter: MangaPipelineStopAfter | null,
    pageNumber: number | null,
    runStage: MangaPipelineArtifactStage | null,
  ) => void;
  onGenerateMissingPortraits?: () => void;
  onCancelPipeline: () => void;
  onDeleteManga: () => void;
  onPagePromptsChanged: () => Promise<boolean>;
  onPagePromptChanged: (prompt: MangaPagePromptSummary) => void;
  onPagePromptDeleted: (pageNumber: number) => void;
  onPageArtifactChanged: (artifact: MangaPipelineArtifactRecord) => void;
  onPageArtifactsDeleted: (artifacts: MangaPipelineArtifactDeletionItem[]) => void;
  onProjectImageImported: (image: GeneratedImage) => void;
  onRetryPagePrompts: () => void;
}

type NoticeTone = "ok" | "warning" | "danger" | "muted";
type InlineNoticeTone = "warning" | "muted" | "danger";
type PipelineStageActionButton = {
  runStage: MangaPipelineArtifactStage | null;
  stopAfter: MangaPipelineStopAfter;
  label: string;
  prerequisiteLabel: string;
  prerequisiteReady: boolean;
};

const noticeClass: Record<NoticeTone, string> = {
  ok: "border-success/40 text-success",
  warning: "border-warning/50 text-warning",
  danger: "border-error/50 text-error",
  muted: "border-book-border/50 text-book-text-sub",
};

const noticeIcon = (tone: InlineNoticeTone) => {
  if (tone !== "muted") return <AlertTriangle size={13} />;
  return <CircleDashed size={13} />;
};

export function MangaChapterDetailPanel({
  stageMode,
  onDirtyChange,
  projectId,
  isSingleMode,
  selectedChapter,
  selectedChapterLoading,
  selectedChapterLoadError,
  onRetryChapter,
  selectedStatus,
  pagePrompts,
  pageSlotsNeedsGeneration = false,
  pageSlotsArtifactStale = false,
  preprocessingArtifactStale = false,
  paginationActions,
  selectedPageNumber,
  onSelectedPageChange,
  pagePromptsLoading,
  pagePromptsError,
  currentContentHash,
  characterAssetsLoading,
  characterAssetsError,
  projectImagesLoading,
  profiles,
  portraits,
  projectImages,
  imageSize,
  imageSizes,
  imageSizeLoading,
  imageSizeError,
  onImageSizeChange,
  primaryPortraitCount,
  characterProfileCount,
  portraitCount,
  missingPortraitCount,
  acting,
  canEditManga,
  onPreviewPipeline,
  onRunPipeline,
  onGenerateMissingPortraits,
  onCancelPipeline,
  onDeleteManga,
  onPagePromptsChanged,
  onPagePromptChanged,
  onPagePromptDeleted,
  onPageArtifactChanged,
  onPageArtifactsDeleted,
  onProjectImageImported,
  onRetryPagePrompts,
}: MangaChapterDetailPanelProps) {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const { runProgressTask: runPageImageProgressTask } = useProgressTask();
  const [generatingPageNumber, setGeneratingPageNumber] = useState<
    number | null
  >(null);
  const [promptActionBusy, setPromptActionBusy] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [optimizingPromptId, setOptimizingPromptId] = useState<number | null>(
    null,
  );
  const [uploadingReferenceBindingKey, setUploadingReferenceBindingKey] =
    useState<string | null>(null);
  const selectedChapterId = selectedChapter?.id ?? null;
  const scopeKey = `${projectId}:${selectedChapterId ?? "none"}`;
  const mountedRef = useRef(false);
  const activeScopeRef = useRef(scopeKey);
  const pagePromptsRef = useRef<MangaPagePromptSummary[]>(pagePrompts);
  const actionSeqRef = useRef(0);
  const actionBusyRef = useRef(false);
  const referenceUploadBusyRef = useRef(false);
  const previewSeqRef = useRef(0);
  const previewBusyRef = useRef(false);
  const selectedChapterReady =
    !selectedChapterLoading &&
    selectedChapterLoadError == null &&
    Boolean(selectedChapter?.content?.trim());
  const selectedChapterGenerating = isChapterGenerating(selectedChapter);
  const pipelineRunning =
    selectedStatus?.pipeline_status === "running" || selectedStatus?.can_cancel === true;
  const pagePromptReadUnavailable = pagePromptsLoading || pagePromptsError != null;
  const detailBusy =
    promptActionBusy ||
    requestPreviewLoading ||
    uploadingReferenceBindingKey != null ||
    pipelineRunning ||
    pagePromptReadUnavailable;
  const ensurePagePromptsCurrent = () => {
    if (!pagePromptReadUnavailable) return true;
    addToast(pagePromptsError ?? t("common.operationInProgress"), "error");
    return false;
  };
  const refreshPagePromptsAfterConflict = (error: unknown) => {
    if (extractErrorMessage(error, "").includes("已被其他操作修改")) {
      void onPagePromptsChanged();
    }
  };

  activeScopeRef.current = scopeKey;
  pagePromptsRef.current = pagePrompts;

  const isCurrentScope = useCallback(
    (requestScope: string) =>
      mountedRef.current && activeScopeRef.current === requestScope,
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
        addToast(t("comic.prompt.operationInProgress"), "error");
        return null;
      }
      actionBusyRef.current = true;
      actionSeqRef.current += 1;
      setPromptActionBusy(true);
      return actionSeqRef.current;
    },
    [addToast, isCurrentScope, t],
  );

  const finishAction = useCallback((requestSeq: number) => {
    if (actionSeqRef.current === requestSeq) {
      actionBusyRef.current = false;
      setPromptActionBusy(false);
    }
  }, []);

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
    pagePromptsRef.current = pagePrompts;
    setGeneratingPageNumber(null);
    setPromptActionBusy(false);
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
    setOptimizingPromptId(null);
  }, [scopeKey]);

  useEffect(() => {
    pagePromptsRef.current = pagePrompts;
  }, [pagePrompts]);
  const stalePagePromptCount =
    selectedChapter != null
      ? countStaleMangaPagePrompts(pagePrompts, currentContentHash)
      : 0;
  const pipelineHasStoppedWithError =
    selectedStatus?.pipeline_status === "failed" ||
    selectedStatus?.pipeline_status === "interrupted";
  const pipelineStageMatchesWorkspace = mangaPipelineStageMatchesWorkspaceStage(
    stageMode,
    selectedStatus?.stage,
  );
  const pipelineStoppedWithError =
    pipelineHasStoppedWithError && pipelineStageMatchesWorkspace;
  const pipelineStatusVisible =
    stageMode == null ||
    selectedStatus?.pipeline_status === "running" ||
    pipelineStageMatchesWorkspace;
  const pipelineCanResumeForStage =
    selectedStatus?.can_resume === true &&
    selectedStatus.pipeline_status !== "running" &&
    selectedStatus.can_cancel !== true &&
    mangaPipelineStageMatchesWorkspaceStage(stageMode, selectedStatus.stage);

  const stageText = !pipelineStatusVisible
    ? ""
    : selectedStatus?.stage
      ? mangaStageLabel(selectedStatus.stage, t)
      : selectedStatus?.pipeline_status
        ? mangaStageLabel(selectedStatus.pipeline_status, t)
        : t("comic.pipelineStages.notStarted");
  const selectedPagePrompt = pagePrompts.find(
    (prompt) => prompt.page_number === selectedPageNumber,
  );
  const visualDesignRunStage = selectedPagePrompt?.page_plan != null
    ? "reference_discovery"
    : "storyboard";
  const visualDesignPrerequisiteAction = mangaPipelineActionsForWorkspaceStage("visual_design")
    .find((action) => action.runStage === visualDesignRunStage);
  const visualDesignTargetAlreadyGenerated =
    mangaPagePipelineStageHasOutput(selectedPagePrompt, "reference_discovery") ||
    (visualDesignRunStage === "storyboard" &&
      mangaPagePipelineStageHasOutput(selectedPagePrompt, "storyboard"));
  const visualDesignPrerequisitesReady =
    !pageSlotsNeedsGeneration &&
    !pagePromptsLoading &&
    pagePromptsError == null &&
    selectedPagePrompt != null &&
    !selectedPagePrompt.orphaned &&
    visualDesignPrerequisiteAction != null &&
    mangaPipelineActionIsReady(visualDesignPrerequisiteAction, selectedPagePrompt);
  const pipelineTarget = stageMode
    ? mangaPipelineTargetForWorkspaceStage(stageMode, selectedPagePrompt)
    : null;
  const pipelineStopAfter = pipelineTarget?.stopAfter ?? null;
  const pipelineRunStage = pipelineTarget?.runStage ?? null;
  const pageScopedStage = stageMode != null && stageMode !== "page_planning";
  const pipelinePageNumber = mangaPageNumberForPipelineStage(
    stageMode,
    selectedPageNumber,
  );
  const pipelineTargetLabel = stageMode === "page_planning"
    ? t("comic.pipelineStages.pagePlanning")
    : pipelineRunStage
      ? mangaStageLabel(pipelineRunStage, t)
      : null;
  const pipelineActionTargetLabel =
    pipelineTargetLabel && pipelinePageNumber != null
      ? `${pipelineTargetLabel} · ${t("comic.prompt.pageLabel", { page: pipelinePageNumber })}`
      : pipelineTargetLabel;
  const pipelineStageActionLabels = {
    storyboard: t("comic.prompt.generateStoryboard"),
    reference_discovery: t("comic.prompt.discoverReferences"),
    drawing_prompt: t("comic.prompt.generateDrawingPrompt"),
    image: t("comic.prompt.generatePageImage"),
  };
  const pipelineStageActions: PipelineStageActionButton[] = stageMode === "visual_design"
    ? visualDesignTargetAlreadyGenerated
      ? []
      : [{
          runStage: visualDesignRunStage,
          stopAfter: "reference_discovery",
          label: visualDesignRunStage === "storyboard"
            ? t("comic.prompt.generatePageVisualDesign")
            : pipelineStageActionLabels.reference_discovery,
          prerequisiteLabel: pageSlotsNeedsGeneration || !visualDesignPrerequisiteAction
            ? t("comic.pipelineStages.pagePlanning")
            : mangaStageLabel(visualDesignPrerequisiteAction.prerequisite, t),
          prerequisiteReady: visualDesignPrerequisitesReady,
        }]
    : stageMode
    ? mangaPipelineActionsForWorkspaceStage(stageMode)
      .filter((action) => !mangaPagePipelineStageHasOutput(selectedPagePrompt, action.runStage))
      .map((action) => ({
        ...action,
        label: pipelineStageActionLabels[action.runStage],
        prerequisiteLabel: mangaStageLabel(action.prerequisite, t),
        prerequisiteReady: mangaPipelineActionIsReady(action, selectedPagePrompt),
      }))
    : [];
  const statusTone: NoticeTone =
    pipelineStoppedWithError
      ? "danger"
      : pipelineStatusVisible && selectedStatus?.pipeline_status === "running"
        ? "warning"
        : pipelineStatusVisible && selectedStatus?.pipeline_status === "completed"
          ? "ok"
          : "muted";
  const chapterMeta = [
    t("comic.chapter.bodyMeta", { count: selectedChapter?.content?.length ?? 0 }),
    characterProfileCount > 0 || portraitCount > 0
      ? t("comic.chapter.portraitMeta", {
          primary: primaryPortraitCount,
          total: characterProfileCount,
        })
      : null,
    portraitCount > 0 ? t("comic.chapter.assetMeta", { count: portraitCount }) : null,
    pipelineStatusVisible && selectedStatus?.current != null && selectedStatus.total != null
      ? t("comic.chapter.pipelineMeta", {
          current: selectedStatus.current,
          total: selectedStatus.total,
        })
      : null,
  ].filter((item): item is string => item != null);

  const notice = (() => {
    if (selectedChapterLoading) {
      return {
        tone: "muted" as InlineNoticeTone,
        text: t("comic.loadingChapters"),
      };
    }
    if (selectedChapterLoadError) {
      return {
        tone: "danger" as InlineNoticeTone,
        text: selectedChapterLoadError,
      };
    }
    if (!selectedChapterReady) {
      return {
        tone: "warning" as InlineNoticeTone,
        text: t("comic.chapter.emptyBodyNotice"),
      };
    }
    if (selectedChapterGenerating) {
      return {
        tone: "warning" as InlineNoticeTone,
        text: t("comic.chapter.generatingNotice"),
      };
    }
    if (pipelineStoppedWithError) return null;
    if (pipelineCanResumeForStage) {
      return {
        tone: "warning" as InlineNoticeTone,
        text: selectedStatus.checkpoint_valid
          ? t("comic.chapter.resumableNotice")
          : t("comic.chapter.staleCheckpointNotice"),
      };
    }
    if (missingPortraitCount > 0 && onGenerateMissingPortraits) {
      return {
        tone: "warning" as InlineNoticeTone,
        text: t("comic.chapter.missingPortraitNotice", {
          count: missingPortraitCount,
        }),
      };
    }
    if (stalePagePromptCount > 0) {
      return {
        tone: "warning" as InlineNoticeTone,
        text: t("comic.chapter.stalePromptNotice", {
          count: stalePagePromptCount,
        }),
      };
    }
    if (pagePrompts.length > 0 || stageMode != null) return null;
    return {
      tone: "muted" as InlineNoticeTone,
      text: t("comic.chapter.readyNotice"),
    };
  })();

  const handleGeneratePageImage = async (pageNumber: number) => {
    if (!ensurePagePromptsCurrent()) return;
    if (!canEditManga) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    const requestScope = scopeKey;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (!selectedChapterReady) {
      addToast(t("comic.chapter.emptyBodyGenerate"), "error");
      return;
    }
    if (selectedChapterGenerating || pipelineRunning) {
      addToast(t("comic.chapter.generatingGenerate"), "error");
      return;
    }
    const pagePrompt = pagePromptsRef.current.find(
      (prompt) => prompt.page_number === pageNumber,
    );
    if (!pagePrompt) {
      addToast(t("comic.prompt.notFound"), "error");
      return;
    }
    if (pagePrompt.generated_image_id != null) return;
    const imageBlockReason = mangaPageImageBlockReason(pagePrompt, t);
    if (imageBlockReason) {
      addToast(imageBlockReason, "error");
      return;
    }
    if (!imageSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
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
          width: imageSize.width,
          height: imageSize.height,
        },
      });
      await runGenerationProgressTaskThen(
        runPageImageProgressTask,
        handle,
        {
          title: t("comic.image.title"),
          startMessage: t("comic.image.start", { page: pageNumber }),
          successMessage: t("comic.image.success"),
          errorMessage: t("comic.image.failed"),
          cancelLabel: t("comic.stopGeneration"),
          runningHint: t("comic.image.runningHint"),
        },
        async (task) => {
          ensureCurrentAction(requestScope, actionSeq);
          await verifyImageGenerationResult(task, projectId);
        },
      );
      if (!isCurrentAction(requestScope, actionSeq)) return;
      void onPagePromptsChanged();
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast(t("comic.image.generated", { page: pageNumber }), "success");
    } catch (err) {
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const message = extractErrorMessage(err, t("comic.image.failed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      if (isCurrentAction(requestScope, actionSeq))
        setGeneratingPageNumber(null);
      finishAction(actionSeq);
    }
  };

  const handlePreviewPageImage = async (pageNumber: number) => {
    if (!ensurePagePromptsCurrent()) return;
    if (!canEditManga) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    const requestScope = scopeKey;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (!selectedChapterReady) {
      addToast(t("comic.chapter.emptyBodyPreview"), "error");
      return;
    }
    if (selectedChapterGenerating || pipelineRunning) {
      addToast(t("comic.chapter.generatingPreview"), "error");
      return;
    }
    const pagePrompt = pagePromptsRef.current.find(
      (prompt) => prompt.page_number === pageNumber,
    );
    if (!pagePrompt) {
      addToast(t("comic.prompt.notFound"), "error");
      return;
    }
    const imageBlockReason = mangaPageImageBlockReason(pagePrompt, t);
    if (imageBlockReason) {
      addToast(imageBlockReason, "error");
      return;
    }
    if (!imageSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
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
        imageSize.width,
        imageSize.height,
      );
      if (!isCurrentPreview(requestScope, requestSeq)) return;
      setRequestPreviewData(preview);
    } catch (err) {
      if (isCurrentPreview(requestScope, requestSeq)) {
        addToast(extractErrorMessage(err, t("comic.image.previewFailed")), "error");
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
    input: BindMangaReferenceImageInput,
    sourceImage?: GeneratedImage,
  ) => {
    if (!ensurePagePromptsCurrent()) return;
    if (!canEditManga) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    if (selectedChapterGenerating || pipelineRunning) {
      addToast(t("comic.chapter.generatingBind"), "error");
      return;
    }
    const requestScope = scopeKey;
    const prompt =
      pagePromptsRef.current.find((prompt) => prompt.id === input.prompt_id) ?? null;
    if (!prompt) {
      addToast(t("comic.prompt.notFound"), "error");
      return;
    }
    if (actionBusyRef.current) {
      addToast(t("comic.prompt.operationInProgress"), "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    try {
      if (!isCurrentAction(requestScope, actionSeq)) return;
      const currentPrompt =
        pagePromptsRef.current.find((prompt) => prompt.id === input.prompt_id) ?? null;
      if (!currentPrompt) {
        addToast(t("comic.prompt.notFound"), "error");
        return;
      }
      let bindingInput = input;
      if (
        input.action === "add_user_image" &&
        input.asset_kind === "generated_image" &&
        sourceImage?.id === input.asset_id &&
        sourceImage.project_id !== projectId
      ) {
        const bindingKey = `${input.prompt_id}:${input.character_name}`;
        setUploadingReferenceBindingKey(bindingKey);
        const importedImage = await importReferenceImage(sourceImage);
        if (!isCurrentAction(requestScope, actionSeq)) return;
        bindingInput = { ...input, asset_id: importedImage.id };
      }
      const updatedPrompt = await bindMangaReferenceImage(bindingInput);
      if (!isCurrentAction(requestScope, actionSeq)) return;
      onPagePromptChanged(updatedPrompt);
      void onPagePromptsChanged();
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast(
        input.action === "remove_user_image"
          ? t("comic.prompt.removedUserReference")
          : input.action === "add_user_image"
            ? t("comic.prompt.addedUserReference")
            : t("comic.prompt.savedReference"),
        "success",
      );
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, t("comic.prompt.bindFailed")), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) {
        setUploadingReferenceBindingKey(null);
      }
      finishAction(actionSeq);
    }
  };

  const handleUploadReferenceImage = async (
    promptId: number,
    characterName: string,
    file: File,
  ): Promise<void> => {
    if (!ensurePagePromptsCurrent()) return;
    if (!canEditManga || !selectedChapter) return;
    if (referenceUploadBusyRef.current) return;
    if (selectedChapterGenerating || pipelineRunning) {
      addToast(t("comic.chapter.generatingBind"), "error");
      return;
    }
    if (!pagePromptsRef.current.some((prompt) => prompt.id === promptId)) {
      addToast(t("comic.prompt.notFound"), "error");
      return;
    }
    if (actionBusyRef.current) {
      addToast(t("comic.prompt.operationInProgress"), "error");
      return;
    }
    if (!isSupportedImageMime(file.type)) {
      addToast(t("characterDetail.portrait.invalidImage"), "error");
      return;
    }
    if (file.size > MAX_UPLOAD_IMAGE_BYTES) {
      addToast(
        t("characterDetail.portrait.fileTooLarge", {
          size: MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024,
        }),
        "error",
      );
      return;
    }

    const requestScope = scopeKey;
    const bindingKey = `${promptId}:${characterName}`;
    referenceUploadBusyRef.current = true;
    setUploadingReferenceBindingKey(bindingKey);
    try {
      const dataBase64 = await readFileAsBase64(file);
      if (!isCurrentScope(requestScope)) return;
      if (!ensurePagePromptsCurrent()) return;
      const image = await uploadImage(projectId, {
        file_name: file.name,
        mime_type: file.type,
        data_base64: dataBase64,
        image_type: "chapter_reference",
        prompt: `漫画页角色参考图：${characterName}（${file.name}）`,
        chapter_id: selectedChapter.id,
        panel_id: null,
        metadata_json: {
          source: "manga_page_reference_upload",
          character_name: characterName,
          manual_upload: true,
        },
      });
      if (!isCurrentScope(requestScope)) return;
      onProjectImageImported(image);
      if (actionBusyRef.current) {
        addToast(t("comic.prompt.operationInProgress"), "error");
        return;
      }
      await handleBindReferenceImage({
        action: "add_user_image",
        prompt_id: promptId,
        character_name: characterName,
        asset_kind: "generated_image",
        asset_id: image.id,
      });
    } catch (err) {
      if (isCurrentScope(requestScope)) {
        addToast(
          extractErrorMessage(err, t("comic.prompt.referenceUploadFailed")),
          "error",
        );
      }
    } finally {
      referenceUploadBusyRef.current = false;
      if (isCurrentScope(requestScope)) {
        setUploadingReferenceBindingKey(null);
      }
    }
  };

  const handleUpdatePrompt = async (promptId: number, imagePrompt: string) => {
    if (!ensurePagePromptsCurrent()) throw new Error(pagePromptsError ?? t("common.operationInProgress"));
    if (!canEditManga) {
      const message = t("comic.noEditPermission");
      addToast(message, "error");
      throw new Error(message);
    }
    if (selectedChapterGenerating || pipelineRunning) {
      const message = t("comic.chapter.generatingUpdate");
      addToast(message, "error");
      throw new Error(message);
    }
    const requestScope = scopeKey;
    const prompt =
      pagePromptsRef.current.find((prompt) => prompt.id === promptId) ?? null;
    if (!prompt) {
      const message = t("comic.prompt.notFound");
      addToast(message, "error");
      throw new Error(message);
    }
    if (actionBusyRef.current) {
      const message = t("comic.prompt.operationInProgress");
      addToast(message, "error");
      throw new Error(message);
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) throw new Error("任务已停止");
    try {
      if (!isCurrentAction(requestScope, actionSeq))
        throw new Error("任务已停止");
      const currentPrompt =
        pagePromptsRef.current.find((prompt) => prompt.id === promptId) ?? null;
      if (!currentPrompt) {
        const message = t("comic.prompt.notFound");
        addToast(message, "error");
        throw new Error(message);
      }
      const updatedPrompt = await updateMangaPrompt({
        prompt_id: promptId,
        image_prompt: imagePrompt,
        expected_updated_at: currentPrompt.updated_at,
      });
      if (!isCurrentAction(requestScope, actionSeq))
        throw new Error("任务已停止");
      onPagePromptChanged(updatedPrompt);
      void onPagePromptsChanged();
      if (!isCurrentAction(requestScope, actionSeq)) throw new Error("任务已停止");
      addToast(t("comic.prompt.saved"), "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        refreshPagePromptsAfterConflict(err);
        const message = extractErrorMessage(err, t("comic.prompt.saveFailed"));
        if (
          !isCancelledOperation(message) &&
          !message.includes("任务已停止") &&
          !message.includes(t("comic.prompt.notFound")) &&
          !message.includes(t("comic.loadPagePromptsFailed"))
        ) {
          addToast(message, "error");
        }
      }
      throw err;
    } finally {
      finishAction(actionSeq);
    }
  };

  const handleOptimizePrompt = async (prompt: MangaPagePromptSummary) => {
    if (!ensurePagePromptsCurrent()) return;
    if (!canEditManga) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    if (selectedChapterGenerating || pipelineRunning) {
      addToast(t("comic.chapter.generatingOptimize"), "error");
      return;
    }
    const requestScope = scopeKey;
    const requestPrompt =
      pagePromptsRef.current.find((current) => current.id === prompt.id) ?? null;
    if (!requestPrompt) {
      addToast(t("comic.prompt.notFound"), "error");
      return;
    }
    if (actionBusyRef.current) {
      addToast(t("comic.prompt.operationInProgress"), "error");
      return;
    }
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    try {
      const instruction = await promptDialog({
        title: t("comic.prompt.optimizeTitle"),
        message:
          t("comic.prompt.optimizeMessage"),
        placeholder:
          t("comic.prompt.optimizePlaceholder"),
        multiline: true,
        required: true,
        confirmText: t("comic.prompt.startOptimize"),
      });
      if (instruction == null) return;
      const trimmed = instruction.trim();
      if (!trimmed) return;
      if (!isCurrentAction(requestScope, actionSeq)) return;
      if (!ensurePagePromptsCurrent()) return;
      const currentPrompt =
        pagePromptsRef.current.find((current) => current.id === requestPrompt.id) ??
        null;
      if (!currentPrompt) {
        addToast(t("comic.prompt.notFound"), "error");
        return;
      }
      setOptimizingPromptId(requestPrompt.id);
      const updatedPrompt = await optimizeMangaPrompt({
        prompt_id: requestPrompt.id,
        instruction: trimmed,
        expected_updated_at: currentPrompt.updated_at,
      });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      onPagePromptChanged(updatedPrompt);
      void onPagePromptsChanged();
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast(t("comic.prompt.optimized"), "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        refreshPagePromptsAfterConflict(err);
        addToast(extractErrorMessage(err, t("comic.prompt.optimizeFailed")), "error");
      }
    } finally {
      if (isCurrentAction(requestScope, actionSeq)) setOptimizingPromptId(null);
      finishAction(actionSeq);
    }
  };

  const handleDeletePrompt = async (prompt: MangaPagePromptSummary) => {
    if (!canEditManga) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    if (selectedChapterGenerating || pipelineRunning) {
      addToast(t("comic.chapter.generatingDelete"), "error");
      return;
    }
    const requestPrompt =
      pagePromptsRef.current.find((current) => current.id === prompt.id) ?? null;
    if (!requestPrompt) {
      addToast(t("comic.prompt.notFound"), "error");
      return;
    }
    await handleDeleteStageArtifact("drawing_prompt", requestPrompt.page_number);
  };

  const handleSaveStageArtifact = async (
    stage: MangaPipelineArtifactStage,
    pageNumber: number,
    payload: unknown,
  ) => {
    if (!ensurePagePromptsCurrent()) return;
    if (!canEditManga || !selectedChapter) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    if (selectedChapterGenerating || pipelineRunning) {
      addToast(t("comic.chapter.generatingUpdate"), "error");
      return;
    }
    const pagePrompt = pagePromptsRef.current.find(
      (prompt) => prompt.page_number === pageNumber,
    );
    if (!pagePrompt) {
      addToast(t("comic.prompt.notFound"), "error");
      return;
    }
    const actionSeq = beginAction(scopeKey);
    if (actionSeq == null) return;
    try {
      const savedArtifact = await saveMangaPipelineArtifact({
        project_id: projectId,
        chapter_id: selectedChapter.id,
        stage,
        page_number: pageNumber,
        expected_prompt_updated_at: pagePrompt.updated_at,
        payload,
      });
      if (!isCurrentAction(scopeKey, actionSeq)) return;
      onPageArtifactChanged(savedArtifact);
      void onPagePromptsChanged();
      addToast(t("comic.prompt.saved"), "success");
    } catch (err) {
      if (isCurrentAction(scopeKey, actionSeq)) {
        refreshPagePromptsAfterConflict(err);
        addToast(extractErrorMessage(err, t("comic.prompt.saveFailed")), "error");
      }
      throw err;
    } finally {
      finishAction(actionSeq);
    }
  };

  const importReferenceImage = async (sourceImage: GeneratedImage): Promise<GeneratedImage> => {
    const isAvailableToMangaProject = projectImages.some(
      (image) => image.id === sourceImage.id && image.project_id === sourceImage.project_id,
    );
    if (!isAvailableToMangaProject) {
      throw new Error(t("comic.prompt.referenceImageOutsideLibrary"));
    }
    const reusableMaterial =
      sourceImage.image_type === "scene_material" ||
      sourceImage.image_type === "object_material";
    if (
      sourceImage.project_id === projectId &&
      sourceImage.image_type !== "manga_page" &&
      (sourceImage.chapter_id == null ||
        sourceImage.chapter_id === selectedChapter?.id ||
        reusableMaterial)
    ) {
      return sourceImage;
    }
    const existingCopy = projectImages.find((image) => {
      if (image.project_id !== projectId) return false;
      const metadata = image.metadata_json;
      if (typeof metadata !== "object" || metadata === null || Array.isArray(metadata)) return false;
      const source = metadata as Record<string, unknown>;
      return source.source === "manga_reference_project_import" &&
        source.source_project_id === sourceImage.project_id &&
        source.source_image_id === sourceImage.id;
    });
    if (existingCopy) return existingCopy;
    const mediaUrl = toMediaSrc(sourceImage.file_path);
    if (!mediaUrl) throw new Error(t("comic.prompt.referenceImageUnavailable"));
    let response: Response;
    try {
      response = await fetch(mediaUrl);
    } catch {
      throw new Error(t("comic.prompt.referenceImageReadUnavailable"));
    }
    if (!response.ok) {
      throw new Error(t("comic.prompt.referenceImageReadFailed", { status: response.status }));
    }
    const contentLength = Number(response.headers.get("content-length"));
    if (Number.isFinite(contentLength) && contentLength > MAX_UPLOAD_IMAGE_BYTES) {
      throw new Error(t("characterDetail.portrait.fileTooLarge", { size: MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024 }));
    }
    const blob = await response.blob();
    if (blob.size > MAX_UPLOAD_IMAGE_BYTES) {
      throw new Error(t("characterDetail.portrait.fileTooLarge", { size: MAX_UPLOAD_IMAGE_BYTES / 1024 / 1024 }));
    }
    const mimeType = blob.type.split(";")[0].trim().toLowerCase();
    if (!isSupportedImageMime(mimeType)) throw new Error(t("characterDetail.portrait.invalidImage"));
    const fileName = sourceImage.file_path.split(/[?#]/, 1)[0].split(/[\\/]/).pop() || `reference-${sourceImage.id}`;
    const importedImage = await uploadImage(projectId, {
      file_name: fileName,
      mime_type: mimeType,
      data_base64: await readFileAsBase64(new File([blob], fileName, { type: mimeType })),
      image_type: "chapter_reference",
      prompt: sourceImage.prompt.trim() || t("comic.prompt.projectImage"),
      chapter_id: selectedChapter?.id ?? null,
      panel_id: null,
      metadata_json: {
        source: "manga_reference_project_import",
        source_project_id: sourceImage.project_id,
        source_image_id: sourceImage.id,
      },
    });
    onProjectImageImported(importedImage);
    return importedImage;
  };

  const handleBindDiscoveredReference = async (
    sourceImage: GeneratedImage,
    target: { targetKind: MangaPageReferenceTargetKind; targetName: string },
  ) => {
    if (!ensurePagePromptsCurrent() || !canEditManga || !selectedChapter) return;
    if (referenceUploadBusyRef.current || actionBusyRef.current) return;
    const pagePrompt = pagePromptsRef.current.find((prompt) => prompt.page_number === selectedPageNumber);
    if (!pagePrompt) {
      addToast(t("comic.prompt.notFound"), "error");
      return;
    }
    const requestScope = scopeKey;
    referenceUploadBusyRef.current = true;
    setUploadingReferenceBindingKey(`${pagePrompt.id}:manual-reference`);
    try {
      const discovery = pagePrompt.reference_discovery ?? {
        schema_version: "manga_page_reference_discovery_v1",
        requirements: [],
        references: [],
      };
      const targetReferences = discovery.references.filter(
        (reference) =>
          reference.target_kind === target.targetKind &&
          reference.target_name === target.targetName,
      );
      if (targetReferences.length >= 2) {
        throw new Error(t("comic.prompt.referenceTargetLimitReached"));
      }
      const imported = await importReferenceImage(sourceImage);
      if (!isCurrentScope(requestScope)) return;
      const requirement = discovery.requirements.find(
        (item) => item.target_kind === target.targetKind && item.target_name === target.targetName,
      );
      const payload = {
        ...discovery,
        requirements: discovery.requirements,
        references: [
          ...discovery.references.filter(
            (reference) => reference.asset.asset_kind !== "generated_image" || reference.asset.asset_id !== imported.id,
          ),
          {
            asset: { asset_kind: "generated_image" as const, asset_id: imported.id, image_path: imported.file_path },
            target_kind: target.targetKind,
            target_name: target.targetName,
            description: requirement?.description ?? (imported.prompt.trim() || t("comic.prompt.projectImage")),
            usage: requirement?.usage ?? t("comic.prompt.manualReferenceUsage"),
          },
        ],
      };
      await handleSaveStageArtifact("reference_discovery", pagePrompt.page_number, payload);
    } catch (error) {
      if (!isCurrentScope(requestScope)) return;
      throw error;
    } finally {
      referenceUploadBusyRef.current = false;
      if (isCurrentScope(requestScope)) setUploadingReferenceBindingKey(null);
    }
  };

  const handleUnbindDiscoveredReference = async (
    assetKind: MangaReferenceAssetKind,
    assetId: number,
  ) => {
    if (!ensurePagePromptsCurrent() || !canEditManga || !selectedChapter) return;
    const pagePrompt = pagePromptsRef.current.find(
      (prompt) => prompt.page_number === selectedPageNumber,
    );
    if (!pagePrompt) {
      addToast(t("comic.prompt.notFound"), "error");
      return;
    }
    const discovery = pagePrompt.reference_discovery;
    if (!discovery?.references.some(
      (reference) => reference.asset.asset_kind === assetKind && reference.asset.asset_id === assetId,
    )) return;
    await handleSaveStageArtifact("reference_discovery", pagePrompt.page_number, {
      ...discovery,
      references: discovery.references.filter(
        (reference) => reference.asset.asset_kind !== assetKind || reference.asset.asset_id !== assetId,
      ),
    });
  };

  const handleDeleteStageArtifact = async (
    stage: MangaPipelineArtifactStage,
    pageNumber: number,
  ) => {
    if (!ensurePagePromptsCurrent()) return;
    if (!canEditManga || !selectedChapter) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    if (selectedChapterGenerating || pipelineRunning) {
      addToast(t("comic.chapter.generatingDelete"), "error");
      return;
    }
    const actionSeq = beginAction(scopeKey);
    if (actionSeq == null) return;
    try {
      const preview = await previewDeleteMangaPipelineArtifact({
        project_id: projectId,
        chapter_id: selectedChapter.id,
        stage,
        page_number: pageNumber,
      });
      if (!isCurrentAction(scopeKey, actionSeq)) return;
      const downstream = mangaDeletionDownstreamSummary(preview, t);
      const message = downstream
        ? t("comic.prompt.deleteStageCascadeMessage", {
            page: pageNumber,
            stage: mangaStageLabel(stage, t),
            downstream,
          })
        : t("comic.prompt.deleteStageMessage", {
            page: pageNumber,
            stage: mangaStageLabel(stage, t),
          });
      const confirmed = await confirmDialog({
        title: t("comic.prompt.deleteStageTitle"),
        message,
        confirmText: t("common.delete"),
        dialogType: "danger",
      });
      if (!confirmed || !isCurrentAction(scopeKey, actionSeq)) return;
      if (!ensurePagePromptsCurrent()) return;
      const deletedArtifacts = [preview.target, ...preview.downstream];
      await deleteMangaPipelineArtifact({
        project_id: projectId,
        chapter_id: selectedChapter.id,
        stage,
        page_number: pageNumber,
        expected_artifact_ids: [
          preview.target.artifact_id,
          ...preview.downstream.map((artifact) => artifact.artifact_id),
        ],
      });
      if (!isCurrentAction(scopeKey, actionSeq)) return;
      if (stage === "drawing_prompt") onPagePromptDeleted(pageNumber);
      else onPageArtifactsDeleted(deletedArtifacts);
      void onPagePromptsChanged();
      addToast(t("comic.prompt.deleted"), "success");
    } catch (err) {
      if (isCurrentAction(scopeKey, actionSeq)) {
        addToast(extractErrorMessage(err, t("comic.prompt.deleteFailed")), "error");
      }
    } finally {
      finishAction(actionSeq);
    }
  };

  const handleGenerateStageArtifact = (
    stage: MangaPipelineArtifactStage,
    pageNumber: number,
  ) => {
    const pagePrompt = pagePromptsRef.current.find(
      (prompt) => prompt.page_number === pageNumber,
    );
    if (!pagePrompt || mangaPagePipelineStageHasOutput(pagePrompt, stage)) return;
    const stopAfter: Partial<Record<MangaPipelineArtifactStage, MangaPipelineStopAfter>> = {
      page_adaptation: "page_adaptation",
      storyboard: "storyboard",
      reference_discovery: "reference_discovery",
    };
    const target = stopAfter[stage];
    if (target) onRunPipeline("start", target, pageNumber, stage);
  };

  const handleDeleteOrphanedPage = async (pageNumber: number) => {
    if (!ensurePagePromptsCurrent()) return;
    if (!canEditManga || !selectedChapter) return;
    const requestProjectId = projectId;
    const requestChapterId = selectedChapter.id;
    const requestScope = scopeKey;
    const actionSeq = beginAction(requestScope);
    if (actionSeq == null) return;
    try {
      const confirmed = await confirmDialog({
        title: t("comic.prompt.deleteOrphanedPageTitle"),
        message: t("comic.prompt.deleteOrphanedPageMessage", { page: pageNumber }),
        confirmText: t("common.delete"),
        dialogType: "danger",
      });
      if (!confirmed || !isCurrentAction(requestScope, actionSeq)) return;
      if (!ensurePagePromptsCurrent()) return;
      await deleteMangaPage({
        project_id: requestProjectId,
        chapter_id: requestChapterId,
        page_number: pageNumber,
      });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      onPagePromptDeleted(pageNumber);
      void onPagePromptsChanged();
      addToast(t("comic.prompt.orphanedPageDeleted"), "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, t("comic.prompt.deleteFailed")), "error");
      }
    } finally {
      finishAction(actionSeq);
    }
  };

  const pipelineCanCancel = selectedStatus?.can_cancel === true;
  const pipelineCanResume = pipelineCanResumeForStage;
  const hasMangaResult = pagePrompts.length > 0;
  const pipelineTargetPages =
    pipelinePageNumber == null
      ? pagePrompts
      : pagePrompts.filter((prompt) => prompt.page_number === pipelinePageNumber);
  const hasIncompletePages = stageMode == null
    ? pagePrompts.some(
        (prompt) => prompt.pipeline_status != null && prompt.pipeline_status !== "completed",
      )
    : (stageMode === "page_planning" && pageSlotsNeedsGeneration) ||
      pipelineTargetPages.some((prompt) =>
        mangaPageStageNeedsGeneration(prompt, stageMode),
      );
  const mangaResultStale = stalePagePromptCount > 0;
  const pagePlanningHasStaleOutput = pageSlotsArtifactStale || pipelineTargetPages.some(
    (prompt) =>
      (mangaPagePipelineStageHasOutput(prompt, "page_content") &&
        prompt.stale_stages.includes("page_content")) ||
      (mangaPagePipelineStageHasOutput(prompt, "page_adaptation") &&
        prompt.stale_stages.includes("page_adaptation")),
  );
  const pipelineHasStaleOutput =
    mangaResultStale || pageSlotsArtifactStale || preprocessingArtifactStale;
  const pipelineFailed =
    pipelineStoppedWithError && !pipelineCanResume;
  const showPipelineDirectAction =
    stageMode !== "visual_design" &&
    stageMode !== "image_creation" &&
    (!hasMangaResult || pipelineCanResume || pipelineFailed || mangaResultStale || hasIncompletePages);
  const pipelineDirectVariant =
    mangaResultStale && !pipelineCanResume && !pipelineFailed
      ? "secondary"
      : "primary";
  const pipelineDirectLabel = pipelineActionTargetLabel
    ? pipelineCanResume || (hasIncompletePages && !pipelineFailed)
      ? t("comic.continueToStage", { stage: pipelineActionTargetLabel })
      : pipelineFailed
        ? t("comic.regenerateToStage", { stage: pipelineActionTargetLabel })
        : mangaResultStale
          ? t("comic.updateToStage", { stage: pipelineActionTargetLabel })
          : t("comic.generateToStage", { stage: pipelineActionTargetLabel })
    : pipelineCanResume
      ? t("comic.continueChapter")
      : hasIncompletePages && !pipelineFailed
        ? t("comic.continueChapter")
        : pipelineFailed
          ? t("comic.regenerateChapter")
          : mangaResultStale
            ? t("comic.updateChapter")
            : t("comic.generateChapter");
  const pipelinePreviewLabel = pipelineActionTargetLabel
    ? t("comic.prompt.viewStageRequest", { stage: pipelineActionTargetLabel })
    : pipelineCanResume
      ? t("comic.prompt.viewContinueRequest")
      : pipelineFailed
        ? t("comic.prompt.viewRegenerateRequest")
        : hasIncompletePages
          ? t("comic.prompt.viewContinueRequest")
          : mangaResultStale
            ? t("comic.prompt.viewUpdateRequest")
            : hasMangaResult
              ? t("comic.prompt.viewRegenerateRequest")
              : t("comic.prompt.viewGenerateRequest");
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
    selectedChapterGenerating ||
    (pageScopedStage && selectedPageNumber == null) ||
    (stageMode == null && pipelineHasStaleOutput) ||
    (stageMode === "page_planning" && pagePlanningHasStaleOutput);
  const pipelineMenuItems: DropdownItem[] = [];
  if (canEditManga && selectedChapter && !pipelineRunning && stageMode == null) {
    if (showPipelineDirectAction) {
      pipelineMenuItems.push({
        label: pipelinePreviewLabel,
        icon: <Eye size={12} />,
        disabled:
          pipelineMainDisabled || pipelineResumeBusy || pipelineStartBusy,
        onClick: () =>
          onPreviewPipeline(
            pipelineCanResume ? "resume" : "start",
            pipelineStopAfter,
            pipelinePageNumber,
            pipelineRunStage,
          ),
      });
    }
    if (missingPortraitCount > 0) {
      pipelineMenuItems.push({ type: "divider" });
      pipelineMenuItems.push({
        label: t("comic.prompt.fillPortraits", { count: missingPortraitCount }),
        icon: <ImagePlus size={12} />,
        disabled:
          detailBusy ||
          acting === "generate-missing-portraits" ||
          characterAssetsLoading ||
          characterAssetsError != null,
        onClick: () => onGenerateMissingPortraits?.(),
      });
    }
    if (
      !detailBusy &&
      acting !== `delete-${selectedChapter.id}` &&
      !selectedChapterGenerating
    ) {
      pipelineMenuItems.push({ type: "divider" });
      pipelineMenuItems.push({
        label: t("comic.prompt.deleteChapter"),
        icon: <Trash2 size={12} />,
        danger: true,
        onClick: onDeleteManga,
      });
    }
  }
  return (
    <>
      <section className="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
        {!selectedChapter ? (
          <div className="a4n-flat-surface flex min-h-[180px] flex-1 items-center justify-center">
            <EmptyHint
              text={isSingleMode
                ? t("comic.chapter.noSelectionSingle")
                : t("comic.chapter.noSelection")}
            />
          </div>
        ) : (
          <>
            <header className={`shrink-0 border-b border-book-border/55 ${stageMode === "image_creation" ? "pb-2 sm:pb-3" : "pb-3"}`}>
              <div className={`flex flex-wrap items-end ${stageMode === "image_creation" ? "gap-2 sm:gap-3" : "gap-3"}`}>
                <div className={`min-w-0 flex-1 ${stageMode === "image_creation" ? "basis-0 sm:basis-64" : "basis-64"}`}>
                  <h2 className="truncate text-sm font-semibold text-book-text-main">
                    {chapterHeading(selectedChapter, t)}
                  </h2>
                  <p className={`mt-1 text-small ${noticeClass[statusTone]}`}>
                    {[...chapterMeta, stageText].filter(Boolean).join(" · ")}
                  </p>
                </div>
                {mangaImageStatusMatchesWorkspaceStage(stageMode) && <div className={`max-w-full ${stageMode === "image_creation" ? "w-36 sm:w-44" : "w-44"}`}>
                  <ImageSizeSelect
                    label={t("settingsForm.selectImageSize")}
                    value={imageSize}
                    sizes={imageSizes}
                    loading={imageSizeLoading}
                    error={imageSizeError}
                    disabled={!canEditManga || detailBusy || selectedChapterGenerating}
                    onChange={onImageSizeChange}
                  />
                </div>}
                {canEditManga ? (
                  <div className="flex flex-wrap items-center justify-end gap-2">
                    {pipelineMenuItems.length > 0 ? (
                      <Dropdown items={pipelineMenuItems} />
                    ) : null}
                    {pipelineRunning && pipelineCanCancel ? (
                      <BookButton
                        size="sm"
                        variant="secondary"
                        onClick={onCancelPipeline}
                        disabled={
                          detailBusy ||
                          acting === `cancel-${selectedChapter.id}`
                        }
                      >
                        {acting === `cancel-${selectedChapter.id}` ? (
                          <LoadingSpinner size="sm" />
                        ) : (
                          <PauseCircle size={13} />
                        )}
                        {t("comic.stopGeneration")}
                      </BookButton>
                    ) : pipelineRunning ? (
                      <span role="status" className="inline-flex min-h-8 items-center gap-1.5 px-1 text-small text-book-text-muted">
                        <LoadingSpinner size="sm" />
                        {selectedStatus?.task_handle?.status === "cancelling"
                          ? t("comic.pipelineCancelRequested")
                          : t("comic.pipelineRunningHint")}
                      </span>
                    ) : pipelineStageActions.length > 0 ? (
                      pipelineStageActions.map((action) => {
                        const mode = stageMode === "visual_design"
                          ? "start"
                          : pipelineCanResume ? "resume" : "start";
                        const disabled =
                          pipelineMainDisabled ||
                          pipelineResumeBusy ||
                          pipelineStartBusy ||
                          !action.prerequisiteReady;
                        return (
                          <BookButton
                            key={action.stopAfter}
                            size="sm"
                            variant={action.runStage === "image" ? "primary" : "secondary"}
                            onClick={() =>
                              onRunPipeline(
                                mode,
                                action.stopAfter,
                                pipelinePageNumber,
                                action.runStage,
                              )
                            }
                            disabled={disabled}
                            title={disabled && !action.prerequisiteReady
                              ? t("comic.prompt.stageDependencyRequired", {
                                  stage: action.prerequisiteLabel,
                                })
                              : action.label}
                          >
                            {pipelineResumeBusy || pipelineStartBusy ? (
                              <LoadingSpinner size="sm" />
                            ) : action.runStage === "image" ? (
                              <ImagePlus size={13} />
                            ) : (
                              <Play size={13} />
                            )}
                            {action.label}
                          </BookButton>
                        );
                      })
                    ) : showPipelineDirectAction ? (
                      <BookButton
                        size="sm"
                        variant={pipelineDirectVariant}
                        onClick={() =>
                          onRunPipeline(
                            pipelineCanResume ? "resume" : "start",
                            pipelineStopAfter,
                            pipelinePageNumber,
                            pipelineRunStage,
                          )
                        }
                        disabled={
                          pipelineMainDisabled ||
                          pipelineResumeBusy ||
                          pipelineStartBusy
                        }
                      >
                        {pipelineResumeBusy || pipelineStartBusy ? (
                          <LoadingSpinner size="sm" />
                        ) : pipelineCanResume ||
                          pipelineFailed ||
                          mangaResultStale ? (
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

              {notice ? (
                <div
                  role={selectedChapterLoading ? "status" : selectedChapterLoadError ? "alert" : undefined}
                  className={`flex items-start gap-2 rounded-xl border bg-book-bg-paper/50 px-2.5 text-xs ${stageMode === "image_creation" ? "mt-2 py-1.5 sm:mt-3 sm:py-2" : "mt-3 py-2"} ${noticeClass[notice.tone]}`}
                >
                  <span className="mt-0.5 shrink-0">
                    {noticeIcon(notice.tone)}
                  </span>
                  <span className="min-w-0 flex-1 whitespace-pre-wrap break-words leading-relaxed">
                    {notice.text}
                  </span>
                  {selectedChapterLoadError ? (
                    <BookButton variant="secondary" size="sm" onClick={onRetryChapter}>
                      <RefreshCw size={12} /> {t("common.retry")}
                    </BookButton>
                  ) : null}
                </div>
              ) : null}
            </header>

            <div className="mt-3 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
              <MangaPagePromptList
                stageMode={stageMode}
                onDirtyChange={onDirtyChange}
                prompts={pagePrompts}
                paginationActions={paginationActions}
                selectedPageNumber={selectedPageNumber}
                onSelectedPageChange={onSelectedPageChange}
                promptsLoading={pagePromptsLoading}
                pagePromptsError={pagePromptsError}
                currentContentHash={currentContentHash}
                chapterLocked={
                  selectedChapterGenerating || pipelineRunning || !selectedChapterReady
                }
                generatingPageNumber={generatingPageNumber}
                profiles={profiles}
                portraits={portraits}
                projectImages={projectImages}
                assetsLoading={characterAssetsLoading || projectImagesLoading}
                pipelineStatus={selectedStatus}
                optimizingPromptId={optimizingPromptId}
                actionsDisabled={detailBusy}
                canEditManga={canEditManga}
                onPreviewPageImage={
                  stageMode == null || stageMode === "image_creation"
                    ? handlePreviewPageImage
                    : undefined
                }
                onGeneratePageImage={
                  stageMode == null || stageMode === "image_creation"
                    ? handleGeneratePageImage
                    : undefined
                }
                onBindReferenceImage={mangaStageAllowsReferenceBinding(stageMode) ? handleBindReferenceImage : undefined}
                onBindDiscoveredReference={
                  mangaStageAllowsReferenceBinding(stageMode)
                    ? handleBindDiscoveredReference
                    : undefined
                }
                onUnbindDiscoveredReference={
                  mangaStageAllowsReferenceBinding(stageMode)
                    ? handleUnbindDiscoveredReference
                    : undefined
                }
                onUploadReferenceImage={
                  mangaStageAllowsReferenceBinding(stageMode)
                    ? handleUploadReferenceImage
                    : undefined
                }
                uploadingReferenceBindingKey={uploadingReferenceBindingKey}
                onUpdatePrompt={handleUpdatePrompt}
                onSaveStageArtifact={handleSaveStageArtifact}
                onGenerateStageArtifact={
                  stageMode === "visual_design" ? undefined : handleGenerateStageArtifact
                }
                onDeleteStageArtifact={handleDeleteStageArtifact}
                onDeleteOrphanedPage={handleDeleteOrphanedPage}
                onOptimizePrompt={handleOptimizePrompt}
                onDeletePrompt={handleDeletePrompt}
                onRetryPagePrompts={onRetryPagePrompts}
              />
            </div>
          </>
        )}
      </section>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("comic.image.requestPreview")}
        onClose={closeRequestPreview}
      />
    </>
  );
}
