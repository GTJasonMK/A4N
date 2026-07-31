import React, { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  ArrowLeft,
  Database,
  Edit2,
  FileText,
  GitBranch,
  MoreHorizontal,
  Network,
  Sparkles,
  Users,
  Wand2,
} from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { useToast } from "../components/feedback/Toast";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { promptDialog } from "../components/feedback/PromptDialog";
import { extractErrorMessage } from "../api";
import { getProject, updateProject, type NovelProject } from "../api/project";
import { refineBlueprint } from "../api/blueprint";
import { formatRagIngestResultSummary, ragIngestAll } from "../api/rag";
import { OverviewTab } from "./novel-detail/OverviewTab";
import { ImagesTab } from "./novel-detail/ImagesTab";
import { WorldTab } from "./novel-detail/WorldTab";
import { CharactersTab } from "./novel-detail/CharactersTab";
import { ObjectProfilesTab } from "./novel-detail/ObjectProfilesTab";
import { SceneProfilesTab } from "./novel-detail/SceneProfilesTab";
import { RelationshipsTab } from "./novel-detail/RelationshipsTab";
import { OutlinesTab } from "./novel-detail/OutlinesTab";
import { ChaptersTab } from "./novel-detail/ChaptersTab";
import { StoryEventsTab } from "./novel-detail/StoryEventsTab";
import { ExportPanel } from "./novel-detail/ExportPanel";
import { ImportsTab } from "./novel-detail/ImportsTab";
import { AssetDependencyGraphDialog } from "./novel-detail/asset-graph/AssetDependencyGraphDialog";
import { ProjectCollaborationPanel } from "./novel-detail/overview/ProjectCollaborationPanel";
import { useStatusBarPermanent } from "../components/ui/StatusBar";
import { getProjectProgressText } from "../utils/projectStatus";
import { usePersistedTab } from "../hooks/usePersistedTab";
import { StatusMachineDialog } from "./StatusMachineDialog";
import { BackButton } from "../components/ui/BackButton";
import { Modal } from "../components/ui/Modal";
import { BookButton } from "../components/ui/BookButton";
import { PageHeader } from "../components/shell/PageHeader";
import { allocateHostTaskId, useProgressTask } from "../hooks/useProgressTask";
import { isCancelledOperation } from "../utils/cancellation";
import { nonEmptyText } from "../utils/text";
import {
  getProjectHomeEntryLabel,
  getProjectHomeEntryRoute,
  getProjectWorkspaceLabel,
  getProjectWorkspaceRoute,
} from "../utils/projectRouting";

/**
 * 项目详情页
 *
 * 主视图：概览/世界观/角色/关系/章节大纲/图像/已生成章节。
 * Header 只保留当前页面高频操作，低频操作进入弹窗。
 */

type Tab =
  | "overview"
  | "world"
  | "characters"
  | "object_profiles"
  | "scene_profiles"
  | "relationships"
  | "story_events"
  | "chapter_outline"
  | "images"
  | "imports"
  | "chapters";

// 详情页固定 Tab 顺序
const TABS: { id: Tab; label: string; accessibleLabel: string }[] = [
  { id: "overview", label: "概览", accessibleLabel: "项目概览" },
  { id: "world", label: "世界观", accessibleLabel: "世界观" },
  { id: "characters", label: "角色", accessibleLabel: "角色档案" },
  { id: "object_profiles", label: "物体", accessibleLabel: "物体档案" },
  { id: "scene_profiles", label: "场景", accessibleLabel: "场景档案" },
  { id: "relationships", label: "关系", accessibleLabel: "角色关系" },
  { id: "story_events", label: "事件", accessibleLabel: "事件脉络" },
  { id: "chapter_outline", label: "大纲", accessibleLabel: "章节大纲" },
  { id: "images", label: "图像", accessibleLabel: "项目图像" },
  { id: "imports", label: "导入", accessibleLabel: "内容导入" },
  { id: "chapters", label: "章节", accessibleLabel: "已生成章节" },
];
const TAB_IDS = TABS.map((t) => t.id) as readonly Tab[];

export const NovelDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const projectId = Number(id);
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();

  const [project, setProject] = useState<NovelProject | null>(null);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = usePersistedTab<Tab>(
    Number.isFinite(projectId) && projectId > 0 ? `a4n:novel-detail:${projectId}:tab` : null,
    "overview",
    TAB_IDS,
  );
  const [saving, setSaving] = useState(false);
  const [ragSyncing, setRagSyncing] = useState(false);
  const [refining, setRefining] = useState(false);
  const [stateMachineOpen, setStateMachineOpen] = useState(false);
  const [assetGraphOpen, setAssetGraphOpen] = useState(false);
  const [actionsOpen, setActionsOpen] = useState(false);
  const [collaborationOpen, setCollaborationOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [exportBusy, setExportBusy] = useState(false);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const refreshSeqRef = useRef(0);
  const ragSeqRef = useRef(0);
  const refineSeqRef = useRef(0);
  const titleSeqRef = useRef(0);
  const ragActionBusyRef = useRef(false);
  const refineActionBusyRef = useRef(false);
  const titleActionBusyRef = useRef(false);
  const activeTabRef = useRef<HTMLButtonElement | null>(null);
  const canEditCurrentProject =
    project?.my_role === "owner" || project?.my_role === "manager" || project?.my_role === "editor";

  const isCurrentProject = (requestProjectId: number) =>
    mountedRef.current && Object.is(activeProjectIdRef.current, requestProjectId);

  const isCurrentAction = (
    requestProjectId: number,
    ref: React.MutableRefObject<number>,
    sequence: number,
  ) => isCurrentProject(requestProjectId) && ref.current === sequence;

  const invalidateProjectScopedRequests = () => {
    refreshSeqRef.current += 1;
    ragSeqRef.current += 1;
    refineSeqRef.current += 1;
    titleSeqRef.current += 1;
    ragActionBusyRef.current = false;
    refineActionBusyRef.current = false;
    titleActionBusyRef.current = false;
  };

  const refresh = async (requestProjectId = projectId) => {
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    if (!Number.isFinite(requestProjectId) || requestProjectId <= 0) {
      addToast("无效的项目 ID", "error");
      navigate("/");
      return;
    }
    setLoading(true);
    try {
      const p = await getProject(String(requestProjectId));
      if (!isCurrentAction(requestProjectId, refreshSeqRef, requestSeq)) return;
      if (p.project_kind !== "text") {
        navigate(getProjectHomeEntryRoute(p), { replace: true });
        return;
      }
      setProject(p);
    } catch (err) {
      if (isCurrentAction(requestProjectId, refreshSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "加载项目失败"), "error");
        navigate("/");
      }
    } finally {
      if (isCurrentAction(requestProjectId, refreshSeqRef, requestSeq)) {
        setLoading(false);
      }
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      invalidateProjectScopedRequests();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    invalidateProjectScopedRequests();
    setProject(null);
    setLoading(true);
    setSaving(false);
    setRagSyncing(false);
    setRefining(false);
    setStateMachineOpen(false);
    setAssetGraphOpen(false);
    setActionsOpen(false);
    setCollaborationOpen(false);
    setExportOpen(false);
    setExportBusy(false);
    void refresh(projectId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  useEffect(() => {
    if (!project || canEditCurrentProject) return;
    if (tab !== "characters" && tab !== "relationships" && tab !== "imports") return;
    setTab("overview");
  }, [canEditCurrentProject, project, setTab, tab]);

  useEffect(() => {
    if (!window.matchMedia("(max-width: 767px)").matches) return;
    const frame = window.requestAnimationFrame(() => {
      activeTabRef.current?.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [project?.id, tab]);

  // 状态栏右侧：当前项目 + 当前 Tab
  useStatusBarPermanent(
    "novel-detail",
    `${project?.title || "项目"} · ${TABS.find((t) => t.id === tab)?.label || tab}`,
  );

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center text-book-text-muted">
        <LoadingSpinner size="lg" text="加载项目…" />
      </div>
    );
  }
  if (!project) return null;

  const homeEntryRoute = getProjectHomeEntryRoute(project);
  const homeEntryLabel = getProjectHomeEntryLabel(project);
  const canEditProject =
    project.my_role === "owner" || project.my_role === "manager" || project.my_role === "editor";
  const canOwnProject = project.my_role === "owner";
  const visibleTabs = canEditProject
    ? TABS
    : TABS.filter(
        (item) => item.id !== "characters" && item.id !== "relationships" && item.id !== "imports",
      );
  const workspaceRoute = getProjectWorkspaceRoute(project);
  const workspaceLabel = getProjectWorkspaceLabel(project) ?? "进入写作桌";
  const showHomeEntryButton =
    canEditProject &&
    homeEntryRoute !== `/novel/${projectId}` &&
    homeEntryRoute !== workspaceRoute;
  const statusMachineDescription = canOwnProject
    ? "查看状态历史，推进或回滚项目阶段。"
    : canEditProject
      ? "查看状态历史，推进项目阶段。"
      : "查看项目流程和状态历史。";

  const handleRagSync = async () => {
    if (!canEditProject) {
      addToast("当前账号没有编辑此项目的权限", "error");
      return;
    }
    if (ragActionBusyRef.current) return;
    ragActionBusyRef.current = true;
    const requestProjectId = projectId;
    let requestSeq: number | null = null;
    try {
      const ok = await confirmDialog({
        title: "同步资料索引",
        message: "将章节、分析数据、大纲、蓝图、世界观和角色档案同步到资料检索索引？未变化的内容会自动跳过，并清理已不存在的记录。",
        confirmText: "开始同步",
        dialogType: "warning",
      });
      if (!ok || !isCurrentProject(requestProjectId)) return;
      requestSeq = ragSeqRef.current + 1;
      ragSeqRef.current = requestSeq;
      const activeRequestSeq = requestSeq;
      const taskId = allocateHostTaskId("rag-ingest-all", requestProjectId);
      setActionsOpen(false);
      setRagSyncing(true);
      const result = await runProgressTask(
        {
          cancelTaskId: taskId,
          title: "同步资料索引",
          startMessage: "正在统计待同步内容…",
          successMessage: "资料索引同步完成",
          errorMessage: "资料索引同步失败",
          total: 1,
          cancelLabel: "停止同步",
          runningHint: "同步过程中请保持当前窗口打开",
          formatProgress: formatRagProgress,
        },
        async ({ setProgress }) => {
          if (!isCurrentAction(requestProjectId, ragSeqRef, activeRequestSeq)) {
            throw new Error("任务已停止");
          }
          setProgress("正在统计待同步内容…", 0, 1);
          const res = await ragIngestAll(requestProjectId, taskId);
          setProgress(`资料索引同步完成：${formatRagIngestResultSummary(res)}`, 1, 1);
          return res;
        },
      );
      if (!isCurrentAction(requestProjectId, ragSeqRef, activeRequestSeq)) return;
      addToast(
        result.success === false
          ? `资料索引同步完成，但有 ${result.errors.length} 个问题：${formatRagIngestResultSummary(result)}`
          : `资料索引同步完成：${formatRagIngestResultSummary(result)}`,
        result.success === false ? "error" : "success",
      );
    } catch (err) {
      if (requestSeq == null) {
        if (isCurrentProject(requestProjectId)) {
          addToast(extractErrorMessage(err, "资料索引同步失败"), "error");
        }
        return;
      }
      if (!isCurrentAction(requestProjectId, ragSeqRef, requestSeq)) return;
      const message = extractErrorMessage(err, "资料索引同步失败");
      if (isCancelledOperation(message)) {
        addToast("资料索引同步已停止", "info");
      } else {
        addToast(message, "error");
      }
    } finally {
      if (requestSeq == null) {
        ragActionBusyRef.current = false;
      } else if (isCurrentAction(requestProjectId, ragSeqRef, requestSeq)) {
        setRagSyncing(false);
        ragActionBusyRef.current = false;
      }
    }
  };

  const handleRefineBlueprint = async () => {
    if (!canEditProject) {
      addToast("当前账号没有编辑此项目的权限", "error");
      return;
    }
    if (refineActionBusyRef.current) return;
    refineActionBusyRef.current = true;
    const requestProject = project;
    const requestProjectId = projectId;
    if (!requestProject) {
      refineActionBusyRef.current = false;
      return;
    }
    let requestSeq: number | null = null;
    try {
      const focus = await promptDialog({
        title: "优化蓝图",
        message: "可选：输入优化方向。留空则全面优化。",
        placeholder: "例如：强化主角动机、增加中段反转、压缩支线…",
        multiline: true,
        confirmText: "开始优化",
      });
      if (focus === null || !isCurrentProject(requestProjectId)) return;
      requestSeq = refineSeqRef.current + 1;
      refineSeqRef.current = requestSeq;
      setRefining(true);
      await refineBlueprint({
        project_id: requestProjectId,
        focus: nonEmptyText(focus),
        variables: null,
      });
      if (!isCurrentAction(requestProjectId, refineSeqRef, requestSeq)) return;
      addToast("蓝图已优化", "success");
      await refresh(requestProjectId);
    } catch (err) {
      if (requestSeq == null) {
        if (isCurrentProject(requestProjectId)) {
          addToast(extractErrorMessage(err, "优化蓝图失败"), "error");
        }
      } else if (isCurrentAction(requestProjectId, refineSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, "优化蓝图失败"), "error");
      }
    } finally {
      if (requestSeq == null) {
        refineActionBusyRef.current = false;
      } else if (isCurrentAction(requestProjectId, refineSeqRef, requestSeq)) {
        setRefining(false);
        refineActionBusyRef.current = false;
      }
    }
  };

  const handleEditTitle = async () => {
    if (!canEditProject) {
      addToast("当前账号没有编辑此项目的权限", "error");
      return;
    }
    if (titleActionBusyRef.current) return;
    titleActionBusyRef.current = true;
    const requestProject = project;
    const requestProjectId = projectId;
    if (!requestProject) {
      titleActionBusyRef.current = false;
      return;
    }
    try {
      const next = await promptDialog({
        title: "修改项目标题",
        message: "输入新的项目标题。",
        defaultValue: requestProject.title,
        placeholder: "项目标题",
        required: true,
        confirmText: "保存标题",
      });
      if (next != null && next.trim() && next.trim() !== requestProject.title) {
        if (!isCurrentProject(requestProjectId)) return;
        const requestSeq = titleSeqRef.current + 1;
        titleSeqRef.current = requestSeq;
        setSaving(true);
        try {
          await updateProject(requestProjectId, { title: next.trim() });
          if (!isCurrentAction(requestProjectId, titleSeqRef, requestSeq)) return;
          addToast("项目标题已更新", "success");
          await refresh(requestProjectId);
        } catch (err) {
          if (isCurrentAction(requestProjectId, titleSeqRef, requestSeq)) {
            addToast(extractErrorMessage(err, "更新标题失败"), "error");
          }
        } finally {
          if (isCurrentAction(requestProjectId, titleSeqRef, requestSeq)) {
            setSaving(false);
          }
        }
      }
    } catch (err) {
      addToast(extractErrorMessage(err, "更新标题失败"), "error");
    } finally {
      titleActionBusyRef.current = false;
    }
  };

  const handleOpenExport = () => {
    if (!canEditProject) {
      addToast("当前账号没有导出此项目的权限", "error");
      return;
    }
    setActionsOpen(false);
    setExportBusy(false);
    setExportOpen(true);
  };

  return (
    <div className="h-full w-full flex flex-col bg-book-bg">
      {/* 项目身份、当前进度和高频入口；维护操作统一收进更多菜单。 */}
      <PageHeader
        className="a4n-app-page-header--detail h-auto shrink-0 py-2 sm:flex-nowrap sm:py-0"
        leading={<BackButton iconOnly title="返回上一页" />}
        title={project.title}
        subtitle={getProjectProgressText(project)}
        actions={
          <>
            {showHomeEntryButton && (
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => navigate(homeEntryRoute)}
                title={homeEntryLabel}
                className="!min-h-8 !px-2.5 !text-xs"
              >
                <ArrowLeft size={12} />
                {homeEntryLabel}
              </BookButton>
            )}
            {workspaceRoute && canEditProject && (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => navigate(workspaceRoute)}
                title={workspaceLabel}
                className="!min-h-8 !px-3 !text-xs"
              >
                {workspaceLabel}
              </BookButton>
            )}
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => setActionsOpen(true)}
              title="更多项目操作"
              className="!min-h-8 !w-8 !px-0"
              aria-label="更多"
            >
              <MoreHorizontal size={14} />
            </BookButton>
          </>
        }
      />

      <nav
        className="tab-bar a4n-mobile-scroll-rail shrink-0 scroll-px-[var(--a4n-content-gutter)] border-b border-book-border/80 bg-book-bg-paper/90 px-[var(--a4n-content-gutter)]"
        role="tablist"
        aria-label="项目详情栏目"
      >
        {visibleTabs.map((t) => (
          <button
            ref={tab === t.id ? activeTabRef : undefined}
            key={t.id}
            type="button"
            role="tab"
            aria-selected={tab === t.id}
            onClick={() => setTab(t.id)}
            data-active={tab === t.id ? "1" : undefined}
            className="pyqt-tab"
            aria-label={t.accessibleLabel}
            title={t.accessibleLabel}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar a4n-page-container">
        {tab === "overview" && <OverviewTab project={project} onRefresh={refresh} />}
        {tab === "world" && <WorldTab projectId={projectId} canEditWorld={canEditProject} />}
        {tab === "characters" && canEditProject && <CharactersTab projectId={projectId} />}
        {tab === "object_profiles" && (
          <ObjectProfilesTab projectId={projectId} canEditProfiles={canEditProject} />
        )}
        {tab === "scene_profiles" && (
          <SceneProfilesTab projectId={projectId} canEditProfiles={canEditProject} />
        )}
        {tab === "relationships" && canEditProject && <RelationshipsTab projectId={projectId} />}
        {tab === "story_events" && (
          <StoryEventsTab projectId={projectId} canOpenWritingDesk={canEditProject} />
        )}
        {tab === "chapter_outline" && <OutlinesTab project={project} />}
        {tab === "images" && <ImagesTab projectId={projectId} canEditImages={canEditProject} />}
        {tab === "imports" && canEditProject && <ImportsTab projectId={projectId} />}
        {tab === "chapters" && (
          <ChaptersTab projectId={projectId} canEditChapters={canEditProject} />
        )}
      </div>

      {/* 项目状态机对话框 */}
      <StatusMachineDialog
        open={stateMachineOpen}
        onClose={() => setStateMachineOpen(false)}
        project={project}
        canForwardStatus={canEditProject}
        canRollbackStatus={canOwnProject}
        onMutated={refresh}
      />

      <Modal
        isOpen={actionsOpen}
        onClose={() => setActionsOpen(false)}
        title="项目操作"
        subtitle="管理项目内容、协作和维护操作。"
        icon={<MoreHorizontal size={15} />}
        maxWidthClassName="max-w-2xl"
        bodyClassName="px-4 py-4"
      >
        <div className="space-y-4">
          <ProjectActionGroup title="项目管理">
            {canEditProject ? (
              <ProjectActionButton
                icon={saving ? <LoadingSpinner size="sm" /> : <Edit2 size={14} />}
                title="修改标题"
                description="调整项目在列表、写作台和导出文件中显示的名称。"
                disabled={saving}
                onClick={() => {
                  setActionsOpen(false);
                  void handleEditTitle();
                }}
              />
            ) : null}
            <ProjectActionButton
              icon={<Users size={14} />}
              title="协作成员"
              description="邀请协作者，调整成员权限或管理待处理邀请。"
              onClick={() => {
                setActionsOpen(false);
                setCollaborationOpen(true);
              }}
            />
            <ProjectActionButton
              icon={<GitBranch size={14} />}
              title="项目阶段"
              description={statusMachineDescription}
              onClick={() => {
                setActionsOpen(false);
                setStateMachineOpen(true);
              }}
            />
          </ProjectActionGroup>

          {canEditProject ? (
            <ProjectActionGroup title="内容维护">
              {project.status === "draft" ? (
                <ProjectActionButton
                  icon={<Sparkles size={14} />}
                  title="灵感对话"
                  description="继续补充灵感，生成或调整首版蓝图。"
                  onClick={() => {
                    setActionsOpen(false);
                    navigate(`/inspiration/${projectId}`);
                  }}
                />
              ) : null}
              <ProjectActionButton
                icon={
                  ragSyncing ? <LoadingSpinner size="sm" /> : <Database size={14} />
                }
                title="同步资料索引"
                description="更新资料检索所使用的章节、大纲、蓝图和角色档案。"
                disabled={ragSyncing}
                onClick={handleRagSync}
              />
              <ProjectActionButton
                icon={
                  refining ? <LoadingSpinner size="sm" /> : <Wand2 size={14} />
                }
                title="优化蓝图"
                description="通过 LLM 重新打磨故事蓝图。"
                disabled={refining}
                onClick={handleRefineBlueprint}
              />
            </ProjectActionGroup>
          ) : null}

          <ProjectActionGroup title="输出与查看">
            {canEditProject ? (
              <ProjectActionButton
                icon={<FileText size={14} />}
                title="导出小说"
                description="导出正文、章节大纲和角色结构数据。"
                onClick={handleOpenExport}
              />
            ) : null}
            <ProjectActionButton
              icon={<Network size={14} />}
              title="内容关联"
              description="查看文字、图片和视频之间的来源与下游关系。"
              onClick={() => {
                setActionsOpen(false);
                setAssetGraphOpen(true);
              }}
            />
          </ProjectActionGroup>
        </div>
      </Modal>

      <Modal
        isOpen={collaborationOpen}
        onClose={() => setCollaborationOpen(false)}
        title="协作成员"
        subtitle="所有者和管理员可以邀请成员、调整权限和撤销待处理邀请。"
        icon={<Users size={15} />}
        maxWidthClassName="max-w-3xl"
        bodyClassName="px-4 py-4"
      >
        <ProjectCollaborationPanel project={project} onRefresh={refresh} />
      </Modal>

      <Modal
        isOpen={exportOpen}
        onClose={() => {
          if (!exportBusy) setExportOpen(false);
        }}
        title="导出小说"
        subtitle="这里导出正文和结构数据；漫画页与视频请在对应工作区导出。"
        icon={<FileText size={15} />}
        maxWidthClassName="max-w-4xl"
        bodyClassName="px-4 py-4"
        closeOnBackdrop={!exportBusy}
        closeOnEscape={!exportBusy}
        showCloseButton={!exportBusy}
      >
        <ExportPanel project={project} onBusyChange={setExportBusy} />
      </Modal>

      <AssetDependencyGraphDialog
        open={assetGraphOpen}
        projectId={projectId}
        onClose={() => setAssetGraphOpen(false)}
      />
    </div>
  );
};

function formatRagProgress(current: number, total: number, percent: number): React.ReactNode {
  return `${current}/${total} 条 (${percent}%)`;
}

const ProjectActionGroup: React.FC<{
  title: string;
  children: React.ReactNode;
}> = ({ title, children }) => (
  <section>
    <h3 className="mb-1.5 px-1 text-small font-semibold text-book-text-muted">{title}</h3>
    <div className="a4n-flat-surface divide-y divide-book-border/40 overflow-hidden">
      {children}
    </div>
  </section>
);

const ProjectActionButton: React.FC<{
  icon: React.ReactNode;
  title: string;
  description: string;
  disabled?: boolean;
  onClick: () => void;
}> = ({ icon, title, description, disabled, onClick }) => (
  <button
    type="button"
    disabled={disabled}
    onClick={onClick}
    className="flex min-h-16 w-full items-start gap-3 px-1 py-3 text-left transition-colors hover:bg-book-bg-tertiary/30 disabled:cursor-not-allowed disabled:opacity-60"
  >
    <span className="mt-0.5 inline-flex h-7 w-7 shrink-0 items-center justify-center text-book-primary">
      {icon}
    </span>
    <span className="min-w-0">
      <span className="block text-sm font-semibold text-book-text-main">{title}</span>
      <span className="mt-1 block text-small leading-relaxed text-book-text-muted">
        {description}
      </span>
    </span>
  </button>
);
