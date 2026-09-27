import React, { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { usePersistedTab } from "../hooks/usePersistedTab";
import { createWorkflowTaskId, useProgressTask } from "../hooks/useProgressTask";
import {
  getProjectHomeEntryRoute,
  getProjectPrimaryRoute,
  getProjectWorkspaceRoute,
} from "../utils/projectRouting";
import { isCancelledOperation } from "../utils/cancellation";
import { useToast } from "../components/feedback/Toast";
import { choiceDialog, confirmDialog } from "../components/feedback/ConfirmDialog";
import { promptDialog } from "../components/feedback/PromptDialog";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../api";
import { formatRagIngestResultSummary, ragIngestAll } from "../api/rag";
import { cancelWorkflowTask } from "../api/streaming";
import {
  acceptProjectInvite,
  createDerivedProject,
  createFreeformVideoProject,
  createProject,
  declineProjectInvite,
  deleteProject,
  duplicateProject,
  exportProjectBundleData,
  importProjectBundleUpload,
  importProjectFamilyUpload,
  importVideoStudioProjectUpload,
  listAllProjects,
  listMyProjectInvites,
  listProjects,
  PROJECT_BUNDLE_PACKAGE_SUFFIX,
  PROJECT_FAMILY_PACKAGE_SUFFIX,
  VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
  type NovelProject,
  type ProjectAccessScope,
  type ProjectInvite,
  type ProjectKind,
} from "../api/project";
import { NovelListSidebar } from "./novel-list/NovelListSidebar";
import type { CreateMode } from "./novel-list/NovelListDialogs";
import { ProjectListPanel } from "./novel-list/ProjectListPanel";
import { createProjectActions } from "./novel-list/projectActions";
import { usePackagePreferences } from "../store/packagePreferences";
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
import { useI18n } from "../i18n";

const ProjectCreationDialog = lazy(() =>
  import("./novel-list/NovelListDialogs").then((module) => ({
    default: module.ProjectCreationDialog,
  })),
);
const ProjectImportDialog = lazy(() =>
  import("./novel-list/NovelListDialogs").then((module) => ({
    default: module.ProjectImportDialog,
  })),
);
const ProjectTextImportDialog = lazy(() =>
  import("./novel-list/NovelListDialogs").then((module) => ({
    default: module.ProjectTextImportDialog,
  })),
);

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

export const NovelList: React.FC<NovelListProps> = () => {
  const { isVisible: isPackageVisible } = usePackagePreferences();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { locale, t } = useI18n();
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
  ): Promise<boolean> => {
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
        return false;
      }
      setProjects(items);
      setLoadError(null);
      return true;
    } catch (err) {
      if (
        mountedRef.current &&
        refreshSeqRef.current === requestSeq &&
        activeTabRef.current === requestTab &&
        activeAccessScopeRef.current === requestScope &&
        activeProjectKindFilterRef.current === requestKind
      ) {
        const message = extractErrorMessage(err, t("novel.projectLoadFailed"));
        setLoadError(message);
      }
      return false;
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
        addToast(extractErrorMessage(err, t("novel.projectLoadFailed")), "error");
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
    () => (tab === "recent" ? getRecentProjectFamilies(projects) : []),
    [projects, tab],
  );
  const groupedProjectFamilies = useMemo(
    () => (tab === "all" ? groupProjectFamiliesByTitle(projects) : []),
    [projects, tab],
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
          title: t("novel.projectListActions.syncTitle", { title: project.title }),
          startMessage: t("novel.projectListActions.syncStart"),
          successMessage: t("novel.projectListActions.syncSuccess"),
          errorMessage: t("novel.projectListActions.syncFailed"),
          total: 1,
          cancelLabel: t("novel.stopSync"),
          runningHint: t("novel.projectListActions.syncRunningHint"),
        },
        async ({ setProgress }) => {
          const result = await ragIngestAll(projectId, taskId);
          setProgress(t("novel.projectListActions.syncProgress", {
            summary: formatRagIngestResultSummary(result, locale, t),
          }), 1, 1);
        },
      );
    }, t);
    if (warning && mountedRef.current) {
      addToast(t("novel.projectListActions.savedWithWarning", { title: project.title, warning }), "info");
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
              title: t("novel.projectListActions.unnamedVideoProject"),
              description: "",
            })
          : await createProject({
              title: t("novel.unnamedProject"),
              description: "",
              project_type: "novel_long",
            });
      if (!mountedRef.current || createSeqRef.current !== requestSeq) return;
      if (!await refresh(activeTabRef.current)) return;
      addToast(t("novel.projectListActions.created", { title: project.title }), "success");
      setCreateDialogOpen(false);
      if (!mountedRef.current || createSeqRef.current !== requestSeq) return;
      if (requestMode === "ai") {
        navigate(`/inspiration/${project.id}`);
      } else if (requestMode === "free") {
        navigate(getProjectPrimaryRoute(project));
      } else if (requestMode === "video_canvas") {
        navigate(getProjectPrimaryRoute(project));
      }
    } catch (err) {
      if (mountedRef.current && createSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, t("novel.projectListActions.createFailed")), "error");
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
      title: t("novel.projectListActions.duplicateTitle"),
      message: t("novel.projectListActions.duplicateMessage", { title: project.title }),
      defaultValue: t("novel.projectListActions.duplicateDefault", { title: project.title }),
      placeholder: t("novel.projectListActions.duplicatePlaceholder"),
      required: true,
      confirmText: t("novel.projectListActions.duplicateConfirm"),
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
      if (!await refresh(activeTabRef.current)) return;
      addToast(t("novel.projectListActions.duplicated", { title: np.title }), "success");
      if (!mountedRef.current) return;
      await syncCreatedProjectMaterials(np);
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.projectListActions.duplicateFailed")), "error");
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleCreateDerivedProject = async (
    project: NovelProject,
    kind: DerivedProjectKind,
  ) => {
    const label = kind === "comic"
      ? t("novel.comicProject")
      : kind === "video"
        ? t("novel.videoProject")
        : t("novel.galgameProject");
    if (project.project_kind !== "text") {
      addToast(t("novel.projectListActions.derivedSourceOnly", { label }), "error");
      return;
    }
    const actionKey = `derived-${kind}-${project.id}`;
    if (!beginProjectAction(actionKey)) return;
    const title = await promptDialog({
      title: t("novel.projectListActions.derivedTitle", { label }),
      message: t("novel.projectListActions.derivedMessage", { title: project.title, label }),
      defaultValue: `${project.title} ${label}`,
      placeholder: t("novel.projectListActions.derivedPlaceholder", { label }),
      required: true,
      confirmText: t("novel.projectListActions.nextStep"),
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
      title: t("novel.projectListActions.chapterRangeTitle"),
      message: t("novel.projectListActions.chapterRangeMessage"),
      defaultValue: "1-1",
      placeholder: t("novel.projectListActions.chapterRangePlaceholder"),
      required: true,
      confirmText: t("novel.projectListActions.create"),
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
      addToast(t("novel.projectListActions.chapterRangeFormat"), "error");
      finishProjectAction(actionKey);
      return;
    }
    const start = Number(match[1]);
    const end = Number(match[2]);
    if (!Number.isInteger(start) || !Number.isInteger(end) || start <= 0 || end < start) {
      addToast(t("novel.projectListActions.chapterRangeInvalid"), "error");
      finishProjectAction(actionKey);
      return;
    }

    try {
      const derived = await createDerivedProject({
        root_project_id: project.id,
        derived_kind: kind,
        title: title.trim(),
        description: t("novel.projectListActions.derivedDescription", { title: project.title, label }),
        start_chapter: start,
        end_chapter: end,
      });
      if (!mountedRef.current) return;
      if (!await refresh(activeTabRef.current)) return;
      addToast(t("novel.projectListActions.derivedCreated", { label, title: derived.title }), "success");
      if (!mountedRef.current) return;
      navigate(getProjectHomeEntryRoute(derived));
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.projectListActions.derivedCreateFailed", { label })), "error");
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleExport = async (project: NovelProject) => {
    const actionKey = `export-${project.id}`;
    if (!beginProjectAction(actionKey)) return;
    try {
      let includeDerivedProjects = false;
      if (project.project_kind === "text") {
        const scope = await choiceDialog({
          title: t("novel.projectListActions.exportTitle"),
          message: t("novel.projectListActions.exportScopeMessage", { title: project.title }),
          confirmText: t("novel.projectListActions.exportCurrent"),
          secondaryText: t("novel.projectListActions.exportFamily"),
          cancelText: t("common.cancel"),
        });
        if (scope === "cancel") return;
        includeDerivedProjects = scope === "secondary";
      }
      const result = await exportProjectBundleData({
        project_id: project.id,
        include_derived_projects: includeDerivedProjects,
      });
      if (!mountedRef.current) return;
      downloadBlob(
        result.file_name,
        base64DataToBlob(result.data_base64, "application/zip"),
      );
      addToast(t("novel.projectListActions.exported", { file: result.file_name }), "success");
    } catch (err) {
      if (!mountedRef.current) return;
      const message = extractErrorMessage(err, t("novel.projectListActions.exportFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleDelete = async (project: NovelProject) => {
    const actionKey = `delete-${project.id}`;
    if (!beginProjectAction(actionKey)) return;
    const ok = await confirmDialog({
      title: t("novel.projectListActions.deleteTitle"),
      message: t("novel.projectListActions.deleteMessage", { title: project.title }),
      confirmText: t("novel.projectListActions.permanentDelete"),
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
      setProjects((current) => current.filter((item) => item.id !== project.id));
      if (!await refresh(activeTabRef.current)) return;
      addToast(t("novel.projectListActions.deleted", { title: project.title }), "success");
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.projectListActions.deleteFailed")), "error");
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };


  const handleImport = async () => {
    if (!mountedRef.current || importBusyRef.current) return;
    const selectedFile = importFile;
    if (!selectedFile) {
      addToast(t("novel.projectListActions.selectImportFile"), "error");
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
      if (normalizedFileName.endsWith(PROJECT_BUNDLE_PACKAGE_SUFFIX)) {
        const report = await importProjectBundleUpload(input);
        if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
        const imported = report.project_summaries;
        addToast(
          t("novel.projectListActions.importedBundle", {
            titles: imported.map((item) => item.title).join(" / "),
          }),
          "success",
        );
      } else if (normalizedFileName.endsWith(PROJECT_FAMILY_PACKAGE_SUFFIX)) {
        const imported = await importProjectFamilyUpload(input);
        if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
        importedTextProject = imported.find((item) => item.project_kind === "text") ?? null;
        addToast(t("novel.projectListActions.importedFamily", { titles: imported.map((item) => item.title).join(" / ") }), "success");
      } else if (normalizedFileName.endsWith(VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX)) {
        const imported = await importVideoStudioProjectUpload(input);
        if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
        addToast(t("novel.projectListActions.importedVideo", { title: imported.title }), "success");
      } else {
        throw new Error(
          t("novel.projectListActions.invalidImportFile", {
            bundleSuffix: PROJECT_BUNDLE_PACKAGE_SUFFIX,
            familySuffix: PROJECT_FAMILY_PACKAGE_SUFFIX,
            videoSuffix: VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
          }),
        );
      }
      setImportDialogOpenState(false);
      await refresh(activeTabRef.current);
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      if (importedTextProject) await syncCreatedProjectMaterials(importedTextProject);
    } catch (err) {
      if (mountedRef.current && importSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, t("novel.projectListActions.importFailed")), "error");
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
        `${PROJECT_BUNDLE_PACKAGE_SUFFIX},${PROJECT_FAMILY_PACKAGE_SUFFIX},${VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX},application/zip`,
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
        addToast(extractErrorMessage(err, t("novel.projectListActions.filePickerFailed")), "error");
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
      addToast(t("novel.projectListActions.inviteAccepted"), "success");
      await loadPendingInvites();
      await refresh(activeTabRef.current);
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.projectListActions.acceptInviteFailed")), "error");
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
      addToast(t("novel.projectListActions.inviteDeclined"), "success");
      await loadPendingInvites();
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, t("novel.projectListActions.declineInviteFailed")), "error");
      }
    } finally {
      finishInviteAction(invite.id);
    }
  };

  const handleOpenProject = (project: NovelProject) => {
    if (project.lifecycle_gate !== "active" && project.project_kind === "text") {
      navigate(getProjectPrimaryRoute(project));
      return;
    }
    navigate(getProjectHomeEntryRoute(project));
  };

  const handleOpenWorkspace = (project: NovelProject) => {
    navigate(getProjectWorkspaceRoute(project));
  };

  const projectActions = (project: NovelProject) =>
    createProjectActions(project, {
      onOpenDetails: (item) => navigate(getProjectPrimaryRoute(item)),
      onOpenInspiration: (item) => navigate(`/inspiration/${item.id}`),
      onDuplicate: (item) => void handleDuplicate(item),
      onCreateDerivedProject: (item, kind) => void handleCreateDerivedProject(item, kind),
      onExport: (item) => void handleExport(item),
      onImportText: (item) => setImportTextProjectId(item.id),
      onDelete: (item) => void handleDelete(item),
    }, t, isPackageVisible);

  return (
    <div className="a4n-home-page relative h-full w-full overflow-y-auto overflow-x-hidden bg-book-bg md:overflow-hidden">
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
          onCreateProject={() => setCreateDialogOpen(true)}
          onImportProject={() => setImportDialogOpenState(true)}
          onOpenSettings={() => navigate("/settings")}
          onRetry={() => void refresh()}
          onAccessScopeChange={handleAccessScopeChange}
          onProjectKindFilterChange={handleProjectKindFilterChange}
          onAcceptInvite={(invite) => void handleAcceptInvite(invite)}
          onDeclineInvite={(invite) => void handleDeclineInvite(invite)}
          onOpenProject={handleOpenProject}
          onOpenWorkspace={handleOpenWorkspace}
          getActions={projectActions}
        />
      </div>

      <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
        {createDialogOpen ? (
          <ProjectCreationDialog
            open
            creating={creating}
            onClose={() => setCreateDialogOpen(false)}
            onSelectMode={handleSelectMode}
          />
        ) : null}
        {importDialogOpen ? (
          <ProjectImportDialog
            open
            importing={importing}
            importFileName={importFile?.name ?? ""}
            onClose={() => setImportDialogOpenState(false)}
            onSelectFile={handleSelectImportFile}
            onImport={handleImport}
          />
        ) : null}
        {importTextProjectId != null ? (
          <ProjectTextImportDialog
            projectTitle={projects.find((project) => project.id === importTextProjectId)?.title}
            projectId={importTextProjectId}
            onClose={() => setImportTextProjectId(null)}
          />
        ) : null}
      </Suspense>
    </div>
  );
};
