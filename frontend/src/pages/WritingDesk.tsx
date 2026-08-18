import React, {
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
  projectLifecycleBlockedReason,
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
import { RagQueryPanel } from "./writing-desk/RagQueryPanel";
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
import { WritingDeskDialogs } from "./writing-desk/WritingDeskDialogs";
import { WritingAssistantPanel } from "./writing-desk/WritingAssistantPanel";
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
import { RequestPreviewModal } from "../components/feedback/RequestPreviewModal";
import type { OptimizationPatchPreviewController } from "./novel-detail/optimize/patchPreviewTypes";
import { useChapterAnalysisActions } from "./writing-desk/useChapterAnalysisActions";
import { useChapterPromptPreview } from "./writing-desk/useChapterPromptPreview";
import { useChapterStreamGeneration } from "./writing-desk/useChapterStreamGeneration";
import { useWritingDeskDrafts } from "./writing-desk/useWritingDeskDrafts";
import { useWritingDeskChapterDerivedState } from "./writing-desk/useWritingDeskChapterDerivedState";

/**
 * 写作桌：章节列表、正文编辑、生成控制和辅助面板。
 */

export const WritingDesk: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const projectId = Number(id);
  const navigate = useNavigate();
  const { addToast } = useToast();

  const [project, setProject] = useState<NovelProject | null>(null);
  const lifecycleBlockedReason = project
    ? projectLifecycleBlockedReason(project)
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
  const [resetGeneratedDataBusy, setResetGeneratedDataBusy] = useState(false);
  const [leftSidebarVisible, setLeftSidebarVisible] = useState(true);
  const [isMobileLayout, setIsMobileLayout] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth < 768 : false,
  );
  const [rightPanel, setRightPanel] = useState<RightPanel>("none");
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
    lifecycleBlockedReason ?? "只有当前页面在“正文”时才能打开资料检索或正文优化";
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
    () => parseGenerationWordCountRange(minWordCount, maxWordCount),
    [minWordCount, maxWordCount],
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
        setRightPanel("none");
      }
    };
    updateMobileLayout();
    window.addEventListener("resize", updateMobileLayout);
    return () => window.removeEventListener("resize", updateMobileLayout);
  }, []);

  useEffect(() => {
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    saveContentBusyRef.current = false;
    resetGeneratedDataBusyRef.current = false;
    derivedSyncBusyRef.current = false;
    setWritingNotesByChapter({});
    const isCurrentLoad = () => mountedRef.current && loadSeqRef.current === requestSeq;

    if (!Number.isFinite(projectId) || projectId <= 0) {
      addToast("无效的项目 ID", "error");
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
          addToast("当前账号没有编辑此项目的权限", "error");
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
        addToast(extractErrorMessage(err, "加载项目失败"), "error");
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
      const ok = await confirmCurrentEdit("切换章节");
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
    const ok = await confirmCurrentEdit("新建章节");
    if (!ok) return;

    // chapterNumbers 还包含“当前正在编辑但尚未保存”的占位章节；默认值只看已持久化资料。
    const defaultChapterNumber = Math.max(
      chapters.at(-1)?.chapter_number ?? 0,
      outlines.at(-1)?.chapter_number ?? 0,
    ) + 1;
    const input = await promptDialog({
      title: "新建章节",
      message: "输入章节号。可以跳过空档，直接开始任意章节。",
      defaultValue: String(defaultChapterNumber),
      placeholder: "例如：10",
      required: true,
      confirmText: "进入章节",
    });
    if (input == null) return;

    const normalized = input.trim();
    if (!/^[1-9]\d*$/.test(normalized)) {
      addToast("章节号必须是正整数", "error");
      return;
    }
    const chapterNumber = Number(normalized);
    if (!Number.isSafeInteger(chapterNumber) || chapterNumber > 2_147_483_647) {
      addToast("章节号超出可用范围", "error");
      return;
    }
    const chapterAlreadyExists =
      chapters.some((chapter) => chapter.chapter_number === chapterNumber) ||
      outlines.some((outline) => outline.chapter_number === chapterNumber);
    if (chapterAlreadyExists) {
      addToast(`第 ${chapterNumber} 章已存在，请直接选择该章节`, "error");
      return;
    }

    setChapterQuery("");
    setActiveNumber(chapterNumber);
  }, [addToast, chapters, confirmCurrentEdit, lifecycleBlockedReason, outlines]);

  const safeNavigateTo = useCallback(
    async (to: string) => {
      const ok = await confirmCurrentEdit("离开写作桌");
      if (ok) navigate(to);
    },
    [confirmCurrentEdit, navigate],
  );

  const safeSmartBack = useCallback(async () => {
    const ok = await confirmCurrentEdit("返回上一页");
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
      ? "正文已保存，派生资料仍在同步；同步完成后才能开始分析"
      : activeChapter?.derived_sync_status &&
          activeChapter.derived_sync_status !== "completed"
        ? "正文已保存，但派生资料同步未完成；请先重试资料同步"
        : null;
  // 状态栏持续显示字符数、保存状态和当前章节号。
  useStatusBarPermanent(
    "writing-desk",
    `${activeNumber != null ? `第 ${activeNumber} 章` : "未选择"} · ${wordCount} 字${dirty ? " *" : ""}`,
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
          addToast(extractErrorMessage(err, "刷新章节失败"), "error");
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
        addToast(extractErrorMessage(err, "刷新章节大纲失败"), "error");
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
      addToast("当前章节没有可重试的资料同步任务", "error");
      return;
    }
    derivedSyncBusyRef.current = true;
    try {
      await resumeWorkflowRun(workflowRunId);
      await reloadChapters({ force: true });
      addToast("章节派生资料已同步", "success");
    } catch (error) {
      await reloadChapters({ force: true });
      addToast(
        extractErrorMessage(error, "正文已保留，但派生资料同步仍未完成"),
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
      ? "当前章节有未保存正文，请先保存后再运行 AI 操作"
      : activeChapterBusy
        ? "当前章节正文仍在生成，请等待完成后再运行 AI 操作"
        : !activeChapter?.content.trim()
          ? "当前章节还没有已保存正文，请先完成并保存正文"
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
          addToast("项目重置收尾完成，可以继续创作", "success");
        })
        .catch((error) => {
          if (errorReported || !mountedRef.current) return;
          errorReported = true;
          addToast(extractErrorMessage(error, "查询项目重置进度失败"), "error");
        })
        .finally(() => {
          checking = false;
        });
    }, 1500);
    return () => window.clearInterval(timer);
  }, [addToast, clearResetProjectState, project?.lifecycle_gate, projectId]);

  const resetGeneratedDataDisabledReason = useMemo(() => {
    if (!project) return "项目尚未加载";
    if (lifecycleBlockedReason) return lifecycleBlockedReason;
    if (resetGeneratedDataBusy) return "正在重置项目生成数据";
    if (streamTaskId) return "章节流式生成中，不能重置项目数据";
    if (streamBusy) return "章节流式生成状态正在更新，不能重置项目数据";
    if (chapters.some((chapter) => chapter.status === "generating")) {
      return "有章节正在生成，不能重置项目数据";
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
          "内容优化已写回章节正文；当前编辑器有未保存修改，未自动覆盖。",
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
      addToast("正在重置项目生成数据", "error");
      return;
    }
    if (resetGeneratedDataDisabledReason) {
      addToast(resetGeneratedDataDisabledReason, "error");
      return;
    }
    resetGeneratedDataBusyRef.current = true;
    const ok = await confirmDialog({
      title: "重置项目生成数据",
      message:
        "将删除当前蓝图、章节正文、分部/章节大纲、导入审核、优化会话、生成图片/视频、任务队列、RAG 索引、角色快照、角色当前同步态和正文派生角色档案。\n\n会保留项目本身、蓝图版本历史、非正文派生角色基础档案和提示词配置。重置后可以从蓝图版本恢复当前蓝图。\n\n此操作不可恢复，是否继续？",
      confirmText: "重置",
      cancelText: "取消",
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
          warnings.push(extractErrorMessage(error, "刷新项目状态失败"));
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

      const warningText = warnings.length > 0 ? `；${warnings.join("；")}` : "";
      if (response.status === "pending") {
        addToast(
          formalResetCommitted
            ? `正式资料已重置，后台正在清理索引并恢复编辑权限${warningText}`
            : `重置请求已接受，后台正在执行；完成前项目暂不可编辑${warningText}`,
          "info",
        );
      } else if (response.result) {
        addToast(
          `已重置：删除 ${response.result.deleted_chapters} 章、${response.result.deleted_chapter_outlines} 条章纲、${response.result.deleted_character_profiles} 个正文派生角色，保留 ${response.result.preserved_blueprint_versions} 个蓝图版本${warningText}。`,
          warnings.length > 0 ? "info" : "success",
        );
      } else {
        addToast(
          `项目生成数据已重置${warningText}`,
          warnings.length > 0 ? "info" : "success",
        );
      }
    } catch (err) {
      addToast(extractErrorMessage(err, "重置项目生成数据失败"), "error");
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
      addToast("正在保存章节正文", "error");
      return;
    }
    saveContentBusyRef.current = true;
    try {
      const chapterNumber = activeNumber;
      if (!chapterNumber) {
        addToast("请先选择章节", "error");
        return;
      }
      if (activeChapterBusy) {
        addToast(activeChapterGeneratingMessage, "error");
        return;
      }
      const contentToSave = editorContent;
      if (!contentToSave.trim()) {
        addToast("正文为空，未保存", "error");
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
        const title = activeOutline?.title?.trim() || `第 ${chapterNumber} 章`;
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
            "章节正文已保存，但派生资料同步失败；可在章节顶部重试",
            chapterContentCommitToastType(savedChapter.derived_sync_status),
          );
        } else if (savedChapter?.derived_sync_status === "running") {
          addToast("章节正文已保存，派生资料正在同步", "info");
        } else {
          addToast("章节正文与派生资料已保存", "success");
        }
        return;
      }

      saveLocalDraftForCurrent();
      addToast("章节正文已保存；当前编辑器还有未保存修改", "info");
    } catch (err) {
      addToast(extractErrorMessage(err, "保存失败"), "error");
    } finally {
      saveContentBusyRef.current = false;
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
      addToast("请先选择已生成的章节", "error");
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
        <LoadingSpinner size="lg" className="mr-2" /> 加载写作桌…
      </div>
    );
  }
  if (!project) return null;

  const writingAssistantPanelProps = {
    projectId,
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
          {lifecycleBlockedReason}。页面会在收尾完成后自动恢复。
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
        leftSidebarVisible={!isMobileLayout && leftSidebarVisible}
        rightPanel={rightPanel}
        dirty={dirty}
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
          <RagQueryPanel projectId={projectId} chapterNumber={activeNumber} />
        )}

        {!isMobileLayout && assistantOpen && (
          <WritingAssistantPanel {...writingAssistantPanelProps} />
        )}
      </div>

      {isMobileLayout && rightPanel !== "none" && (
        <Modal
          isOpen
          onClose={() => setRightPanel("none")}
          title={rightPanel === "rag" ? "资料检索" : "写作助手"}
          maxWidthClassName="max-w-3xl"
          bodyClassName="min-h-0 flex-1 overflow-hidden p-0"
        >
          {rightPanel === "rag" ? (
            <RagQueryPanel
              projectId={projectId}
              chapterNumber={activeNumber}
              embeddedInModal
            />
          ) : (
            <WritingAssistantPanel {...writingAssistantPanelProps} embeddedInModal />
          )}
        </Modal>
      )}

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
      <RequestPreviewModal
        open={analysisRequestPreviewOpen}
        loading={analysisRequestPreviewLoading}
        data={analysisRequestPreviewData}
        fallbackTitle="章节分析请求预览"
        onClose={() => setAnalysisRequestPreviewOpen(false)}
      />
    </div>
  );
};
