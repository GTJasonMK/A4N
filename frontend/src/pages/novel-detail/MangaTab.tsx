import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import {
  ProgressDialog,
} from "../../components/feedback/ProgressDialog";
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
import {
  cancelMangaPipeline,
  deleteMangaByChapter,
  getMangaPipelineStatus,
  listMangaPagePrompts,
  listMangaPipelineStatuses,
  previewResumeMangaPipelineRequest,
  previewStartMangaPipelineRequest,
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
import { MANGA_PIPELINE_TOTAL_STEPS } from "./manga/mangaTabUtils";
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
  createMangaPipelineProgressState,
  mangaPipelineConfigBlockReason,
  type MangaPipelineProgressState,
  pipelineStatusIsTerminal,
  progressStateFromPipelineStatus,
  type MangaTabProps,
} from "./manga/mangaPipelineProgress";

const MANGA_PIPELINE_POLL_MS = 1500;

export const MangaTab: React.FC<MangaTabProps> = ({
  projectId,
  canEditManga = true,
  snapshotChapters,
  assetProjectId,
  imageProjectIds,
  fixedChapter,
  onDirtyChange,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const staged = snapshotChapters !== undefined;
  const [authoringDirty, setAuthoringDirty] = useState(false);
  const [authoringBusy, setAuthoringBusy] = useState(false);
  const [promptDirty, setPromptDirty] = useState(false);
  const [discardVersion, setDiscardVersion] = useState(0);
  const discardStageDrafts = () => {
    setAuthoringDirty(false);
    setPromptDirty(false);
    setDiscardVersion((version) => version + 1);
  };
  const isMutationBusy = () => actionBusyKeyRef.current != null || pipelineTaskId != null || authoringBusy;
  useEffect(() => { onDirtyChange?.(authoringDirty || promptDirty); }, [authoringDirty, promptDirty, onDirtyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  const discardMangaDrafts = () => {
    if (isMutationBusy() || acting != null || pipelineCancelling) return false;
    discardStageDrafts();
  };
  useNovelDetailDirty(`manga-editor:${projectId}`, authoringDirty || promptDirty, discardMangaDrafts, () => !isMutationBusy() && acting == null && !pipelineCancelling);
  const authoring = useMediaAuthoring(projectId, staged);
  const stageId = `comic-stages-${projectId}`;
  const confirmLeaveStage = async () => {
    if (isMutationBusy()) {
      addToast(t("common.operationInProgress"), "info");
      return false;
    }
    if (!authoringDirty && !promptDirty) return true;
    const confirmed = await confirmDiscardMediaChanges(true, t);
    if (confirmed) { setAuthoringDirty(false); setPromptDirty(false); setDiscardVersion((version) => version + 1); }
    return confirmed;
  };
  const { runProgressTask: runPortraitProgressTask } = useProgressTask();
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
  const [loading, setLoading] = useState(true);
  const [acting, setActing] = useState<string | null>(null);
  const [selectedChapter, setSelectedChapter] = useState<ChapterRow | null>(
    fixedChapter ?? null,
  );
  const outputState = authoring.workspace?.artifacts.find((artifact) => artifact.kind === "manga_pages" && artifact.source_chapter_id === selectedChapter?.id);
  const stageTabs = useMemo(() => [
    { id: "materials" as const, label: t("common.mediaAuthoring.materials") },
    { id: "adaptation" as const, label: t("common.mediaAuthoring.adaptation"), status: authoringStageLabel(authoring.workspace, "adaptation_plan", selectedChapter?.id, t) },
    { id: "page_plan" as const, label: t("common.mediaAuthoring.pagePlanning"), status: authoringStageLabel(authoring.workspace, "manga_page_plan", selectedChapter?.id, t) },
    { id: "prompts" as const, label: t("common.mediaAuthoring.pagePrompts"), status: outputState?.count ? t(outputState.stale ? "common.mediaAuthoring.needsUpdate" : "common.mediaAuthoring.ready") : undefined },
    { id: "images" as const, label: t("common.mediaAuthoring.images") },
  ], [authoring.workspace, outputState, selectedChapter?.id, t]);
  const [workspaceStage, setWorkspaceStage] = usePersistedTab(
    staged ? `a4n:comic:${projectId}:stage` : null,
    staged ? "materials" : "prompts",
    stageTabs.map((tab) => tab.id),
  );
  const [pagePrompts, setPagePrompts] = useState<MangaPagePromptSummary[]>([]);
  const [pagePromptsLoading, setPagePromptsLoading] = useState(false);
  const [currentContentHash, setCurrentContentHash] = useState<string | null>(
    null,
  );
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [portraits, setPortraits] = useState<Portrait[]>([]);
  const [projectImages, setProjectImages] = useState<GeneratedImage[]>([]);
  const [projectImagesLoading, setProjectImagesLoading] = useState(false);
  const [characterAssetsLoading, setCharacterAssetsLoading] = useState(false);
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [pipelineProgressOpen, setPipelineProgressOpen] = useState(false);
  const [pipelineProgress, setPipelineProgress] =
    useState<MangaPipelineProgressState>(() =>
      createMangaPipelineProgressState(t),
    );
  const [pipelineTaskId, setPipelineTaskId] = useState<string | null>(null);
  const [pipelineChapterId, setPipelineChapterId] = useState<number | null>(
    null,
  );
  const [pipelineCancelling, setPipelineCancelling] = useState(false);
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
  const chapterLoadSeqRef = useRef(0);
  const statusLoadSeqRef = useRef(0);
  const pagePromptsLoadSeqRef = useRef(0);
  const characterAssetsLoadSeqRef = useRef(0);
  const projectImagesLoadSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const pipelineWatchSeqRef = useRef(0);
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

  const isPipelineWatchCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) &&
      pipelineWatchSeqRef.current === requestSeq,
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
      pipelineWatchSeqRef.current += 1;
      resetInteractionLocks();
    };
  }, [resetInteractionLocks]);

  const refreshPipelineTarget = async (
    chapterId: number,
    requestProjectId = projectId,
  ): Promise<boolean> => {
    if (!isProjectCurrent(requestProjectId)) return false;
    const [statusOk, promptsOk] = await Promise.all([
      loadSingleStatus(chapterId, requestProjectId).catch((err) => {
        addToast(extractErrorMessage(err, t("comic.loadPipelineStatusFailed")), "error");
        return false;
      }),
      isChapterCurrent(requestProjectId, chapterId)
        ? loadPagePrompts(chapterId, requestProjectId, { showLoading: false }).catch((err) => {
            addToast(extractErrorMessage(err, t("comic.loadPagePromptsFailed")), "error");
            return false;
          })
        : Promise.resolve(false),
    ]);
    return statusOk && promptsOk;
  };

  /** 漫画统一 workflow 状态经 getMangaPipelineStatus 投影，禁止订阅旧 Host 进度通道。 */
  const watchMangaPipelineStatus = async (
    chapterId: number,
    taskId: string,
    requestProjectId: number,
    requestWatchSeq: number,
    title: string,
  ) => {
    while (isPipelineWatchCurrent(requestProjectId, requestWatchSeq)) {
      try {
        const status = await getMangaPipelineStatus(
          requestProjectId,
          chapterId,
        );
        if (!isPipelineWatchCurrent(requestProjectId, requestWatchSeq)) return;
        if (status.task_id && status.task_id !== taskId) {
          // 状态尚未切到本次任务，继续轮询。
        } else {
          setStatuses((current) => ({ ...current, [chapterId]: status }));
          setPipelineProgress(progressStateFromPipelineStatus(status, title, t));
          if (pipelineStatusIsTerminal(status.pipeline_status)) {
            setPipelineTaskId(null);
            setPipelineChapterId(null);
            setPipelineCancelling(false);
            const refreshed = await refreshPipelineTarget(chapterId, requestProjectId);
            if (!refreshed && isPipelineWatchCurrent(requestProjectId, requestWatchSeq)) {
              setPipelineProgress((prev) => ({
                ...prev,
                status: "error",
                message: t("comic.pipelineFailed"),
                error: t("comic.loadPagePromptsFailed"),
              }));
            }
            return;
          }
        }
      } catch (err) {
        if (!isPipelineWatchCurrent(requestProjectId, requestWatchSeq)) return;
        addToast(extractErrorMessage(err, t("comic.loadPipelineStatusFailed")), "error");
      }
      await new Promise<void>((resolve) => {
        window.setTimeout(resolve, MANGA_PIPELINE_POLL_MS);
      });
    }
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
      if (profilesResult.status === "fulfilled") {
        setProfiles(profilesResult.value);
      } else {
        addToast(
          extractErrorMessage(profilesResult.reason, t("comic.loadCharacterAssetsFailed")),
          "error",
        );
      }
      if (portraitsResult.status === "fulfilled") {
        setPortraits(portraitsResult.value);
      } else {
        addToast(
          extractErrorMessage(portraitsResult.reason, t("comic.loadCharacterAssetsFailed")),
          "error",
        );
      }
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
      return true;
    } catch (err) {
      if (
        isProjectImagesCurrent(
          requestProjectId,
          requestImageSourceKey,
          requestSeq,
        )
      ) {
        addToast(extractErrorMessage(err, t("comic.loadProjectImagesFailed")), "error");
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

  const resetChapterDetail = () => {
    selectedChapterIdRef.current = null;
    pagePromptsLoadSeqRef.current += 1;
    statusLoadSeqRef.current += 1;
    resetRequestPreview();
    setSelectedChapter(null);
    setPagePrompts([]);
    setCurrentContentHash(null);
  };

  const openChapterDetail = (row: ChapterRow) => {
    selectedChapterIdRef.current = row.id;
    resetRequestPreview();
    setSelectedChapter(row);
    setPagePrompts([]);
    setCurrentContentHash(null);
    void loadPagePrompts(row.id, projectId).catch((err) =>
      addToast(extractErrorMessage(err, t("comic.loadPagePromptsFailed")), "error"),
    );
    void loadSingleStatus(row.id, projectId).catch((err) =>
      addToast(extractErrorMessage(err, t("comic.loadPipelineStatusFailed")), "error"),
    );
  };

  const loadSingleStatus = async (
    chapterId: number,
    requestProjectId = projectId,
  ): Promise<boolean> => {
    const requestSeq = statusLoadSeqRef.current + 1;
    statusLoadSeqRef.current = requestSeq;
    const status = await getMangaPipelineStatus(requestProjectId, chapterId);
    if (
      !isProjectCurrent(requestProjectId) ||
      statusLoadSeqRef.current !== requestSeq
    )
      return false;
    setStatuses((current) => ({ ...current, [chapterId]: status }));
    return true;
  };

  const loadChapters = async (requestProjectId = projectId) => {
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
    if (
      isProjectCurrent(requestProjectId) &&
      chapterLoadSeqRef.current === requestSeq
    ) {
      setLoading(true);
    }
    try {
      const rows: ChapterRow[] = snapshotChapters
        ? snapshotChapters.map((chapter) => ({
            id: chapter.source_chapter_id,
            chapter_number: chapter.chapter_number,
            title: chapter.title,
            content: chapter.content,
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
      setStatuses({});
      selectedChapterIdRef.current = nextSelectedChapter?.id ?? null;
      setSelectedChapter(nextSelectedChapter);
      if (nextSelectedChapter && nextSelectedChapter.id !== previousChapterId) {
        resetRequestPreview();
        setPagePrompts([]);
        setCurrentContentHash(null);
        void loadPagePrompts(nextSelectedChapter.id, requestProjectId).catch((err) =>
          addToast(extractErrorMessage(err, t("comic.loadPagePromptsFailed")), "error"),
        );
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
      } catch (err) {
        if (
          isProjectCurrent(requestProjectId) &&
          chapterLoadSeqRef.current === requestSeq
        ) {
          addToast(extractErrorMessage(err, t("comic.loadPipelineStatusFailed")), "error");
        }
      }
    } catch (err) {
      if (
        isProjectCurrent(requestProjectId) &&
        chapterLoadSeqRef.current === requestSeq
      ) {
        addToast(extractErrorMessage(err, t("comic.loadChaptersFailed")), "error");
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
    pipelineWatchSeqRef.current += 1;
    resetInteractionLocks();
    setChapters([]);
    setStatuses({});
    setPagePrompts([]);
    setCurrentContentHash(null);
    setProfiles([]);
    setPortraits([]);
    setProjectImages([]);
    setProjectImagesLoading(false);
    setCharacterAssetsLoading(false);
    setPagePromptsLoading(false);
    setActing(null);
    resetRequestPreview();
    setPipelineProgressOpen(false);
    setPipelineProgress(createMangaPipelineProgressState(t));
    setPipelineTaskId(null);
    setPipelineChapterId(null);
    setPipelineCancelling(false);
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
    try {
      const data = await listMangaPagePrompts(requestProjectId, chapterId);
      if (
        !isChapterCurrent(requestProjectId, chapterId) ||
        pagePromptsLoadSeqRef.current !== requestSeq
      ) {
        return false;
      }
      setPagePrompts(data);
      return true;
    } finally {
      if (
        isChapterCurrent(requestProjectId, chapterId) &&
        pagePromptsLoadSeqRef.current === requestSeq
      ) {
        setPagePromptsLoading(false);
      }
    }
  };

  useEffect(() => {
    let cancelled = false;
    const chapterId = selectedChapter?.id;
    const content = selectedChapter?.content ?? "";
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
    if (selectedChapter?.id === row.id || await confirmLeaveStage()) openChapterDetail(row);
  };

  const wrapAct = async (
    key: string,
    fn: () => Promise<unknown>,
    ok: string,
    options: { actionAlreadyHeld?: boolean } = {},
  ) => {
    const requestProjectId = projectId;
    const targetChapterId = selectedChapter?.id ?? null;
    const ownsAction = options.actionAlreadyHeld
      ? actionBusyKeyRef.current === key
      : beginAction(key);
    if (!ownsAction) return;
    if (
      selectedChapter &&
      isChapterGenerating(selectedChapter) &&
      !key.startsWith("cancel-") &&
      !key.startsWith("generate-missing-portraits")
    ) {
      addToast(
        chapterGeneratingMessage(selectedChapter, t("comic.operationFailed"), t),
        "error",
      );
      endAction(key);
      return;
    }
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    setActing(key);
    try {
      try {
        await fn();
      } catch (err) {
        if (isActionCurrent(requestProjectId, actionSeq)) {
          addToast(extractErrorMessage(err, t("comic.operationFailed")), "error");
        }
        return;
      }
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (targetChapterId) {
        const [promptsOk, statusOk] = await Promise.all([
          isChapterCurrent(requestProjectId, targetChapterId)
            ? loadPagePrompts(targetChapterId, requestProjectId, {
                showLoading: false,
              }).catch((err) => {
                if (isActionCurrent(requestProjectId, actionSeq)) {
                  addToast(extractErrorMessage(err, t("comic.loadPagePromptsFailed")), "error");
                }
                return false;
              })
            : Promise.resolve(false),
          loadSingleStatus(targetChapterId, requestProjectId).catch((err) => {
            if (isActionCurrent(requestProjectId, actionSeq)) {
              addToast(extractErrorMessage(err, t("comic.loadPipelineStatusFailed")), "error");
            }
            return false;
          }),
        ]);
        if (!isActionCurrent(requestProjectId, actionSeq)) return;
        if (!promptsOk || !statusOk) return;
        window.setTimeout(() => {
          if (isActionCurrent(requestProjectId, actionSeq)) {
            refreshPipelineTarget(targetChapterId, requestProjectId);
          }
        }, 1200);
      }
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(ok, "success");
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setActing(null);
      endAction(key);
    }
  };

  const handleRunPipeline = async (mode: "start" | "resume", stopAfter: MangaPipelineStopAfter | null = null, guard = true) => {
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
    const isResume = mode === "resume";
    const actionKey = `${mode}-${chapter.chapter_number}`;
    const title = isResume
      ? t("comic.pipelineResumeTitle")
      : t("comic.pipelineStartTitle");
    const configBlockReason = mangaPipelineConfigBlockReason(t);
    if (configBlockReason) {
      setPipelineProgressOpen(true);
      setPipelineProgress({
        title,
        status: "error",
        current: 0,
        total: MANGA_PIPELINE_TOTAL_STEPS,
        message: isResume
          ? t("comic.pipelineResumeFailed")
          : t("comic.pipelineStartFailed"),
        error: configBlockReason,
      });
      addToast(configBlockReason, "error");
      return;
    }
    if (!beginAction(actionKey)) return;
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    const watchSeq = pipelineWatchSeqRef.current + 1;
    pipelineWatchSeqRef.current = watchSeq;

    setActing(actionKey);
    setPipelineProgressOpen(true);
    setPipelineProgress({
      title,
      status: "running",
      current: 0,
      total: MANGA_PIPELINE_TOTAL_STEPS,
      message: isResume
        ? t("comic.pipelineResumeMessage", { steps: MANGA_PIPELINE_TOTAL_STEPS })
        : t("comic.pipelineStartMessage", { steps: MANGA_PIPELINE_TOTAL_STEPS }),
      error: null,
    });
    setPipelineTaskId(null);
    setPipelineChapterId(chapter.id);
    setPipelineCancelling(false);

    try {
      const taskId = isResume
        ? await resumeMangaPipeline(requestProjectId, chapter.chapter_number, stopAfter)
        : await startMangaPipeline(requestProjectId, chapter.chapter_number, stopAfter);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      setPipelineTaskId(taskId);
      setPipelineProgress((prev) => ({
        ...prev,
        status: "running",
        message: t("comic.pipelineSubmitted"),
      }));
      // 启动请求返回后即可释放 acting；进度由状态轮询驱动。
      setActing(null);
      endAction(actionKey);
      void watchMangaPipelineStatus(
        chapter.id,
        taskId,
        requestProjectId,
        watchSeq,
        title,
      );
    } catch (err) {
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      setPipelineTaskId(null);
      setPipelineChapterId(null);
      setPipelineCancelling(false);
      pipelineWatchSeqRef.current += 1;
      const message = explainMangaPipelineAiConfigError(
        extractErrorMessage(
          err,
          isResume
            ? t("comic.pipelineResumeFailed")
            : t("comic.pipelineStartFailed"),
        ),
        t,
      );
      setPipelineProgress((prev) => ({
        ...prev,
        status: "error",
        message: isResume
          ? t("comic.pipelineResumeFailed")
          : t("comic.pipelineStartFailed"),
        error: message,
      }));
      addToast(message, "error");
      if (isActionCurrent(requestProjectId, actionSeq)) setActing(null);
      endAction(actionKey);
    }
  };

  const handlePreviewPipeline = async (mode: "start" | "resume") => {
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
            )
          : await previewStartMangaPipelineRequest(
              requestProjectId,
              chapter.chapter_number,
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

  const handleCancelPipelineProgress = async () => {
    if (!ensureCanEditManga()) return;
    const requestProjectId = projectId;
    const chapterId = pipelineChapterId ?? selectedChapter?.id;
    if (!chapterId) return;
    const cancelKey = `${requestProjectId}:${chapterId}`;
    if (progressCancelBusyKeyRef.current) return;
    progressCancelBusyKeyRef.current = cancelKey;
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    setPipelineCancelling(true);
    addToast(t("comic.pipelineCancelRequested"), "info");
    try {
      await cancelMangaPipeline(requestProjectId, chapterId);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const [statusResult, promptsResult] = await Promise.allSettled([
        loadSingleStatus(chapterId, requestProjectId),
        isChapterCurrent(requestProjectId, chapterId)
          ? loadPagePrompts(chapterId, requestProjectId, { showLoading: false })
          : Promise.resolve(false),
      ]);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (statusResult.status === "rejected") {
        addToast(extractErrorMessage(statusResult.reason, t("comic.loadPipelineStatusFailed")), "error");
      }
      if (promptsResult.status === "rejected") {
        addToast(extractErrorMessage(promptsResult.reason, t("comic.loadPagePromptsFailed")), "error");
      }
      if (
        statusResult.status === "rejected" ||
        promptsResult.status === "rejected" ||
        statusResult.value === false ||
        promptsResult.value === false
      ) {
        setPipelineProgress((prev) => ({
          ...prev,
          status: "error",
          message: t("comic.pipelineCancelFailed"),
          error: t("comic.loadPagePromptsFailed"),
        }));
        setPipelineCancelling(false);
        return;
      }
      setPipelineProgress((prev) => ({
        ...prev,
        status: "cancelled",
        message: t("comic.pipelineCancelled"),
        error: null,
      }));
      setPipelineCancelling(false);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, t("comic.pipelineCancelFailed")), "error");
        setPipelineCancelling(false);
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
        runPortraitProgressTask,
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
          if (!await loadCharacterAssets(requestProjectId, requestAssetProjectId)) {
            throw new Error(t("comic.loadCharacterAssetsFailed"));
          }
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
    } else {
      resetChapterDetail();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    fixedChapter?.content,
    fixedChapter?.chapter_number,
    fixedChapter?.id,
    fixedChapter?.status,
  ]);

  // 详情页状态条：选中章节在运行中时同步刷新列表状态与整页提示词。
  // 进度对话框期间由 watchMangaPipelineStatus 独占轮询，避免双倍请求。
  useEffect(() => {
    const chapterId = selectedChapter?.id;
    const shouldPoll =
      selectedStatus?.pipeline_status === "running" ||
      selectedStatus?.can_cancel === true;
    if (!chapterId || !shouldPoll) return;
    if (
      pipelineProgressOpen &&
      pipelineChapterId === chapterId &&
      pipelineTaskId
    )
      return;
    const timer = window.setInterval(() => {
      if (!isChapterCurrent(projectId, chapterId)) return;
      void loadSingleStatus(chapterId, projectId).catch((err) =>
        addToast(extractErrorMessage(err, t("comic.loadPipelineStatusFailed")), "error"),
      );
      void loadPagePrompts(chapterId, projectId, { showLoading: false }).catch(
        (err) =>
          addToast(extractErrorMessage(err, t("comic.loadPagePromptsFailed")), "error"),
      );
    }, 2500);
    return () => window.clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    pipelineChapterId,
    pipelineProgressOpen,
    pipelineTaskId,
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

  if (!isSingleMode && chapters.length === 0) {
    return (
      <div className="content-main">
        <MangaChapterList
          chapters={chapters}
          statuses={statuses}
          selectedChapter={selectedChapter}
          onSelect={handleSelect}
        />
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-col gap-3 lg:h-full">
      {staged && <MediaStageHeader id={stageId} label={t("common.mediaAuthoring.stages")} tabs={stageTabs} activeTab={workspaceStage}
        onTabChange={async (stage) => {
          if (!await confirmLeaveStage()) return false;
          setWorkspaceStage(stage);
          if (workspaceStage === "materials") void loadCharacterAssets().catch((error) => addToast(extractErrorMessage(error, t("common.mediaAuthoring.requestFailed")), "error"));
          return true;
        }}>
        {canEditManga && <BookButton size="sm" variant="secondary" disabled={!selectedChapter || selectedStatus?.can_cancel || acting != null} onClick={() => void handleRunPipeline("start", "drawing_prompt")}>{t("common.mediaAuthoring.oneClick")}</BookButton>}
      </MediaStageHeader>}
      {outputState?.stale && (workspaceStage === "prompts" || workspaceStage === "images") && <p role="status" className="text-xs text-warning">{t("common.mediaAuthoring.outputsStale")}</p>}
      {authoring.error && workspaceStage !== "adaptation" && workspaceStage !== "page_plan" && <p role="alert" className="text-xs text-error">{authoring.error}</p>}
      <div
      className={
        isSingleMode
          ? "h-full"
          : showChapterNavigation
            ? "content-wide grid grid-cols-1 gap-3 lg:h-full lg:min-h-0 lg:grid-cols-[220px_minmax(0,1fr)] lg:gap-4"
            : "content-wide lg:h-full lg:min-h-0"
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

      <div id={staged ? `${stageId}-panel` : undefined} role={staged ? "tabpanel" : undefined} aria-labelledby={staged ? `${stageId}-tab-${workspaceStage}` : undefined} className="flex min-h-0 min-w-0 flex-col">
      {workspaceStage === "adaptation" || workspaceStage === "page_plan" ? <MediaAuthoringPanel
        key={`${projectId}:${selectedChapter?.id}:${workspaceStage}:${discardVersion}`} projectId={projectId} kind={workspaceStage === "adaptation" ? "adaptation_plan" : "manga_page_plan"}
        workspace={authoring.workspace} loading={authoring.loading} error={authoring.error} chapterId={selectedChapter?.id ?? null}
        canEdit={canEditManga} stageRunning={selectedStatus?.can_cancel === true} onReload={authoring.reload} onDirtyChange={setAuthoringDirty}
        onBusyChange={setAuthoringBusy}
        onGenerateStage={() => handleRunPipeline("start", "page_planning", false)}
        onNext={() => setWorkspaceStage(workspaceStage === "adaptation" ? "page_plan" : "prompts")}
      /> : workspaceStage === "materials" ? <BaseMaterialsWorkspace projectId={characterAssetProjectId} canEdit={canEditManga}
        onChanged={loadCharacterAssets} onNext={() => { void loadCharacterAssets(); setWorkspaceStage("adaptation"); }} /> : <MangaChapterDetailPanel
        key={`${projectId}:${selectedChapter?.id}:${workspaceStage}:${discardVersion}`}
        stageMode={staged ? workspaceStage === "images" ? "images" : "prompts" : undefined}
        onDirtyChange={setPromptDirty}
        projectId={projectId}
        isSingleMode={isSingleMode}
        selectedChapter={selectedChapter}
        selectedStatus={selectedStatus}
        pagePrompts={pagePrompts}
        pagePromptsLoading={pagePromptsLoading}
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
        onPreviewPipeline={(mode) => void handlePreviewPipeline(mode)}
        onRunPipeline={(mode) => void handleRunPipeline(mode)}
        onGenerateMissingPortraits={() => void handleGenerateMissingPortraits()}
        onCancelPipeline={() => {
          if (!ensureCanEditManga()) return;
          const chapter = selectedChapter;
          const requestProjectId = projectId;
          if (!chapter) return;
          void wrapAct(
            `cancel-${chapter.id}`,
            () => cancelMangaPipeline(requestProjectId, chapter.id),
            t("comic.pipelineCancelRequested"),
          );
        }}
        onDeleteManga={() => {
          if (!ensureCanEditManga()) return;
          const chapter = selectedChapter;
          const requestProjectId = projectId;
          if (!chapter) return;
          const actionKey = `delete-${chapter.id}`;
          if (!beginAction(actionKey)) return;
          void (async () => {
            const ok = await confirmDialog({
              title: t("comic.deleteArtifactsTitle"),
              message: t("comic.deleteArtifactsMessage"),
              confirmText: t("common.delete"),
              dialogType: "danger",
            });
            if (!ok) {
              endAction(actionKey);
              return;
            }
            if (!isChapterCurrent(requestProjectId, chapter.id)) {
              endAction(actionKey);
              return;
            }
            await wrapAct(
              actionKey,
              () => deleteMangaByChapter(requestProjectId, chapter.id),
              t("comic.deletedArtifacts"),
              { actionAlreadyHeld: true },
            );
          })();
        }}
        onPagePromptsChanged={async () => {
          const [promptsOk, imagesOk] = await Promise.all([
            selectedChapter
              ? loadPagePrompts(selectedChapter.id, projectId, { showLoading: false }).catch((err) => {
                  addToast(extractErrorMessage(err, t("comic.loadPagePromptsFailed")), "error");
                  return false;
                })
              : Promise.resolve(false),
            loadProjectImages().catch((err) => {
              addToast(extractErrorMessage(err, t("comic.loadProjectImagesFailed")), "error");
              return false;
            }),
          ]);
          return promptsOk && imagesOk;
        }}
      />}
      </div>
      <ProgressDialog
        open={pipelineProgressOpen}
        title={pipelineProgress.title}
        status={pipelineProgress.status}
        current={pipelineProgress.current}
        total={pipelineProgress.total}
        message={pipelineProgress.message}
        error={pipelineProgress.error}
        onClose={() => setPipelineProgressOpen(false)}
        onCancel={
          pipelineChapterId != null ? handleCancelPipelineProgress : undefined
        }
        cancelLabel={t("comic.cancelPipeline")}
        cancelling={pipelineCancelling}
        runningHint={t("comic.pipelineRunningHint")}
      />
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
