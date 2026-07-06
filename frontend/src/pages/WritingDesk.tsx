import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { Loader2 } from "lucide-react";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { useToast } from "../components/feedback/Toast";
import { extractErrorMessage } from "../api";
import {
  getProject,
  resetProjectGeneratedData,
  type NovelProject,
} from "../api/project";
import {
  importChapter,
  listChapters,
  updateChapterContent,
  type ChapterData,
} from "../api/chapter";
import { listChapterOutlines, type ChapterOutline } from "../api/outline";
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
import { getProjectHomeEntryRoute } from "../utils/projectRouting";
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

const canEditWritingProject = (project: NovelProject): boolean =>
  project.my_role === "owner" ||
  project.my_role === "manager" ||
  project.my_role === "editor";

export const WritingDesk: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const projectId = Number(id);
  const navigate = useNavigate();
  const { addToast } = useToast();

  const [project, setProject] = useState<NovelProject | null>(null);
  const [chapters, setChapters] = useState<ChapterData[]>([]);
  const [outlines, setOutlines] = useState<ChapterOutline[]>([]);
  const [activeNumber, setActiveNumber] = useState<number | null>(null);
  const [chapterQuery, setChapterQuery] = useState("");
  const [writingNotes, setWritingNotes] = useState("");
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
  // 旧 AI 助手 aside 直接对应"正文优化"模式
  const assistantOpen = rightPanel === "optimize";
  const rightPanelAvailable = bottomTab === "content";
  const rightPanelUnavailableTip = "只有当前页面在“正文”时才能打开 RAG 查询或正文优化";
  // 章节场景管理对话框（对应 A4N 后端 chapter_scene 子结构）
  const [scenesOpen, setScenesOpen] = useState(false);
  const [profilesOpen, setProfilesOpen] = useState(false);
  const [profilesRefreshKey, setProfilesRefreshKey] = useState(0);
  const mountedRef = useRef(false);
  const loadSeqRef = useRef(0);
  const reloadChaptersInFlightRef = useRef<{
    projectId: number;
    token: symbol;
    promise: Promise<ChapterData[] | null>;
  } | null>(null);
  const saveContentBusyRef = useRef(false);
  const resetGeneratedDataBusyRef = useRef(false);
  const smartBack = useSmartBack("/");
  const projectIdKey =
    Number.isFinite(projectId) && projectId > 0 ? String(projectId) : "";

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

  const {
    activeChapter,
    activeOutline,
    activeOutlineTitle,
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
    const range = defaultGenerationWordCountRange(activeOutline?.word_count_target);
    setMinWordCount(String(range.min));
    setMaxWordCount(String(range.max));
  }, [activeNumber, activeOutline?.word_count_target]);
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
        if (!canEditWritingProject(p)) {
          addToast("当前账号没有编辑此项目的权限", "error");
          navigate(`/novel/${projectId}`, { replace: true });
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
        setActiveNumber(outlineNumbers[0] ?? null);
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
  const analysisBlockedReason = useMemo(() => {
    if (!activeChapter) return null;
    const incomplete = chapters
      .filter((chapter) => chapter.chapter_number < activeChapter.chapter_number)
      .filter((chapter) => chapter.status !== "deleted")
      .map((chapter) => {
        if (chapter.status === "generating") return `第 ${chapter.chapter_number} 章正在生成正文`;
        if (!chapter.content.trim()) return `第 ${chapter.chapter_number} 章正文为空`;
        if (chapter.analysis_data == null) return `第 ${chapter.chapter_number} 章未生成结构化分析`;
        return null;
      })
      .filter((message): message is string => message != null);
    if (incomplete.length === 0) return null;
    const visible = incomplete.slice(0, 6).join("；");
    const omitted = incomplete.length > 6 ? `；另有 ${incomplete.length - 6} 章未列出` : "";
    return `请先完成前序章节分析：${visible}${omitted}`;
  }, [activeChapter, chapters]);

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
        if (!mountedRef.current || projectId !== requestProjectId) return null;
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
        if (mountedRef.current && projectId === requestProjectId) {
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
    activeOutlineTitle,
    activeOutlineText,
    chapters,
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

  const resetGeneratedDataDisabledReason = useMemo(() => {
    if (!project) return "项目尚未加载";
    if (resetGeneratedDataBusy) return "正在重置项目生成数据";
    if (streamTaskId) return "章节流式生成中，不能重置项目数据";
    if (streamBusy) return "章节流式生成状态正在更新，不能重置项目数据";
    if (chapters.some((chapter) => chapter.status === "generating")) {
      return "有章节正在生成，不能重置项目数据";
    }
    return null;
  }, [chapters, project, resetGeneratedDataBusy, streamBusy, streamTaskId]);
  const canResetGeneratedData = resetGeneratedDataDisabledReason == null;

  const handleOptimizedChapterContentChanged = useCallback(
    async (chapterId: number) => {
      const next = await reloadChapters({ force: true });
      const updated = next?.find((chapter) => chapter.id === chapterId) ?? null;
      if (!updated || activeChapter?.id !== chapterId) return;

      if (dirtyRef.current) {
        saveLocalDraftForCurrent();
        addToast(
          "Agent 已写回章节正文；当前编辑器有未保存修改，未自动覆盖。",
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
      const reset = await resetProjectGeneratedData(projectId);
      clearStreamState(null);
      setProject(reset.project);
      setChapters([]);
      setOutlines([]);
      setActiveNumber(null);
      setChapterQuery("");
      setWritingNotes("");
      setOptimizationPatchPreview(null);
      setScenesOpen(false);
      resetPromptPreview();
      setBottomTab("content");
      setEditorState("", false);
      resetDraftTracking();
      clearProjectDrafts();
      setProfilesRefreshKey((value) => value + 1);
      addToast(
        `已重置：删除 ${reset.deleted_chapters} 章、${reset.deleted_chapter_outlines} 条章纲、${reset.deleted_character_profiles} 个正文派生角色，保留 ${reset.preserved_blueprint_versions} 个蓝图版本。`,
        "success",
      );
    } catch (err) {
      addToast(extractErrorMessage(err, "重置项目生成数据失败"), "error");
    } finally {
      resetGeneratedDataBusyRef.current = false;
      setResetGeneratedDataBusy(false);
    }
  };

  const handleSaveContent = async () => {
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
      if (activeChapter) {
        const updated = await updateChapterContent({
          chapter_id: activeChapter.id,
          content: contentToSave,
          title: activeChapter.title,
        });
        savedContent = updated.content;
        setChapters((prev) =>
          prev.map((ch) => (ch.id === updated.id ? updated : ch)),
        );
      } else {
        const title = activeOutline?.title?.trim();
        if (!title) {
          addToast("章纲标题为空，不能创建章节", "error");
          return;
        }
        await importChapter({
          project_id: projectId,
          chapter_number: chapterNumber,
          content: contentToSave,
          title,
        });
        const nextChapters = await reloadChapters({ force: true });
        const created = nextChapters?.find(
          (chapter) => chapter.chapter_number === chapterNumber,
        );
        savedContent = created?.content ?? contentToSave;
      }
      const currentContent = getCurrentEditorContent();
      const saveStillMatchesEditor =
        activeNumberRef.current === chapterNumber &&
        currentContent === contentToSave;
      if (saveStillMatchesEditor) {
        removeDraftForChapter(chapterNumber);
        setEditorState(savedContent, false);
        addToast("章节正文已保存", "success");
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
    if (isMobileLayout) {
      addToast("移动端已隐藏章节侧栏，可用上一章/下一章切换", "info");
      return;
    }
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
        <Loader2 size={20} className="animate-spin mr-2" /> 加载写作桌…
      </div>
    );
  }
  if (!project) return null;

  return (
    <div className="flex h-full w-full min-w-0 flex-col overflow-hidden">
      {project && outlines.length === 0 && chapters.length === 0 && (
        <MissingOutlineBanner
          project={project}
          onOpenProjectDetail={() => safeNavigateTo(`/novel/${projectId}`)}
        />
      )}

      <WritingDeskHeader
        project={project}
        activeNumber={activeNumber}
        canGoPrev={previousChapterNumber != null}
        canGoNext={nextChapterNumber != null}
        leftSidebarVisible={!isMobileLayout && leftSidebarVisible}
        rightPanel={rightPanel}
        dirty={dirty}
        activeChapterBusy={activeChapterBusy}
        activeChapterGeneratingMessage={activeChapterGeneratingMessage}
        rightPanelAvailable={rightPanelAvailable}
        rightPanelUnavailableTip={rightPanelUnavailableTip}
        onSmartBack={safeSmartBack}
        onOpenProjectDetail={() => safeNavigateTo(`/novel/${projectId}`)}
        onPrev={handlePrev}
        onNext={handleNext}
        onToggleLeftSidebar={handleToggleLeftSidebar}
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
            onSelectChapter={safeSelectChapter}
            onOpenProjectDetail={() => safeNavigateTo(`/novel/${projectId}`)}
          />
        )}

        <ChapterWorkspace
          projectId={projectId}
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
          <WritingAssistantPanel
            projectId={projectId}
            writingNotes={writingNotes}
            minWordCount={minWordCount}
            maxWordCount={maxWordCount}
            wordCountRangeError={generationWordCountError}
            activeNumber={activeNumber}
            activeChapter={activeChapter}
            activeChapterBusy={activeChapterBusy}
            activeChapterGeneratingMessage={activeChapterGeneratingMessage}
            canGenerateChapter={canGenerateChapter}
            generationBlockReason={generationBlockReason}
            streamTaskId={streamTaskId}
            streamBusy={streamBusy}
            streamMessage={streamMessage}
            resetBusy={resetGeneratedDataBusy}
            canResetGeneratedData={canResetGeneratedData}
            resetDisabledReason={resetGeneratedDataDisabledReason}
            previewing={previewing}
            chapters={chapters}
            outlines={outlines}
            onWritingNotesChange={setWritingNotes}
            onMinWordCountChange={setMinWordCount}
            onMaxWordCountChange={setMaxWordCount}
            onStreamStart={handleStreamStart}
            onStreamCancel={handleStreamCancel}
            onResetGeneratedData={handleResetGeneratedData}
            onPreviewPrompt={handlePreviewPrompt}
            onOpenScenes={handleOpenScenes}
            onChapterContentChanged={handleOptimizedChapterContentChanged}
            onPatchPreviewChange={setOptimizationPatchPreview}
          />
        )}
      </div>

      {isMobileLayout && rightPanel !== "none" && (
        <Modal
          isOpen
          onClose={() => setRightPanel("none")}
          title={rightPanel === "rag" ? "RAG 查询" : "正文优化"}
          maxWidthClassName="max-w-3xl"
          bodyClassName="min-h-0 flex-1 overflow-hidden p-0"
        >
          {rightPanel === "rag" ? (
            <RagQueryPanel projectId={projectId} chapterNumber={activeNumber} />
          ) : (
            <WritingAssistantPanel
              projectId={projectId}
              writingNotes={writingNotes}
              minWordCount={minWordCount}
              maxWordCount={maxWordCount}
              wordCountRangeError={generationWordCountError}
              activeNumber={activeNumber}
              activeChapter={activeChapter}
              activeChapterBusy={activeChapterBusy}
              activeChapterGeneratingMessage={activeChapterGeneratingMessage}
              canGenerateChapter={canGenerateChapter}
              generationBlockReason={generationBlockReason}
              streamTaskId={streamTaskId}
              streamBusy={streamBusy}
              streamMessage={streamMessage}
              resetBusy={resetGeneratedDataBusy}
              canResetGeneratedData={canResetGeneratedData}
              resetDisabledReason={resetGeneratedDataDisabledReason}
              previewing={previewing}
              chapters={chapters}
              outlines={outlines}
              onWritingNotesChange={setWritingNotes}
              onMinWordCountChange={setMinWordCount}
              onMaxWordCountChange={setMaxWordCount}
              onStreamStart={handleStreamStart}
              onStreamCancel={handleStreamCancel}
              onResetGeneratedData={handleResetGeneratedData}
              onPreviewPrompt={handlePreviewPrompt}
              onOpenScenes={handleOpenScenes}
              onChapterContentChanged={handleOptimizedChapterContentChanged}
              onPatchPreviewChange={setOptimizationPatchPreview}
            />
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
        previewing={previewing}
        previewData={previewData}
        onScenesClose={() => setScenesOpen(false)}
        onPreviewClose={closePromptPreview}
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

export default WritingDesk;
