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
  type ProgressDialogStatus,
} from "../../components/feedback/ProgressDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../../api";
import { listChapters } from "../../api/chapter";
import type { SourceSnapshotChapterSummary } from "../../api/project";
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
import {
  MANGA_PIPELINE_TOTAL_STEPS,
  mangaStageLabel,
} from "./manga/mangaTabUtils";
import { useProgressTask } from "../../hooks/useProgressTask";
import {
  hasSystemDefaultLlmApiKey,
  usesSystemDefaultLlmConfig,
} from "../../api/localRuntimeSettings";
import { explainMangaPipelineAiConfigError } from "./manga/mangaPipelineErrors";
import { useI18n, type MessageTranslator } from "../../i18n";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";

interface MangaTabProps {
  projectId: number;
  canEditManga?: boolean;
  snapshotChapters?: SourceSnapshotChapterSummary[];
  assetProjectId?: number;
  imageProjectIds?: number[];
  /**
   * 当前章节（写作桌联动用）。
   * 设置后跳过左侧章节列表，直接显示该章节的漫画详情。
   */
  fixedChapter?: ChapterRow | null;
}

interface MangaPipelineProgressState {
  title: string;
  status: ProgressDialogStatus;
  current: number;
  total: number;
  message: string;
  error: string | null;
}

const MANGA_PIPELINE_POLL_MS = 1500;

const createMangaPipelineProgressState = (
  t: MessageTranslator,
): MangaPipelineProgressState => ({
  title: t("comic.pipelineTitle"),
  status: "idle",
  current: 0,
  total: MANGA_PIPELINE_TOTAL_STEPS,
  message: "",
  error: null,
});

const pipelineStatusToProgressStatus = (
  status: MangaPipelineStatusSummary["pipeline_status"],
): ProgressDialogStatus => {
  switch (status) {
    case "completed":
      return "success";
    case "failed":
    case "interrupted":
      return "error";
    case "cancelled":
      return "cancelled";
    case "running":
      return "running";
    default:
      return "running";
  }
};

const pipelineStatusIsTerminal = (
  status: MangaPipelineStatusSummary["pipeline_status"],
): boolean =>
  status === "completed" ||
  status === "failed" ||
  status === "interrupted" ||
  status === "cancelled";

const pipelineProgressMessage = (
  status: MangaPipelineStatusSummary,
  t: MessageTranslator,
): string => {
  const message = status.message?.trim();
  if (message) return explainMangaPipelineAiConfigError(message, t);
  const stage = status.stage ? mangaStageLabel(status.stage, t) : "";
  return stage || status.pipeline_status || t("comic.pipelineRunning");
};

const progressStateFromPipelineStatus = (
  status: MangaPipelineStatusSummary,
  title: string,
  t: MessageTranslator,
): MangaPipelineProgressState => {
  const nextStatus = pipelineStatusToProgressStatus(status.pipeline_status);
  return {
    title,
    status: nextStatus,
    current: status.current ?? 0,
    total: Math.max(1, status.total ?? MANGA_PIPELINE_TOTAL_STEPS),
    message: pipelineProgressMessage(status, t),
    error:
      nextStatus === "error"
        ? explainMangaPipelineAiConfigError(
            status.message ?? t("comic.pipelineFailed"),
            t,
          )
        : null,
  };
};

const mangaPipelineConfigBlockReason = (
  t: MessageTranslator,
): string | null => {
  if (
    usesLocalRuntimeSettings() &&
    usesSystemDefaultLlmConfig() &&
    !hasSystemDefaultLlmApiKey()
  ) {
    return t("comic.llmApiKeyRequired");
  }
  return null;
};

export const MangaTab: React.FC<MangaTabProps> = ({
  projectId,
  canEditManga = true,
  snapshotChapters,
  assetProjectId,
  imageProjectIds,
  fixedChapter,
}) => {
  const addToast = useToast((state) => state.addToast);
  const { t } = useI18n();
  const { runProgressTask: runPortraitProgressTask } = useProgressTask();
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

  const refreshPipelineTarget = (
    chapterId: number,
    requestProjectId = projectId,
  ) => {
    if (!isProjectCurrent(requestProjectId)) return;
    void loadSingleStatus(chapterId, requestProjectId).catch((err) =>
      addToast(extractErrorMessage(err, t("comic.loadPipelineStatusFailed")), "error"),
    );
    if (!isChapterCurrent(requestProjectId, chapterId)) return;
    void loadPagePrompts(chapterId, requestProjectId, { showLoading: false }).catch(
      (err) =>
        addToast(extractErrorMessage(err, t("comic.loadPagePromptsFailed")), "error"),
    );
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
            refreshPipelineTarget(chapterId, requestProjectId);
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
  ) => {
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
      const [profileRows, portraitRows] = await Promise.all([
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
        return;
      setProfiles(profileRows);
      setPortraits(portraitRows);
    } catch (err) {
      if (
        isCharacterAssetsCurrent(
          requestProjectId,
          requestAssetProjectId,
          requestSeq,
        )
      ) {
        addToast(extractErrorMessage(err, t("comic.loadCharacterAssetsFailed")), "error");
      }
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
  ) => {
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
        return;
      const byId = new Map<number, GeneratedImage>();
      for (const image of images) byId.set(image.id, image);
      setProjectImages([...byId.values()]);
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
  ) => {
    const requestSeq = statusLoadSeqRef.current + 1;
    statusLoadSeqRef.current = requestSeq;
    const status = await getMangaPipelineStatus(requestProjectId, chapterId);
    if (
      !isProjectCurrent(requestProjectId) ||
      statusLoadSeqRef.current !== requestSeq
    )
      return;
    setStatuses((current) => ({ ...current, [chapterId]: status }));
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
      const statuses = rows.length
        ? await listMangaPipelineStatuses(
            requestProjectId,
            rows.map((row) => row.id),
          )
        : [];
      if (
        !isProjectCurrent(requestProjectId) ||
        chapterLoadSeqRef.current !== requestSeq
      )
        return;
      const map: Record<number, MangaPipelineStatusSummary | null> = {};
      for (const status of statuses) map[status.chapter_id] = status;
      const previousChapterId = selectedChapterIdRef.current;
      const nextSelectedChapter =
        rows.find((row) => row.id === previousChapterId) ??
        (rows.length === 1 ? rows[0] : null);
      setChapters(rows);
      setStatuses(map);
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
  ) => {
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
        return;
      }
      setPagePrompts(data);
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

  const handleSelect = (row: ChapterRow) => openChapterDetail(row);

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
      await fn();
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (targetChapterId) {
        await Promise.all([
          isChapterCurrent(requestProjectId, targetChapterId)
            ? loadPagePrompts(targetChapterId, requestProjectId, {
                showLoading: false,
              })
            : Promise.resolve(),
          loadSingleStatus(targetChapterId, requestProjectId),
        ]);
        if (!isActionCurrent(requestProjectId, actionSeq)) return;
        window.setTimeout(() => {
          if (isActionCurrent(requestProjectId, actionSeq)) {
            refreshPipelineTarget(targetChapterId, requestProjectId);
          }
        }, 1200);
      }
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(ok, "success");
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, t("comic.operationFailed")), "error");
      }
    } finally {
      if (isActionCurrent(requestProjectId, actionSeq)) setActing(null);
      endAction(key);
    }
  };

  const handleRunPipeline = async (mode: "start" | "resume") => {
    if (!ensureCanEditManga()) return;
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
        ? await resumeMangaPipeline(requestProjectId, chapter.chapter_number)
        : await startMangaPipeline(requestProjectId, chapter.chapter_number);
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
      await Promise.all([
        loadSingleStatus(chapterId, requestProjectId),
        isChapterCurrent(requestProjectId, chapterId)
          ? loadPagePrompts(chapterId, requestProjectId, { showLoading: false })
          : Promise.resolve(),
      ]);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
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
    const actionKey = `generate-missing-portraits:${requestAssetProjectId}`;
    if (!beginAction(actionKey)) return;
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    setActing("generate-missing-portraits");
    try {
      const handle = await startPortraitGeneration({
        project_id: requestAssetProjectId,
        operation: { kind: "generate_missing" },
      });
      const createdCount = await runGenerationProgressTaskThen(
        runPortraitProgressTask,
        handle,
        {
          title: t("comic.portraitGenerationTitle"),
          startMessage: t("comic.portraitGenerationStart"),
          successMessage: t("comic.portraitGenerationSuccess"),
          errorMessage: t("comic.portraitGenerationFailed"),
          total: 1,
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

      {/* 右侧详情 */}
      <MangaChapterDetailPanel
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
          await Promise.all([
            selectedChapter
              ? loadPagePrompts(selectedChapter.id, projectId, {
                  showLoading: false,
                }).catch((err) =>
                  addToast(
                    extractErrorMessage(err, t("comic.loadPagePromptsFailed")),
                    "error",
                  ),
                )
              : Promise.resolve(),
            loadProjectImages(),
          ]);
        }}
      />
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
  );
};
