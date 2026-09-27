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
  type NovelProject,
} from "../api/project";
import {
  listChapters,
  type ChapterSummary,
} from "../api/chapter";
import {
  applyOptimizationPatches,
  discardOptimizationPatchSet,
  listOptimizationPatchSets,
} from "../api/optimization";
import {
  isDerivedChapterOutline,
  listChapterOutlines,
  type ChapterOutline,
} from "../api/outline";
import type { BottomTab } from "./writing-desk/ChapterWorkspace";
import type { RightPanel } from "./writing-desk/WritingDeskHeader";
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
import { usePersistedTab } from "../hooks/usePersistedTab";
import {
  getProjectHomeEntryRoute,
  getProjectPrimaryRoute,
} from "../utils/projectRouting";
import { getProjectLifecycleBlockedReason } from "../utils/projectStatus";
import { useChapterAnalysisActions } from "./writing-desk/useChapterAnalysisActions";
import { useChapterPromptPreview } from "./writing-desk/useChapterPromptPreview";
import { useChapterStreamGeneration } from "./writing-desk/useChapterStreamGeneration";
import { useWritingDeskDrafts } from "./writing-desk/useWritingDeskDrafts";
import { useWritingDeskChapterDerivedState } from "./writing-desk/useWritingDeskChapterDerivedState";
import { WritingDeskView } from "./writing-desk/WritingDeskView";
import { useWritingDeskResetGeneratedData } from "./writing-desk/useWritingDeskResetGeneratedData";
import { useWritingDeskSaveContent } from "./writing-desk/useWritingDeskSaveContent";
import { formatNumber, useI18n } from "../i18n";
import type { ContentDiffAction } from "../components/agent/ContentDiffPreview";
import type { PendingContentDiff } from "../components/agent/contentOptimizationDiff";

const WRITING_DESK_BOTTOM_TABS = [
  "content",
  "analysis",
  "images",
  "visual_plan",
  "shooting_script",
] as const satisfies readonly BottomTab[];
const WRITING_DESK_RIGHT_PANELS = ["none", "rag", "optimize"] as const satisfies readonly RightPanel[];

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
  const projectIdKey =
    Number.isFinite(projectId) && projectId > 0 ? String(projectId) : "";

  const [project, setProject] = useState<NovelProject | null>(null);
  const lifecycleBlockedReason = project
    ? getProjectLifecycleBlockedReason(project, t)
    : null;
  const canEditWritingProject = project
    ? canEditProjectMaterials(project)
    : false;
  const [chapters, setChapters] = useState<ChapterSummary[]>([]);
  const [outlines, setOutlines] = useState<ChapterOutline[]>([]);
  const [loading, setLoading] = useState(true);
  const searchParamsRef = useRef(searchParams);
  searchParamsRef.current = searchParams;
  const requestedChapterNumber = () => {
    const value = searchParamsRef.current.get("chapter");
    if (!value || !/^[1-9]\d*$/.test(value)) return null;
    const number = Number(value);
    return Number.isSafeInteger(number) && number <= 2_147_483_647 ? number : null;
  };
  // URL owns chapter selection; changing chapters must not remount the workspace.
  const activeNumber = loading ? null : requestedChapterNumber();
  const setActiveNumber = useCallback((number: number | null) => {
    const next = new URLSearchParams(searchParamsRef.current);
    const value = number == null ? null : String(number);
    if (next.get("chapter") === value) return;
    if (value == null) next.delete("chapter");
    else next.set("chapter", value);
    searchParamsRef.current = next;
    setSearchParams(next, { replace: true });
  }, [setSearchParams]);
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
  const [savingContent, setSavingContent] = useState(false);
  const [pendingContentDiff, setPendingContentDiff] = useState<PendingContentDiff | null>(null);
  const [contentDiffAction, setContentDiffAction] = useState<ContentDiffAction | null>(null);
  const [resetGeneratedDataBusy, setResetGeneratedDataBusy] = useState(false);
  const [leftSidebarVisible, setLeftSidebarVisible] = useState(true);
  const [isMobileLayout, setIsMobileLayout] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth < 768 : false,
  );
  const [rightPanel, setRightPanel] = usePersistedTab<RightPanel>(
    projectIdKey ? `a4n:writing-desk:${projectIdKey}:right-panel` : null,
    agentRequested ? "optimize" : "none",
    WRITING_DESK_RIGHT_PANELS,
  );
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
  const [bottomTab, setBottomTab] = usePersistedTab<BottomTab>(
    projectIdKey ? `a4n:writing-desk:${projectIdKey}:bottom-tab` : null,
    "content",
    WRITING_DESK_BOTTOM_TABS,
  );
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
  const contentDiffActionRef = useRef(false);
  const resolvedContentDiffPatchSetsRef = useRef(new Set<number>());
  const resetGeneratedDataBusyRef = useRef(false);
  const derivedSyncBusyRef = useRef(false);
  const smartBack = useSmartBack("/");
  projectIdRef.current = projectId;

  useEffect(() => {
    if (!agentRequested) return;
    // 显式助手链接决定本次入口，之后仍允许用户切换或关闭面板。
    setBottomTab("content");
    setRightPanel("optimize");
  }, [agentRequested, projectIdKey, setBottomTab, setRightPanel]);

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
    activeChapterContext,
    chapterNumbers,
    filteredChapterNumbers,
  } = useWritingDeskChapterDerivedState({
    chapters,
    outlines,
    activeNumber,
    chapterQuery,
  });
  useEffect(() => {
    if (pendingContentDiff && pendingContentDiff.chapterId !== activeChapter?.id) {
      setPendingContentDiff(null);
    }
  }, [activeChapter?.id, pendingContentDiff]);

  const pendingContentDiffKey = pendingContentDiff
    ? `${pendingContentDiff.sessionId ?? "missing"}:${pendingContentDiff.patchSetId ?? "missing"}`
    : null;

  const handlePendingContentDiffChange = useCallback(
    (diff: PendingContentDiff | null) => {
      if (
        diff?.patchSetId != null &&
        resolvedContentDiffPatchSetsRef.current.has(diff.patchSetId)
      ) {
        setPendingContentDiff(null);
        return;
      }
      setPendingContentDiff(diff);
    },
    [],
  );

  useEffect(() => {
    const diff = pendingContentDiff;
    if (
      !diff ||
      diff.sessionId == null ||
      diff.patchSetId == null ||
      resolvedContentDiffPatchSetsRef.current.has(diff.patchSetId)
    ) {
      return;
    }
    const sessionId = diff.sessionId;
    const patchSetId = diff.patchSetId;
    let cancelled = false;
    void listOptimizationPatchSets({ session_id: sessionId })
      .then((patchSets) => {
        if (cancelled) return;
        const patchSet = patchSets.find((item) => item.id === patchSetId);
        if (patchSet && patchSet.status !== "pending") {
          resolvedContentDiffPatchSetsRef.current.add(patchSetId);
          setPendingContentDiff(null);
        }
      })
      .catch((error) => {
        if (cancelled) return;
        addToast(
          extractErrorMessage(error, t("agent.domain.diffStatusCheckFailed")),
          "error",
        );
      });
    return () => {
      cancelled = true;
    };
  }, [addToast, pendingContentDiffKey, t]);
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
    editorRevisionRef,
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
        const [chaptersResult, outlinesResult] = await Promise.allSettled([
          listChapters(projectId),
          listChapterOutlines(projectId),
        ]);
        if (!isCurrentLoad()) return;
        if (chaptersResult.status === "rejected") {
          addToast(extractErrorMessage(chaptersResult.reason, t("novel.loadChaptersFailed")), "error");
        }
        if (outlinesResult.status === "rejected") {
          addToast(extractErrorMessage(outlinesResult.reason, t("outline.loadFailed")), "error");
        }
        const ch = chaptersResult.status === "fulfilled" ? chaptersResult.value : [];
        const ol = outlinesResult.status === "fulfilled" ? outlinesResult.value : [];
        const outlineNumbers = ol.map((outline) => outline.chapter_number);
        setProject(p);
        setChapters(ch);
        setOutlines(ol);

        const fromUrlNumber = requestedChapterNumber();
        if (
          fromUrlNumber != null &&
          (ch.some((chapter) => chapter.chapter_number === fromUrlNumber) ||
            outlineNumbers.includes(fromUrlNumber) ||
            draftSet.has(fromUrlNumber))
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

  const safeSelectChapter = useCallback(
    async (chapterNumber: number) => {
      if (chapterNumber === activeNumberRef.current) return true;
      const ok = await confirmCurrentEdit(t("writing.switchChapter"));
      if (!ok) return false;
      setActiveNumber(chapterNumber);
      return true;
    },
    [confirmCurrentEdit, setActiveNumber],
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
  }, [addToast, chapters, confirmCurrentEdit, lifecycleBlockedReason, outlines, setActiveNumber]);

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
        const current = activeNumberRef.current;
        if (
          current == null ||
          (!next.some((ch) => ch.chapter_number === current) &&
            !outlines.some((outline) => outline.chapter_number === current))
        ) {
          setActiveNumber(
            next[next.length - 1]?.chapter_number ??
            outlines[0]?.chapter_number ??
            null,
          );
        }
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
  }, [projectId, addToast, outlines, activeNumberRef, setActiveNumber]);

  const handleRefreshChapters = useCallback(async () => {
    const chapterNumber = activeNumberRef.current;
    if (!(await confirmCurrentEdit(t("common.refresh")))) return;
    const next = await reloadChapters({ force: true });
    if (!next || activeNumberRef.current !== chapterNumber) return;
    const refreshed = next.find((chapter) => chapter.chapter_number === chapterNumber);
    if (!refreshed) return;
    setEditorState(refreshed.content, false);
    editorRevisionRef.current = refreshed.edit_revision;
  }, [activeNumberRef, confirmCurrentEdit, editorRevisionRef, reloadChapters, setEditorState, t]);

  const reloadOutlines = useCallback(async (): Promise<boolean> => {
    const requestProjectId = projectId;
    try {
      const next = await listChapterOutlines(requestProjectId);
      if (!mountedRef.current || projectIdRef.current !== requestProjectId) return false;
      setOutlines(next);
      return true;
    } catch (err) {
      if (mountedRef.current && projectIdRef.current === requestProjectId) {
        addToast(extractErrorMessage(err, t("writing.refreshOutlinesFailed")), "error");
      }
      return false;
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
      if (!await reloadChapters({ force: true })) return;
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
    streamThinking,
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

  const clearResetProjectState = useCallback(async () => {
    clearStreamState(null);
    setChapters([]);
    setOutlines([]);
    setActiveNumber(null);
    setChapterQuery("");
    setWritingNotesByChapter({});
    setScenesOpen(false);
    setRightPanel("none");
    resetPromptPreview();
    setBottomTab("content");
    setEditorState("", false);
    resetDraftTracking();
    await clearProjectDrafts();
    setProfilesRefreshKey((value) => value + 1);
  }, [
    clearProjectDrafts,
    clearStreamState,
    resetDraftTracking,
    resetPromptPreview,
    setActiveNumber,
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
        .then(async (nextProject) => {
          if (!mountedRef.current || projectIdRef.current !== projectId) return;
          if (nextProject.lifecycle_gate !== "active") return;
          window.clearInterval(timer);
          setProject(nextProject);
          await clearResetProjectState();
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

  const {
    canResetGeneratedData,
    handleResetGeneratedData,
  } = useWritingDeskResetGeneratedData({
    projectId,
    project,
    chapters,
    lifecycleBlockedReason,
    resetGeneratedDataBusy,
    streamTaskId,
    streamBusy,
    resetGeneratedDataBusyRef,
    setResetGeneratedDataBusy,
    setProject,
    clearResetProjectState,
    addToast,
    t,
  });

  const handleOptimizedChapterContentChanged = useCallback(
    async (chapterId: number) => {
      const next = await reloadChapters({ force: true });
      const updated = next?.find((chapter) => chapter.id === chapterId) ?? null;
      if (!updated || activeChapter?.id !== chapterId) return;

      if (dirtyRef.current) {
        await saveLocalDraftForCurrent();
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

  const handleApplyContentDiff = useCallback(async () => {
    const diff = pendingContentDiff;
    if (
      !diff ||
      diff.sessionId == null ||
      diff.patchSetId == null ||
      diff.baseContentHash == null ||
      contentDiffActionRef.current
    ) {
      return;
    }
    const confirmed = await confirmDialog({
      title: t("agent.domain.diffApplyTitle"),
      message: t("agent.domain.diffApplyMessage"),
      confirmText: t("agent.domain.diffApply"),
      cancelText: t("common.cancel"),
      dialogType: "warning",
    });
    if (!confirmed || contentDiffActionRef.current) return;

    contentDiffActionRef.current = true;
    setContentDiffAction("apply");
    try {
      await applyOptimizationPatches({
        session_id: diff.sessionId,
        base_content_hash: diff.baseContentHash,
        patches: diff.patches,
      });
      resolvedContentDiffPatchSetsRef.current.add(diff.patchSetId);
      setPendingContentDiff(null);
      try {
        await handleOptimizedChapterContentChanged(diff.chapterId);
        addToast(t("agent.domain.diffApplied"), "success");
      } catch (error) {
        addToast(
          extractErrorMessage(error, t("agent.domain.resourceRefreshFailed")),
          "error",
        );
      }
    } catch (error) {
      addToast(
        extractErrorMessage(error, t("agent.domain.diffApplyFailed")),
        "error",
      );
    } finally {
      contentDiffActionRef.current = false;
      setContentDiffAction(null);
    }
  }, [addToast, handleOptimizedChapterContentChanged, pendingContentDiff, t]);

  const handleRejectContentDiff = useCallback(async () => {
    const diff = pendingContentDiff;
    if (
      !diff ||
      diff.sessionId == null ||
      diff.patchSetId == null ||
      contentDiffActionRef.current
    ) {
      return;
    }
    const confirmed = await confirmDialog({
      title: t("agent.domain.diffRejectTitle"),
      message: t("agent.domain.diffRejectMessage"),
      confirmText: t("agent.domain.diffReject"),
      cancelText: t("common.cancel"),
      dialogType: "danger",
    });
    if (!confirmed || contentDiffActionRef.current) return;

    contentDiffActionRef.current = true;
    setContentDiffAction("reject");
    try {
      await discardOptimizationPatchSet({ patch_set_id: diff.patchSetId });
      resolvedContentDiffPatchSetsRef.current.add(diff.patchSetId);
      setPendingContentDiff(null);
      addToast(t("agent.domain.diffRejected"), "success");
    } catch (error) {
      addToast(
        extractErrorMessage(error, t("agent.domain.diffRejectFailed")),
        "error",
      );
    } finally {
      contentDiffActionRef.current = false;
      setContentDiffAction(null);
    }
  }, [addToast, pendingContentDiff, t]);

  const handleSaveContent = useWritingDeskSaveContent({
    projectId,
    locale,
    lifecycleBlockedReason,
    activeNumber,
    activeChapter,
    activeOutline,
    activeChapterBusy,
    activeChapterGeneratingMessage,
    editorContent,
    editorRevisionRef,
    saveContentBusyRef,
    activeNumberRef,
    addToast,
    t,
    setSavingContent,
    setChapters,
    reloadChapters,
    getCurrentEditorContent,
    removeDraftForChapter,
    setEditorState,
    saveLocalDraftForCurrent,
  });

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
    streamThinking,
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
    onPendingContentDiffChange: handlePendingContentDiffChange,
  };

  return (
    <WritingDeskView
      projectId={projectId}
      activeNumber={activeNumber}
      chapters={chapters}
      outlines={outlines}
      lifecycleBlockedReason={lifecycleBlockedReason}
      isMobileLayout={isMobileLayout}
      leftSidebarVisible={leftSidebarVisible}
      rightPanel={rightPanel}
      assistantOpen={assistantOpen}
      writingDeskGridTemplateColumns={writingDeskGridTemplateColumns}
      onCloseRightPanel={() => setRightPanel("none")}
      onCloseAnalysisRequestPreview={() => setAnalysisRequestPreviewOpen(false)}
      headerProps={{
        project,
        activeNumber,
        isMobileLayout,
        canGoPrev: previousChapterNumber != null,
        canGoNext: nextChapterNumber != null,
        chapterNumbers,
        chapters,
        outlines,
        leftSidebarVisible: !isMobileLayout && leftSidebarVisible,
        rightPanel,
        dirty,
        savingContent,
        editBlockedReason: lifecycleBlockedReason,
        activeChapterBusy,
        activeChapterGeneratingMessage,
        rightPanelAvailable,
        rightPanelUnavailableTip,
        onSmartBack: safeSmartBack,
        onOpenProjectDetail: () => safeNavigateTo(getProjectPrimaryRoute(project)),
        onCreateChapter: handleCreateChapter,
        onPrev: handlePrev,
        onNext: handleNext,
        onSelectChapter: safeSelectChapter,
        onToggleLeftSidebar: handleToggleLeftSidebar,
        onOpenProfiles: () => setProfilesOpen(true),
        onRightPanelChange: handleRightPanelChange,
        onSaveContent: handleSaveContent,
      }}
      sidebarProps={{
        projectId,
        chapters,
        outlines,
        chapterNumbers,
        filteredChapterNumbers,
        activeNumber,
        activeEditorContent: editorContent,
        activeOutline,
        chapterQuery,
        draftSet,
        profilesRefreshKey,
        onChapterQueryChange: setChapterQuery,
        onClearChapterQuery: () => setChapterQuery(""),
        onOpenProfiles: () => setProfilesOpen(true),
        onRefreshChapters: handleRefreshChapters,
        onCreateChapter: handleCreateChapter,
        onSelectChapter: safeSelectChapter,
      }}
      workspaceProps={{
        projectId,
        activeNumber,
        activeChapter,
        activeOutline,
        activeChapterContext,
        activeChapterBusy,
        activeChapterGeneratingMessage,
        analysisBlockedReason,
        editorContent,
        dirty,
        savingContent,
        autoScrollEditorContent: streamingActiveChapter,
        wordCount,
        generatingAnalysis: analysisBusy,
        bottomTab,
        onBottomTabChange: handleBottomTabChange,
        onEditorContentChange: (content) => setEditorState(content, true),
        pendingContentDiff:
          pendingContentDiff?.chapterId === activeChapter?.id
            ? pendingContentDiff
            : null,
        contentDiffAction,
        onApplyContentDiff: handleApplyContentDiff,
        onRejectContentDiff: handleRejectContentDiff,
        onGenerateAnalysis: handleGenerateAnalysis,
        onPreviewGenerateAnalysis: handlePreviewAnalysisRequest,
        onResumeAnalysisSync: handleResumeAnalysisSync,
        onPreviewResumeAnalysisSync: handlePreviewResumeAnalysisSyncRequest,
        onResumeDerivedSync: handleResumeDerivedSync,
        onOpenEmbeddingSettings: () => safeNavigateTo("/settings#embedding"),
        canEdit: canEditWritingProject,
      }}
      resizeProps={
        rightPanel === "none"
          ? null
          : {
              panel: rightPanel,
              width: rightPanelWidth,
              maxWidth: rightPanelMaxWidth,
              isResizing: isResizingRightPanel,
              onResizeStart: handleRightPanelResizeStart,
              onResizeMove: handleRightPanelResizeMove,
              onResizeEnd: finishRightPanelResize,
              onSetWidth: setRightPanelWidth,
              onResetWidth: resetRightPanelWidth,
            }
      }
      assistantPanelProps={writingAssistantPanelProps}
      dialogProps={{
        projectId,
        activeNumber,
        activeChapterId: activeChapter?.id ?? null,
        scenesOpen,
        previewOpen,
        profilesOpen,
        profilesRefreshKey,
        sceneAiBlockedReason: currentChapterAiBlockedReason,
        characterAiBlockedReason: currentChapterAiBlockedReason,
        previewing,
        previewData,
        onScenesClose: () => setScenesOpen(false),
        onPreviewClose: closePromptPreview,
        onCharacterMaterialSync: handleCharacterMaterialSync,
        onProfilesClose: () => {
          setProfilesOpen(false);
          setProfilesRefreshKey((value) => value + 1);
        },
      }}
      scenesOpen={scenesOpen}
      previewOpen={previewOpen}
      profilesOpen={profilesOpen}
      analysisRequestPreviewOpen={analysisRequestPreviewOpen}
      analysisRequestPreviewLoading={analysisRequestPreviewLoading}
      analysisRequestPreviewData={analysisRequestPreviewData}
    />
  );
};
