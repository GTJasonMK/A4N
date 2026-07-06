import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { useToast } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import {
  ProgressDialog,
  type ProgressDialogStatus,
} from "../../components/feedback/ProgressDialog";
import { RequestPreviewModal } from "../../components/feedback/RequestPreviewModal";
import { extractErrorMessage } from "../../api";
import {
  listenAppEvent,
  type UnlistenFn,
} from "../../api/runtime";
import { listChapters } from "../../api/chapter";
import type { SourceSnapshotChapter } from "../../api/project";
import { listProfiles, type CharacterProfile } from "../../api/character";
import { listImagesByProject, type GeneratedImage } from "../../api/image";
import {
  generateMissingPortraits,
  listPortraitsByProject,
  type Portrait,
} from "../../api/portrait";
import {
  cancelMangaPipeline,
  deleteMangaByChapter,
  getMangaPipelineStatus,
  listMangaPanels,
  previewResumeMangaPipelineRequest,
  previewStartMangaPipelineRequest,
  resumeMangaPipeline,
  startMangaPipeline,
  type MangaPanel,
  type PipelineStatus,
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
  mangaStageLabel,
  pipelineStartStageValue,
  type MangaPipelineStartStageOption,
} from "./manga/mangaTabUtils";
import { createProgressTaskId, useProgressTask } from "../../hooks/useProgressTask";
import {
  hasSystemDefaultLlmApiKey,
  usesSystemDefaultLlmConfig,
} from "../../api/localRuntimeSettings";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";
import { explainMangaPipelineAiConfigError } from "./manga/mangaPipelineErrors";

interface MangaTabProps {
  projectId: number;
  canEditManga?: boolean;
  snapshotChapters?: SourceSnapshotChapter[];
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

type WorkflowEvent =
  | {
      type: "Progress";
      data: {
        current: number;
        total: number;
        message: string;
      };
    }
  | { type: "Complete"; data: { summary?: string | null } }
  | { type: "Error"; data: { code?: string; message: string } };

const createMangaPipelineProgressState = (): MangaPipelineProgressState => ({
  title: "漫画流水线",
  status: "idle",
  current: 0,
  total: 8,
  message: "",
  error: null,
});

const workflowMessageIsCancelled = (message: string): boolean =>
  isCancelledOperation(message) || message.includes("取消");

const pipelineStatusToProgressStatus = (
  status: PipelineStatus["pipeline_status"],
): ProgressDialogStatus => {
  switch (status) {
    case "completed":
      return "success";
    case "failed":
      return "error";
    case "cancelled":
      return "cancelled";
    case "running":
      return "running";
    default:
      return "running";
  }
};

const pipelineStatusIsTerminal = (status: PipelineStatus["pipeline_status"]): boolean =>
  status === "completed" || status === "failed" || status === "cancelled";

const pipelineProgressMessage = (status: PipelineStatus): string => {
  const message = status.message?.trim();
  if (message) return explainMangaPipelineAiConfigError(message);
  const stage = status.stage ? mangaStageLabel(status.stage) : "";
  return stage || status.pipeline_status || "漫画流水线运行中";
};

const mangaPipelineConfigBlockReason = (): string | null => {
  if (
    usesLocalRuntimeSettings() &&
    usesSystemDefaultLlmConfig() &&
    !hasSystemDefaultLlmApiKey()
  ) {
    return explainMangaPipelineAiConfigError(
      "使用系统默认 LLM 需要先在用户配置页填写并保存你的个人 API Key",
    );
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
  const { addToast } = useToast();
  const { runProgressTask: runPortraitProgressTask } = useProgressTask();
  const mangaWorkflowUnlistenRef = useRef<UnlistenFn | null>(null);
  const hasFixedChapterContext = fixedChapter !== undefined;
  const [chapters, setChapters] = useState<ChapterRow[]>([]);
  const [statuses, setStatuses] = useState<Record<number, PipelineStatus | null>>({});
  const [loading, setLoading] = useState(true);
  const [acting, setActing] = useState<string | null>(null);
  const [selectedChapter, setSelectedChapter] = useState<ChapterRow | null>(fixedChapter ?? null);
  const [panels, setPanels] = useState<MangaPanel[]>([]);
  const [panelsLoading, setPanelsLoading] = useState(false);
  const [currentContentHash, setCurrentContentHash] = useState<string | null>(null);
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [portraits, setPortraits] = useState<Portrait[]>([]);
  const [projectImages, setProjectImages] = useState<GeneratedImage[]>([]);
  const [projectImagesLoading, setProjectImagesLoading] = useState(false);
  const [characterAssetsLoading, setCharacterAssetsLoading] = useState(false);
  const [pipelineStartStage, setPipelineStartStage] =
    useState<MangaPipelineStartStageOption>("");
  const [requestPreviewOpen, setRequestPreviewOpen] = useState(false);
  const [requestPreviewLoading, setRequestPreviewLoading] = useState(false);
  const [requestPreviewData, setRequestPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [pipelineProgressOpen, setPipelineProgressOpen] = useState(false);
  const [pipelineProgress, setPipelineProgress] = useState<MangaPipelineProgressState>(
    createMangaPipelineProgressState,
  );
  const [pipelineTaskId, setPipelineTaskId] = useState<string | null>(null);
  const [pipelineChapterId, setPipelineChapterId] = useState<number | null>(null);
  const [pipelineCancelling, setPipelineCancelling] = useState(false);
  const selectedChapterIdRef = useRef<number | null>(fixedChapter?.id ?? null);

  const isSingleMode = hasFixedChapterContext;
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
  const panelsLoadSeqRef = useRef(0);
  const characterAssetsLoadSeqRef = useRef(0);
  const projectImagesLoadSeqRef = useRef(0);
  const actionSeqRef = useRef(0);
  const previewSeqRef = useRef(0);
  const workflowSeqRef = useRef(0);
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
    addToast("当前账号没有编辑此漫画项目的权限", "error");
    return false;
  }, [addToast, canEditManga]);

  const isCharacterAssetsCurrent = useCallback(
    (requestProjectId: number, requestAssetProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) &&
      activeCharacterAssetProjectIdRef.current === requestAssetProjectId &&
      characterAssetsLoadSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isProjectImagesCurrent = useCallback(
    (requestProjectId: number, requestImageSourceKey: string, requestSeq: number) =>
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
      isProjectCurrent(requestProjectId) && previewSeqRef.current === requestSeq,
    [isProjectCurrent],
  );

  const isWorkflowCurrent = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isProjectCurrent(requestProjectId) && workflowSeqRef.current === requestSeq,
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
    const profileIdsWithPortrait = new Set(portraits.map((portrait) => portrait.profile_id));
    return profiles.filter((profile) => !profileIdsWithPortrait.has(profile.id)).length;
  }, [profiles, portraits]);

  const detachMangaWorkflowListener = () => {
    mangaWorkflowUnlistenRef.current?.();
    mangaWorkflowUnlistenRef.current = null;
  };

  useEffect(() => {
    selectedChapterIdRef.current = selectedChapter?.id ?? null;
  }, [selectedChapter?.id]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      chapterLoadSeqRef.current += 1;
      statusLoadSeqRef.current += 1;
      panelsLoadSeqRef.current += 1;
      characterAssetsLoadSeqRef.current += 1;
      projectImagesLoadSeqRef.current += 1;
      actionSeqRef.current += 1;
      previewSeqRef.current += 1;
      workflowSeqRef.current += 1;
      resetInteractionLocks();
      detachMangaWorkflowListener();
    };
  }, [resetInteractionLocks]);

  const refreshPipelineTarget = (chapterId: number, requestProjectId = projectId) => {
    if (!isProjectCurrent(requestProjectId)) return;
    void loadSingleStatus(chapterId, requestProjectId).catch((err) =>
      addToast(extractErrorMessage(err, "加载漫画流水线状态失败"), "error"),
    );
    if (!isChapterCurrent(requestProjectId, chapterId)) return;
    void loadPanels(chapterId, requestProjectId, { showLoading: false }).catch((err) =>
      addToast(extractErrorMessage(err, "加载整页提示词失败"), "error"),
    );
  };

  const attachMangaWorkflowListener = async (
    taskId: string,
    chapterId: number,
    requestProjectId: number,
    requestWorkflowSeq: number,
  ) => {
    detachMangaWorkflowListener();
    mangaWorkflowUnlistenRef.current = await listenAppEvent<WorkflowEvent>(
      `workflow:${taskId}`,
      (event) => {
        if (!isWorkflowCurrent(requestProjectId, requestWorkflowSeq)) return;
        const payload = event.payload;
        if (payload.type === "Progress") {
          setPipelineProgress((prev) => ({
            ...prev,
            status: "running",
            current: payload.data.current,
            total: Math.max(1, payload.data.total),
            message: payload.data.message,
            error: null,
          }));
          return;
        }
        if (payload.type === "Complete") {
          setPipelineProgress((prev) => ({
            ...prev,
            status: "success",
            current: prev.total,
            message: payload.data.summary ?? "漫画流水线完成",
            error: null,
          }));
          setPipelineTaskId(null);
          setPipelineChapterId(null);
          setPipelineCancelling(false);
          detachMangaWorkflowListener();
          refreshPipelineTarget(chapterId, requestProjectId);
          return;
        }
        if (payload.type === "Error") {
          const cancelled =
            payload.data.code === "cancelled" || workflowMessageIsCancelled(payload.data.message);
          const message = explainMangaPipelineAiConfigError(payload.data.message);
          setPipelineProgress((prev) => ({
            ...prev,
            status: cancelled ? "cancelled" : "error",
            message: cancelled ? message : "漫画流水线失败",
            error: cancelled ? null : message,
          }));
          setPipelineTaskId(null);
          setPipelineChapterId(null);
          setPipelineCancelling(false);
          detachMangaWorkflowListener();
          refreshPipelineTarget(chapterId, requestProjectId);
        }
      },
    );
  };

  const loadCharacterAssets = async (
    requestProjectId = projectId,
    requestAssetProjectId = characterAssetProjectId,
  ) => {
    const requestSeq = characterAssetsLoadSeqRef.current + 1;
    characterAssetsLoadSeqRef.current = requestSeq;
    if (isCharacterAssetsCurrent(requestProjectId, requestAssetProjectId, requestSeq)) {
      setCharacterAssetsLoading(true);
    }
    try {
      const [profileRows, portraitRows] = await Promise.all([
        listProfiles(requestAssetProjectId),
        listPortraitsByProject(requestAssetProjectId),
      ]);
      if (!isCharacterAssetsCurrent(requestProjectId, requestAssetProjectId, requestSeq)) return;
      setProfiles(profileRows);
      setPortraits(portraitRows);
    } catch (err) {
      if (isCharacterAssetsCurrent(requestProjectId, requestAssetProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, "加载角色立绘状态失败"), "error");
      }
    } finally {
      if (isCharacterAssetsCurrent(requestProjectId, requestAssetProjectId, requestSeq)) {
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
    if (isProjectImagesCurrent(requestProjectId, requestImageSourceKey, requestSeq)) {
      setProjectImagesLoading(true);
    }
    try {
      const imageGroups = await Promise.all(requestImageSourceProjectIds.map(listImagesByProject));
      if (!isProjectImagesCurrent(requestProjectId, requestImageSourceKey, requestSeq)) return;
      const byId = new Map<number, GeneratedImage>();
      for (const group of imageGroups) {
        for (const image of group) byId.set(image.id, image);
      }
      setProjectImages([...byId.values()]);
    } catch (err) {
      if (isProjectImagesCurrent(requestProjectId, requestImageSourceKey, requestSeq)) {
        addToast(extractErrorMessage(err, "加载项目图片素材失败"), "error");
      }
    } finally {
      if (isProjectImagesCurrent(requestProjectId, requestImageSourceKey, requestSeq)) {
        setProjectImagesLoading(false);
      }
    }
  };

  const resetChapterDetail = () => {
    selectedChapterIdRef.current = null;
    panelsLoadSeqRef.current += 1;
    statusLoadSeqRef.current += 1;
    resetRequestPreview();
    setSelectedChapter(null);
    setPanels([]);
    setCurrentContentHash(null);
  };

  const openChapterDetail = (row: ChapterRow) => {
    selectedChapterIdRef.current = row.id;
    resetRequestPreview();
    setSelectedChapter(row);
    setPanels([]);
    setCurrentContentHash(null);
    void loadPanels(row.id, projectId).catch((err) =>
      addToast(extractErrorMessage(err, "加载整页提示词失败"), "error"),
    );
    void loadSingleStatus(row.id, projectId).catch((err) =>
      addToast(extractErrorMessage(err, "加载漫画流水线状态失败"), "error"),
    );
  };

  const loadSingleStatus = async (chapterId: number, requestProjectId = projectId) => {
    const requestSeq = statusLoadSeqRef.current + 1;
    statusLoadSeqRef.current = requestSeq;
    const status = await getMangaPipelineStatus(requestProjectId, chapterId);
    if (!isProjectCurrent(requestProjectId) || statusLoadSeqRef.current !== requestSeq) return;
    setStatuses((current) => ({ ...current, [chapterId]: status }));
  };

  const loadChapters = async (requestProjectId = projectId) => {
    const requestSeq = chapterLoadSeqRef.current + 1;
    chapterLoadSeqRef.current = requestSeq;
    if (isSingleMode) {
      if (isProjectCurrent(requestProjectId) && chapterLoadSeqRef.current === requestSeq) {
        setLoading(false);
      }
      return;
    }
    if (isProjectCurrent(requestProjectId) && chapterLoadSeqRef.current === requestSeq) {
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
      const entries = await Promise.all(
        rows.map(async (row) => [row.id, await getMangaPipelineStatus(requestProjectId, row.id)] as const),
      );
      if (!isProjectCurrent(requestProjectId) || chapterLoadSeqRef.current !== requestSeq) return;
      const map: Record<number, PipelineStatus | null> = {};
      for (const [id, status] of entries) map[id] = status;
      setChapters(rows);
      setStatuses(map);
      setSelectedChapter((current) => {
        if (!current) return null;
        return rows.find((row) => row.id === current.id) ?? null;
      });
    } catch (err) {
      if (isProjectCurrent(requestProjectId) && chapterLoadSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, "加载章节失败"), "error");
      }
    } finally {
      if (isProjectCurrent(requestProjectId) && chapterLoadSeqRef.current === requestSeq) {
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
    panelsLoadSeqRef.current += 1;
    characterAssetsLoadSeqRef.current += 1;
    projectImagesLoadSeqRef.current += 1;
    actionSeqRef.current += 1;
    previewSeqRef.current += 1;
    workflowSeqRef.current += 1;
    resetInteractionLocks();
    detachMangaWorkflowListener();
    setChapters([]);
    setStatuses({});
    setPanels([]);
    setCurrentContentHash(null);
    setProfiles([]);
    setPortraits([]);
    setProjectImages([]);
    setProjectImagesLoading(false);
    setCharacterAssetsLoading(false);
    setPanelsLoading(false);
    setActing(null);
    resetRequestPreview();
    setPipelineProgressOpen(false);
    setPipelineProgress(createMangaPipelineProgressState());
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
  }, [
    characterAssetProjectId,
    imageSourceKey,
    imageSourceProjectIds,
    isSingleMode,
    projectId,
    snapshotChapters,
    resetInteractionLocks,
    resetRequestPreview,
  ]);

  const loadPanels = async (
    chapterId: number,
    requestProjectId = projectId,
    options: { showLoading?: boolean } = {},
  ) => {
    const showLoading = options.showLoading ?? true;
    const requestSeq = panelsLoadSeqRef.current + 1;
    panelsLoadSeqRef.current = requestSeq;
    if (showLoading && isChapterCurrent(requestProjectId, chapterId)) {
      setPanelsLoading(true);
    }
    try {
      const data = await listMangaPanels(requestProjectId, chapterId);
      if (
        !isChapterCurrent(requestProjectId, chapterId) ||
        panelsLoadSeqRef.current !== requestSeq
      ) {
        return;
      }
      setPanels(data);
    } finally {
      if (
        isChapterCurrent(requestProjectId, chapterId) &&
        panelsLoadSeqRef.current === requestSeq
      ) {
        setPanelsLoading(false);
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
          addToast(extractErrorMessage(err, "计算章节正文指纹失败"), "error");
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
      addToast(chapterGeneratingMessage(selectedChapter, "操作漫画素材"), "error");
      endAction(key);
      return;
    }
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    setActing(key);
    try {
      await fn();
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      await loadChapters(requestProjectId);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      if (targetChapterId) {
        await Promise.all([
          isChapterCurrent(requestProjectId, targetChapterId)
            ? loadPanels(targetChapterId, requestProjectId)
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
        addToast(extractErrorMessage(err, "操作失败"), "error");
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
      addToast(chapterGeneratingMessage(chapter, "启动漫画流水线"), "error");
      return;
    }
    const isResume = mode === "resume";
    const actionKey = `${mode}-${chapter.chapter_number}`;
    const title = isResume ? "续传漫画流水线" : "启动漫画流水线";
    const requestedStartStage = pipelineStartStageValue(pipelineStartStage);
    const configBlockReason = mangaPipelineConfigBlockReason();
    if (configBlockReason) {
      setPipelineProgressOpen(true);
      setPipelineProgress({
        title,
        status: "error",
        current: 0,
        total: 8,
        message: isResume ? "续传流水线失败" : "启动流水线失败",
        error: configBlockReason,
      });
      addToast(configBlockReason, "error");
      return;
    }
    if (!beginAction(actionKey)) return;
    const actionSeq = actionSeqRef.current + 1;
    actionSeqRef.current = actionSeq;
    const workflowSeq = workflowSeqRef.current + 1;
    workflowSeqRef.current = workflowSeq;

    setActing(actionKey);
    setPipelineProgressOpen(true);
    setPipelineProgress({
      title,
      status: "running",
      current: 0,
      total: 8,
      message: isResume ? "正在续传漫画 8 步流水线" : "正在启动漫画 8 步流水线",
      error: null,
    });
    setPipelineTaskId(null);
    setPipelineChapterId(chapter.id);
    setPipelineCancelling(false);

    try {
      const taskId = isResume
        ? await resumeMangaPipeline(requestProjectId, chapter.chapter_number)
        : await startMangaPipeline(
            requestProjectId,
            chapter.chapter_number,
            requestedStartStage,
          );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      setPipelineTaskId(taskId);
      await attachMangaWorkflowListener(taskId, chapter.id, requestProjectId, workflowSeq);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      setPipelineProgress((prev) => ({
        ...prev,
        status: "running",
        message: "后台任务已提交，正在等待阶段进度",
      }));
      await loadSingleStatus(chapter.id, requestProjectId);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
    } catch (err) {
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      setPipelineTaskId(null);
      setPipelineChapterId(null);
      setPipelineCancelling(false);
      detachMangaWorkflowListener();
      const message = explainMangaPipelineAiConfigError(
        extractErrorMessage(err, isResume ? "续传流水线失败" : "启动流水线失败"),
      );
      setPipelineProgress((prev) => ({
        ...prev,
        status: "error",
        message: isResume ? "续传流水线失败" : "启动流水线失败",
        error: message,
      }));
      addToast(message, "error");
    } finally {
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
      addToast(chapterGeneratingMessage(chapter, "预览漫画流水线请求"), "error");
      return;
    }
    if (!chapter.content?.trim()) {
      addToast("当前章节正文为空，无法预览漫画流水线请求", "error");
      return;
    }
    const requestStartStage = pipelineStartStageValue(pipelineStartStage);
    const previewKey = `${mode}:${requestProjectId}:${chapter.id}:${requestStartStage ?? ""}`;
    if (!beginPreview(previewKey)) return;
    const requestSeq = previewSeqRef.current + 1;
    previewSeqRef.current = requestSeq;
    setRequestPreviewOpen(true);
    setRequestPreviewLoading(true);
    setRequestPreviewData(null);
    try {
      const preview =
        mode === "resume"
          ? await previewResumeMangaPipelineRequest(requestProjectId, chapter.chapter_number)
          : await previewStartMangaPipelineRequest(
              requestProjectId,
              chapter.chapter_number,
              requestStartStage,
            );
      if (!isPreviewCurrent(requestProjectId, requestSeq) || !isChapterCurrent(requestProjectId, chapter.id)) {
        return;
      }
      setRequestPreviewData(preview);
    } catch (err) {
      if (isPreviewCurrent(requestProjectId, requestSeq)) {
        addToast(extractErrorMessage(err, "预览漫画流水线请求失败"), "error");
        setRequestPreviewOpen(false);
        endPreview(previewKey);
      }
    } finally {
      if (isPreviewCurrent(requestProjectId, requestSeq)) setRequestPreviewLoading(false);
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
    addToast("已发送取消漫画流水线请求", "info");
    try {
      await cancelMangaPipeline(requestProjectId, chapterId);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      await Promise.all([
        loadSingleStatus(chapterId, requestProjectId),
        isChapterCurrent(requestProjectId, chapterId)
          ? loadPanels(chapterId, requestProjectId, { showLoading: false })
          : Promise.resolve(),
      ]);
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      setPipelineProgress((prev) => ({
        ...prev,
        status: "cancelled",
        message: "已请求取消漫画流水线",
        error: null,
      }));
      setPipelineCancelling(false);
    } catch (err) {
      if (isActionCurrent(requestProjectId, actionSeq)) {
        addToast(extractErrorMessage(err, "取消漫画流水线失败"), "error");
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
    const taskId = createProgressTaskId("manga-missing-portraits", requestProjectId);
    setActing("generate-missing-portraits");
    try {
      let createdCount = 0;
      await runPortraitProgressTask(
        {
          title: "补齐角色立绘",
          startMessage: "正在统计缺失立绘的角色…",
          successMessage: "角色立绘补齐完成",
          errorMessage: "补齐立绘失败",
          total: 1,
          taskId,
          cancelLabel: "停止生成",
          runningHint: "角色立绘生成过程中请保持当前窗口打开",
        },
        async ({ setProgress }) => {
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          setProgress("正在提交角色立绘补齐任务…", 0, 1);
          const created = await generateMissingPortraits(requestAssetProjectId, taskId);
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
          createdCount = created.length;
          setProgress("正在刷新角色立绘状态…", 1, 1);
          await loadCharacterAssets(requestProjectId, requestAssetProjectId);
          if (!isActionCurrent(requestProjectId, actionSeq)) throw new Error("任务已停止");
        },
      );
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      addToast(
        createdCount > 0 ? `已生成 ${createdCount} 张缺失立绘` : "角色立绘已齐全",
        "success",
      );
    } catch (err) {
      if (!isActionCurrent(requestProjectId, actionSeq)) return;
      const message = extractErrorMessage(err, "补齐立绘失败");
      if (isCancelledOperation(message)) {
        addToast("角色立绘补齐已停止", "info");
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
  }, [fixedChapter?.content, fixedChapter?.chapter_number, fixedChapter?.id, fixedChapter?.status]);

  useEffect(() => {
    if (!pipelineProgressOpen || !selectedStatus) return;
    if (pipelineChapterId != null && selectedChapter?.id !== pipelineChapterId) return;
    if (pipelineChapterId != null && !pipelineTaskId) return;
    if (pipelineTaskId && selectedStatus.task_id !== pipelineTaskId) return;

    const nextStatus = pipelineStatusToProgressStatus(selectedStatus.pipeline_status);
    setPipelineProgress((prev) => ({
      ...prev,
      status: nextStatus,
      current: selectedStatus.current ?? prev.current,
      total: selectedStatus.total ?? prev.total,
      message: pipelineProgressMessage(selectedStatus),
      error:
        nextStatus === "error"
          ? explainMangaPipelineAiConfigError(selectedStatus.message ?? "漫画流水线失败")
          : null,
    }));

    if (pipelineStatusIsTerminal(selectedStatus.pipeline_status)) {
      setPipelineTaskId(null);
      setPipelineChapterId(null);
      setPipelineCancelling(false);
      detachMangaWorkflowListener();
    }
  }, [
    pipelineChapterId,
    pipelineProgressOpen,
    pipelineTaskId,
    selectedChapter?.id,
    selectedStatus,
  ]);

  useEffect(() => {
    const chapterId = selectedChapter?.id;
    const shouldPoll =
      selectedStatus?.pipeline_status === "running" || selectedStatus?.can_cancel === true;
    if (!chapterId || !shouldPoll) return;
    const timer = window.setInterval(() => {
      if (!isChapterCurrent(projectId, chapterId)) return;
      void loadSingleStatus(chapterId, projectId).catch((err) =>
        addToast(extractErrorMessage(err, "加载漫画流水线状态失败"), "error"),
      );
      void loadPanels(chapterId, projectId, { showLoading: false }).catch((err) =>
        addToast(extractErrorMessage(err, "加载整页提示词失败"), "error"),
      );
    }, 2500);
    return () => window.clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedChapter?.id, selectedStatus?.pipeline_status, selectedStatus?.can_cancel]);

  if (loading) {
    return (
      <div className="text-xs text-book-text-muted flex items-center gap-2">
        <Loader2 size={12} className="animate-spin" /> 加载章节…
      </div>
    );
  }

  return (
    <div
      className={
        isSingleMode
          ? "h-full"
          : "content-wide grid grid-cols-1 lg:grid-cols-[280px_minmax(0,1fr)] gap-4"
      }
    >
      {/* 章节列表（仅完整模式） */}
      {!isSingleMode && (
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
        panels={panels}
        panelsLoading={panelsLoading}
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
        pipelineStartStage={pipelineStartStage}
        acting={acting}
        canEditManga={canEditManga}
        onPipelineStartStageChange={setPipelineStartStage}
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
            "已请求取消漫画流水线",
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
              title: "删除漫画产物",
              message: "删除本章全部漫画提示词与相关产物？此操作不可撤销。",
              confirmText: "删除",
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
              "已删除本章漫画产物",
              { actionAlreadyHeld: true },
            );
          })();
        }}
        onPanelsChanged={() => {
          if (selectedChapter) void loadPanels(selectedChapter.id);
          void loadProjectImages();
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
        onCancel={pipelineChapterId != null ? handleCancelPipelineProgress : undefined}
        cancelLabel="取消流水线"
        cancelling={pipelineCancelling}
        runningHint="漫画流水线运行过程中请保持当前窗口打开"
      />
      <RequestPreviewModal
        open={requestPreviewOpen}
        loading={requestPreviewLoading}
        data={requestPreviewData}
        fallbackTitle="漫画流水线请求预览"
        onClose={closeRequestPreview}
      />
    </div>
  );
};
