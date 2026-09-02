import React, {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { promptDialog } from "../components/feedback/PromptDialog";
import { useToast } from "../components/feedback/Toast";
import { extractErrorMessage } from "../api";
import { resumeWorkflowRun } from "../api/streaming";
import {
  canEditProjectMaterials,
  getProject,
  hasProjectEditorRole,
  resetProjectGeneratedData,
  type NovelProject,
} from "../api/project";
import {
  chapterContentCommitToastType,
  importChapter,
  listChapters,
  updateChapterContent,
  type ChapterSummary,
} from "../api/chapter";
import {
  isDerivedChapterOutline,
  listChapterOutlines,
  type ChapterOutline,
} from "../api/outline";
import { ChapterSidebar } from "./writing-desk/ChapterSidebar";
import {
  ChapterWorkspace,
  type BottomTab,
} from "./writing-desk/ChapterWorkspace";
import { MissingOutlineBanner } from "./writing-desk/MissingOutlineBanner";
import {
  WritingDeskHeader,
  type RightPanel,
} from "./writing-desk/WritingDeskHeader";
import { RightPanelResizeHandle } from "./writing-desk/RightPanelResizeHandle";
import {
  DEFAULT_GENERATION_WORD_COUNT_RANGE,
  defaultGenerationWordCountRange,
  parseGenerationWordCountRange,
} from "./writing-desk/generationWordCount";
import { countChapterBodyCharacters } from "./writing-desk/wordCount";
import {
  LEFT_SIDEBAR_WIDTH,
  RIGHT_PANEL_RESIZER_WIDTH,
  useRightPanelLayout,
} from "./writing-desk/rightPanelLayout";
import { useStatusBarPermanent } from "../components/ui/StatusBar";
import { useSmartBack } from "../components/ui/BackButton";
import { Modal } from "../components/ui/Modal";
import {
  getProjectHomeEntryRoute,
  getProjectPrimaryRoute,
} from "../utils/projectRouting";
import { getProjectLifecycleBlockedReason } from "../utils/projectStatus";
import type { OptimizationPatchPreviewController } from "./novel-detail/optimize/patchPreviewTypes";
import { useChapterAnalysisActions } from "./writing-desk/useChapterAnalysisActions";
import { useChapterPromptPreview } from "./writing-desk/useChapterPromptPreview";
import { useChapterStreamGeneration } from "./writing-desk/useChapterStreamGeneration";
import { useWritingDeskDrafts } from "./writing-desk/useWritingDeskDrafts";
import { useWritingDeskChapterDerivedState } from "./writing-desk/useWritingDeskChapterDerivedState";
import { formatNumber, useI18n } from "../i18n";

const RagQueryPanel = lazy(() =>
  import("./writing-desk/RagQueryPanel").then((module) => ({
    default: module.RagQueryPanel,
  })),
);
const WritingAssistantPanel = lazy(() =>
  import("./writing-desk/WritingAssistantPanel").then((module) => ({
    default: module.WritingAssistantPanel,
  })),
);
const WritingDeskDialogs = lazy(() =>
  import("./writing-desk/WritingDeskDialogs").then((module) => ({
    default: module.WritingDeskDialogs,
  })),
);
const RequestPreviewModal = lazy(() =>
  import("../components/feedback/RequestPreviewModal").then((module) => ({
    default: module.RequestPreviewModal,
  })),
);

/**
 * 写作桌：章节列表、正文编辑、生成控制和辅助面板。
 */

export const WritingDesk: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const projectId = Number(id);
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const agentRequested = searchParams.get("agent") === "1";

  const [project, setProject] = useState<NovelProject | null>(null);
  const lifecycleBlockedReason = project
    ? getProjectLifecycleBlockedReason(project, t)
    : null;
  const canEditWritingProject = project
    ? canEditProjectMaterials(project)
    : false;
  const [chapters, setChapters] = useState<ChapterSummary[]>([]);
  const [outlines, setOutlines] = useState<ChapterOutline[]>([]);
  const [activeNumber, setActiveNumber] = useState<number | null>(null);
  const [chapterQuery, setChapterQuery] = useState("");
  const [writingNotesByChapter, setWritingNotesByChapter] = useState<Record<string, string>>({});
  const writingNotesKey =
    activeNumber == null ? null : `${projectId}:${activeNumber}`;
  const writingNotes =
    writingNotesKey == null ? "" : writingNotesByChapter[writingNotesKey] ?? "";
  const handleWritingNotesChange = useCallback(
    (value: string) => {
      if (writingNotesKey == null) return;
      setWritingNotesByChapter((current) =>
        current[writingNotesKey] === value
          ? current
          : { ...current, [writingNotesKey]: value },
      );
    },
    [writingNotesKey],
  );
  const [minWordCount, setMinWordCount] = useState(
    String(DEFAULT_GENERATION_WORD_COUNT_RANGE.min),
  );
  const [maxWordCount, setMaxWordCount] = useState(
    String(DEFAULT_GENERATION_WORD_COUNT_RANGE.max),
  );
  const [loading, setLoading] = useState(true);
  const [savingContent, setSavingContent] = useState(false);
  const [resetGeneratedDataBusy, setResetGeneratedDataBusy] = useState(false);
  const [leftSidebarVisible, setLeftSidebarVisible] = useState(true);
  const [isMobileLayout, setIsMobileLayout] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth < 768 : false,
  );
  const [rightPanel, setRightPanel] = useState<RightPanel>(() =>
    agentRequested ? "optimize" : "none",
  );
  const [optimizationPatchPreview, setOptimizationPatchPreview] =
    useState<OptimizationPatchPreviewController | null>(null);
  const {
    rightPanelWidth,
    rightPanelMaxWidth,
    isResizingRightPanel,
    setRightPanelWidth,
    resetRightPanelWidth,
    handleRightPanelResizeStart,
    handleRightPanelResizeMove,
    finishRightPanelResize,
  } = useRightPanelLayout(rightPanel);
  const [bottomTab, setBottomTab] = useState<BottomTab>("content");
  // 写作助手面板沿用 optimize 路由状态，内部可在生成与优化之间切换。
  const assistantOpen = rightPanel === "optimize";
  const rightPanelAvailable = bottomTab === "content" && !lifecycleBlockedReason;
  const rightPanelUnavailableTip =
    lifecycleBlockedReason ?? t("writing.onlyBodyPanel");
  // 章节场景管理对话框（对应 A4N 后端 chapter_scene 子结构）
  const [scenesOpen, setScenesOpen] = useState(false);
  const [profilesOpen, setProfilesOpen] = useState(false);
  const [profilesRefreshKey, setProfilesRefreshKey] = useState(0);
  const mountedRef = useRef(false);
  const projectIdRef = useRef(projectId);
  const loadSeqRef = useRef(0);
  const reloadChaptersInFlightRef = useRef<{
    projectId: number;
    token: symbol;
    promise: Promise<ChapterSummary[] | null>;
  } | null>(null);
  const saveContentBusyRef = useRef(false);
  const resetGeneratedDataBusyRef = useRef(false);
  const derivedSyncBusyRef = useRef(false);
  const smartBack = useSmartBack("/");
  const projectIdKey =
    Number.isFinite(projectId) && projectId > 0 ? String(projectId) : "";
  projectIdRef.current = projectId;

  const handleRightPanelChange = useCallback(
    (panel: RightPanel) => {
      if (panel !== "none" && !rightPanelAvailable) {
        addToast(rightPanelUnavailableTip, "info");
        return;
      }
      setRightPanel(panel);
    },
    [addToast, rightPanelAvailable, rightPanelUnavailableTip],
  );

  const handleBottomTabChange = useCallback((tab: BottomTab) => {
    setBottomTab(tab);
    if (tab !== "content") {
      setRightPanel("none");
    }
  }, []);

  useEffect(() => {
    if (lifecycleBlockedReason) setRightPanel("none");
  }, [lifecycleBlockedReason]);

  const {
    activeChapter,
    activeOutline,
    activeOutlineText,
    visibleOptimizationPatchPreview,
    activeChapterContext,
    chapterNumbers,
    filteredChapterNumbers,
  } = useWritingDeskChapterDerivedState({
    chapters,
    outlines,
    activeNumber,
    chapterQuery,
    optimizationPatchPreview,
  });
  const activeChapterIndex = useMemo(
    () => (activeNumber == null ? -1 : chapterNumbers.indexOf(activeNumber)),
    [activeNumber, chapterNumbers],
  );
  const previousChapterNumber =
    activeChapterIndex > 0 ? chapterNumbers[activeChapterIndex - 1] : null;
  const nextChapterNumber =
    activeChapterIndex >= 0 && activeChapterIndex < chapterNumbers.length - 1
      ? chapterNumbers[activeChapterIndex + 1]
      : null;
  useEffect(() => {
    const range = defaultGenerationWordCountRange(
      isDerivedChapterOutline(activeOutline) ? undefined : activeOutline?.word_count_target,
    );
    setMinWordCount(String(range.min));
    setMaxWordCount(String(range.max));
  }, [activeNumber, activeOutline?.source_kind, activeOutline?.word_count_target]);
  const parsedWordCountRange = useMemo(
    () => parseGenerationWordCountRange(minWordCount, maxWordCount, {
      min: t("writing.minWordCount"),
      max: t("writing.maxWordCount"),
      required: t("writing.wordCountRequiredSuffix"),
      positive: t("writing.wordCountPositiveSuffix"),
      minGreaterMax: t("writing.wordCountMinGreaterMax"),
    }),
    [minWordCount, maxWordCount, t],
  );
  const generationWordCountError = parsedWordCountRange.error;
  const {
    editorContent,
    dirty,
    dirtyRef,
    activeNumberRef,
    draftSet,
    setEditorState,
    getCurrentEditorContent,
    saveLocalDraftForCurrent,
    confirmCurrentEdit,
    refreshDraftSet,
    removeDraftForChapter,
    clearProjectDrafts,
    resetDraftTracking,
  } = useWritingDeskDrafts({
    projectIdKey,
    activeNumber,
    activeChapter,
    addToast,
  });
  // 初始化加载
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      reloadChaptersInFlightRef.current = null;
    };
  }, []);

  useEffect(() => {
    const updateMobileLayout = () => {
      const mobile = window.innerWidth < 768;
      setIsMobileLayout(mobile);
      if (mobile) {
        setLeftSidebarVisible(false);
        if (!agentRequested) setRightPanel("none");
      }
    };
    updateMobileLayout();
    window.addEventListener("resize", updateMobileLayout);
    return () => window.removeEventListener("resize", updateMobileLayout);
  }, [agentRequested]);

  useEffect(() => {
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    saveContentBusyRef.current = false;
    resetGeneratedDataBusyRef.current = false;
    derivedSyncBusyRef.current = false;
    setWritingNotesByChapter({});
    setSavingContent(false);
    const isCurrentLoad = () => mountedRef.current && loadSeqRef.current === requestSeq;

    if (!Number.isFinite(projectId) || projectId <= 0) {
      addToast(t("novel.projectIdInvalid"), "error");
      navigate("/");
      return () => {
        loadSeqRef.current += 1;
      };
    }
    (async () => {
      setLoading(true);
      try {
        const p = await getProject(String(projectId));
        if (!isCurrentLoad()) return;
        if (p.project_kind !== "text") {
          navigate(getProjectHomeEntryRoute(p), { replace: true });
          return;
        }
        if (!hasProjectEditorRole(p)) {
          addToast(t("novel.editPermissionDenied"), "error");
          navigate(getProjectPrimaryRoute(p), { replace: true });
          return;
        }
        const [ch, ol] = await Promise.all([
          listChapters(projectId),
          listChapterOutlines(projectId),
        ]);
        if (!isCurrentLoad()) return;
        const outlineNumbers = ol.map((outline) => outline.chapter_number);
        setProject(p);
        setChapters(ch);
        setOutlines(ol);

        const chapterParam = searchParams.get("chapter");
        const fromUrl = chapterParam == null ? null : Number(chapterParam);
        const fromUrlNumber =
          fromUrl != null && Number.isFinite(fromUrl) && fromUrl > 0
            ? Math.floor(fromUrl)
            : null;
        if (
          fromUrlNumber != null &&
          (ch.some((chapter) => chapter.chapter_number === fromUrlNumber) ||
            outlineNumbers.includes(fromUrlNumber))
        ) {
          setActiveNumber(fromUrlNumber);
          return;
        }
        const latestChapter = ch[ch.length - 1]?.chapter_number;
        if (latestChapter != null) {
          setActiveNumber(latestChapter);
          return;
        }
        setActiveNumber(outlineNumbers[0] ?? 1);
      } catch (err) {
        if (!isCurrentLoad()) return;
        addToast(extractErrorMessage(err, t("novel.loadProject")), "error");
        navigate("/");
      } finally {
        if (isCurrentLoad()) {
          setLoading(false);
        }
      }
    })();
    return () => {
      loadSeqRef.current += 1;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  useEffect(() => {
    if (
      optimizationPatchPreview &&
      optimizationPatchPreview.proposal.chapter_id !== activeChapter?.id
    ) {
      setOptimizationPatchPreview(null);
    }
  }, [activeChapter?.id, optimizationPatchPreview]);

  useEffect(() => {
    if (visibleOptimizationPatchPreview) {
      setBottomTab("content");
    }
  }, [visibleOptimizationPatchPreview]);

  const safeSelectChapter = useCallback(
    async (chapterNumber: number) => {
      if (chapterNumber === activeNumberRef.current) return true;
      const ok = await confirmCurrentEdit(t("writing.switchChapter"));
      if (!ok) return false;
      setActiveNumber(chapterNumber);
      return true;
    },
    [confirmCurrentEdit],
  );

  const handleCreateChapter = useCallback(async () => {
    if (lifecycleBlockedReason) {
      addToast(lifecycleBlockedReason, "info");
      return;
    }
    const ok = await confirmCurrentEdit(t("writing.createChapter"));
    if (!ok) return;

    // chapterNumbers 还包含“当前正在编辑但尚未保存”的占位章节；默认值只看已持久化资料。
    const defaultChapterNumber = Math.max(
      chapters.at(-1)?.chapter_number ?? 0,
      outlines.at(-1)?.chapter_number ?? 0,
    ) + 1;
    const input = await promptDialog({
      title: t("writing.newChapter"),
      message: t("writing.enterChapterNumber"),
      defaultValue: String(defaultChapterNumber),
      placeholder: t("writing.chapterNumberPlaceholder"),
      required: true,
      confirmText: t("writing.enterChapter"),
    });
    if (input == null) return;

    const normalized = input.trim();
    if (!/^[1-9]\d*$/.test(normalized)) {
      addToast(t("writing.chapterNumberPositive"), "error");
      return;
    }
    const chapterNumber = Number(normalized);
    if (!Number.isSafeInteger(chapterNumber) || chapterNumber > 2_147_483_647) {
      addToast(t("writing.chapterNumberRange"), "error");
      return;
    }
    const chapterAlreadyExists =
      chapters.some((chapter) => chapter.chapter_number === chapterNumber) ||
      outlines.some((outline) => outline.chapter_number === chapterNumber);
    if (chapterAlreadyExists) {
      addToast(t("writing.chapterAlreadyExists", { number: formatNumber(chapterNumber, locale) }), "error");
      return;
    }

    setChapterQuery("");
    setActiveNumber(chapterNumber);
  }, [addToast, chapters, confirmCurrentEdit, lifecycleBlockedReason, outlines]);

  const safeNavigateTo = useCallback(
    async (to: string) => {
      const ok = await confirmCurrentEdit(t("writing.leaveWritingDesk"));
      if (ok) navigate(to);
    },
    [confirmCurrentEdit, navigate],
  );

  const safeSmartBack = useCallback(async () => {
    const ok = await confirmCurrentEdit(t("common.back"));
    if (ok) smartBack();
  }, [confirmCurrentEdit, smartBack]);

  useEffect(() => {
    if (activeNumber == null) return;
    const cur = searchParams.get("chapter");
    if (cur === String(activeNumber)) return;

    const next = new URLSearchParams(searchParams);
    next.set("chapter", String(activeNumber));
    setSearchParams(next, { replace: true });
  }, [activeNumber, searchParams, setSearchParams]);

  const wordCount = useMemo(() => {
    // 与后端 chapter.word_count 的 content.chars().count() 保持一致。
    return countChapterBodyCharacters(editorContent);
  }, [editorContent]);
  const analysisBlockedReason = lifecycleBlockedReason
    ? lifecycleBlockedReason
    : activeChapter?.derived_sync_status === "running"
      ? t("writing.savedSyncRunning")
      : activeChapter?.derived_sync_status &&
          activeChapter.derived_sync_status !== "completed"
        ? t("writing.savedSyncFailed")
        : null;
  // 状态栏持续显示字符数、保存状态和当前章节号。
  useStatusBarPermanent(
    "writing-desk",
    `${activeNumber != null ? t("writing.chapterLabel", { number: formatNumber(activeNumber, locale) }) : t("writing.chapterNotSelected")} · ${t("writing.chapterWords", { count: formatNumber(wordCount, locale) })}${dirty ? " *" : ""}`,
  );

  const reloadChapters = useCallback(async (options: { force?: boolean } = {}) => {
    if (
      !options.force &&
      reloadChaptersInFlightRef.current?.projectId === projectId
    ) {
      return reloadChaptersInFlightRef.current.promise;
    }
    const requestProjectId = projectId;
    const requestToken = Symbol(String(requestProjectId));
    const promise = (async () => {
      try {
        const next = await listChapters(requestProjectId);
        if (!mountedRef.current || projectIdRef.current !== requestProjectId) return null;
        setChapters(next);
        setActiveNumber((current) => {
          if (
            current &&
            (next.some((ch) => ch.chapter_number === current) ||
              outlines.some((outline) => outline.chapter_number === current))
          ) {
            return current;
          }
          return (
            next[next.length - 1]?.chapter_number ??
            outlines[0]?.chapter_number ??
            null
          );
        });
        return next;
      } catch (err) {
        if (mountedRef.current && projectIdRef.current === requestProjectId) {
          addToast(extractErrorMessage(err, t("writing.refreshChaptersFailed")), "error");
        }
        return null;
      } finally {
        if (
          reloadChaptersInFlightRef.current?.projectId === requestProjectId &&
          reloadChaptersInFlightRef.current.token === requestToken
        ) {
          reloadChaptersInFlightRef.current = null;
        }
      }
    })();
    reloadChaptersInFlightRef.current = {
      projectId: requestProjectId,
      token: requestToken,
      promise,
    };
    return promise;
  }, [projectId, addToast, outlines]);

  const reloadOutlines = useCallback(async () => {
    const requestProjectId = projectId;
    try {
      const next = await listChapterOutlines(requestProjectId);
      if (!mountedRef.current || projectIdRef.current !== requestProjectId) return;
      setOutlines(next);
    } catch (err) {
      if (mountedRef.current && projectIdRef.current === requestProjectId) {
        addToast(extractErrorMessage(err, t("writing.refreshOutlinesFailed")), "error");
      }
    }
  }, [addToast, projectId]);

  const handleCharacterMaterialSync = useCallback(
    async (requestProjectId: number) => {
      if (projectIdRef.current !== requestProjectId) return;
      await reloadChapters({ force: true });
    },
    [reloadChapters],
  );

  const handleResumeDerivedSync = useCallback(async () => {
    if (lifecycleBlockedReason) {
      addToast(lifecycleBlockedReason, "info");
      return;
    }
    const workflowRunId = activeChapter?.derived_sync_workflow_run_id;
    if (
      !workflowRunId ||
      !activeChapter?.derived_sync_can_resume ||
      derivedSyncBusyRef.current
    ) {
      addToast(t("writing.noSyncTask"), "error");
      return;
    }
    derivedSyncBusyRef.current = true;
    try {
      await resumeWorkflowRun(workflowRunId);
      await reloadChapters({ force: true });
      addToast(t("writing.derivedMaterialsSynced"), "success");
    } catch (error) {
      await reloadChapters({ force: true });
      addToast(
        extractErrorMessage(error, t("writing.derivedSyncFailed")),
        "error",
      );
    } finally {
      derivedSyncBusyRef.current = false;
    }
  }, [activeChapter, addToast, lifecycleBlockedReason, reloadChapters]);

  useEffect(() => {
    if (!chapters.some((chapter) => chapter.derived_sync_status === "running")) {
      return;
    }
    const timer = window.setTimeout(() => {
      void reloadChapters({ force: true });
    }, 1500);
    return () => window.clearTimeout(timer);
  }, [chapters, reloadChapters]);

  const showContentTab = useCallback(() => setBottomTab("content"), []);
  const refreshProfiles = useCallback(
    () => setProfilesRefreshKey((value) => value + 1),
    [],
  );
  const {
    streamTaskId,
    streamBusy,
    streamMessage,
    streamingActiveChapter,
    activeChapterBusy,
    activeChapterGeneratingMessage,
    generationBlockReason,
    canGenerateChapter,
    clearStreamState,
    handleStreamStart,
    handleStreamCancel,
  } = useChapterStreamGeneration({
    projectId,
    projectIdKey,
    activeNumber,
    activeChapter,
    activeOutline,
    activeOutlineText,
    projectBlockedReason: lifecycleBlockedReason,
    writingNotes,
    wordCountRange: parsedWordCountRange.range,
    wordCountRangeError: generationWordCountError,
    addToast,
    setEditorState,
    getCurrentEditorContent,
    dirtyRef,
    reloadChapters,
    onDraftRemoved: refreshDraftSet,
    onShowContentTab: showContentTab,
  });
  const currentChapterAiBlockedReason = lifecycleBlockedReason
    ? lifecycleBlockedReason
    : dirty
      ? t("writing.unsavedAiBlocked")
      : activeChapterBusy
        ? t("writing.generatingAiBlocked")
        : !activeChapter?.content.trim()
          ? t("writing.noSavedBodyAiBlocked")
          : null;
  const {
    analysisBusy,
    handleGenerateAnalysis,
    handleResumeAnalysisSync,
    handlePreviewAnalysisRequest,
    handlePreviewResumeAnalysisSyncRequest,
    requestPreviewOpen: analysisRequestPreviewOpen,
    requestPreviewLoading: analysisRequestPreviewLoading,
    requestPreviewData: analysisRequestPreviewData,
    setRequestPreviewOpen: setAnalysisRequestPreviewOpen,
  } = useChapterAnalysisActions({
    projectId,
    activeChapter,
    activeChapterBusy,
    activeChapterGeneratingMessage,
    analysisBlockedReason,
    dirtyRef,
    addToast,
    reloadChapters,
    reloadOutlines,
    setChapters,
    refreshProfiles,
  });
  const {
    previewOpen,
    previewing,
    previewData,
    handlePreviewPrompt,
    closePromptPreview,
    resetPromptPreview,
  } = useChapterPromptPreview({
    projectId,
    activeNumber,
    writingNotes,
    wordCountRange: parsedWordCountRange.range,
    wordCountRangeError: generationWordCountError,
    addToast,
  });

  const clearResetProjectState = useCallback(() => {
    clearStreamState(null);
    setChapters([]);
    setOutlines([]);
    setActiveNumber(null);
    setChapterQuery("");
    setWritingNotesByChapter({});
    setOptimizationPatchPreview(null);
    setScenesOpen(false);
    setRightPanel("none");
    resetPromptPreview();
    setBottomTab("content");
    setEditorState("", false);
    resetDraftTracking();
    clearProjectDrafts();
    setProfilesRefreshKey((value) => value + 1);
  }, [
    clearProjectDrafts,
    clearStreamState,
    resetDraftTracking,
    resetPromptPreview,
    setEditorState,
  ]);

  useEffect(() => {
    if (project?.lifecycle_gate !== "resetting") return;
    let checking = false;
    let errorReported = false;
    const timer = window.setInterval(() => {
      if (checking) return;
      checking = true;
      void getProject(projectId)
        .then((nextProject) => {
          if (!mountedRef.current || projectIdRef.current !== projectId) return;
          if (nextProject.lifecycle_gate !== "active") return;
          window.clearInterval(timer);
          setProject(nextProject);
          clearResetProjectState();
          addToast(t("novel.resetFinishingComplete"), "success");
        })
        .catch((error) => {
          if (errorReported || !mountedRef.current) return;
          errorReported = true;
          addToast(extractErrorMessage(error, t("novel.resetProgressFailed")), "error");
        })
        .finally(() => {
          checking = false;
        });
    }, 1500);
    return () => window.clearInterval(timer);
  }, [addToast, clearResetProjectState, project?.lifecycle_gate, projectId]);

  const resetGeneratedDataDisabledReason = useMemo(() => {
    if (!project) return t("writing.projectNotLoaded");
    if (lifecycleBlockedReason) return lifecycleBlockedReason;
    if (resetGeneratedDataBusy) return t("writing.resettingProjectData");
    if (streamTaskId) return t("writing.streamGenerationResetBlocked");
    if (streamBusy) return t("writing.streamStateResetBlocked");
    if (chapters.some((chapter) => chapter.status === "generating")) {
      return t("writing.chapterGenerationResetBlocked");
    }
    return null;
  }, [
    chapters,
    lifecycleBlockedReason,
    project,
    resetGeneratedDataBusy,
    streamBusy,
    streamTaskId,
  ]);
  const canResetGeneratedData = resetGeneratedDataDisabledReason == null;

  const handleOptimizedChapterContentChanged = useCallback(
    async (chapterId: number) => {
      const next = await reloadChapters({ force: true });
      const updated = next?.find((chapter) => chapter.id === chapterId) ?? null;
      if (!updated || activeChapter?.id !== chapterId) return;

      if (dirtyRef.current) {
        saveLocalDraftForCurrent();
        addToast(
          t("writing.optimizedBodyNotOverwritten"),
          "info",
        );
        return;
      }

      setEditorState(updated.content, false);
    },
    [
      activeChapter?.id,
      addToast,
      reloadChapters,
      saveLocalDraftForCurrent,
      setEditorState,
    ],
  );

  const handleResetGeneratedData = async () => {
    if (resetGeneratedDataBusyRef.current) {
      addToast(t("writing.resettingProjectData"), "error");
      return;
    }
    if (resetGeneratedDataDisabledReason) {
      addToast(resetGeneratedDataDisabledReason, "error");
      return;
    }
    resetGeneratedDataBusyRef.current = true;
    const ok = await confirmDialog({
      title: t("writing.resetProjectData"),
      message: t("writing.resetProjectDataMessage"),
      confirmText: t("common.reset"),
      cancelText: t("common.cancel"),
      dialogType: "danger",
    });
    if (!ok) {
      resetGeneratedDataBusyRef.current = false;
      return;
    }

    setResetGeneratedDataBusy(true);
    try {
      const response = await resetProjectGeneratedData(projectId);
      const warnings = [...response.warnings];
      const formalResetCommitted = response.database_committed;
      let nextProject = response.result?.project ?? null;
      if (!nextProject) {
        try {
          nextProject = await getProject(projectId);
        } catch (error) {
          warnings.push(extractErrorMessage(error, t("writing.refreshProjectStatusFailed")));
        }
      }
      if (nextProject) {
        setProject(nextProject);
      } else if (response.status === "pending") {
        setProject((current) =>
          current ? { ...current, lifecycle_gate: "resetting" } : current,
        );
      }
      if (formalResetCommitted) clearResetProjectState();

      const warningText = warnings.length > 0
        ? `${t("common.summarySeparator")}${warnings.join(t("common.summarySeparator"))}`
        : "";
      if (response.status === "pending") {
        addToast(
          formalResetCommitted
            ? t("writing.resetProjectDataCommitted", { warnings: warningText })
            : t("writing.resetProjectDataAccepted", { warnings: warningText }),
          "info",
        );
      } else if (response.result) {
        addToast(
          t("writing.resetProjectDataSummary", {
            chapters: response.result.deleted_chapters,
            outlines: response.result.deleted_chapter_outlines,
            profiles: response.result.deleted_character_profiles,
            versions: response.result.preserved_blueprint_versions,
            warnings: warningText,
          }),
          warnings.length > 0 ? "info" : "success",
        );
      } else {
        addToast(
          t("writing.resetProjectDataDone", { warnings: warningText }),
          warnings.length > 0 ? "info" : "success",
        );
      }
    } catch (err) {
      addToast(extractErrorMessage(err, t("writing.resetProjectDataFailed")), "error");
    } finally {
      resetGeneratedDataBusyRef.current = false;
      setResetGeneratedDataBusy(false);
    }
  };

  const handleSaveContent = async () => {
    if (lifecycleBlockedReason) {
      addToast(lifecycleBlockedReason, "info");
      return;
    }
    if (saveContentBusyRef.current) {
      addToast(t("writing.savingChapterBody"), "error");
      return;
    }
    saveContentBusyRef.current = true;
    setSavingContent(true);
    try {
      const chapterNumber = activeNumber;
      if (!chapterNumber) {
        addToast(t("writing.chooseChapterFirst"), "error");
        return;
      }
      if (activeChapterBusy) {
        addToast(activeChapterGeneratingMessage, "error");
        return;
      }
      const contentToSave = editorContent;
      if (!contentToSave.trim()) {
        addToast(t("writing.savingEmptyBody"), "error");
        return;
      }
      let savedContent = contentToSave;
      let savedChapter: ChapterSummary | null = null;
      if (activeChapter) {
        const updated = await updateChapterContent({
          chapter_id: activeChapter.id,
          content: contentToSave,
          title: activeChapter.title,
        });
        savedContent = updated.content;
        savedChapter = updated;
        setChapters((prev) =>
          prev.map((ch) => (ch.id === updated.id ? updated : ch)),
        );
      } else {
        const title = activeOutline?.title?.trim() || t("writing.chapterLabel", { number: formatNumber(chapterNumber, locale) });
        const imported = await importChapter({
          project_id: projectId,
          chapter_number: chapterNumber,
          content: contentToSave,
          title,
        });
        const nextChapters = await reloadChapters({ force: true });
        const created = nextChapters?.find(
          (chapter) => chapter.chapter_number === chapterNumber,
        );
        savedChapter = created ?? imported;
        savedContent = savedChapter.content;
      }
      const currentContent = getCurrentEditorContent();
      const saveStillMatchesEditor =
        activeNumberRef.current === chapterNumber &&
        currentContent === contentToSave;
      if (saveStillMatchesEditor) {
        removeDraftForChapter(chapterNumber);
        setEditorState(savedContent, false);
        if (savedChapter?.derived_sync_status === "failed") {
          addToast(
            t("writing.savedBodySyncFailed"),
            chapterContentCommitToastType(savedChapter.derived_sync_status),
          );
        } else if (savedChapter?.derived_sync_status === "running") {
          addToast(t("writing.savedBodySyncRunning"), "info");
        } else {
          addToast(t("writing.savedBodyAndMaterials"), "success");
        }
        return;
      }

      saveLocalDraftForCurrent();
      addToast(t("writing.savedBodyWithUnsavedEdits"), "info");
    } catch (err) {
      addToast(extractErrorMessage(err, t("errors.saveFailed")), "error");
    } finally {
      saveContentBusyRef.current = false;
      setSavingContent(false);
    }
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "s")
        return;
      event.preventDefault();
      if (!dirtyRef.current) return;
      void handleSaveContent();
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  const handleOpenScenes = () => {
    if (lifecycleBlockedReason) {
      addToast(lifecycleBlockedReason, "info");
      return;
    }
    if (!activeChapter) {
      addToast(t("writing.chooseGeneratedChapterFirst"), "error");
      return;
    }
    if (activeChapterBusy) {
      addToast(activeChapterGeneratingMessage, "error");
      return;
    }
    setScenesOpen(true);
  };

  const handlePrev = () => {
    if (previousChapterNumber == null) return;
    void safeSelectChapter(previousChapterNumber);
  };
  const handleNext = () => {
    if (nextChapterNumber == null) return;
    void safeSelectChapter(nextChapterNumber);
  };
  const handleToggleLeftSidebar = () => {
    setLeftSidebarVisible((visible) => !visible);
  };

  const writingDeskGridTemplateColumns =
    `${!isMobileLayout && leftSidebarVisible ? `${LEFT_SIDEBAR_WIDTH}px ` : ""}minmax(0,1fr)` +
    (isMobileLayout || rightPanel === "none"
      ? ""
      : ` ${RIGHT_PANEL_RESIZER_WIDTH}px ${rightPanelWidth}px`);

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center text-book-text-muted">
        <LoadingSpinner size="lg" className="mr-2" /> {t("writing.loadingDesk")}
      </div>
    );
  }
  if (!project) return null;

  const writingAssistantPanelProps = {
    projectId,
    initialMode: agentRequested ? "optimize" as const : undefined,
    writingNotes,
    minWordCount,
    maxWordCount,
    wordCountRangeError: generationWordCountError,
    activeNumber,
    activeChapter,
    activeChapterBusy,
    canGenerateChapter,
    generationBlockReason,
    streamTaskId,
    streamBusy,
    streamMessage,
    resetBusy: resetGeneratedDataBusy,
    canResetGeneratedData,
    previewing,
    chapters,
    outlines,
    onWritingNotesChange: handleWritingNotesChange,
    onMinWordCountChange: setMinWordCount,
    onMaxWordCountChange: setMaxWordCount,
    onStreamStart: handleStreamStart,
    onStreamCancel: handleStreamCancel,
    onResetGeneratedData: handleResetGeneratedData,
    onPreviewPrompt: handlePreviewPrompt,
    onOpenScenes: handleOpenScenes,
    onChapterContentChanged: handleOptimizedChapterContentChanged,
    onPatchPreviewChange: setOptimizationPatchPreview,
  };

  return (
    <div className="flex h-full w-full min-w-0 flex-col overflow-hidden">
      {lifecycleBlockedReason ? (
        <div
          role="status"
          className="shrink-0 border-b border-warning/30 bg-warning-bg px-4 py-2 text-small text-book-text-main"
        >
          {lifecycleBlockedReason}
        </div>
      ) : null}

      {!lifecycleBlockedReason && outlines.length === 0 && chapters.length === 0 && (
        <MissingOutlineBanner
          onOpenProjectDetail={() => safeNavigateTo(getProjectPrimaryRoute(project))}
        />
      )}

      <WritingDeskHeader
        project={project}
        activeNumber={activeNumber}
        isMobileLayout={isMobileLayout}
        canGoPrev={previousChapterNumber != null}
        canGoNext={nextChapterNumber != null}
        chapterNumbers={chapterNumbers}
        chapters={chapters}
        outlines={outlines}
        leftSidebarVisible={!isMobileLayout && leftSidebarVisible}
        rightPanel={rightPanel}
        dirty={dirty}
        savingContent={savingContent}
        editBlockedReason={lifecycleBlockedReason}
        activeChapterBusy={activeChapterBusy}
        activeChapterGeneratingMessage={activeChapterGeneratingMessage}
        rightPanelAvailable={rightPanelAvailable}
        rightPanelUnavailableTip={rightPanelUnavailableTip}
        onSmartBack={safeSmartBack}
        onOpenProjectDetail={() => safeNavigateTo(getProjectPrimaryRoute(project))}
        onCreateChapter={handleCreateChapter}
        onPrev={handlePrev}
        onNext={handleNext}
        onSelectChapter={safeSelectChapter}
        onToggleLeftSidebar={handleToggleLeftSidebar}
        onOpenProfiles={() => setProfilesOpen(true)}
        onRightPanelChange={handleRightPanelChange}
        onSaveContent={handleSaveContent}
      />

      {/* 布局：左 sidebar（可隐藏）+ 中 workspace + 右 panel（按 rightPanel 切换） */}
      <div
        className="grid min-h-0 min-w-0 flex-1 gap-0 overflow-hidden"
        style={{ gridTemplateColumns: writingDeskGridTemplateColumns }}
      >
        {!isMobileLayout && leftSidebarVisible && (
          <ChapterSidebar
            projectId={projectId}
            chapters={chapters}
            outlines={outlines}
            chapterNumbers={chapterNumbers}
            filteredChapterNumbers={filteredChapterNumbers}
            activeNumber={activeNumber}
            activeEditorContent={editorContent}
            activeOutline={activeOutline}
            chapterQuery={chapterQuery}
            draftSet={draftSet}
            profilesRefreshKey={profilesRefreshKey}
            onChapterQueryChange={setChapterQuery}
            onClearChapterQuery={() => setChapterQuery("")}
            onOpenProfiles={() => setProfilesOpen(true)}
            onRefreshChapters={reloadChapters}
            onCreateChapter={handleCreateChapter}
            onSelectChapter={safeSelectChapter}
          />
        )}

        <ChapterWorkspace
          projectId={projectId}
          activeNumber={activeNumber}
          activeChapter={activeChapter}
          activeOutline={activeOutline}
          activeChapterContext={activeChapterContext}
          activeChapterBusy={activeChapterBusy}
          activeChapterGeneratingMessage={activeChapterGeneratingMessage}
          analysisBlockedReason={analysisBlockedReason}
          editorContent={editorContent}
          dirty={dirty}
          savingContent={savingContent}
          optimizationPatchPreview={visibleOptimizationPatchPreview}
          autoScrollEditorContent={streamingActiveChapter}
          wordCount={wordCount}
          generatingAnalysis={analysisBusy}
          bottomTab={bottomTab}
          onBottomTabChange={handleBottomTabChange}
          onEditorContentChange={(content) => setEditorState(content, true)}
          onGenerateAnalysis={handleGenerateAnalysis}
          onPreviewGenerateAnalysis={handlePreviewAnalysisRequest}
          onResumeAnalysisSync={handleResumeAnalysisSync}
          onPreviewResumeAnalysisSync={handlePreviewResumeAnalysisSyncRequest}
          onResumeDerivedSync={handleResumeDerivedSync}
          onOpenEmbeddingSettings={() => safeNavigateTo("/settings#embedding")}
          canEdit={canEditWritingProject}
        />

        {!isMobileLayout && rightPanel !== "none" && (
          <RightPanelResizeHandle
            panel={rightPanel}
            width={rightPanelWidth}
            maxWidth={rightPanelMaxWidth}
            isResizing={isResizingRightPanel}
            onResizeStart={handleRightPanelResizeStart}
            onResizeMove={handleRightPanelResizeMove}
            onResizeEnd={finishRightPanelResize}
            onSetWidth={setRightPanelWidth}
            onResetWidth={resetRightPanelWidth}
          />
        )}

        {!isMobileLayout && rightPanel === "rag" && (
          <Suspense fallback={<WritingDeskPanelFallback text={t("writing.loadingMaterials")} />}>
            <RagQueryPanel projectId={projectId} chapterNumber={activeNumber} />
          </Suspense>
        )}

        {!isMobileLayout && assistantOpen && (
          <Suspense fallback={<WritingDeskPanelFallback text={t("writing.loadingAssistant")} />}>
            <WritingAssistantPanel {...writingAssistantPanelProps} />
          </Suspense>
        )}
      </div>

      {isMobileLayout && rightPanel !== "none" && (
        <Modal
          isOpen
          onClose={() => setRightPanel("none")}
          title={rightPanel === "rag" ? t("writing.rag") : t("writing.assistant")}
          maxWidthClassName="max-w-3xl"
          bodyClassName="min-h-0 flex-1 overflow-hidden p-0"
        >
          <Suspense fallback={<WritingDeskPanelFallback text={t("writing.loadingPanel")} />}>
            {rightPanel === "rag" ? (
              <RagQueryPanel
                projectId={projectId}
                chapterNumber={activeNumber}
                embeddedInModal
              />
            ) : (
              <WritingAssistantPanel {...writingAssistantPanelProps} embeddedInModal />
            )}
          </Suspense>
        </Modal>
      )}

      {scenesOpen || previewOpen || profilesOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <WritingDeskDialogs
            projectId={projectId}
            activeNumber={activeNumber}
            activeChapterId={activeChapter?.id ?? null}
            scenesOpen={scenesOpen}
            previewOpen={previewOpen}
            profilesOpen={profilesOpen}
            profilesRefreshKey={profilesRefreshKey}
            sceneAiBlockedReason={currentChapterAiBlockedReason}
            characterAiBlockedReason={currentChapterAiBlockedReason}
            previewing={previewing}
            previewData={previewData}
            onScenesClose={() => setScenesOpen(false)}
            onPreviewClose={closePromptPreview}
            onCharacterMaterialSync={handleCharacterMaterialSync}
            onProfilesClose={() => {
              setProfilesOpen(false);
              setProfilesRefreshKey((value) => value + 1);
            }}
          />
        </Suspense>
      ) : null}
      {analysisRequestPreviewOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <RequestPreviewModal
            open
            loading={analysisRequestPreviewLoading}
            data={analysisRequestPreviewData}
            fallbackTitle={t("writing.chapterAnalysisPreview")}
            onClose={() => setAnalysisRequestPreviewOpen(false)}
          />
        </Suspense>
      ) : null}
    </div>
  );
};

const WritingDeskPanelFallback: React.FC<{ text: string }> = ({ text }) => (
  <div className="a4n-desk-rail a4n-motion-panel-enter flex h-full min-h-0 items-center justify-center border-l border-book-border/80 text-xs text-book-text-muted">
    <LoadingSpinner size="sm" className="mr-2" />
    {text}
  </div>
);
