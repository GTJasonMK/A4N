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
  MangaPipelineArtifactStage,
  MangaPipelineStopAfter,
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
import { MAX_UPLOAD_IMAGE_BYTES } from "../../../api/generated/config";
import { isSupportedImageMime } from "../../../utils/imageUpload";
import {
  chapterHeading,
  countStaleMangaPagePrompts,
  mangaDeletionDownstreamSummary,
  mangaPageImageBlockReason,
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
  selectedSnapshotLoading: boolean;
  selectedStatus: MangaPipelineStatusSummary | null | undefined;
  pagePrompts: MangaPagePromptSummary[];
  pageSlotsNeedsGeneration?: boolean;
  paginationActions?: React.ReactNode;
  selectedPageNumber: number | null;
  onSelectedPageChange: (pageNumber: number | null) => void;
  pagePromptsLoading: boolean;
  pagePromptsError: string | null;
  currentContentHash: string | null;
  characterAssetsLoading: boolean;
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
  onProjectImageImported: (image: GeneratedImage) => void;
  onRetryPagePrompts: () => void;
}

type NoticeTone = "ok" | "warning" | "danger" | "muted";
type InlineNoticeTone = "warning" | "muted";

const noticeClass: Record<NoticeTone, string> = {
  ok: "border-success/40 text-success",
  warning: "border-warning/50 text-warning",
  danger: "border-error/50 text-error",
  muted: "border-book-border/50 text-book-text-sub",
};

const noticeIcon = (tone: InlineNoticeTone) => {
  if (tone === "warning") return <AlertTriangle size={13} />;
  return <CircleDashed size={13} />;
};

export function MangaChapterDetailPanel({
  stageMode,
  onDirtyChange,
  projectId,
  isSingleMode,
  selectedChapter,
  selectedSnapshotLoading,
  selectedStatus,
  pagePrompts,
  pageSlotsNeedsGeneration = false,
  paginationActions,
  selectedPageNumber,
  onSelectedPageChange,
  pagePromptsLoading,
  pagePromptsError,
  currentContentHash,
  characterAssetsLoading,
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
  const selectedChapterReady = Boolean(selectedChapter?.content?.trim());
  const selectedChapterGenerating = isChapterGenerating(selectedChapter);
  const detailBusy =
    promptActionBusy ||
    requestPreviewLoading ||
    uploadingReferenceBindingKey != null;

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
  const pipelineTarget = stageMode
    ? mangaPipelineTargetForWorkspaceStage(stageMode, selectedPagePrompt)
    : null;
  const pipelineRestartTarget = stageMode
    ? mangaPipelineTargetForWorkspaceStage(stageMode, selectedPagePrompt, true)
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
  const pipelineStageActions = stageMode
    ? mangaPipelineActionsForWorkspaceStage(stageMode).map((action) => ({
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
    if (selectedChapterGenerating) {
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
      if (!await onPagePromptsChanged()) return;
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
    if (selectedChapterGenerating) {
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
  ) => {
    if (!canEditManga) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    if (selectedChapterGenerating) {
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
      await bindMangaReferenceImage(input);
      if (!isCurrentAction(requestScope, actionSeq)) return;
      if (!await onPagePromptsChanged()) return;
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
      finishAction(actionSeq);
    }
  };

  const handleUploadReferenceImage = async (
    promptId: number,
    characterName: string,
    file: File,
  ): Promise<void> => {
    if (!canEditManga || !selectedChapter) return;
    if (referenceUploadBusyRef.current) return;
    if (selectedChapterGenerating) {
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
    if (!canEditManga) {
      const message = t("comic.noEditPermission");
      addToast(message, "error");
      throw new Error(message);
    }
    if (selectedChapterGenerating) {
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
      await updateMangaPrompt({
        prompt_id: promptId,
        image_prompt: imagePrompt,
      });
      if (!isCurrentAction(requestScope, actionSeq))
        throw new Error("任务已停止");
      if (!await onPagePromptsChanged()) {
        throw new Error(t("comic.loadPagePromptsFailed"));
      }
      if (!isCurrentAction(requestScope, actionSeq)) throw new Error("任务已停止");
      addToast(t("comic.prompt.saved"), "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
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
    if (!canEditManga) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    if (selectedChapterGenerating) {
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
      const currentPrompt =
        pagePromptsRef.current.find((current) => current.id === requestPrompt.id) ??
        null;
      if (!currentPrompt) {
        addToast(t("comic.prompt.notFound"), "error");
        return;
      }
      setOptimizingPromptId(requestPrompt.id);
      await optimizeMangaPrompt({
        prompt_id: requestPrompt.id,
        instruction: trimmed,
      });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      if (!await onPagePromptsChanged()) return;
      if (!isCurrentAction(requestScope, actionSeq)) return;
      addToast(t("comic.prompt.optimized"), "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
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
    if (selectedChapterGenerating) {
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
    if (!canEditManga || !selectedChapter) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    if (selectedChapterGenerating) {
      addToast(t("comic.chapter.generatingUpdate"), "error");
      return;
    }
    const actionSeq = beginAction(scopeKey);
    if (actionSeq == null) return;
    try {
      await saveMangaPipelineArtifact({
        project_id: projectId,
        chapter_id: selectedChapter.id,
        stage,
        page_number: pageNumber,
        payload,
      });
      if (!isCurrentAction(scopeKey, actionSeq)) return;
      if (!await onPagePromptsChanged()) return;
      addToast(t("comic.prompt.saved"), "success");
    } catch (err) {
      if (isCurrentAction(scopeKey, actionSeq)) {
        addToast(extractErrorMessage(err, t("comic.prompt.saveFailed")), "error");
      }
      throw err;
    } finally {
      finishAction(actionSeq);
    }
  };

  const handleDeleteStageArtifact = async (
    stage: MangaPipelineArtifactStage,
    pageNumber: number,
  ) => {
    if (!canEditManga || !selectedChapter) {
      addToast(t("comic.noEditPermission"), "error");
      return;
    }
    if (selectedChapterGenerating) {
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
      if (!await onPagePromptsChanged()) return;
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
    const stopAfter: Partial<Record<MangaPipelineArtifactStage, MangaPipelineStopAfter>> = {
      page_adaptation: "page_adaptation",
      storyboard: "storyboard",
      reference_discovery: "reference_discovery",
    };
    const target = stopAfter[stage];
    if (target) onRunPipeline("start", target, pageNumber, stage);
  };

  const handleDeleteOrphanedPage = async (pageNumber: number) => {
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
      await deleteMangaPage({
        project_id: requestProjectId,
        chapter_id: requestChapterId,
        page_number: pageNumber,
      });
      if (!isCurrentAction(requestScope, actionSeq)) return;
      if (!await onPagePromptsChanged() || !isCurrentAction(requestScope, actionSeq)) return;
      addToast(t("comic.prompt.orphanedPageDeleted"), "success");
    } catch (err) {
      if (isCurrentAction(requestScope, actionSeq)) {
        addToast(extractErrorMessage(err, t("comic.prompt.deleteFailed")), "error");
      }
    } finally {
      finishAction(actionSeq);
    }
  };

  const pipelineRunning =
    selectedStatus?.pipeline_status === "running" || selectedStatus?.can_cancel === true;
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
  const pipelineFailed =
    pipelineStoppedWithError && !pipelineCanResume;
  const showPipelineDirectAction =
    !hasMangaResult || pipelineCanResume || pipelineFailed || mangaResultStale || hasIncompletePages;
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
    (pageScopedStage && selectedPageNumber == null);
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
    if (
      pipelineCanResume ||
      (hasMangaResult && !mangaResultStale && !pipelineFailed)
    ) {
      pipelineMenuItems.push({
        label: t("comic.prompt.fromStart"),
        icon: <Play size={12} />,
        disabled: pipelineMainDisabled || pipelineStartBusy,
        onClick: () =>
          onRunPipeline(
            "start",
            pipelineRestartTarget?.stopAfter ?? pipelineStopAfter,
            pipelinePageNumber,
            pipelineRestartTarget?.runStage ?? pipelineRunStage,
          ),
      });
      pipelineMenuItems.push({
        label: t("comic.prompt.fromStartRequest"),
        icon: <Eye size={12} />,
        disabled: pipelineMainDisabled || pipelineStartBusy,
        onClick: () =>
          onPreviewPipeline(
            "start",
            pipelineRestartTarget?.stopAfter ?? pipelineStopAfter,
            pipelinePageNumber,
            pipelineRestartTarget?.runStage ?? pipelineRunStage,
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
          characterAssetsLoading,
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
        {selectedSnapshotLoading ? (
          <div role="status" className="a4n-flat-surface flex min-h-[180px] flex-1 items-center justify-center gap-2 text-xs text-book-text-muted">
            <LoadingSpinner size="sm" /> {t("comic.loadingChapters")}
          </div>
        ) : !selectedChapter ? (
          <div className="a4n-flat-surface flex min-h-[180px] flex-1 items-center justify-center">
            <EmptyHint
              text={isSingleMode
                ? t("comic.chapter.noSelectionSingle")
                : t("comic.chapter.noSelection")}
            />
          </div>
        ) : (
          <>
            <header className="shrink-0 border-b border-book-border/55 pb-3">
              <div className="flex flex-wrap items-end gap-3">
                <div className="min-w-0 flex-1 basis-64">
                  <h2 className="truncate text-sm font-semibold text-book-text-main">
                    {chapterHeading(selectedChapter, t)}
                  </h2>
                  <p className={`mt-1 text-small ${noticeClass[statusTone]}`}>
                    {[...chapterMeta, stageText].filter(Boolean).join(" · ")}
                  </p>
                </div>
                {mangaImageStatusMatchesWorkspaceStage(stageMode) && <div className="w-44 max-w-full">
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
                        const disabled =
                          pipelineMainDisabled ||
                          pipelineResumeBusy ||
                          pipelineStartBusy ||
                          !action.prerequisiteReady;
                        return (
                          <BookButton
                            key={action.runStage}
                            size="sm"
                            variant={action.runStage === "image" ? "primary" : "secondary"}
                            onClick={() =>
                              onRunPipeline(
                                pipelineCanResume ? "resume" : "start",
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
                  className={`mt-3 flex items-start gap-2 rounded-xl border bg-book-bg-paper/50 px-2.5 py-2 text-xs ${noticeClass[notice.tone]}`}
                >
                  <span className="mt-0.5 shrink-0">
                    {noticeIcon(notice.tone)}
                  </span>
                  <span className="min-w-0 flex-1 whitespace-pre-wrap break-words leading-relaxed">
                    {notice.text}
                  </span>
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
                  selectedChapterGenerating || !selectedChapterReady
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
                onUploadReferenceImage={
                  mangaStageAllowsReferenceBinding(stageMode)
                    ? handleUploadReferenceImage
                    : undefined
                }
                uploadingReferenceBindingKey={uploadingReferenceBindingKey}
                onUpdatePrompt={handleUpdatePrompt}
                onSaveStageArtifact={handleSaveStageArtifact}
                onGenerateStageArtifact={handleGenerateStageArtifact}
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
