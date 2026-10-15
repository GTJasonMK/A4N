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
import { isPositiveI32 } from "../utils/numeric";
import { useToast } from "../components/feedback/Toast";
import { choiceDialog, confirmDialog } from "../components/feedback/ConfirmDialog";
import { promptDialog } from "../components/feedback/PromptDialog";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../api";
import { formatRagIngestResultSummary, ragIngestAll } from "../api/rag";
import { cancelWorkflowTask } from "../api/streaming";
import {
  acceptProjectInvite,
  canEditProjectMaterials,
  createDerivedProject,
  createCanvasProject,
  createStandaloneComicProject,
  createProject,
  declineProjectInvite,
  deleteProject,
  duplicateProject,
  exportProjectBundleData,
  exportCanvasProjectData,
  importProjectBundleUpload,
  importProjectFamilyUpload,
  importCanvasProjectUpload,
  listAllProjects,
  listMyProjectInvites,
  listProjects,
  PROJECT_BUNDLE_PACKAGE_SUFFIX,
  CANVAS_PROJECT_PACKAGE_SUFFIX,
  PROJECT_FAMILY_PACKAGE_SUFFIX,
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
import { UnsavedNavigationGuard } from "../components/navigation/UnsavedNavigationGuard";
import { useNovelDetailDirty } from "./novel-detail/novelDetailDirty";

const ProjectCreationDialog = lazy(() =>
  import("./novel-list/NovelListDialogs").then((module) => ({
    default: module.ProjectCreationDialog,
  })),
);
const VideoProjectSourceDialog = lazy(() =>
  import("./novel-list/NovelListDialogs").then((module) => ({
    default: module.VideoProjectSourceDialog,
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

type DerivedProjectKind = Exclude<ProjectKind, "text" | "canvas">;
const PROJECT_ACCESS_SCOPES: readonly ProjectAccessScope[] = ["all", "owned", "collaborating"];

export const NovelList: React.FC<NovelListProps> = () => {
  const { isVisible: isPackageVisible } = usePackagePreferences();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const { runProgressTask } = useProgressTask();
  const tRef = useRef(t);
  tRef.current = t;
  const localeRef = useRef(locale);
  localeRef.current = locale;

  const [projects, setProjects] = useState<NovelProject[]>([]);
  const [accessScope, setAccessScope] = usePersistedTab<ProjectAccessScope>(
    "a4n:home:access-scope",
    "all",
    PROJECT_ACCESS_SCOPES,
  );
  const [inviteActionIds, setInviteActionIds] = useState<ReadonlySet<number>>(new Set());
  const [projectKindFilter, setProjectKindFilter] = usePersistedTab<ProjectKindFilter>(
    "a4n:home:project-kind-filter",
    "all",
    PROJECT_KIND_FILTERS,
  );
  const [pendingInvites, setPendingInvites] = useState<ProjectInvite[]>([]);
  const [pendingInvitesLoading, setPendingInvitesLoading] = useState(true);
  const [pendingInvitesError, setPendingInvitesError] = useState<unknown | null>(null);
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
  const videoProjectSourceSeqRef = useRef(0);
  const importDialogOpenRef = useRef(false);
  const importFileSeqRef = useRef(0);
  const importSeqRef = useRef(0);
  const createBusyRef = useRef(false);
  const importBusyRef = useRef(false);
  const inviteActionIdsRef = useRef<Set<number>>(new Set());
  const projectActionKeysRef = useRef<Set<string>>(new Set());
  const [projectActionKeys, setProjectActionKeys] = useState<ReadonlySet<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [videoProjectSourceDialogOpen, setVideoProjectSourceDialogOpen] = useState(false);
  const [videoProjectSourceProjects, setVideoProjectSourceProjects] = useState<NovelProject[]>([]);
  const [selectedVideoProjectSourceId, setSelectedVideoProjectSourceId] = useState<number | null>(null);
  const [videoProjectSourceLoading, setVideoProjectSourceLoading] = useState(false);
  const [videoProjectSourceError, setVideoProjectSourceError] = useState<string | null>(null);
  const [importDialogOpen, setImportDialogOpen] = useState(false);
  const [importTextProjectId, setImportTextProjectId] = useState<number | null>(null);
  const importTextProjectReadSeqRef = useRef<number | null>(null);
  const [importFile, setImportFile] = useState<File | null>(null);
  const [importing, setImporting] = useState(false);
  const [loadError, setLoadError] = useState<unknown | null>(null);
  const projectsRef = useRef(projects);
  const pendingInvitesRef = useRef(pendingInvites);
  const pendingInvitesLoadingRef = useRef(pendingInvitesLoading);
  const pendingInvitesErrorRef = useRef(pendingInvitesError);
  const loadingRef = useRef(loading);
  const loadErrorRef = useRef(loadError);
  const videoProjectSourceProjectsRef = useRef(videoProjectSourceProjects);
  const videoProjectSourceLoadingRef = useRef(videoProjectSourceLoading);
  const videoProjectSourceErrorRef = useRef(videoProjectSourceError);
  projectsRef.current = projects;
  pendingInvitesRef.current = pendingInvites;
  pendingInvitesLoadingRef.current = pendingInvitesLoading;
  pendingInvitesErrorRef.current = pendingInvitesError;
  loadingRef.current = loading;
  loadErrorRef.current = loadError;
  videoProjectSourceProjectsRef.current = videoProjectSourceProjects;
  videoProjectSourceLoadingRef.current = videoProjectSourceLoading;
  videoProjectSourceErrorRef.current = videoProjectSourceError;
  useNovelDetailDirty(
    "novel-list-import-file",
    importDialogOpen && importFile != null,
    undefined,
    () => !importBusyRef.current,
  );
  useNovelDetailDirty(
    "novel-list-operations",
    creating || importing || inviteActionIds.size > 0 || projectActionKeys.size > 0,
    undefined,
    () => !createBusyRef.current && !importBusyRef.current &&
      inviteActionIdsRef.current.size === 0 && projectActionKeysRef.current.size === 0,
  );

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
    const isCurrentRequest = () =>
      refreshSeqRef.current === requestSeq &&
      activeTabRef.current === requestTab &&
      activeAccessScopeRef.current === requestScope &&
      activeProjectKindFilterRef.current === requestKind;
    loadingRef.current = true;
    loadErrorRef.current = null;
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
      if (!mountedRef.current) return false;
      if (!isCurrentRequest()) return true;
      projectsRef.current = items;
      setProjects(items);
      loadErrorRef.current = null;
      setLoadError(null);
      return true;
    } catch (err) {
      if (!mountedRef.current) return false;
      if (!isCurrentRequest()) return true;
      loadErrorRef.current = err;
      setLoadError(err);
      return false;
    } finally {
      if (mountedRef.current && isCurrentRequest()) {
        loadingRef.current = false;
        setLoading(false);
      }
    }
  };

  const loadPendingInvites = async (): Promise<boolean> => {
    const requestSeq = inviteSeqRef.current + 1;
    inviteSeqRef.current = requestSeq;
    pendingInvitesLoadingRef.current = true;
    pendingInvitesErrorRef.current = null;
    setPendingInvitesLoading(true);
    setPendingInvitesError(null);
    try {
      const invites = await listMyProjectInvites();
      if (!mountedRef.current || inviteSeqRef.current !== requestSeq) return false;
      pendingInvitesRef.current = invites;
      pendingInvitesErrorRef.current = null;
      setPendingInvites(invites);
      setPendingInvitesError(null);
      return true;
    } catch (err) {
      if (mountedRef.current && inviteSeqRef.current === requestSeq) {
        pendingInvitesRef.current = [];
        pendingInvitesErrorRef.current = err;
        setPendingInvitesError(err);
        setPendingInvites([]);
      }
      return false;
    } finally {
      if (mountedRef.current && inviteSeqRef.current === requestSeq) {
        pendingInvitesLoadingRef.current = false;
        setPendingInvitesLoading(false);
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
      videoProjectSourceSeqRef.current += 1;
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
    loadingRef.current = false;
    loadErrorRef.current = null;
    setProjectListSnapshot([]);
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

  const beginProjectAction = (key: string, ensureCurrent: () => boolean): boolean => {
    if (!mountedRef.current) return false;
    if (projectActionKeysRef.current.has(key)) {
      addToast(tRef.current("novel.projectListActions.actionInProgress"), "info");
      return false;
    }
    if (!ensureCurrent()) return false;
    const next = new Set(projectActionKeysRef.current);
    next.add(key);
    projectActionKeysRef.current = next;
    setProjectActionKeys(next);
    return true;
  };

  const isProjectReadCurrent = (projectId: number, requestSeq: number): boolean =>
    mountedRef.current &&
    refreshSeqRef.current === requestSeq &&
    !loadingRef.current &&
    loadErrorRef.current == null &&
    projectsRef.current.some((item) => item.id === projectId);

  const ensureProjectReadCurrent = (projectId: number, requestSeq: number): boolean => {
    if (!isProjectReadCurrent(projectId, requestSeq)) {
      if (mountedRef.current) addToast(tRef.current("common.listChangedRetry"), "info");
      return false;
    }
    return true;
  };

  const ensureInviteReadCurrent = (inviteId: number, requestSeq: number): boolean => {
    if (
      !mountedRef.current ||
      inviteSeqRef.current !== requestSeq ||
      pendingInvitesLoadingRef.current ||
      pendingInvitesErrorRef.current != null ||
      !pendingInvitesRef.current.some((item) => item.id === inviteId)
    ) {
      if (mountedRef.current) addToast(tRef.current("common.listChangedRetry"), "info");
      return false;
    }
    return true;
  };

  const finishProjectAction = (key: string) => {
    if (!projectActionKeysRef.current.has(key)) return;
    const next = new Set(projectActionKeysRef.current);
    next.delete(key);
    projectActionKeysRef.current = next;
    if (mountedRef.current) setProjectActionKeys(next);
  };

  const setProjectListSnapshot = (nextProjects: NovelProject[]) => {
    projectsRef.current = nextProjects;
    setProjects(nextProjects);
  };

  const syncCreatedProjectMaterials = async (
    project: Pick<NovelProject, "id" | "project_kind" | "title">,
  ) => {
    const warning = await trySyncTextProjectMaterials(project, async (projectId) => {
      const taskId = createWorkflowTaskId("new-project-rag-ingest", projectId);
      await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: tRef.current("novel.projectListActions.syncTitle", { title: project.title }),
          startMessage: tRef.current("novel.projectListActions.syncStart"),
          successMessage: tRef.current("novel.projectListActions.syncSuccess"),
          errorMessage: tRef.current("novel.projectListActions.syncFailed"),
          total: 1,
          cancelLabel: tRef.current("novel.stopSync"),
          runningHint: tRef.current("novel.projectListActions.syncRunningHint"),
        },
        async ({ setProgress }) => {
          const result = await ragIngestAll(projectId, taskId);
          setProgress(tRef.current("novel.projectListActions.syncProgress", {
            summary: formatRagIngestResultSummary(result, localeRef.current, tRef.current),
          }), 1, 1);
        },
      );
    }, (key, params) => tRef.current(key, params));
    if (warning && mountedRef.current) {
      addToast(tRef.current("novel.projectListActions.savedWithWarning", { title: project.title, warning }), "info");
    }
  };

  const refreshAfterMutation = async (): Promise<boolean> => {
    const requestTab = activeTabRef.current;
    const requestScope = activeAccessScopeRef.current;
    const requestKind = activeProjectKindFilterRef.current;
    loadingRef.current = true;
    loadErrorRef.current = null;
    setLoading(true);
    const refreshed = await refresh(requestTab, requestScope, requestKind);
    if (
      !mountedRef.current ||
      activeTabRef.current !== requestTab ||
      activeAccessScopeRef.current !== requestScope ||
      activeProjectKindFilterRef.current !== requestKind
    ) {
      return true;
    }
    return refreshed;
  };

  const beginInviteAction = (inviteId: number): boolean => {
    if (
      !mountedRef.current ||
      inviteActionIdsRef.current.has(inviteId) ||
      pendingInvitesLoadingRef.current ||
      pendingInvitesErrorRef.current != null ||
      !pendingInvitesRef.current.some((invite) => invite.id === inviteId)
    ) {
      if (mountedRef.current) {
        addToast(tRef.current("common.listChangedRetry"), "info");
      }
      return false;
    }
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

  const retryPendingInvites = () => {
    if (pendingInvitesLoadingRef.current) {
      addToast(tRef.current("common.operationInProgress"), "info");
      return;
    }
    void loadPendingInvites();
  };

  const loadVideoProjectSources = async () => {
    const requestSeq = videoProjectSourceSeqRef.current + 1;
    videoProjectSourceSeqRef.current = requestSeq;
    videoProjectSourceProjectsRef.current = [];
    videoProjectSourceLoadingRef.current = true;
    videoProjectSourceErrorRef.current = null;
    setVideoProjectSourceProjects([]);
    setSelectedVideoProjectSourceId(null);
    setVideoProjectSourceLoading(true);
    setVideoProjectSourceError(null);
    try {
      const projects = await listAllProjects({
        project_kind: "text",
        access_scope: "all",
      });
      if (!mountedRef.current || videoProjectSourceSeqRef.current !== requestSeq) return;
      const editableProjects = projects.filter(
        (project) => project.project_kind === "text" && canEditProjectMaterials(project),
      );
      videoProjectSourceProjectsRef.current = editableProjects;
      setVideoProjectSourceProjects(editableProjects);
      setSelectedVideoProjectSourceId(editableProjects[0]?.id ?? null);
    } catch (error) {
      if (mountedRef.current && videoProjectSourceSeqRef.current === requestSeq) {
        const message = extractErrorMessage(error, tRef.current("novel.videoProjectSourceLoadFailed"));
        videoProjectSourceErrorRef.current = message;
        setVideoProjectSourceError(message);
      }
    } finally {
      if (mountedRef.current && videoProjectSourceSeqRef.current === requestSeq) {
        videoProjectSourceLoadingRef.current = false;
        setVideoProjectSourceLoading(false);
      }
    }
  };

  const closeVideoProjectSourceDialog = () => {
    videoProjectSourceSeqRef.current += 1;
    videoProjectSourceLoadingRef.current = false;
    setVideoProjectSourceLoading(false);
    setVideoProjectSourceDialogOpen(false);
  };

  const handleBackToProjectCreation = () => {
    closeVideoProjectSourceDialog();
    setCreateDialogOpen(true);
  };

  const handleCreate = async (requestMode: Exclude<CreateMode, "video">) => {
    if (!mountedRef.current || createBusyRef.current) return;
    createBusyRef.current = true;
    const requestSeq = createSeqRef.current + 1;
    createSeqRef.current = requestSeq;
    setCreating(true);
    try {
      const project =
        requestMode === "canvas"
          ? await createCanvasProject({
              title: t("novel.canvasProject"),
              description: "",
            })
          : requestMode === "comic_workspace"
            ? await createStandaloneComicProject({
                title: t("novel.comicProject"),
                description: "",
              })
          : await createProject({
              title: t("novel.unnamedProject"),
              description: "",
              project_type: "novel_long",
            });
      if (!mountedRef.current || createSeqRef.current !== requestSeq) return;
      addToast(tRef.current("novel.projectListActions.created", { title: project.title }), "success");
      void refreshAfterMutation().then((refreshed) => {
        if (mountedRef.current && !refreshed) {
          addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
        }
      });
      setCreateDialogOpen(false);
      if (requestMode === "ai") {
        navigate(`/inspiration/${project.id}`);
      } else if (requestMode === "free") {
        navigate(getProjectPrimaryRoute(project));
      } else if (requestMode === "canvas" || requestMode === "comic_workspace") {
        navigate(getProjectPrimaryRoute(project));
      }
    } catch (err) {
      if (mountedRef.current && createSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.createFailed")), "error");
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
    if (mode === "video") {
      setCreateDialogOpen(false);
      setVideoProjectSourceDialogOpen(true);
      void loadVideoProjectSources();
      return;
    }
    void handleCreate(mode);
  };

  const handleDuplicate = async (project: NovelProject) => {
    const actionKey = `duplicate-${project.id}`;
    const requestReadSeq = refreshSeqRef.current;
    if (!beginProjectAction(actionKey, () => ensureProjectReadCurrent(project.id, requestReadSeq))) return;
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
    if (!ensureProjectReadCurrent(project.id, requestReadSeq)) {
      finishProjectAction(actionKey);
      return;
    }
    try {
      const np = await duplicateProject(project.id, title.trim());
      if (!mountedRef.current) return;
      addToast(tRef.current("novel.projectListActions.duplicated", { title: np.title }), "success");
      void refreshAfterMutation().then((refreshed) => {
        if (mountedRef.current && !refreshed) {
          addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
        }
      });
      await syncCreatedProjectMaterials(np);
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.duplicateFailed")), "error");
        void refreshAfterMutation().then((refreshed) => {
          if (mountedRef.current && !refreshed) {
            addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
          }
        });
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleCreateDerivedProject = async (
    project: NovelProject,
    kind: DerivedProjectKind,
    readGuard?: () => boolean,
  ) => {
    const label = kind === "comic"
      ? t("novel.comicProject")
      : kind === "video"
        ? t("novel.videoProject")
        : t("novel.galgameProject");
    if (project.project_kind !== "text") {
      addToast(tRef.current("novel.projectListActions.derivedSourceOnly", { label }), "error");
      return;
    }
    const actionKey = `derived-${kind}-${project.id}`;
    const requestReadSeq = refreshSeqRef.current;
    const ensureCurrent = readGuard ?? (() => ensureProjectReadCurrent(project.id, requestReadSeq));
    if (!beginProjectAction(actionKey, ensureCurrent)) return;
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
    if (!ensureCurrent()) {
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
    if (!ensureCurrent()) {
      finishProjectAction(actionKey);
      return;
    }
    const match = chapterRange.trim().match(/^(\d+)\s*-\s*(\d+)$/);
    if (!match) {
      addToast(tRef.current("novel.projectListActions.chapterRangeFormat"), "error");
      finishProjectAction(actionKey);
      return;
    }
    const start = Number(match[1]);
    const end = Number(match[2]);
    if (!isPositiveI32(start) || !isPositiveI32(end) || end < start) {
      addToast(tRef.current("novel.projectListActions.chapterRangeInvalid"), "error");
      finishProjectAction(actionKey);
      return;
    }

    try {
      if (!ensureCurrent()) {
        finishProjectAction(actionKey);
        return;
      }
      const derived = await createDerivedProject({
        root_project_id: project.id,
        derived_kind: kind,
        title: title.trim(),
        description: tRef.current("novel.projectListActions.derivedDescription", { title: project.title, label }),
        start_chapter: start,
        end_chapter: end,
      });
      if (!mountedRef.current) return;
      const canNavigateToCreatedProject = ensureCurrent();
      addToast(tRef.current("novel.projectListActions.derivedCreated", { label, title: derived.title }), "success");
      void refreshAfterMutation().then((refreshed) => {
        if (mountedRef.current && !refreshed) {
          addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
        }
      });
      if (canNavigateToCreatedProject) navigate(getProjectHomeEntryRoute(derived));
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.derivedCreateFailed", { label })), "error");
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleCreateVideoProjectFromSelectedSource = async () => {
    if (videoProjectSourceLoadingRef.current || videoProjectSourceErrorRef.current != null) {
      addToast(tRef.current("common.listChangedRetry"), "info");
      return;
    }
    const source = videoProjectSourceProjects.find(
      (project) => project.id === selectedVideoProjectSourceId,
    );
    if (!source || !canEditProjectMaterials(source)) return;
    if (!videoProjectSourceProjectsRef.current.some((item) => item.id === source.id)) {
      addToast(tRef.current("common.listChangedRetry"), "info");
      return;
    }
    const sourceId = source.id;
    closeVideoProjectSourceDialog();
    const sourceReadSeq = videoProjectSourceSeqRef.current;
    const ensureSourceCurrent = () => {
      const isCurrent =
      mountedRef.current &&
      videoProjectSourceSeqRef.current === sourceReadSeq &&
      !videoProjectSourceLoadingRef.current &&
      videoProjectSourceErrorRef.current == null &&
      videoProjectSourceProjectsRef.current.some((item) => item.id === sourceId);
      if (!isCurrent && mountedRef.current) addToast(tRef.current("common.listChangedRetry"), "info");
      return isCurrent;
    };
    void handleCreateDerivedProject(source, "video", ensureSourceCurrent);
  };

  const handleExport = async (project: NovelProject) => {
    const actionKey = `export-${project.id}`;
    const requestReadSeq = refreshSeqRef.current;
    if (!beginProjectAction(actionKey, () => ensureProjectReadCurrent(project.id, requestReadSeq))) return;
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
      if (!ensureProjectReadCurrent(project.id, requestReadSeq)) return;
      const result = project.project_kind === "canvas"
        ? await exportCanvasProjectData({ project_id: project.id })
        : await exportProjectBundleData({
            project_id: project.id,
            include_derived_projects: includeDerivedProjects,
          });
      if (!mountedRef.current) return;
      downloadBlob(
        result.file_name,
        base64DataToBlob(result.data_base64, "application/zip"),
      );
      addToast(tRef.current("novel.projectListActions.exported", { file: result.file_name }), "success");
    } catch (err) {
      if (!mountedRef.current) return;
      const message = extractErrorMessage(err, tRef.current("novel.projectListActions.exportFailed"));
      addToast(message, isCancelledOperation(message) ? "info" : "error");
    } finally {
      finishProjectAction(actionKey);
    }
  };

  const handleDelete = async (project: NovelProject) => {
    const actionKey = `delete-${project.id}`;
    const requestReadSeq = refreshSeqRef.current;
    if (!beginProjectAction(actionKey, () => ensureProjectReadCurrent(project.id, requestReadSeq))) return;
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
    if (!ensureProjectReadCurrent(project.id, requestReadSeq)) {
      finishProjectAction(actionKey);
      return;
    }
    try {
      await deleteProject(project.id);
      if (!mountedRef.current) return;
      addToast(tRef.current("novel.projectListActions.deleted", { title: project.title }), "success");
      if (!isProjectReadCurrent(project.id, requestReadSeq)) {
        void refreshAfterMutation();
        return;
      }
      setProjectListSnapshot(projectsRef.current.filter((item) => item.id !== project.id));
      void refreshAfterMutation().then((refreshed) => {
        if (mountedRef.current && !refreshed) {
          addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
        }
      });
    } catch (err) {
      if (mountedRef.current) {
        addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.deleteFailed")), "error");
        void refreshAfterMutation().then((refreshed) => {
          if (mountedRef.current && !refreshed) {
            addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
          }
        });
      }
    } finally {
      finishProjectAction(actionKey);
    }
  };


  const handleImport = async () => {
    if (!mountedRef.current || importBusyRef.current) return;
    const selectedFile = importFile;
    if (!selectedFile) {
      addToast(tRef.current("novel.projectListActions.selectImportFile"), "error");
      return;
    }
    importBusyRef.current = true;
    const requestSeq = importSeqRef.current + 1;
    importSeqRef.current = requestSeq;
    setImporting(true);
    try {
      const importedTextProjects: Array<Pick<NovelProject, "id" | "project_kind" | "title">> = [];
      let importWarnings: string[] = [];
      let importSuccessMessage: string | null = null;
      const fileName = selectedFile.name.trim();
      const dataBase64 = await readFileAsBase64(selectedFile);
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      const input = { file_name: fileName, data_base64: dataBase64 };
      const normalizedFileName = fileName.toLowerCase();
      if (normalizedFileName.endsWith(PROJECT_BUNDLE_PACKAGE_SUFFIX)) {
        const report = await importProjectBundleUpload(input);
        if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
        importedTextProjects.push(...report.project_summaries.filter((project) => project.project_kind === "text"));
        importWarnings = report.warnings;
        importSuccessMessage = tRef.current("novel.projectListActions.importedBundle", {
          titles: report.project_summaries.map((item) => item.title).join(" / "),
        });
      } else if (normalizedFileName.endsWith(PROJECT_FAMILY_PACKAGE_SUFFIX)) {
        const imported = await importProjectFamilyUpload(input);
        if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
        importedTextProjects.push(...imported.filter((project) => project.project_kind === "text"));
        importSuccessMessage = tRef.current("novel.projectListActions.importedFamily", { titles: imported.map((item) => item.title).join(" / ") });
      } else if (normalizedFileName.endsWith(CANVAS_PROJECT_PACKAGE_SUFFIX)) {
        const imported = await importCanvasProjectUpload(input);
        if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
        importSuccessMessage = tRef.current("novel.projectListActions.importedCanvas", { title: imported.title });
      } else {
        throw new Error(
          tRef.current("novel.projectListActions.invalidImportFile", {
            bundleSuffix: PROJECT_BUNDLE_PACKAGE_SUFFIX,
            familySuffix: PROJECT_FAMILY_PACKAGE_SUFFIX,
            canvasSuffix: CANVAS_PROJECT_PACKAGE_SUFFIX,
          }),
        );
      }
      setImportDialogOpenState(false);
      if (!mountedRef.current || importSeqRef.current !== requestSeq) return;
      if (importSuccessMessage) addToast(importSuccessMessage, "success");
      for (const warning of importWarnings) addToast(warning, "info");
      void refreshAfterMutation().then((refreshed) => {
        if (mountedRef.current && !refreshed) {
          addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
        }
      });
      for (const project of importedTextProjects) {
        await syncCreatedProjectMaterials(project);
      }
    } catch (err) {
      if (mountedRef.current && importSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.importFailed")), "error");
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
        `${PROJECT_BUNDLE_PACKAGE_SUFFIX},${PROJECT_FAMILY_PACKAGE_SUFFIX},${CANVAS_PROJECT_PACKAGE_SUFFIX},application/zip`,
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
        addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.filePickerFailed")), "error");
      }
    }
  };

  const handleTabChange = (nextTab: ProjectTab) => {
    if (activeTabRef.current === nextTab) return;
    activeTabRef.current = nextTab;
    refreshSeqRef.current += 1;
    loadingRef.current = true;
    loadErrorRef.current = null;
    projectsRef.current = [];
    setProjectListSnapshot([]);
    setLoadError(null);
    setLoading(true);
    setTab(nextTab);
  };

  const handleAccessScopeChange = (nextScope: ProjectAccessScope) => {
    if (activeAccessScopeRef.current === nextScope) return;
    activeAccessScopeRef.current = nextScope;
    refreshSeqRef.current += 1;
    loadingRef.current = true;
    loadErrorRef.current = null;
    projectsRef.current = [];
    setProjectListSnapshot([]);
    setLoadError(null);
    setLoading(true);
    setAccessScope(nextScope);
  };

  const handleProjectKindFilterChange = (nextKind: ProjectKindFilter) => {
    if (activeProjectKindFilterRef.current === nextKind) return;
    activeProjectKindFilterRef.current = nextKind;
    refreshSeqRef.current += 1;
    loadingRef.current = true;
    loadErrorRef.current = null;
    projectsRef.current = [];
    setProjectListSnapshot([]);
    setLoadError(null);
    setLoading(true);
    setProjectKindFilter(nextKind);
  };

  const handleAcceptInvite = async (invite: ProjectInvite) => {
    if (pendingInvitesLoading) {
      addToast(tRef.current("common.operationInProgress"), "info");
      return;
    }
    if (!beginInviteAction(invite.id)) return;
    const requestInviteSeq = inviteSeqRef.current;
    if (!ensureInviteReadCurrent(invite.id, requestInviteSeq)) {
      finishInviteAction(invite.id);
      return;
    }
    try {
      await acceptProjectInvite(invite.id);
      if (!mountedRef.current) return;
      if (inviteSeqRef.current !== requestInviteSeq) {
        pendingInvitesRef.current = pendingInvitesRef.current.filter((item) => item.id !== invite.id);
        setPendingInvites(pendingInvitesRef.current);
        addToast(tRef.current("novel.projectListActions.inviteAccepted"), "success");
        void loadPendingInvites();
        return;
      }
      setPendingInvites((current) => current.filter((item) => item.id !== invite.id));
      if (!mountedRef.current) return;
      addToast(tRef.current("novel.projectListActions.inviteAccepted"), "success");
      void loadPendingInvites();
      void refreshAfterMutation().then((refreshed) => {
        if (mountedRef.current && !refreshed) {
          addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
        }
      });
    } catch (err) {
      if (mountedRef.current) {
        if (inviteSeqRef.current !== requestInviteSeq) {
          addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.acceptInviteFailed")), "error");
          void loadPendingInvites();
          return;
        }
        addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.acceptInviteFailed")), "error");
        await loadPendingInvites();
        const refreshed = await refreshAfterMutation();
        if (mountedRef.current && !refreshed) {
          addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
        }
      }
    } finally {
      finishInviteAction(invite.id);
    }
  };

  const handleDeclineInvite = async (invite: ProjectInvite) => {
    if (pendingInvitesLoading) {
      addToast(tRef.current("common.operationInProgress"), "info");
      return;
    }
    if (!beginInviteAction(invite.id)) return;
    const requestInviteSeq = inviteSeqRef.current;
    if (!ensureInviteReadCurrent(invite.id, requestInviteSeq)) {
      finishInviteAction(invite.id);
      return;
    }
    try {
      await declineProjectInvite(invite.id);
      if (!mountedRef.current) return;
      if (inviteSeqRef.current !== requestInviteSeq) {
        pendingInvitesRef.current = pendingInvitesRef.current.filter((item) => item.id !== invite.id);
        setPendingInvites(pendingInvitesRef.current);
        addToast(tRef.current("novel.projectListActions.inviteDeclined"), "success");
        void loadPendingInvites();
        return;
      }
      setPendingInvites((current) => current.filter((item) => item.id !== invite.id));
      if (!mountedRef.current) return;
      addToast(tRef.current("novel.projectListActions.inviteDeclined"), "success");
      void loadPendingInvites();
    } catch (err) {
      if (mountedRef.current) {
        if (inviteSeqRef.current !== requestInviteSeq) {
          addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.declineInviteFailed")), "error");
          void loadPendingInvites();
          return;
        }
        addToast(extractErrorMessage(err, tRef.current("novel.projectListActions.declineInviteFailed")), "error");
        await loadPendingInvites();
        const refreshed = await refreshAfterMutation();
        if (mountedRef.current && !refreshed) {
          addToast(tRef.current("novel.projectListActions.refreshFailed"), "error");
        }
      }
    } finally {
      finishInviteAction(invite.id);
    }
  };

  const handleOpenProject = (project: NovelProject) => {
    if (!isProjectReadCurrent(project.id, refreshSeqRef.current)) {
      addToast(tRef.current("common.listChangedRetry"), "info");
      return;
    }
    if (project.lifecycle_gate !== "active" && project.project_kind === "text") {
      navigate(getProjectPrimaryRoute(project));
      return;
    }
    navigate(getProjectHomeEntryRoute(project));
  };

  const handleOpenWorkspace = (project: NovelProject) => {
    if (!isProjectReadCurrent(project.id, refreshSeqRef.current)) {
      addToast(tRef.current("common.listChangedRetry"), "info");
      return;
    }
    navigate(getProjectWorkspaceRoute(project));
  };

  const handleImportText = (project: NovelProject) => {
    if (!ensureProjectReadCurrent(project.id, refreshSeqRef.current)) return;
    importTextProjectReadSeqRef.current = refreshSeqRef.current;
    setImportTextProjectId(project.id);
  };

  const isImportTextProjectCurrent = () =>
    importTextProjectId != null &&
    importTextProjectReadSeqRef.current != null &&
    isProjectReadCurrent(importTextProjectId, importTextProjectReadSeqRef.current);

  const projectActions = (project: NovelProject) =>
    createProjectActions(project, {
      onOpenDetails: (item) => navigate(getProjectPrimaryRoute(item)),
      onOpenInspiration: (item) => navigate(`/inspiration/${item.id}`),
      onDuplicate: (item) => void handleDuplicate(item),
      onCreateDerivedProject: (item, kind) => void handleCreateDerivedProject(item, kind),
      onExport: (item) => void handleExport(item),
      onImportText: handleImportText,
      onDelete: (item) => void handleDelete(item),
    }, t, isPackageVisible);

  return (
    <div className="a4n-home-page relative h-full w-full overflow-y-auto overflow-x-hidden bg-book-bg md:overflow-hidden">
      <UnsavedNavigationGuard dirty={importDialogOpen && importFile != null} name={t("novel.importProject")} action={t("common.leavePage")} />
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
          loadError={loadError == null ? null : extractErrorMessage(loadError, t("novel.projectLoadFailed"))}
          empty={!loading && !loadError && projects.length === 0}
          recentProjectFamilies={recentProjectFamilies}
          groupedProjectFamilies={groupedProjectFamilies}
          accessScope={accessScope}
          projectKindFilter={projectKindFilter}
          pendingInvites={pendingInvites}
          pendingInvitesLoading={pendingInvitesLoading}
          pendingInvitesError={pendingInvitesError == null ? null : extractErrorMessage(pendingInvitesError, t("novel.projectLoadFailed"))}
          inviteActionIds={inviteActionIds}
          onTabChange={handleTabChange}
          onCreateProject={() => {
            setCreateDialogOpen(true);
          }}
          onImportProject={() => {
            setImportDialogOpenState(true);
          }}
          onOpenSettings={() => navigate("/settings")}
          onRetry={() => void refresh()}
          onRetryInvites={retryPendingInvites}
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
        {videoProjectSourceDialogOpen ? (
          <VideoProjectSourceDialog
            open
            projects={videoProjectSourceProjects}
            selectedProjectId={selectedVideoProjectSourceId}
            loading={videoProjectSourceLoading}
            error={videoProjectSourceError}
            onClose={closeVideoProjectSourceDialog}
            onBack={handleBackToProjectCreation}
            onRetry={() => void loadVideoProjectSources()}
            onSelectProject={setSelectedVideoProjectSourceId}
            onContinue={handleCreateVideoProjectFromSelectedSource}
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
            isProjectReadCurrent={isImportTextProjectCurrent}
            onClose={() => {
              importTextProjectReadSeqRef.current = null;
              setImportTextProjectId(null);
            }}
          />
        ) : null}
      </Suspense>
    </div>
  );
};
