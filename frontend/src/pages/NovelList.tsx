import React, { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { usePersistedTab } from "../hooks/usePersistedTab";
import { createWorkflowTaskId, useProgressTask } from "../hooks/useProgressTask";
import {
  getProjectHomeEntryRoute,
  getProjectWorkspaceRoute,
} from "../utils/projectRouting";
import { isCancelledOperation } from "../utils/cancellation";
import { useToast } from "../components/feedback/Toast";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { promptDialog } from "../components/feedback/PromptDialog";
import { extractErrorMessage } from "../api";
import { formatRagIngestResultSummary, ragIngestAll } from "../api/rag";
import { cancelWorkflowTask } from "../api/streaming";
import {
  acceptProjectInvite,
  branchProject,
  createDerivedProject,
  createFreeformVideoProject,
  createProject,
  declineProjectInvite,
  deleteProject,
  duplicateProject,
  exportDerivedProjectData,
  exportProjectFamilyData,
  exportVideoStudioProjectData,
  importProjectFamilyUpload,
  importVideoStudioProjectUpload,
  listAllProjects,
  listMyProjectInvites,
  listProjects,
  PROJECT_FAMILY_PACKAGE_SUFFIX,
  VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
  type NovelProject,
  type ProjectAccessScope,
  type ProjectInvite,
  type ProjectKind,
} from "../api/project";
import { NovelListSidebar } from "./novel-list/NovelListSidebar";
import {
  ProjectCreationDialog,
  ProjectImportDialog,
  ProjectTextImportDialog,
  type CreateMode,
} from "./novel-list/NovelListDialogs";
import { ProjectListPanel } from "./novel-list/ProjectListPanel";
import { createProjectActions } from "./novel-list/projectActions";
import {
  getRecentProjectFamilies,
  groupProjectFamiliesByTitle,
  pickRandomQuote,
  PROJECT_KIND_FILTERS,
  PROJECT_TABS,
  trySyncTextProjectMaterials,
  type ProjectKindFilter,
  type ProjectTab,
  type Quote,
} from "./novel-list/novelListUtils";
import { pickLocalFile, readFileAsBase64 } from "../utils/fileUpload";
import { base64DataToBlob, downloadBlob } from "../utils/downloadFile";

/**
 * 首页
 *
 * 页面结构：
 *   - 左右分栏：左侧品牌区，右侧项目区
 *   - 浮动粒子背景
 *   - 右上角设置按钮
 *   - 创作箴言中英文双行随机
 *   - 右侧 Tab：最近项目 / 全部项目，配合项目类型与访问权限筛选
 */

interface NovelListProps {
  isDark?: boolean;
}

type DerivedProjectKind = Exclude<ProjectKind, "text">;

const DERIVED_PROJECT_LABELS: Record<DerivedProjectKind, string> = {
  comic: "漫画项目",
  video: "视频项目",
  galgame: "GALGame 项目",
};
const DEFAULT_TEXT_PROJECT_TITLE = "未命名小说项目";

export const NovelList: React.FC<NovelListProps> = () => {
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();

  const [projects, setProjects] = useState<NovelProject[]>([]);
  const [accessScope, setAccessScope] = useState<ProjectAccessScope>("all");
  const [inviteActionIds, setInviteActionIds] = useState<ReadonlySet<number>>(new Set());
  const [projectKindFilter, setProjectKindFilter] = usePersistedTab<ProjectKindFilter>(
    "a4n:home:project-kind-filter",
    "all",
    PROJECT_KIND_FILTERS,
  );
  const [pendingInvites, setPendingInvites] = useState<ProjectInvite[]>([]);
  const [tab, setTab] = usePersistedTab<ProjectTab>(
    "a4n:home:project-tab",
    "recent",
    PROJECT_TABS,
  );
  const mountedRef = useRef(false);
  const activeTabRef = useRef<ProjectTab>(tab);
  const activeAccessScopeRef = useRef<ProjectAccessScope>(accessScope);
  const activeProjectKindFilterRef = useRef<ProjectKindFilter>(projectKindFilter);
  const refreshSeqRef = useRef(0);
  const inviteSeqRef = useRef(0);
  const createSeqRef = useRef(0);
  const importDialogOpenRef = useRef(false);
  const importFileSeqRef = useRef(0);
  const importSeqRef = useRef(0);
  const createBusyRef = useRef(false);
  const importBusyRef = useRef(false);
  const inviteActionIdsRef = useRef<Set<number>>(new Set());
  const projectActionKeysRef = useRef<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [importDialogOpen, setImportDialogOpen] = useState(false);
  const [importTextProjectId, setImportTextProjectId] = useState<number | null>(null);
  const [importFile, setImportFile] = useState<File | null>(null);
  const [importing, setImporting] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const quote = useMemo<Quote>(() => pickRandomQuote(), []);

  const setImportDialogOpenState = (open: boolean) => {
    importDialogOpenRef.current = open;
    if (!open) {
      importFileSeqRef.current += 1;
      setImportFile(null);
    }
    setImportDialogOpen(open);
  };

  const refresh = async (
    requestTab: ProjectTab = activeTabRef.current,
    requestScope: ProjectAccessScope = activeAccessScopeRef.current,
    requestKind: ProjectKindFilter = activeProjectKindFilterRef.current,
  ) => {
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    setLoading(true);
    setLoadError(null);
    try {
      const filters = {
        project_kind: requestKind === "all" ? undefined : requestKind,
        access_scope: requestScope,
      };
      const items = requestTab === "all"
        ? await listAllProjects(filters)
        : (await listProjects(1, 20, filters)).items;
      if (
        !mountedRef.current ||
        refreshSeqRef.current !== requestSeq ||
        activeTabRef.current !== requestTab ||
        activeAccessScopeRef.current !== requestScope ||
        activeProjectKindFilterRef.current !== requestKind
      ) {
        return;
      }
      setProjects(items);
      setLoadError(null);
    } catch (err) {
      if (
        mountedRef.current &&
        refreshSeqRef.current === requestSeq &&
        activeTabRef.current === requestTab &&
        activeAccessScopeRef.current === requestScope &&
        activeProjectKindFilterRef.current === requestKind
      ) {
        const message = extractErrorMessage(err, "加载项目列表失败");
        setLoadError(message);
        addToast(message, "error");
      }
    } finally {
      if (
        mountedRef.current &&
        refreshSeqRef.current === requestSeq &&
        activeTabRef.current === requestTab &&
        activeAccessScopeRef.current === requestScope &&
        activeProjectKindFilterRef.current === requestKind
      ) {
        setLoading(false);
      }
    }
  };

  const loadPendingInvites = async () => {
    const requestSeq = inviteSeqRef.current + 1;
    inviteSeqRef.current = requestSeq;
    try {
      const invites = await listMyProjectInvites();
      if (!mountedRef.current || inviteSeqRef.current !== requestSeq) return;
      setPendingInvites(invites);
    } catch (err) {
      if (mountedRef.current && inviteSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, "加载项目邀请失败"), "error");
      }
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      refreshSeqRef.current += 1;
      inviteSeqRef.current += 1;
      createSeqRef.current += 1;
      importFileSeqRef.current += 1;
      importSeqRef.current += 1;
      createBusyRef.current = false;
      importBusyRef.current = false;
      inviteActionIdsRef.current.clear();
      projectActionKeysRef.current.clear();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    activeTabRef.current = tab;
    activeAccessScopeRef.current = accessScope;
    activeProjectKindFilterRef.current = projectKindFilter;
    refreshSeqRef.current += 1;
    setProjects([]);
    setLoadError(null);
    setLoading(false);
    void refresh(tab, accessScope, projectKindFilter);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab, accessScope, projectKindFilter]);

  useEffect(() => {
    void loadPendingInvites();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const recentProjectFamilies = useMemo(
    () => getRecentProjectFamilies(projects),
    [projects],
  );
  const groupedProjectFamilies = useMemo(
    () => groupProjectFamiliesByTitle(projects),
    [projects],
  );

  const beginProjectAction = (key: string): boolean => {
    if (!mountedRef.current || projectActionKeysRef.current.has(key)) return false;
    projectActionKeysRef.current.add(key);
    return true;
  };

  const finishProjectAction = (key: string) => {
    projectActionKeysRef.current.delete(key);
  };

  const syncCreatedProjectMaterials = async (project: NovelProject) => {
    const warning = await trySyncTextProjectMaterials(project, async (projectId) => {
      const taskId = createWorkflowTaskId("new-project-rag-ingest", projectId);
      await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: `同步「${project.title}」资料索引`,
          startMessage: "正在读取新项目的正式资料…",
          successMessage: "新项目资料索引已同步",
          errorMessage: "新项目资料索引同步失败",
          total: 1,
          cancelLabel: "停止同步",
          runningHint: "项目已经创建；索引失败或停止后可稍后重新同步",
        },
        async ({ setProgress }) => {
          const result = await ragIngestAll(projectId, taskId);
          if (!result.success) {
            throw new Error(result.errors.join("；") || "资料索引同步未完成");
          }
          setProgress(`资料索引同步完成：${formatRagIngestResultSummary(result)}`, 1, 1);
        },
      );
    });
    if (warning && mountedRef.current) {
      addToast(`「${project.title}」正式资料已保存；${warning}`, "info");
    }
  };

  const beginInviteAction = (inviteId: number): boolean => {
    if (!mountedRef.current || inviteActionIdsRef.current.has(inviteId)) return false;
    const next = new Set(inviteActionIdsRef.current);
    next.add(inviteId);
    inviteActionIdsRef.current = next;
    setInviteActionIds(next);
    return true;
  };

  const finishInviteAction = (inviteId: number) => {
    if (!inviteActionIdsRef.current.has(inviteId)) return;
    const next = new Set(inviteActionIdsRef.current);
    next.delete(inviteId);
    inviteActionIdsRef.current = next;
    if (mountedRef.current) setInviteActionIds(next);
  };

  const handleCreate = async (requestMode: CreateMode) => {
    if (!mountedRef.current || createBusyRef.current) return;
    createBusyRef.current = true;
    const requestSeq = createSeqRef.current + 1;
    createSeqRef.current = requestSeq;
    setCreating(true);
    try {
      const project =
        requestMode === "video_canvas"
          ? await createFreeformVideoProject({
              title: "未命名视频项目",
              description: "",
            })
          : await createProject({
              title: DEFAULT_TEXT_PROJECT_TITLE,
              description: "",
              project_type: "novel_long",
            });
      if (!mountedRef.current || createSeqRef.current !== requestSeq) return;
      addToast(`已创建：${project.title}`, "success");
      setCreateDialogOpen(false);
      await refresh(activeTabRef.current);
      if (!mountedRef.current || createSeqRef.current !== requestSeq) return;
      if (requestMode === "ai") {
        navigate(`/inspiration/${project.id}`);
      } else if (requestMode === "free") {
        navigate(`/novel/${project.id}`);
      } else if (requestMode === "video_canvas") {
        navigate(`/video-project/${project.id}/studio`);
      }
    } catch (err) {
      if (mountedRef.current && createSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, "创建项目失败"), "error");
      }
    } finally {
      if (mountedRef.current && createSeqRef.current === requestSeq) {
        setCreating(false);
        createBusyRef.current = false;
      }
    }
  };

  const handleSelectMode = (mode: CreateMode) => {
    if (creating || createBusyRef.current) return;
    void handleCreate(mode);
  };

  const handleDuplicate = async (project: NovelProject) => {
    const actionKey = `duplicate-${project.id}`;
    if (!beginProjectAction(actionKey)) return;
    const title = await promptDialog({
      title: "复制项目",
      message: `复制项目「${project.title}」为新项目，请输入新标题。`,
      defaultValue: `${project.title} 副本`,
      placeholder: "新项目标题",
      required: true,
      confirmText: "复制",
    });
    if (!title || !title.trim()) {
      finishProjectAction(actionKey);
      return;
    }
    if (!mountedRef.current) {
      finishProjectAction(actionKey);
      return;
    }
    try {
      const np = await duplicateProject(project.id, title.trim());
      if (!mountedRef.current) return;
      addToast(`已复制为：${np.title}`, "success");
      await refresh(activeTabRef.current);
      if (!mountedRef.current) return;
      await syncCreatedProjectMaterials(np);
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "复制失败"), "error");
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleBranch = async (project: NovelProject) => {
    const actionKey = `branch-${project.id}`;
    if (!beginProjectAction(actionKey)) return;
    const title = await promptDialog({
      title: "创建项目分支",
      message: `从「${project.title}」当前状态创建分支，请输入分支标题。`,
      defaultValue: `${project.title} 分支`,
      placeholder: "分支标题",
      required: true,
      confirmText: "创建分支",
    });
    if (!title || !title.trim()) {
      finishProjectAction(actionKey);
      return;
    }
    if (!mountedRef.current) {
      finishProjectAction(actionKey);
      return;
    }
    try {
      const np = await branchProject(project.id, title.trim(), project.status);
      if (!mountedRef.current) return;
      addToast(`已创建分支：${np.title}`, "success");
      await refresh(activeTabRef.current);
      if (!mountedRef.current) return;
      await syncCreatedProjectMaterials(np);
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "分支失败"), "error");
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleCreateDerivedProject = async (
    project: NovelProject,
    kind: DerivedProjectKind,
  ) => {
    const label = DERIVED_PROJECT_LABELS[kind];
    if (project.project_kind !== "text") {
      addToast(`只能从文本项目创建${label}`, "error");
      return;
    }
    const actionKey = `derived-${kind}-${project.id}`;
    if (!beginProjectAction(actionKey)) return;
    const title = await promptDialog({
      title: `创建${label}`,
      message: `从「${project.title}」创建${label}，请输入项目标题。`,
      defaultValue: `${project.title} ${label}`,
      placeholder: `${label}标题`,
      required: true,
      confirmText: "下一步",
    });
    if (!title || !title.trim()) {
      finishProjectAction(actionKey);
      return;
    }
    if (!mountedRef.current) {
      finishProjectAction(actionKey);
      return;
    }

    const chapterRange = await promptDialog({
      title: "章节范围",
      message: "输入派生章节范围，格式为 1-10。",
      defaultValue: "1-1",
      placeholder: "例如：1-10",
      required: true,
      confirmText: "创建",
    });
    if (!chapterRange || !chapterRange.trim()) {
      finishProjectAction(actionKey);
      return;
    }
    if (!mountedRef.current) {
      finishProjectAction(actionKey);
      return;
    }
    const match = chapterRange.trim().match(/^(\d+)\s*-\s*(\d+)$/);
    if (!match) {
      addToast("章节范围格式必须为 1-10", "error");
      finishProjectAction(actionKey);
      return;
    }
    const start = Number(match[1]);
    const end = Number(match[2]);
    if (!Number.isInteger(start) || !Number.isInteger(end) || start <= 0 || end < start) {
      addToast("章节范围无效", "error");
      finishProjectAction(actionKey);
      return;
    }

    try {
      const derived = await createDerivedProject({
        root_project_id: project.id,
        derived_kind: kind,
        title: title.trim(),
        description: `基于「${project.title}」的${label}`,
        start_chapter: start,
        end_chapter: end,
        creation_options: {},
      });
      if (!mountedRef.current) return;
      addToast(`已创建${label}：${derived.title}`, "success");
      await refresh(activeTabRef.current);
      if (!mountedRef.current) return;
      navigate(getProjectHomeEntryRoute(derived));
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, `创建${label}失败`), "error");
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleExport = async (project: NovelProject) => {
    const actionKey = `export-${project.id}`;
    if (!beginProjectAction(actionKey)) return;
    try {
      const result =
        project.project_kind === "text"
          ? await exportProjectFamilyData({
              root_project_id: project.id,
              include_derived_projects: true,
            })
          : project.project_kind === "video" && project.source_mode === "freeform"
            ? await exportVideoStudioProjectData({
                project_id: project.id,
              })
          : await exportDerivedProjectData({
              derived_project_id: project.id,
            });
      if (!mountedRef.current) return;
      downloadBlob(
        result.file_name,
        base64DataToBlob(result.data_base64, "application/zip"),
      );
      addToast(`项目包已下载：${result.file_name}`, "success");
    } catch (err) {
      if (!mountedRef.current) return;
      const message = extractErrorMessage(err, "导出失败");
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleDelete = async (project: NovelProject) => {
    const actionKey = `delete-${project.id}`;
    if (!beginProjectAction(actionKey)) return;
    const ok = await confirmDialog({
      title: "删除项目",
      message: `确定要删除「${project.title}」吗？此操作不可撤销，项目所有数据将被永久清除。`,
      confirmText: "永久删除",
      dialogType: "danger",
    });
    if (!ok) {
      finishProjectAction(actionKey);
      return;
    }
    if (!mountedRef.current) {
      finishProjectAction(actionKey);
      return;
    }
    try {
      await deleteProject(project.id);
      if (!mountedRef.current) return;
      addToast(`已删除：${project.title}`, "success");
      await refresh(activeTabRef.current);
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "删除失败"), "error");
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };


  const handleImport = async () => {
    if (!mountedRef.current || importBusyRef.current) return;
    const selectedFile = importFile;
    if (!selectedFile) {
      addToast("请选择项目导入文件", "error");
      return;
    }
    importBusyRef.current = true;
    const requestSeq = importSeqRef.current + 1;
    importSeqRef.current = requestSeq;
    setImporting(true);
    try {
      let importedTextProject: NovelProject | null = null;
      const fileName = selectedFile.name.trim();
      const dataBase64 = await readFileAsBase64(selectedFile);
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      const input = { file_name: fileName, data_base64: dataBase64 };
      const normalizedFileName = fileName.toLowerCase();
      if (normalizedFileName.endsWith(PROJECT_FAMILY_PACKAGE_SUFFIX)) {
        const imported = await importProjectFamilyUpload(input);
        if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
        importedTextProject = imported.find((item) => item.project_kind === "text") ?? null;
        addToast(`已导入项目族包：${imported.map((item) => item.title).join(" / ")}`, "success");
      } else if (normalizedFileName.endsWith(VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX)) {
        const imported = await importVideoStudioProjectUpload(input);
        if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
        addToast(`已导入视频画布项目：${imported.title}`, "success");
      } else {
        throw new Error(
          `项目导入文件必须以 ${PROJECT_FAMILY_PACKAGE_SUFFIX} 或 ${VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX} 结尾`,
        );
      }
      setImportDialogOpenState(false);
      await refresh(activeTabRef.current);
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      if (importedTextProject) await syncCreatedProjectMaterials(importedTextProject);
    } catch (err) {
      if (mountedRef.current && importSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, "导入失败"), "error");
      }
    } finally {
      if (mountedRef.current && importSeqRef.current === requestSeq) {
        setImporting(false);
        importBusyRef.current = false;
      }
    }
  };

  const handleSelectImportFile = async () => {
    const requestSeq = importFileSeqRef.current + 1;
    importFileSeqRef.current = requestSeq;
    try {
      const selected = await pickLocalFile(
        `${PROJECT_FAMILY_PACKAGE_SUFFIX},${VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX},application/zip`,
      );
      if (
        selected &&
        mountedRef.current &&
        importDialogOpenRef.current &&
        importFileSeqRef.current === requestSeq
      ) {
        setImportFile(selected);
      }
    } catch (err) {
      if (
        mountedRef.current &&
        importDialogOpenRef.current &&
        importFileSeqRef.current === requestSeq
      ) {
        addToast(extractErrorMessage(err, "打开文件选择器失败"), "error");
      }
    }
  };

  const handleTabChange = (nextTab: ProjectTab) => {
    if (activeTabRef.current === nextTab) return;
    activeTabRef.current = nextTab;
    refreshSeqRef.current += 1;
    setProjects([]);
    setLoadError(null);
    setLoading(true);
    setTab(nextTab);
  };

  const handleAccessScopeChange = (nextScope: ProjectAccessScope) => {
    if (activeAccessScopeRef.current === nextScope) return;
    activeAccessScopeRef.current = nextScope;
    refreshSeqRef.current += 1;
    setProjects([]);
    setLoadError(null);
    setLoading(true);
    setAccessScope(nextScope);
  };

  const handleProjectKindFilterChange = (nextKind: ProjectKindFilter) => {
    if (activeProjectKindFilterRef.current === nextKind) return;
    activeProjectKindFilterRef.current = nextKind;
    refreshSeqRef.current += 1;
    setProjects([]);
    setLoadError(null);
    setLoading(true);
    setProjectKindFilter(nextKind);
  };

  const handleAcceptInvite = async (invite: ProjectInvite) => {
    if (!beginInviteAction(invite.id)) return;
    try {
      await acceptProjectInvite(invite.id);
      if (!mountedRef.current) return;
      addToast("已接受项目邀请", "success");
      await loadPendingInvites();
      await refresh(activeTabRef.current);
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "接受邀请失败"), "error");
      }
    } finally {
      finishInviteAction(invite.id);
    }
  };

  const handleDeclineInvite = async (invite: ProjectInvite) => {
    if (!beginInviteAction(invite.id)) return;
    try {
      await declineProjectInvite(invite.id);
      if (!mountedRef.current) return;
      addToast("已拒绝项目邀请", "success");
      await loadPendingInvites();
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, "拒绝邀请失败"), "error");
      }
    } finally {
      finishInviteAction(invite.id);
    }
  };

  const handleOpenProject = (project: NovelProject) => {
    if (project.lifecycle_gate !== "active" && project.project_kind === "text") {
      navigate(`/novel/${project.id}`);
      return;
    }
    navigate(getProjectHomeEntryRoute(project));
  };

  const handleOpenWorkspace = (project: NovelProject) => {
    navigate(getProjectWorkspaceRoute(project));
  };

  const projectActions = (project: NovelProject) =>
    createProjectActions(project, {
      onOpenDetails: (item) => navigate(`/novel/${item.id}`),
      onOpenInspiration: (item) => navigate(`/inspiration/${item.id}`),
      onDuplicate: (item) => void handleDuplicate(item),
      onBranch: (item) => void handleBranch(item),
      onCreateDerivedProject: (item, kind) => void handleCreateDerivedProject(item, kind),
      onExport: (item) => void handleExport(item),
      onImportText: (item) => setImportTextProjectId(item.id),
      onDelete: (item) => void handleDelete(item),
    });

  return (
    <div className="relative h-full w-full overflow-y-auto overflow-x-hidden bg-book-bg md:overflow-hidden">
      <div className="relative z-10 flex min-h-full w-full flex-col md:h-full md:flex-row md:overflow-hidden">
        <NovelListSidebar
          quote={quote}
          onOpenSettings={() => navigate("/settings")}
          onCreateProject={() => setCreateDialogOpen(true)}
          onImportProject={() => setImportDialogOpenState(true)}
        />

        <ProjectListPanel
          tab={tab}
          loading={loading}
          loadError={loadError}
          empty={!loading && !loadError && projects.length === 0}
          recentProjectFamilies={recentProjectFamilies}
          groupedProjectFamilies={groupedProjectFamilies}
          accessScope={accessScope}
          projectKindFilter={projectKindFilter}
          pendingInvites={pendingInvites}
          inviteActionIds={inviteActionIds}
          onTabChange={handleTabChange}
          onAccessScopeChange={handleAccessScopeChange}
          onProjectKindFilterChange={handleProjectKindFilterChange}
          onAcceptInvite={(invite) => void handleAcceptInvite(invite)}
          onDeclineInvite={(invite) => void handleDeclineInvite(invite)}
          onOpenProject={handleOpenProject}
          onOpenWorkspace={handleOpenWorkspace}
          getActions={projectActions}
        />
      </div>

      <ProjectCreationDialog
        open={createDialogOpen}
        creating={creating}
        onClose={() => setCreateDialogOpen(false)}
        onSelectMode={handleSelectMode}
      />

      <ProjectImportDialog
        open={importDialogOpen}
        importing={importing}
        importFileName={importFile?.name ?? ""}
        onClose={() => setImportDialogOpenState(false)}
        onSelectFile={handleSelectImportFile}
        onImport={handleImport}
      />

      <ProjectTextImportDialog
        projectId={importTextProjectId}
        onClose={() => setImportTextProjectId(null)}
      />
    </div>
  );
};
