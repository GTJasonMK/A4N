import React, {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Clapperboard, FileText, ImagePlus, Images, PanelTop, RefreshCw } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../../api";
import { listChapters } from "../../api/chapter";
import { listProfiles, type CharacterProfile } from "../../api/character";
import {
  listImagesByProject,
  listImagesByProjects,
  type GeneratedImage,
} from "../../api/image";
import {
  listPortraitsByProject,
  startPortraitGeneration,
  verifyPortraitGenerationResult,
  type Portrait,
} from "../../api/portrait";
import { runGenerationProgressTaskThen } from "../../hooks/runGenerationProgressTask";
import { generationClient } from "../../api/generationTasks";
import type { GenerationTaskHandle } from "../../api/generated/generationContracts";
import {
  deleteMangaByChapter,
  getMangaPipelineStatus,
  listMangaPagePrompts,
  listMangaPipelineStatuses,
  previewResumeMangaPipelineRequest,
  previewStartMangaPipelineRequest,
  prepareStandaloneComicSource,
  resumeMangaPipeline,
  startMangaPipeline,
  type MangaPipelineStatusSummary,
  type MangaPagePromptSummary,
} from "../../api/manga";
import type { ModelRequestPreview } from "../../api/requestPreview";
import {
  chapterGeneratingMessage,
  type ChapterRow,
  isChapterGenerating,
} from "./shared";
import { sha256Hex } from "../../utils/contentHash";
import { isCancelledOperation } from "../../utils/cancellation";
import { MangaChapterDetailPanel } from "./manga/MangaChapterDetailPanel";
import { MangaChapterList } from "./manga/MangaChapterList";
import { useProgressTask } from "../../hooks/useProgressTask";
import { usePersistedTab } from "../../hooks/usePersistedTab";
import { useNovelDetailDirty } from "./novelDetailDirty";
import { MediaStageHeader } from "../../components/shell/MediaStageHeader";
import { confirmDiscardMediaChanges } from "../../components/feedback/UnsavedChangesGuard";
import { BookButton } from "../../components/ui/BookButton";
import { MediaAuthoringPanel } from "../derived/MediaAuthoringPanel";
import { useMediaAuthoring, authoringStageLabel } from "../derived/useMediaAuthoring";
import { BaseMaterialsWorkspace } from "../derived/BaseMaterialsWorkspace";
import type { MangaPipelineStopAfter } from "../../api/generated/host";
import { useI18n } from "../../i18n";
import { useImageGenerationSize } from "../../components/image/ImageSizeSelect";
import { explainMangaPipelineAiConfigError } from "./manga/mangaPipelineErrors";
import {
  mangaPipelineConfigBlockReason,
  type MangaTabProps,
} from "./manga/mangaPipelineProgress";
import {
  countStaleMangaPagePrompts,
  type MangaPageWorkspaceStage,
} from "./manga/mangaTabUtils";

type MangaTextSourceSelection = import("../derived/ProjectTextMaterialsPanel").ProjectTextSourceSelection;

type MangaPipelineStatusLoadError =
  | { scope: "project"; message: string }
  | { scope: "chapter"; chapterId: number; message: string };

function readMangaTextSource(projectId: number): MangaTextSourceSelection | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(`a4n:comic:${projectId}:source`) ?? "null");
    if (!value || typeof value !== "object") return null;
    const source = value as Partial<MangaTextSourceSelection>;
    if (
      !Number.isInteger(source.assetId) || source.assetId! <= 0 ||
      !Number.isInteger(source.version) || source.version! <= 0 ||
      !Number.isInteger(source.startSection) || source.startSection! < 0 ||
      !Number.isInteger(source.endSection) || source.endSection! < source.startSection!
    ) return null;
    return source as MangaTextSourceSelection;
  } catch {
    return null;
  }
}

export const MangaTab: React.FC<MangaTabProps> = ({
  projectId,
  canEditManga = true,
  snapshotChapters,
  assetProjectId,
  imageProjectIds,
  standalone = false,
  fixedChapter,
  onDirtyChange,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const staged = snapshotChapters !== undefined || standalone;
  const [authoringDirty, setAuthoringDirty] = useState(false);
  const [textMaterialsDirty, setTextMaterialsDirty] = useState(false);
  const [selectedTextSource, setSelectedTextSource] = useState<MangaTextSourceSelection | null>(
    () => standalone ? readMangaTextSource(projectId) : null,
  );
  const [authoringBusy, setAuthoringBusy] = useState(false);
  const [standaloneSourceLoading, setStandaloneSourceLoading] = useState(false);
  const [promptDirty, setPromptDirty] = useState(false);
  const [discardVersion, setDiscardVersion] = useState(0);
  const discardStageDrafts = () => {
    setAuthoringDirty(false);
    setTextMaterialsDirty(false);
    setPromptDirty(false);
    setDiscardVersion((version) => version + 1);
  };
  const isMutationBusy = () =>
    actionBusyKeyRef.current != null ||
    pipelineChapterId != null ||
    progressCancelBusyKeyRef.current != null ||
    authoringBusy ||
    standaloneSourceLoading;
  useEffect(() => { onDirtyChange?.(authoringDirty || promptDirty || textMaterialsDirty); }, [authoringDirty, promptDirty, textMaterialsDirty, onDirtyChange]);
  useEffect(() => {
    if (!standalone) return;
    try {
      const key = `a4n:comic:${projectId}:source`;
      if (selectedTextSource) localStorage.setItem(key, JSON.stringify(selectedTextSource));
      else localStorage.removeItem(key);
    } catch {
      addToast(t("comic.sourceSelectionSaveFailed"), "error");
    }
  }, [addToast, projectId, selectedTextSource, standalone, t]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  const discardMangaDrafts = () => {
    if (isMutationBusy() || acting != null) return false;
    discardStageDrafts();
  };
  useNovelDetailDirty(`manga-editor:${projectId}`, authoringDirty || promptDirty || textMaterialsDirty, discardMangaDrafts, () => !isMutationBusy() && acting == null);
  const stageId = `comic-stages-${projectId}`;
  const confirmLeaveStage = async () => {
    if (isMutationBusy()) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    if (!authoringDirty && !promptDirty && !textMaterialsDirty) return true;
    const confirmed = await confirmDiscardMediaChanges(true, t);
    if (confirmed) { setAuthoringDirty(false); setTextMaterialsDirty(false); setPromptDirty(false); setDiscardVersion((version) => version + 1); }
    return confirmed;
  };
  const { runProgressTask } = useProgressTask();
  const {
    error: imageSizeError,
    loading: imageSizeLoading,
    size: imageSize,
    sizes: imageSizes,
    setSize: setImageSize,
  } = useImageGenerationSize();
  const hasFixedChapterContext = fixedChapter !== undefined;
  const [chapters, setChapters] = useState<ChapterRow[]>([]);
  const [statuses, setStatuses] = useState<
    Record<number, MangaPipelineStatusSummary | null>
  >({});
  const [pipelineStatusError, setPipelineStatusError] =
    useState<MangaPipelineStatusLoadError | null>(null);
  const [loading, setLoading] = useState(true);
  const [chapterLoadError, setChapterLoadError] = useState<string | null>(null);
  const [acting, setActing] = useState<string | null>(null);
  const [selectedChapter, setSelectedChapter] = useState<ChapterRow | null>(
    fixedChapter ?? null,
  );
  const authoring = useMediaAuthoring(
    projectId,
    staged && (!standalone || selectedChapter != null),
    selectedChapter?.id ?? null,
  );
  const outputState = authoring.workspace?.artifacts.find((artifact) => artifact.kind === "manga_pages" && artifact.source_chapter_id === selectedChapter?.id);
  const [pagePrompts, setPagePrompts] = useState<MangaPagePromptSummary[]>([]);
  const [pagePromptsLoading, setPagePromptsLoading] = useState(false);
  const [pagePromptsError, setPagePromptsError] = useState<string | null>(null);
  const [selectedPageNumber, setSelectedPageNumber] = useState<number | null>(null);
  const [currentContentHash, setCurrentContentHash] = useState<string | null>(
    null,
  );
  const stageTabs = useMemo(() => {
    const stageProgress = (ready: number) =>
      pagePrompts.length > 0
        ? t("comic.prompt.stageProgress", {
            ready,
            total: pagePrompts.length,
          })
        : undefined;
    const pagePromptsStale = countStaleMangaPagePrompts(pagePrompts, currentContentHash) > 0;
    return [
      { id: "materials" as const, label: t("common.mediaAuthoring.materials"), icon: <ImagePlus size={13} /> },
      {
        id: "adaptation" as const,
        label: t("common.mediaAuthoring.adaptation"),
        icon: <FileText size={13} />,
        status: authoringStageLabel(authoring.workspace, "adaptation_plan", selectedChapter?.id, t),
      },
      {
        id: "page_plan" as const,
        label: t("comic.pipelineStages.pagination"),
        icon: <PanelTop size={13} />,
        status: stageProgress(pagePrompts.filter((page) => page.page_content != null).length),
      },
      {
        id: "page_adaptation" as const,
        label: t("comic.pipelineStages.pageAdaptation"),
        icon: <FileText size={13} />,
        status: stageProgress(pagePrompts.filter((page) => page.page_adaptation != null).length),
      },
      {
        id: "storyboard" as const,
        label: t("comic.pipelineStages.storyboard"),
        icon: <Clapperboard size={13} />,
        status: stageProgress(pagePrompts.filter((page) => page.page_plan != null).length),
      },
      {
        id: "prompts" as const,
        label: t("common.mediaAuthoring.pagePrompts"),
        icon: <FileText size={13} />,
        status: outputState?.stale || pagePromptsStale
          ? t("common.mediaAuthoring.needsUpdate")
          : stageProgress(pagePrompts.filter((page) => page.drawing_prompt != null).length),
      },
      {
        id: "images" as const,
        label: t("common.mediaAuthoring.images"),
        icon: <Images size={13} />,
        status: outputState?.stale || pagePromptsStale
          ? t("common.mediaAuthoring.needsUpdate")
          : stageProgress(pagePrompts.filter((page) => page.generated_image_id != null).length),
      },
    ];
  }, [authoring.workspace, currentContentHash, pagePrompts, selectedChapter?.id, t]);
  const [workspaceStage, setWorkspaceStage] = usePersistedTab(
    staged ? `a4n:comic:${projectId}:stage` : null,
    staged ? "materials" : "prompts",
    stageTabs.map((tab) => tab.id),
  );
  const activePageStage: MangaPageWorkspaceStage | undefined =
    staged && workspaceStage !== "materials" && workspaceStage !== "adaptation"
      ? workspaceStage
      : undefined;
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [portraits, setPortraits] = useState<Portrait[]>([]);
  const [projectImages, setProjectImages] = useState<GeneratedImage[]>([]);
  const [projectImagesLoading, setProjectImagesLoading] = useState(false);
  const [projectImagesError, setProjectImagesError] = useState<string | null>(null);
  const [characterAssetsLoading, setCharacterAssetsLoading] = useState(false);
  const [characterAssetsError, setCharacterAssetsError] = useState<string | null>(null);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [pipelineChapterId, setPipelineChapterId] = useState<number | null>(
    null,
  );
  const selectedChapterIdRef = useRef<number | null>(fixedChapter?.id ?? null);

  const isSingleMode = hasFixedChapterContext;
  const showChapterNavigation = !isSingleMode && chapters.length > 1;
  const characterAssetProjectId = assetProjectId ?? projectId;
  const imageSourceProjectIds = useMemo(
    () => (imageProjectIds?.length ? imageProjectIds : [projectId]),
    [imageProjectIds, projectId],
  );
  const imageSourceKey = useMemo(
    () => imageSourceProjectIds.join(","),
    [imageSourceProjectIds],
  );
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const activeCharacterAssetProjectIdRef = useRef(characterAssetProjectId);
  const activeImageSourceKeyRef = useRef(imageSourceKey);
  const standaloneSourceLoadSeqRef = useRef(0);
  const chapterLoadSeqRef = useRef(0);
  const statusLoadSeqRef = useRef(0);
  const pagePromptsLoadSeqRef = useRef(0);
  const characterAssetsLoadSeqRef = useRef(0);
  const projectImagesLoadSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const actionBusyKeyRef = useRef<string | null>(null);
  const previewBusyKeyRef = useRef<string | null>(null);
  const progressCancelBusyKeyRef = useRef<string | null>(null);

  activeProjectIdRef.current = projectId;
  activeCharacterAssetProjectIdRef.current = characterAssetProjectId;
  activeImageSourceKeyRef.current = imageSourceKey;

  const isProjectCurrent = useCallback(
    (requestProjectId: number) =>
      mountedRef.current && activeProjectIdRef.current === requestProjectId,
    [],
  );

  const ensureCanEditManga = useCallback(() => {
    if (canEditManga) return true;
    addToast(t("comic.noEditPermission"), "error");
    return false;
  }, [addToast, canEditManga, t]);

  const isCharacterAssetsCurrent = useCallback(
    (
      requestProjectId: number,
      requestAssetProjectId: number,
      requestSeq: number,
    ) =>
      isProjectCurrent(requestProjectId) &&
      activeCharacterAssetProjectIdRef.current === requestAssetProjectId &&
      characterAssetsLoadSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isProjectImagesCurrent = useCallback(
    (
      requestProjectId: number,
      requestImageSourceKey: string,
      requestSeq: number,
    ) =>
      isProjectCurrent(requestProjectId) &&
      activeImageSourceKeyRef.current === requestImageSourceKey &&
      projectImagesLoadSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isChapterCurrent = useCallback(
    (requestProjectId: number, requestChapterId: number) =>
      isProjectCurrent(requestProjectId) &&
      selectedChapterIdRef.current === requestChapterId,
    [isProjectCurrent],
  );

  const isActionCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && actionSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isPreviewCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) &&
      previewSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const beginAction = useCallback((key: string): boolean => {
    if (actionBusyKeyRef.current) return false;
    actionBusyKeyRef.current = key;
    return true;
  }, []);

  const endAction = useCallback((key: string) => {
    if (actionBusyKeyRef.current === key) actionBusyKeyRef.current = null;
  }, []);

  const beginPreview = useCallback((key: string): boolean => {
    if (previewBusyKeyRef.current) return false;
    previewBusyKeyRef.current = key;
    return true;
  }, []);

  const endPreview = useCallback((key: string) => {
    if (previewBusyKeyRef.current === key) previewBusyKeyRef.current = null;
  }, []);

  const resetInteractionLocks = useCallback(() => {
    actionBusyKeyRef.current = null;
    previewBusyKeyRef.current = null;
    progressCancelBusyKeyRef.current = null;
  }, []);

  const resetRequestPreview = useCallback(() => {
    previewSeqRef.current += 1;
    previewBusyKeyRef.current = null;
    setRequestPreviewOpen(false);
    setRequestPreviewLoading(false);
    setRequestPreviewData(null);
  }, []);

  const primaryPortraitCount = useMemo(
    () => portraits.filter((portrait) => portrait.is_primary).length,
    [portraits],
  );
  const missingPortraitCount = useMemo(() => {
    const profileIdsWithPortrait = new Set(
      portraits.map((portrait) => portrait.profile_id),
    );
    return profiles.filter((profile) => !profileIdsWithPortrait.has(profile.id))
      .length;
  }, [profiles, portraits]);

  useEffect(() => {
    selectedChapterIdRef.current = selectedChapter?.id ?? null;
  }, [selectedChapter?.id]);

  useEffect(() => {
    setSelectedPageNumber(null);
  }, [selectedChapter?.id]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      chapterLoadSeqRef.current += 1;
      statusLoadSeqRef.current += 1;
      pagePromptsLoadSeqRef.current += 1;
      characterAssetsLoadSeqRef.current += 1;
      projectImagesLoadSeqRef.current += 1;
      actionSeqRef.current += 1;
      previewSeqRef.current += 1;
      resetInteractionLocks();
    };
  }, [resetInteractionLocks]);

  const refreshPipelineTarget = async (
    chapterId: number,
    requestProjectId = projectId,
  ): Promise<void> => {
    if (!isProjectCurrent(requestProjectId)) return;
    await Promise.all([
      loadSingleStatus(chapterId, requestProjectId),
      isChapterCurrent(requestProjectId, chapterId)
        ? loadPagePrompts(chapterId, requestProjectId, { showLoading: false })
        : Promise.resolve(),
    ]);
  };

  const loadCharacterAssets = async (
    requestProjectId = projectId,
    requestAssetProjectId = characterAssetProjectId,
  ): Promise<boolean> => {
    const requestSeq = characterAssetsLoadSeqRef.current + 1;
    characterAssetsLoadSeqRef.current = requestSeq;
    if (
      isCharacterAssetsCurrent(
        requestProjectId,
        requestAssetProjectId,
        requestSeq,
      )
    ) {
      setCharacterAssetsLoading(true);
      setCharacterAssetsError(null);
    }
    try {
      const [profilesResult, portraitsResult] = await Promise.allSettled([
        listProfiles(requestAssetProjectId),
        listPortraitsByProject(requestAssetProjectId),
      ]);
      if (
        !isCharacterAssetsCurrent(
          requestProjectId,
          requestAssetProjectId,
          requestSeq,
        )
      )
        return false;
      const assetErrors: string[] = [];
      if (profilesResult.status === "fulfilled") {
        setProfiles(profilesResult.value);
      } else {
        const message = extractErrorMessage(profilesResult.reason, t("comic.loadCharacterAssetsFailed"));
        assetErrors.push(message);
      }
      if (portraitsResult.status === "fulfilled") {
        setPortraits(portraitsResult.value);
      } else {
        const message = extractErrorMessage(portraitsResult.reason, t("comic.loadCharacterAssetsFailed"));
        assetErrors.push(message);
      }
      setCharacterAssetsError(assetErrors.length > 0 ? assetErrors.join(" · ") : null);
      return profilesResult.status === "fulfilled" && portraitsResult.status === "fulfilled";
    } finally {
      if (
        isCharacterAssetsCurrent(
          requestProjectId,
          requestAssetProjectId,
          requestSeq,
        )
      ) {
        setCharacterAssetsLoading(false);
      }
    }
  };

  const loadProjectImages = async (
    requestProjectId = projectId,
    requestImageSourceProjectIds = imageSourceProjectIds,
    requestImageSourceKey = imageSourceKey,
  ): Promise<boolean> => {
    const requestSeq = projectImagesLoadSeqRef.current + 1;
    projectImagesLoadSeqRef.current = requestSeq;
    if (
      isProjectImagesCurrent(
        requestProjectId,
        requestImageSourceKey,
        requestSeq,
      )
    ) {
      setProjectImagesLoading(true);
      setProjectImagesError(null);
    }
    try {
      const images =
        requestImageSourceProjectIds.length === 1
          ? await listImagesByProject(requestImageSourceProjectIds[0])
          : await listImagesByProjects(requestImageSourceProjectIds);
      if (
        !isProjectImagesCurrent(
          requestProjectId,
          requestImageSourceKey,
          requestSeq,
        )
      )
        return false;
      const byId = new Map<number, GeneratedImage>();
      for (const image of images) byId.set(image.id, image);
      setProjectImages([...byId.values()]);
      setProjectImagesError(null);
      return true;
    } catch (err) {
      if (
        isProjectImagesCurrent(
          requestProjectId,
          requestImageSourceKey,
          requestSeq,
        )
      ) {
        const message = extractErrorMessage(err, t("comic.loadProjectImagesFailed"));
        setProjectImagesError(message);
      }
      return false;
    } finally {
      if (
        isProjectImagesCurrent(
          requestProjectId,
          requestImageSourceKey,
          requestSeq,
        )
      ) {
        setProjectImagesLoading(false);
      }
    }
  };

  const reloadMangaAssets = async () => {
    const [characterAssetsOk, projectImagesOk] = await Promise.all([
      loadCharacterAssets(),
      loadProjectImages(),
    ]);
    return characterAssetsOk && projectImagesOk;
  };

  const resetChapterDetail = () => {
    selectedChapterIdRef.current = null;
    pagePromptsLoadSeqRef.current += 1;
    statusLoadSeqRef.current += 1;
    resetRequestPreview();
    setSelectedChapter(null);
    setPagePrompts([]);
    setPagePromptsError(null);
    setPipelineStatusError((current) => current?.scope === "chapter" ? null : current);
    setCurrentContentHash(null);
  };

  const openChapterDetail = (row: ChapterRow) => {
    selectedChapterIdRef.current = row.id;
    resetRequestPreview();
    setSelectedChapter(row);
    setPagePrompts([]);
    setPagePromptsError(null);
    setPipelineStatusError((current) => current?.scope === "chapter" ? null : current);
    setCurrentContentHash(row.content_hash ?? null);
    void loadPagePrompts(row.id, projectId);
    void loadSingleStatus(row.id, projectId);
  };

  const prepareStandaloneSource = async (
    selection: MangaTextSourceSelection,
    requestProjectId = projectId,
    showLoading = false,
  ): Promise<boolean> => {
    const requestSeq = standaloneSourceLoadSeqRef.current + 1;
    standaloneSourceLoadSeqRef.current = requestSeq;
    setStandaloneSourceLoading(true);
    if (showLoading) setLoading(true);
    resetChapterDetail();
    setChapters([]);
    setChapterLoadError(null);
    setStatuses({});
    try {
      const source = await prepareStandaloneComicSource(
        requestProjectId,
        selection.assetId,
        selection.version,
        selection.startSection,
        selection.endSection,
      );
      if (!isProjectCurrent(requestProjectId) || standaloneSourceLoadSeqRef.current !== requestSeq) {
        return false;
      }
      const row: ChapterRow = {
        id: source.chapter_id,
        chapter_number: source.chapter_number,
        title: source.title,
        content: source.content,
        content_hash: source.content_sha256,
        summary: null,
        status: "completed",
        analysis_data: null,
      };
      setChapters([row]);
      openChapterDetail(row);
      return true;
    } catch (error) {
      if (isProjectCurrent(requestProjectId) && standaloneSourceLoadSeqRef.current === requestSeq) {
        const message = extractErrorMessage(error, t("comic.prepareSourceFailed"));
        setChapterLoadError(message);
      }
      return false;
    } finally {
      if (standaloneSourceLoadSeqRef.current === requestSeq) {
        setStandaloneSourceLoading(false);
        if (showLoading) setLoading(false);
      }
    }
  };

  const selectMangaTextSource = (selection: MangaTextSourceSelection) => {
    if (!ensureCanEditManga()) return;
    if (
      selectedTextSource?.assetId === selection.assetId &&
      selectedTextSource.version === selection.version &&
      selectedTextSource.startSection === selection.startSection &&
      selectedTextSource.endSection === selection.endSection
    ) return;
    if (isMutationBusy()) {
      addToast(t("common.operationInProgress"), "info");
      return;
    }
    setSelectedTextSource(selection);
    if (standalone) void prepareStandaloneSource(selection);
  };

  const loadSingleStatus = async (
    chapterId: number,
    requestProjectId = projectId,
  ): Promise<void> => {
    const requestSeq = statusLoadSeqRef.current + 1;
    statusLoadSeqRef.current = requestSeq;
    if (isProjectCurrent(requestProjectId)) {
      setPipelineStatusError((current) =>
        current?.scope === "chapter" && current.chapterId === chapterId ? null : current,
      );
    }
    try {
      const status = await getMangaPipelineStatus(requestProjectId, chapterId);
      if (
        !isProjectCurrent(requestProjectId) ||
        statusLoadSeqRef.current !== requestSeq
      ) return;
      setStatuses((current) => ({ ...current, [chapterId]: status }));
      setPipelineStatusError((current) =>
        current?.scope === "chapter" && current.chapterId === chapterId ? null : current,
      );
    } catch (error) {
      if (
        isProjectCurrent(requestProjectId) &&
        statusLoadSeqRef.current === requestSeq
      ) {
        setPipelineStatusError({
          scope: "chapter",
          chapterId,
          message: extractErrorMessage(error, t("comic.loadPipelineStatusFailed")),
        });
      }
    }
  };

  const loadChapters = async (
    requestProjectId = projectId,
    textSource = selectedTextSource,
  ) => {
    const requestSeq = chapterLoadSeqRef.current + 1;
    chapterLoadSeqRef.current = requestSeq;
    if (isSingleMode) {
      if (
        isProjectCurrent(requestProjectId) &&
        chapterLoadSeqRef.current === requestSeq
      ) {
        setLoading(false);
      }
      return;
    }
    if (standalone) {
      setChapters([]);
      setStatuses({});
      setSelectedChapter(null);
      selectedChapterIdRef.current = null;
      if (textSource) await prepareStandaloneSource(textSource, requestProjectId, true);
      else if (isProjectCurrent(requestProjectId) && chapterLoadSeqRef.current === requestSeq) setLoading(false);
      return;
    }
    if (
      isProjectCurrent(requestProjectId) &&
      chapterLoadSeqRef.current === requestSeq
    ) {
      setLoading(true);
      setChapterLoadError(null);
      setPipelineStatusError(null);
    }
    try {
      const rows: ChapterRow[] = snapshotChapters
        ? snapshotChapters.map((chapter) => ({
            id: chapter.source_chapter_id,
            chapter_number: chapter.chapter_number,
            title: chapter.title,
            content: chapter.content,
            content_hash: chapter.content_hash,
            summary: chapter.summary,
            status: "completed",
            analysis_data: chapter.analysis_data_json,
          }))
        : (await listChapters(requestProjectId)).map((c) => ({
            id: c.id,
            chapter_number: c.chapter_number,
            title: c.title,
            content: c.content,
            summary: c.summary,
            status: c.status,
            analysis_data: c.analysis_data,
          }));
      if (
        !isProjectCurrent(requestProjectId) ||
        chapterLoadSeqRef.current !== requestSeq
      )
        return;
      const previousChapterId = selectedChapterIdRef.current;
      const nextSelectedChapter =
        rows.find((row) => row.id === previousChapterId) ??
        (snapshotChapters || rows.length === 1 ? rows[0] ?? null : null);
      setChapters(rows);
      setChapterLoadError(null);
      setStatuses({});
      selectedChapterIdRef.current = nextSelectedChapter?.id ?? null;
      setSelectedChapter(nextSelectedChapter);
      if (nextSelectedChapter && nextSelectedChapter.id !== previousChapterId) {
        resetRequestPreview();
        setPagePrompts([]);
        setCurrentContentHash(nextSelectedChapter.content_hash ?? null);
        void loadPagePrompts(nextSelectedChapter.id, requestProjectId);
      } else if (!nextSelectedChapter) {
        setPagePrompts([]);
        setCurrentContentHash(null);
      }
      setLoading(false);
      if (rows.length === 0) return;
      try {
        const statuses = await listMangaPipelineStatuses(
          requestProjectId,
          rows.map((row) => row.id),
        );
        if (
          !isProjectCurrent(requestProjectId) ||
          chapterLoadSeqRef.current !== requestSeq
        ) return;
        const statusMap: Record<number, MangaPipelineStatusSummary | null> = {};
        for (const status of statuses) statusMap[status.chapter_id] = status;
        setStatuses(statusMap);
        setPipelineStatusError(null);
      } catch (err) {
        if (
          isProjectCurrent(requestProjectId) &&
          chapterLoadSeqRef.current === requestSeq
        ) {
          const message = extractErrorMessage(err, t("comic.loadPipelineStatusFailed"));
          setPipelineStatusError({ scope: "project", message });
        }
      }
    } catch (err) {
      if (
        isProjectCurrent(requestProjectId) &&
        chapterLoadSeqRef.current === requestSeq
      ) {
        const message = extractErrorMessage(err, t("comic.loadChaptersFailed"));
        setChapterLoadError(message);
      }
    } finally {
      if (
        isProjectCurrent(requestProjectId) &&
        chapterLoadSeqRef.current === requestSeq
      ) {
        setLoading(false);
      }
    }
  };

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    activeCharacterAssetProjectIdRef.current = characterAssetProjectId;
    activeImageSourceKeyRef.current = imageSourceKey;
    chapterLoadSeqRef.current += 1;
    statusLoadSeqRef.current += 1;
    pagePromptsLoadSeqRef.current += 1;
    characterAssetsLoadSeqRef.current += 1;
    projectImagesLoadSeqRef.current += 1;
    actionSeqRef.current += 1;
    previewSeqRef.current += 1;
    resetInteractionLocks();
    setChapters([]);
    setStatuses({});
    setPipelineStatusError(null);
    setPagePrompts([]);
    setPagePromptsError(null);
    setCurrentContentHash(null);
    setProfiles([]);
    setPortraits([]);
    setProjectImages([]);
    setProjectImagesError(null);
    setProjectImagesLoading(false);
    setCharacterAssetsError(null);
    setCharacterAssetsLoading(false);
    setPagePromptsLoading(false);
    setActing(null);
    resetRequestPreview();
    setPipelineChapterId(null);
    if (!isSingleMode) {
      selectedChapterIdRef.current = null;
      setSelectedChapter(null);
      setLoading(true);
    }
    void loadChapters(projectId);
    void loadCharacterAssets(projectId, characterAssetProjectId);
    void loadProjectImages(projectId, imageSourceProjectIds, imageSourceKey);
    // snapshotChapters is an external source list; when it changes, the visible chapter list must be rebuilt.
    // imageSourceKey 已表示图片来源的值身份；不能再依赖数组引用，否则父级重渲染会清空当前页。
  }, [
    characterAssetProjectId,
    imageSourceKey,
    isSingleMode,
    projectId,
    snapshotChapters,
    resetInteractionLocks,
    resetRequestPreview,
  ]);

  const loadPagePrompts = async (
    chapterId: number,
    requestProjectId = projectId,
    options: { showLoading?: boolean } = {},
  ): Promise<boolean> => {
    const showLoading = options.showLoading ?? true;
    const requestSeq = pagePromptsLoadSeqRef.current + 1;
    pagePromptsLoadSeqRef.current = requestSeq;
    if (showLoading && isChapterCurrent(requestProjectId, chapterId)) {
      setPagePromptsLoading(true);
    }
    if (isChapterCurrent(requestProjectId, chapterId)) setPagePromptsError(null);
    try {
      const data = await listMangaPagePrompts(requestProjectId, chapterId);
      if (
        !isChapterCurrent(requestProjectId, chapterId) ||
        pagePromptsLoadSeqRef.current !== requestSeq
      ) {
        return false;
      }
      setPagePrompts(data);
      setPagePromptsError(null);
      return true;
    } catch (error) {
      if (
        isChapterCurrent(requestProjectId, chapterId) &&
        pagePromptsLoadSeqRef.current === requestSeq
      ) {
        setPagePromptsError(extractErrorMessage(error, t("comic.loadPagePromptsFailed")));
      }
      return false;
    } finally {
      if (
        isChapterCurrent(requestProjectId, chapterId) &&
        pagePromptsLoadSeqRef.current === requestSeq
      ) {
        setPagePromptsLoading(false);
      }
    }
  };

  useLayoutEffect(() => {
    let cancelled = false;
    const chapterId = selectedChapter?.id;
    const content = selectedChapter?.content ?? "";
    if (selectedChapter?.content_hash) {
      setCurrentContentHash(selectedChapter.content_hash);
      return;
    }
    if (!chapterId || !content.trim()) {
      setCurrentContentHash(null);
      return;
    }
    void sha256Hex(content)
      .then((hash) => {
        if (!cancelled) setCurrentContentHash(hash);
      })
      .catch((err) => {
        if (!cancelled) {
          setCurrentContentHash(null);
          addToast(extractErrorMessage(err, t("comic.contentHashFailed")), "error");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [addToast, selectedChapter?.content, selectedChapter?.id]);

  const handleSelect = async (row: ChapterRow) => {
    if (selectedChapter?.id === row.id) return;
    if (await confirmLeaveStage()) {
      setSelectedPageNumber(null);
      openChapterDetail(row);
    }
  };

  const handleDeleteManga = async () => {
    if (!ensureCanEditManga()) return;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (isChapterGenerating(chapter)) {
      addToast(
        chapterGeneratingMessage(chapter, t("comic.operationFailed"), t),
        "error",
      );
      return;
    }

    const requestProjectId = projectId;
    const actionKey = `delete-${chapter.id}`;
    if (!beginAction(actionKey)) return;
    let actionSeq: number | null = null;
    try {
      const confirmed = await confirmDialog({
        title: t("comic.deleteArtifactsTitle"),
        message: t("comic.deleteArtifactsMessage"),
        confirmText: t("common.delete"),
        dialogType: "danger",
      });
      if (!confirmed || !isChapterCurrent(requestProjectId, chapter.id)) return;

      actionSeq = actionSeqRef.current + 1;
      actionSeqRef.current = actionSeq;
      setActing(actionKey);
      try {
        await deleteMangaByChapter(requestProjectId, chapter.id);
      } catch (error) {
        if (isActionCurrent(requestProjectId, actionSeq)) {
          addToast(extractErrorMessage(error, t("comic.operationFailed")), "error");
        }
        return;
      }
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(t("comic.deletedArtifacts"), "success");
      await refreshPipelineTarget(chapter.id, requestProjectId);
    } finally {
      if (actionSeq != null && isActionCurrent(requestProjectId, actionSeq)) {
        setActing(null);
      }
      endAction(actionKey);
    }
  };

  const handleRunPipeline = async (
    mode: "start" | "resume",
    stopAfter: MangaPipelineStopAfter | null = null,
    guard = true,
    pageNumber: number | null = null,
  ) => {
    if (!ensureCanEditManga()) return;
    if (guard && !await confirmLeaveStage()) return;
    const requestProjectId = projectId;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (isChapterGenerating(chapter)) {
      addToast(
        chapterGeneratingMessage(chapter, t("comic.pipelineStartTitle"), t),
        "error",
      );
      return;
    }
    if (
      !imageSize &&
      (stopAfter == null || stopAfter === "image")
    ) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const isResume = mode === "resume";
    const actionKey = `${mode}-${chapter.chapter_number}`;
    const title = isResume
      ? t("comic.pipelineResumeTitle")
      : t("comic.pipelineStartTitle");
    const configBlockReason = mangaPipelineConfigBlockReason(t);
    if (configBlockReason) {
      addToast(configBlockReason, "error");
      return;
    }
    if (!beginAction(actionKey)) return;
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    setActing(actionKey);
    setPipelineChapterId(chapter.id);

    let taskStarted = false;
    let terminalTaskCompleted = false;
    try {
      const handle = isResume
        ? await resumeMangaPipeline(requestProjectId, chapter.chapter_number, stopAfter, imageSize, pageNumber)
        : await startMangaPipeline(requestProjectId, chapter.chapter_number, stopAfter, imageSize, pageNumber);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      taskStarted = true;
      void loadSingleStatus(chapter.id, requestProjectId);
      await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title,
          startMessage: t("comic.pipelineSubmitted"),
          successMessage: t("comic.pipelineCompleted"),
          errorMessage: t("comic.pipelineFailed"),
          cancelLabel: t("comic.cancelPipeline"),
          runningHint: t("comic.pipelineRunningHint"),
        },
        async (task) => {
          if (task.taskStatus === "cancelled") {
            throw new Error("Generation task cancelled");
          }
          if (task.taskStatus !== "completed") {
            throw new Error(task.error?.message ?? t("comic.pipelineFailed"));
          }
          if (!isActionCurrent(requestProjectId, actionSeq)) return;
          terminalTaskCompleted = true;
          await refreshPipelineTarget(chapter.id, requestProjectId);
          if (isActionCurrent(requestProjectId, actionSeq)) {
            setPipelineChapterId(null);
          }
        },
      );
    } catch (error) {
        if (!isActionCurrent(requestProjectId, actionSeq)) return;
        const message = extractErrorMessage(
          error,
          isResume ? t("comic.pipelineResumeFailed") : t("comic.pipelineStartFailed"),
        );
        const cancelled = isCancelledOperation(message);
        if (taskStarted && !terminalTaskCompleted) {
          await refreshPipelineTarget(chapter.id, requestProjectId);
        }
        if (!isActionCurrent(requestProjectId, actionSeq)) return;
        if (!cancelled) {
          addToast(
            explainMangaPipelineAiConfigError(message, t),
            "error",
          );
        }
        setPipelineChapterId(null);
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setActing(null);
      endAction(actionKey);
    }
  };

  const handlePreviewPipeline = async (
    mode: "start" | "resume",
    stopAfter: MangaPipelineStopAfter | null = null,
    pageNumber: number | null = null,
  ) => {
    if (!ensureCanEditManga()) return;
    const requestProjectId = projectId;
    const chapter = selectedChapter;
    if (!chapter) return;
    if (isChapterGenerating(chapter)) {
      addToast(
        chapterGeneratingMessage(chapter, t("comic.pipelineRequestPreview"), t),
        "error",
      );
      return;
    }
    if (!chapter.content?.trim()) {
      addToast(t("comic.emptyChapterPreviewPipeline"), "error");
      return;
    }
    const previewKey = `${mode}:${requestProjectId}:${chapter.id}`;
    if (!beginPreview(previewKey)) return;
    const requestSeq = previewSeqRef.current + 1;
    previewSeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview =
        mode === "resume"
          ? await previewResumeMangaPipelineRequest(
              requestProjectId,
              chapter.chapter_number,
              stopAfter,
              pageNumber,
            )
          : await previewStartMangaPipelineRequest(
              requestProjectId,
              chapter.chapter_number,
              stopAfter,
              pageNumber,
            );
      if (
        !isPreviewCurrent(requestProjectId, requestSeq) ||
        !isChapterCurrent(requestProjectId, chapter.id)
      ) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, t("comic.previewPipelineFailed")), "error");
        setRequestPreviewOpen(false);
        endPreview(previewKey);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, requestSeq))
        setRequestPreviewLoading(false);
    }
  };

  const closeRequestPreview = resetRequestPreview;

  const requestPipelineCancellation = async (
    chapterId: number,
    handle: GenerationTaskHandle,
  ) => {
    const requestProjectId = projectId;
    const cancelKey = `${requestProjectId}:${chapterId}`;
    if (progressCancelBusyKeyRef.current) return;
    progressCancelBusyKeyRef.current = cancelKey;
    try {
      await generationClient.cancel(handle);
      if (!isProjectCurrent(requestProjectId)) return;
      addToast(t("comic.pipelineCancelRequested"), "info");
      await refreshPipelineTarget(chapterId, requestProjectId);
    } catch (error) {
      if (isProjectCurrent(requestProjectId)) {
        addToast(extractErrorMessage(error, t("comic.pipelineCancelFailed")), "error");
      }
    } finally {
      if (progressCancelBusyKeyRef.current === cancelKey) {
        progressCancelBusyKeyRef.current = null;
      }
    }
  };

  const handleGenerateMissingPortraits = async () => {
    if (!ensureCanEditManga()) return;
    const requestProjectId = projectId;
    const requestAssetProjectId = characterAssetProjectId;
    const requestSize = imageSize;
    if (!requestSize) {
      addToast(t("settingsForm.imageSizeUnavailable"), "error");
      return;
    }
    const actionKey = `generate-missing-portraits:${requestAssetProjectId}`;
    if (!beginAction(actionKey)) return;
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    setActing("generate-missing-portraits");
    try {
      const handle = await startPortraitGeneration({
        project_id: requestAssetProjectId,
        operation: {
          kind: "generate_missing",
          width: requestSize.width,
          height: requestSize.height,
        },
      });
      const createdCount = await runGenerationProgressTaskThen(
        runProgressTask,
        handle,
        {
          title: t("comic.portraitGenerationTitle"),
          startMessage: t("comic.portraitGenerationStart"),
          successMessage: t("comic.portraitGenerationSuccess"),
          errorMessage: t("comic.portraitGenerationFailed"),
          cancelLabel: t("comic.stopGeneration"),
        },
        async (task) => {
          const generated = await verifyPortraitGenerationResult(task, requestAssetProjectId);
          if (!isActionCurrent(requestProjectId, actionSeq))
            throw new Error("任务已停止");
          await loadCharacterAssets(requestProjectId, requestAssetProjectId);
          if (!isActionCurrent(requestProjectId, actionSeq))
            throw new Error("任务已停止");
          return generated.length;
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(t("comic.missingPortraitsGenerated", { count: createdCount }), "success");
    } catch (err) {
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const message = extractErrorMessage(err, t("comic.portraitGenerationFailed"));
      if (isCancelledOperation(message)) {
        addToast(t("comic.portraitGenerationStopped"), "info");
      } else {
        addToast(message, "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setActing(null);
      endAction(actionKey);
    }
  };

  const selectedStatus = selectedChapter ? statuses[selectedChapter.id] : null;
  const assetsError = [characterAssetsError, projectImagesError].filter(Boolean).join(" · ") || null;
  useEffect(() => {
    if (staged && selectedStatus?.pipeline_status === "completed") {
      void authoring.reload().catch(() => { /* Stage read errors are shown by the authoring panel. */ });
    }
  }, [staged, selectedStatus?.pipeline_status, authoring.reload]);

  // fixedChapter 变化时联动
  useEffect(() => {
    if (fixedChapter) {
      if (!isProjectCurrent(projectId)) return;
      openChapterDetail(fixedChapter);
    } else if (isSingleMode) {
      resetChapterDetail();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    fixedChapter?.content,
    fixedChapter?.chapter_number,
    fixedChapter?.id,
    fixedChapter?.status,
    isSingleMode,
  ]);

  // 进度对话框订阅 Generation Task；其余运行状态由章节详情轮询。
  useEffect(() => {
    const chapterId = selectedChapter?.id;
    const shouldPoll =
      selectedStatus?.pipeline_status === "running" ||
      selectedStatus?.can_cancel === true;
    if (!chapterId || !shouldPoll) return;
    const timer = window.setInterval(() => {
      if (!isChapterCurrent(projectId, chapterId)) return;
      void loadSingleStatus(chapterId, projectId);
      void loadPagePrompts(chapterId, projectId, { showLoading: false });
    }, 2500);
    return () => window.clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    pipelineChapterId,
    selectedChapter?.id,
    selectedStatus?.pipeline_status,
    selectedStatus?.can_cancel,
  ]);

  if (loading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <LoadingSpinner size="sm" /> {t("comic.loadingChapters")}
      </div>
    );
  }

  if (!standalone && !isSingleMode && chapters.length === 0) {
    return (
      <div className="flex min-h-0 flex-1 flex-col gap-3">
        {chapterLoadError ? (
          <div role="alert" className="flex flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
            <span className="min-w-0 flex-1">{chapterLoadError}</span>
            <BookButton variant="secondary" size="sm" onClick={() => void loadChapters(projectId)} disabled={loading}>
              <RefreshCw size={12} /> {t("common.retry")}
            </BookButton>
          </div>
        ) : null}
        <div className="content-main">
          <MangaChapterList
            chapters={chapters}
            statuses={statuses}
            selectedChapter={selectedChapter}
            onSelect={handleSelect}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 md:h-full">
      {staged && <MediaStageHeader id={stageId} label={t("common.mediaAuthoring.stages")} tabs={stageTabs} activeTab={workspaceStage}
        onTabChange={async (stage) => {
          if (stage === workspaceStage) return true;
          if (!await confirmLeaveStage()) return false;
          if (workspaceStage === "materials") await reloadMangaAssets();
          setWorkspaceStage(stage);
          return true;
        }} />}
      {outputState?.stale && (workspaceStage === "prompts" || workspaceStage === "images") && (
        <p role="status" className="shrink-0 rounded-xl border border-warning/40 bg-warning-bg px-3 py-2 text-small text-warning">
          {t("common.mediaAuthoring.outputsStale")}
        </p>
      )}
      {chapterLoadError ? (
        <div role="alert" className="flex shrink-0 flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{chapterLoadError}</span>
          <BookButton variant="secondary" size="sm" onClick={() => void loadChapters(projectId)} disabled={loading || standaloneSourceLoading}>
            <RefreshCw size={12} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      {pipelineStatusError ? (
        <div role="alert" className="flex shrink-0 flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{pipelineStatusError.message}</span>
          <BookButton variant="secondary" size="sm" onClick={() => void (pipelineStatusError.scope === "chapter" ? loadSingleStatus(pipelineStatusError.chapterId, projectId) : loadChapters(projectId))} disabled={loading}>
            <RefreshCw size={12} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      {assetsError && (!staged || workspaceStage === "materials" || workspaceStage === "prompts" || workspaceStage === "images") ? (
        <div role="alert" className="flex shrink-0 flex-wrap items-center gap-3 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          <span className="min-w-0 flex-1">{assetsError}</span>
          <BookButton
            variant="secondary"
            size="sm"
            disabled={characterAssetsLoading || projectImagesLoading}
            onClick={() => {
              if (characterAssetsError) void loadCharacterAssets(projectId, characterAssetProjectId);
              if (projectImagesError) void loadProjectImages(projectId, imageSourceProjectIds, imageSourceKey);
            }}
          >
            <RefreshCw size={12} /> {t("common.retry")}
          </BookButton>
        </div>
      ) : null}
      {authoring.error && workspaceStage !== "adaptation" ? (
        <p role="alert" className="shrink-0 rounded-xl border border-error/40 bg-error-bg px-3 py-2 text-small text-error">
          {authoring.error}
        </p>
      ) : null}
      <div
      className={
        isSingleMode
          ? "flex min-h-0 flex-1 flex-col"
          : showChapterNavigation
            ? "grid min-h-0 flex-1 grid-cols-1 gap-3 md:h-full md:grid-cols-[220px_minmax(0,1fr)] md:gap-4"
            : "flex min-h-0 flex-1 flex-col md:h-full"
      }
    >
      {/* 章节列表（仅完整模式） */}
      {showChapterNavigation && (
        <MangaChapterList
          chapters={chapters}
          statuses={statuses}
          selectedChapter={selectedChapter}
          onSelect={handleSelect}
        />
      )}

      <div id={staged ? `${stageId}-panel` : undefined} role={staged ? "tabpanel" : undefined} aria-labelledby={staged ? `${stageId}-tab-${workspaceStage}` : undefined} className="flex min-h-0 min-w-0 flex-1 flex-col">
      {workspaceStage === "adaptation" ? <MediaAuthoringPanel
        key={`${projectId}:${selectedChapter?.id}:${workspaceStage}:${discardVersion}`} projectId={projectId} kind="adaptation_plan"
        workspace={authoring.workspace} loading={authoring.loading} error={authoring.error} chapterId={selectedChapter?.id ?? null}
        canEdit={canEditManga} stageRunning={selectedStatus?.can_cancel === true} onReload={authoring.reload} onDirtyChange={setAuthoringDirty}
        onBusyChange={setAuthoringBusy}
        onNext={() => setWorkspaceStage("page_plan")}
      /> : workspaceStage === "materials" ? <BaseMaterialsWorkspace projectId={characterAssetProjectId} textProjectId={projectId} canEdit={canEditManga} showNext={!standalone} standaloneComic={standalone}
        onChanged={loadProjectImages} onTextDirtyChange={setTextMaterialsDirty} selectedTextSource={standalone ? selectedTextSource : null} onSelectTextSource={standalone ? (selection) => void selectMangaTextSource(selection) : undefined} onNext={async () => { if (await reloadMangaAssets()) setWorkspaceStage("adaptation"); }} /> : <MangaChapterDetailPanel
        key={`${projectId}:${selectedChapter?.id}:${workspaceStage}:${discardVersion}`}
        stageMode={activePageStage}
        onDirtyChange={setPromptDirty}
        projectId={projectId}
        isSingleMode={isSingleMode}
        selectedChapter={selectedChapter}
        selectedStatus={selectedStatus}
        pagePrompts={pagePrompts}
        selectedPageNumber={selectedPageNumber}
        onSelectedPageChange={setSelectedPageNumber}
        pagePromptsLoading={pagePromptsLoading}
        pagePromptsError={pagePromptsError}
        currentContentHash={currentContentHash}
        characterAssetsLoading={characterAssetsLoading}
        projectImagesLoading={projectImagesLoading}
        profiles={profiles}
        portraits={portraits}
        projectImages={projectImages}
        imageSize={imageSize}
        imageSizes={imageSizes}
        imageSizeLoading={imageSizeLoading}
        imageSizeError={imageSizeError}
        onImageSizeChange={setImageSize}
        primaryPortraitCount={primaryPortraitCount}
        characterProfileCount={profiles.length}
        portraitCount={portraits.length}
        missingPortraitCount={missingPortraitCount}
        acting={acting}
        canEditManga={canEditManga}
        onPreviewPipeline={(mode, stopAfter, pageNumber) => void handlePreviewPipeline(mode, stopAfter, pageNumber)}
        onRunPipeline={(mode, stopAfter, pageNumber) => void handleRunPipeline(mode, stopAfter, true, pageNumber)}
        {...(!standalone ? { onGenerateMissingPortraits: () => void handleGenerateMissingPortraits() } : {})}
        onCancelPipeline={() => {
          if (!ensureCanEditManga()) return;
          const chapter = selectedChapter;
          const handle = selectedStatus?.task_handle;
          if (!chapter || !handle) {
            addToast(t("comic.pipelineCancelFailed"), "error");
            return;
          }
          void requestPipelineCancellation(chapter.id, handle);
        }}
        onDeleteManga={() => void handleDeleteManga()}
        onPagePromptsChanged={async () => {
          const [promptsOk, imagesOk] = await Promise.all([
            selectedChapter
              ? loadPagePrompts(selectedChapter.id, projectId, { showLoading: false })
              : Promise.resolve(false),
            loadProjectImages(),
          ]);
          return promptsOk && imagesOk;
        }}
        onRetryPagePrompts={() => {
          if (!selectedChapter) return;
          void loadPagePrompts(selectedChapter.id, projectId);
        }}
      />}
      </div>
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle={t("comic.pipelineRequestPreview")}
        onClose={closeRequestPreview}
      />
      </div>
    </div>
  );
};
