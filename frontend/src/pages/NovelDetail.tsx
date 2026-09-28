import React, { lazy, Suspense, useEffect, useRef, useState } from "react";
import { useBeforeUnload, useBlocker, useNavigate, useParams } from "react-router-dom";
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
import {
  canEditProjectMaterials,
  getProject,
  updateProject,
  type NovelProject,
} from "../api/project";
import {
  refineBlueprint,
} from "../api/blueprint";
import { formatRagIngestResultSummary, ragIngestAll } from "../api/rag";
import { OverviewTab } from "./novel-detail/OverviewTab";
import { useStatusBarPermanent } from "../components/ui/StatusBar";
import {
  getProjectLifecycleBlockedReason,
  getProjectProgressText,
} from "../utils/projectStatus";
import { usePersistedTab } from "../hooks/usePersistedTab";
import { BackButton, useSmartBack } from "../components/ui/BackButton";
import { Modal } from "../components/ui/Modal";
import { BookButton } from "../components/ui/BookButton";
import { PageHeader } from "../components/shell/PageHeader";
import { cancelWorkflowTask } from "../api/streaming";
import { createWorkflowTaskId, useProgressTask } from "../hooks/useProgressTask";
import { isCancelledOperation } from "../utils/cancellation";
import { nonEmptyText } from "../utils/text";
import { getScrollRailCue, scrollRailItemIntoView } from "../utils/scrollRail";
import { formatNumber, useI18n, type Locale, type MessageKey, type MessageTranslator } from "../i18n";
import { usePackagePreferences } from "../store/packagePreferences";
import { runBlueprintRefinementWithProgress } from "./novel-detail/blueprintRefinement";
import {
  NovelDetailDirtyProvider,
  useNovelDetailDirtyRegistry,
} from "./novel-detail/novelDetailDirty";

const AssetDependencyGraphDialog = lazy(() =>
  import("./novel-detail/asset-graph/AssetDependencyGraphDialog").then(
    (module) => ({ default: module.AssetDependencyGraphDialog }),
  ),
);
const WorldTab = lazy(() =>
  import("./novel-detail/WorldTab").then((module) => ({ default: module.WorldTab })),
);
const CharactersTab = lazy(() =>
  import("./novel-detail/CharactersTab").then((module) => ({
    default: module.CharactersTab,
  })),
);
const ObjectProfilesTab = lazy(() =>
  import("./novel-detail/ObjectProfilesTab").then((module) => ({
    default: module.ObjectProfilesTab,
  })),
);
const SceneProfilesTab = lazy(() =>
  import("./novel-detail/SceneProfilesTab").then((module) => ({
    default: module.SceneProfilesTab,
  })),
);
const RelationshipsTab = lazy(() =>
  import("./novel-detail/RelationshipsTab").then((module) => ({
    default: module.RelationshipsTab,
  })),
);
const StoryEventsTab = lazy(() =>
  import("./novel-detail/StoryEventsTab").then((module) => ({
    default: module.StoryEventsTab,
  })),
);
const OutlinesTab = lazy(() =>
  import("./novel-detail/OutlinesTab").then((module) => ({
    default: module.OutlinesTab,
  })),
);
const ImagesTab = lazy(() =>
  import("./novel-detail/ImagesTab").then((module) => ({ default: module.ImagesTab })),
);
const ImportsTab = lazy(() =>
  import("./novel-detail/ImportsTab").then((module) => ({ default: module.ImportsTab })),
);
const ChaptersTab = lazy(() =>
  import("./novel-detail/ChaptersTab").then((module) => ({
    default: module.ChaptersTab,
  })),
);
const ModelUsageCostPanel = lazy(() =>
  import("./novel-detail/ModelUsageCostPanel").then((module) => ({
    default: module.ModelUsageCostPanel,
  })),
);
const StatusMachineDialog = lazy(() =>
  import("./StatusMachineDialog").then((module) => ({
    default: module.StatusMachineDialog,
  })),
);
const ProjectCollaborationPanel = lazy(() =>
  import("./novel-detail/overview/ProjectCollaborationPanel").then((module) => ({
    default: module.ProjectCollaborationPanel,
  })),
);
const ExportPanel = lazy(() =>
  import("./novel-detail/ExportPanel").then((module) => ({
    default: module.ExportPanel,
  })),
);
import {
  getProjectHomeEntryLabel,
  getProjectHomeEntryRoute,
  getProjectPrimaryRoute,
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
  | "chapters"
  | "cost";

// 详情页固定 Tab 顺序
const TABS: { id: Tab; labelKey: MessageKey; accessibleLabelKey: MessageKey }[] = [
  { id: "overview", labelKey: "novel.overviewTab", accessibleLabelKey: "novel.overviewTabA11y" },
  { id: "world", labelKey: "novel.worldTab", accessibleLabelKey: "novel.worldTab" },
  { id: "characters", labelKey: "novel.charactersTab", accessibleLabelKey: "novel.charactersTabA11y" },
  { id: "object_profiles", labelKey: "novel.objectProfilesTab", accessibleLabelKey: "novel.objectProfilesTabA11y" },
  { id: "scene_profiles", labelKey: "novel.sceneProfilesTab", accessibleLabelKey: "novel.sceneProfilesTabA11y" },
  { id: "relationships", labelKey: "novel.relationshipsTab", accessibleLabelKey: "novel.relationshipsTabA11y" },
  { id: "story_events", labelKey: "novel.storyEventsTab", accessibleLabelKey: "novel.storyEventsTabA11y" },
  { id: "chapter_outline", labelKey: "novel.outlineTab", accessibleLabelKey: "novel.outlineTabA11y" },
  { id: "images", labelKey: "novel.imagesTab", accessibleLabelKey: "novel.imagesTabA11y" },
  { id: "imports", labelKey: "novel.importsTab", accessibleLabelKey: "novel.importsTabA11y" },
  { id: "chapters", labelKey: "novel.chaptersTab", accessibleLabelKey: "novel.chaptersTabA11y" },
  { id: "cost", labelKey: "novel.costTab", accessibleLabelKey: "novel.costTabA11y" },
];
const TAB_IDS = TABS.map((t) => t.id) as readonly Tab[];

export const NovelDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const projectId = Number(id);
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { runProgressTask } = useProgressTask();
  const { locale, t } = useI18n();
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  const smartBack = useSmartBack();
  const { isVisible: isPackageVisible } = usePackagePreferences();
  const tabs = TABS.filter(item => item.id !== "imports" || isPackageVisible("external-import")).map((item) => ({
    id: item.id,
    label: t(item.labelKey),
    accessibleLabel: t(item.accessibleLabelKey),
  }));

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
  const dirtyNavigationBlocker = useBlocker(() => dirtyRegistry.hasDirty() || exportBusy);
  useBeforeUnload((event) => {
    if (!dirtyRegistry.hasDirty() && !exportBusy) return;
    event.preventDefault();
    event.returnValue = "";
  });
  useEffect(() => {
    if (
      dirtyNavigationBlocker.state === "blocked" &&
      !exportBusy &&
      !dirtyRegistry.hasDirty()
    ) {
      dirtyNavigationBlocker.proceed();
    }
  }, [dirtyNavigationBlocker.proceed, dirtyNavigationBlocker.state, dirtyRegistry, exportBusy]);
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
  const tabListRef = useRef<HTMLElement | null>(null);
  const [tabRailHasBefore, setTabRailHasBefore] = useState(false);
  const [tabRailHasMore, setTabRailHasMore] = useState(false);
  const lifecycleBlockedReason = project
    ? getProjectLifecycleBlockedReason(project, t)
    : null;
  const canEditCurrentProject = project
    ? canEditProjectMaterials(project)
    : false;

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

  const refresh = async (requestProjectId = projectId): Promise<boolean> => {
    const requestSeq = refreshSeqRef.current + 1;
    refreshSeqRef.current = requestSeq;
    if (!Number.isFinite(requestProjectId) || requestProjectId <= 0) {
      addToast(t("novel.projectIdInvalid"), "error");
      navigate("/");
      return false;
    }
    setLoading(true);
    try {
      const p = await getProject(String(requestProjectId));
      if (!isCurrentAction(requestProjectId, refreshSeqRef, requestSeq)) return false;
      if (p.project_kind !== "text") {
        navigate(getProjectHomeEntryRoute(p), { replace: true });
        return false;
      }
      setProject(p);
      return true;
    } catch (err) {
      if (isCurrentAction(requestProjectId, refreshSeqRef, requestSeq)) {
        addToast(extractErrorMessage(err, t("novel.loadProject")), "error");
        if (project?.id !== requestProjectId) navigate("/");
      }
      return false;
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
    if (project?.lifecycle_gate !== "resetting") return;
    let checking = false;
    let errorReported = false;
    const timer = window.setInterval(() => {
      if (checking) return;
      checking = true;
      void getProject(projectId)
        .then((nextProject) => {
          if (!isCurrentProject(projectId) || nextProject.lifecycle_gate !== "active") return;
          window.clearInterval(timer);
          setProject(nextProject);
          addToast(t("novel.resetFinishingComplete"), "success");
        })
        .catch((error) => {
          if (errorReported || !isCurrentProject(projectId)) return;
          errorReported = true;
          addToast(extractErrorMessage(error, t("novel.resetProgressFailed")), "error");
        })
        .finally(() => {
          checking = false;
        });
    }, 1500);
    return () => window.clearInterval(timer);
    // isCurrentProject 只读取 ref，不随 render 改变。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [addToast, project?.lifecycle_gate, projectId]);

  useEffect(() => {
    if (!project || canEditCurrentProject) return;
    if (tab !== "characters" && tab !== "relationships" && tab !== "imports") return;
    setTab("overview");
  }, [canEditCurrentProject, project, setTab, tab]);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      scrollRailItemIntoView(tabListRef.current, activeTabRef.current, "smooth");
    });
    return () => window.cancelAnimationFrame(frame);
  }, [canEditCurrentProject, locale, project?.id, tab]);

  useEffect(() => {
    const rail = tabListRef.current;
    if (!rail) return;
    const updateCue = () => {
      const cue = getScrollRailCue(rail);
      setTabRailHasBefore(cue.hasBefore);
      setTabRailHasMore(cue.hasMore);
    };
    const frame = window.requestAnimationFrame(updateCue);
    rail.addEventListener("scroll", updateCue, { passive: true });
    window.addEventListener("resize", updateCue);
    const resizeObserver =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(updateCue);
    resizeObserver?.observe(rail);
    document.fonts?.addEventListener("loadingdone", updateCue);
    return () => {
      window.cancelAnimationFrame(frame);
      rail.removeEventListener("scroll", updateCue);
      window.removeEventListener("resize", updateCue);
      resizeObserver?.disconnect();
      document.fonts?.removeEventListener("loadingdone", updateCue);
    };
  }, [canEditCurrentProject, locale, project?.id]);

  // 状态栏右侧：当前项目 + 当前 Tab
  useStatusBarPermanent(
    "novel-detail",
    `${project?.title || t("novel.projects")} · ${tabs.find((item) => item.id === tab)?.label || tab}`,
  );

  if (loading && project?.id !== projectId) {
    return (
      <div className="h-full flex items-center justify-center text-book-text-muted">
        <LoadingSpinner size="lg" text={t("novel.loadingProjects")} />
      </div>
    );
  }
  if (!project || project.id !== projectId) return null;

  const homeEntryRoute = getProjectHomeEntryRoute(project);
  const primaryRoute = getProjectPrimaryRoute(project);
  const homeEntryLabel = getProjectHomeEntryLabel(project, t);
  const canEditProject = canEditProjectMaterials(project);
  const canOwnProject = project.my_role === "owner" && !lifecycleBlockedReason;
  const visibleTabs = canEditProject
    ? tabs
    : tabs.filter(
        (item) => item.id !== "characters" && item.id !== "relationships" && item.id !== "imports",
      );
  const confirmDirtyNavigation = async (action: string): Promise<boolean> => {
    if (!dirtyRegistry.hasDirty()) return true;
    return confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", {
        name: t("novel.detailTabs"),
        action,
      }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    });
  };
  const handleTabChange = async (nextTab: Tab) => {
    if (nextTab === tab) return;
    if (!(await confirmDirtyNavigation(t("common.switchTab")))) return;
    setTab(nextTab);
  };
  const workspaceRoute = getProjectWorkspaceRoute(project);
  const workspaceLabel = getProjectWorkspaceLabel(project, t);
  const showHomeEntryButton =
    canEditProject &&
    homeEntryRoute !== primaryRoute &&
    homeEntryRoute !== workspaceRoute;
  const statusMachineDescription = canOwnProject
    ? t("novel.statusHistoryOwnerDescription")
    : canEditProject
      ? t("novel.statusHistoryEditorDescription")
      : t("novel.statusHistoryReadonlyDescription");

  const handleRagSync = async () => {
    if (!canEditProject) {
      addToast(t("novel.editPermissionDenied"), "error");
      return;
    }
    if (ragActionBusyRef.current) return;
    ragActionBusyRef.current = true;
    const requestProjectId = projectId;
    let requestSeq: number | null = null;
    try {
      const ok = await confirmDialog({
        title: t("novel.syncMaterials"),
        message: t("novel.syncMaterialsMessage"),
        confirmText: t("novel.startSync"),
        dialogType: "warning",
      });
      if (!ok || !isCurrentProject(requestProjectId)) return;
      requestSeq = ragSeqRef.current + 1;
      ragSeqRef.current = requestSeq;
      const activeRequestSeq = requestSeq;
      const taskId = createWorkflowTaskId("rag-ingest-all", requestProjectId);
      setActionsOpen(false);
      setRagSyncing(true);
      const result = await runProgressTask(
        {
          cancelTaskId: taskId,
          cancelTask: cancelWorkflowTask,
          pollTaskProgress: true,
          title: t("novel.syncMaterials"),
          startMessage: t("novel.countingSync"),
          successMessage: t("novel.syncCompleted"),
          errorMessage: t("novel.syncFailed"),
          total: 1,
          cancelLabel: t("novel.stopSync"),
          runningHint: t("novel.keepWindowOpenSync"),
          formatProgress: (current, total, percent) =>
            formatRagProgress(current, total, percent, locale, t),
        },
        async ({ setProgress }) => {
          if (!isCurrentAction(requestProjectId, ragSeqRef, activeRequestSeq)) {
            throw new Error(t("workflow.cancelled"));
          }
          setProgress(t("novel.countingSync"), 0, 1);
          const res = await ragIngestAll(requestProjectId, taskId);
          setProgress(t("novel.syncResult", { summary: formatRagIngestResultSummary(res, locale, t) }), 1, 1);
          return res;
        },
      );
      if (!isCurrentAction(requestProjectId, ragSeqRef, activeRequestSeq)) return;
      addToast(
        t("novel.syncResult", { summary: formatRagIngestResultSummary(result, locale, t) }),
        "success",
      );
    } catch (err) {
      if (requestSeq == null) {
        if (isCurrentProject(requestProjectId)) {
          addToast(extractErrorMessage(err, t("novel.syncFailed")), "error");
        }
        return;
      }
      if (!isCurrentAction(requestProjectId, ragSeqRef, requestSeq)) return;
      const message = extractErrorMessage(err, t("novel.syncFailed"));
      if (isCancelledOperation(message)) {
        addToast(t("novel.syncStopped"), "info");
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
      addToast(t("novel.editPermissionDenied"), "error");
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
        title: t("novel.refineBlueprint"),
        message: t("novel.refineBlueprintMessage"),
        placeholder: t("novel.refineBlueprintPlaceholder"),
        multiline: true,
        confirmText: t("novel.startRefine"),
      });
      if (focus === null || !isCurrentProject(requestProjectId)) return;
      const activeRequestSeq = refineSeqRef.current + 1;
      requestSeq = activeRequestSeq;
      refineSeqRef.current = activeRequestSeq;
      setRefining(true);
      const handle = await refineBlueprint({
        project_id: requestProjectId,
        focus: nonEmptyText(focus),
        variables: null,
      });
      if (!isCurrentAction(requestProjectId, refineSeqRef, requestSeq)) return;
      const syncWarnings = await runBlueprintRefinementWithProgress({
        runProgressTask,
        handle,
        projectId: requestProjectId,
        title: t("novel.refineBlueprint"),
        startMessage: t("novel.refiningBlueprint"),
        successMessage: t("novel.blueprintRefined"),
        errorMessage: t("novel.refineBlueprintFailed"),
        isCurrent: () => isCurrentAction(requestProjectId, refineSeqRef, activeRequestSeq),
        refresh: () => refresh(requestProjectId),
        t,
      });
      addToast(
        syncWarnings.length > 0
          ? t("novel.blueprintRefinedWithWarnings", { warnings: syncWarnings.join(t("common.summarySeparator")) })
          : t("novel.blueprintRefined"),
        syncWarnings.length > 0 ? "info" : "success",
      );
    } catch (err) {
      const message = extractErrorMessage(err, t("novel.refineBlueprintFailed"));
      const cancelled = isCancelledOperation(message);
      if (requestSeq == null) {
        if (isCurrentProject(requestProjectId)) {
          addToast(
            cancelled ? t("feedback.progressCancelled") : message,
            cancelled ? "info" : "error",
          );
        }
      } else if (isCurrentAction(requestProjectId, refineSeqRef, requestSeq)) {
        addToast(
          cancelled ? t("feedback.progressCancelled") : message,
          cancelled ? "info" : "error",
        );
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
      addToast(t("novel.editPermissionDenied"), "error");
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
        title: t("novel.editProjectTitle"),
        message: t("novel.projectTitleMessage"),
        defaultValue: requestProject.title,
        placeholder: t("novel.projectTitlePlaceholder"),
        required: true,
        confirmText: t("novel.saveTitle"),
      });
      if (next != null && next.trim() && next.trim() !== requestProject.title) {
        if (!isCurrentProject(requestProjectId)) return;
        const requestSeq = titleSeqRef.current + 1;
        titleSeqRef.current = requestSeq;
        setSaving(true);
        try {
          await updateProject(requestProjectId, { title: next.trim() });
          if (!isCurrentAction(requestProjectId, titleSeqRef, requestSeq)) return;
          if (!await refresh(requestProjectId)) return;
          addToast(t("novel.projectTitleUpdated"), "success");
        } catch (err) {
          if (isCurrentAction(requestProjectId, titleSeqRef, requestSeq)) {
            addToast(extractErrorMessage(err, t("novel.updateTitleFailed")), "error");
          }
        } finally {
          if (isCurrentAction(requestProjectId, titleSeqRef, requestSeq)) {
            setSaving(false);
          }
        }
      }
    } catch (err) {
      addToast(extractErrorMessage(err, t("novel.updateTitleFailed")), "error");
    } finally {
      titleActionBusyRef.current = false;
    }
  };

  const handleOpenExport = () => {
    if (!canEditProject) {
      addToast(t("novel.exportPermissionDenied"), "error");
      return;
    }
    setActionsOpen(false);
    setExportBusy(false);
    setExportOpen(true);
  };
  const closeCollaboration = async () => {
    if (dirtyRegistry.hasDirty(`collaboration:${project.id}`) && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("novel.collaborationTitle"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    setCollaborationOpen(false);
  };
  const closeExport = async () => {
    if (exportBusy) return;
    if (dirtyRegistry.hasDirty(`export:${project.id}`) && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("novel.exportNovel"), action: t("common.close") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    setExportOpen(false);
  };

  return (
    <NovelDetailDirtyProvider>
      <div className="h-full w-full flex flex-col bg-book-bg">
      {dirtyNavigationBlocker.state === "blocked" ? (
        <Modal
          isOpen
          onClose={dirtyNavigationBlocker.reset}
          title={exportBusy ? t("novel.exportPanel.navigationBlockedTitle") : t("common.unsavedChangesTitle")}
          tone="warning"
          maxWidthClassName="max-w-xl"
          footer={(
            <>
              <BookButton variant="secondary" onClick={dirtyNavigationBlocker.reset}>{t("common.cancel")}</BookButton>
              {!exportBusy ? <BookButton variant="warning" onClick={dirtyNavigationBlocker.proceed}>{t("common.discardDraft")}</BookButton> : null}
            </>
          )}
        >
          {exportBusy
            ? t("novel.exportPanel.navigationBlocked")
            : t("common.unsavedChangesMessage", { name: t("novel.detailTabs"), action: t("common.back") })}
        </Modal>
      ) : null}
      {/* 项目身份、当前进度和高频入口；维护操作统一收进更多菜单。 */}
      <PageHeader
        className="a4n-app-page-header--detail h-auto shrink-0 py-2 sm:flex-nowrap sm:py-0"
        leading={<BackButton iconOnly title={t("novel.backToPreviousPage")} onClick={smartBack} />}
        title={project.title}
        subtitle={getProjectProgressText(project, t)}
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
            {canEditProject && (
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => navigate(workspaceRoute)}
                title={workspaceLabel}
                aria-label={workspaceLabel}
                className="!min-h-8 !px-3 !text-xs"
              >
                <span className="sm:hidden">{project.project_kind === "text" ? t("novel.writeShortcut") : workspaceLabel}</span>
                <span className="hidden sm:inline">{workspaceLabel}</span>
              </BookButton>
            )}
            <BookButton
              variant="secondary"
              size="sm"
              onClick={() => setActionsOpen(true)}
              title={t("novel.moreProjectActions")}
              className="!min-h-8 !w-11 !px-0 sm:!w-8"
              aria-label={t("novel.more")}
            >
              <MoreHorizontal size={14} />
            </BookButton>
          </>
        }
      />

      {lifecycleBlockedReason ? (
        <div
          role="status"
          className="shrink-0 border-b border-warning/30 bg-warning-bg px-[var(--a4n-content-gutter)] py-2 text-small text-book-text-main"
        >
          {lifecycleBlockedReason}
        </div>
      ) : null}

      <div
        className="a4n-tab-rail-wrap shrink-0"
        data-has-before={tabRailHasBefore ? "true" : undefined}
        data-has-more={tabRailHasMore ? "true" : undefined}
      >
        <nav
          ref={tabListRef}
          className="tab-bar a4n-mobile-scroll-rail scroll-px-[var(--a4n-content-gutter)] border-b border-book-border/80 bg-book-bg-paper/90 px-[var(--a4n-content-gutter)]"
          role="tablist"
          aria-label={t("novel.detailTabs")}
          aria-describedby="novel-tab-scroll-hint"
        >
          {visibleTabs.map((t) => (
            <button
              ref={tab === t.id ? activeTabRef : undefined}
              key={t.id}
              type="button"
              role="tab"
              aria-selected={tab === t.id}
              onClick={() => void handleTabChange(t.id)}
              onFocus={(event) =>
                scrollRailItemIntoView(
                  tabListRef.current,
                  event.currentTarget,
                  "instant",
                )
              }
              data-active={tab === t.id ? "1" : undefined}
              className="pyqt-tab"
              aria-label={t.accessibleLabel}
              title={t.accessibleLabel}
            >
              {t.label}
            </button>
          ))}
        </nav>
        <span id="novel-tab-scroll-hint" className="sr-only">
          {t("novel.scrollTabsHint")}
        </span>
      </div>

      <div
        key={project.lifecycle_gate}
        className="flex-1 min-h-0 overflow-y-auto custom-scrollbar a4n-page-container"
      >
        <Suspense
          fallback={(
            <div className="flex min-h-48 items-center justify-center">
              <LoadingSpinner size="md" text={t("novel.loadingTabs")} />
            </div>
          )}
        >
          {tab === "overview" && (
            <OverviewTab
              project={project}
              onRefresh={refresh}
              onOpenImports={() => void handleTabChange("imports")}
            />
          )}
          {tab === "world" && (
            <WorldTab
              projectId={projectId}
              canEditWorld={canEditProject}
              onCreateBlueprint={() => void handleTabChange("overview")}
            />
          )}
          {tab === "characters" && canEditProject && <CharactersTab projectId={projectId} />}
          {tab === "object_profiles" && (
            <ObjectProfilesTab projectId={projectId} canEditProfiles={canEditProject} />
          )}
          {tab === "scene_profiles" && (
            <SceneProfilesTab projectId={projectId} canEditProfiles={canEditProject} />
          )}
          {tab === "relationships" && canEditProject && (
            <RelationshipsTab projectId={projectId} />
          )}
          {tab === "story_events" && (
            <StoryEventsTab projectId={projectId} canOpenWritingDesk={canEditProject} />
          )}
          {tab === "chapter_outline" && <OutlinesTab project={project} />}
          {tab === "images" && (
            <ImagesTab projectId={projectId} canEditImages={canEditProject} />
          )}
          {tab === "imports" && canEditProject && <ImportsTab projectId={projectId} />}
          {tab === "chapters" && (
            <ChaptersTab projectId={projectId} canEditChapters={canEditProject} />
          )}
          {tab === "cost" && <ModelUsageCostPanel projectId={projectId} />}
        </Suspense>
      </div>

      {/* 项目状态机对话框 */}
      {stateMachineOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <StatusMachineDialog
            open
            onClose={() => setStateMachineOpen(false)}
            project={project}
            canForwardStatus={canEditProject}
            canRollbackStatus={canOwnProject}
            onMutated={refresh}
          />
        </Suspense>
      ) : null}

      <Modal
        isOpen={actionsOpen}
        onClose={() => setActionsOpen(false)}
        title={t("novel.projectActions")}
        subtitle={t("novel.projectActionsSubtitle")}
        icon={<MoreHorizontal size={15} />}
        maxWidthClassName="max-w-2xl"
        bodyClassName="px-4 py-4"
      >
        <div className="space-y-4">
          <ProjectActionGroup title={t("novel.projectManagement")}>
            {canEditProject ? (
              <ProjectActionButton
                icon={saving ? <LoadingSpinner size="sm" /> : <Edit2 size={14} />}
                title={t("novel.renameProject")}
                description={t("novel.renameProjectDescription")}
                disabled={saving}
                onClick={() => {
                  setActionsOpen(false);
                  void handleEditTitle();
                }}
              />
            ) : null}
            <ProjectActionButton
              icon={<Users size={14} />}
              title={t("novel.collaboration")}
              description={t("novel.collaborationDescription")}
              onClick={() => {
                setActionsOpen(false);
                setCollaborationOpen(true);
              }}
            />
            <ProjectActionButton
              icon={<GitBranch size={14} />}
              title={t("novel.projectStage")}
              description={statusMachineDescription}
              onClick={() => {
                setActionsOpen(false);
                setStateMachineOpen(true);
              }}
            />
          </ProjectActionGroup>

          {canEditProject ? (
            <ProjectActionGroup title={t("novel.contentMaintenance")}>
              {project.status === "draft" ? (
                <ProjectActionButton
                  icon={<Sparkles size={14} />}
                  title={t("novel.inspiration")}
                  description={t("novel.inspirationDescription")}
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
                title={t("novel.syncMaterials")}
                description={t("novel.syncMaterialsDescription")}
                disabled={ragSyncing}
                onClick={handleRagSync}
              />
              <ProjectActionButton
                icon={
                  refining ? <LoadingSpinner size="sm" /> : <Wand2 size={14} />
                }
                title={t("novel.refineBlueprint")}
                description={t("novel.refineBlueprintDescription")}
                disabled={refining}
                onClick={handleRefineBlueprint}
              />
            </ProjectActionGroup>
          ) : null}

          <ProjectActionGroup title={t("novel.outputAndView")}>
            {canEditProject ? (
              <ProjectActionButton
                icon={<FileText size={14} />}
                title={t("novel.exportNovel")}
                description={t("novel.exportNovelDescription")}
                onClick={handleOpenExport}
              />
            ) : null}
            <ProjectActionButton
              icon={<Network size={14} />}
              title={t("novel.contentRelations")}
              description={t("novel.contentRelationsDescription")}
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
        onClose={() => void closeCollaboration()}
        title={t("novel.collaborationTitle")}
        subtitle={t("novel.collaborationSubtitle")}
        icon={<Users size={15} />}
        maxWidthClassName="max-w-3xl"
        bodyClassName="px-4 py-4"
      >
        {collaborationOpen ? (
          <Suspense fallback={<LoadingSpinner size="md" text={t("novel.loadingCollaboration")} />}>
            <ProjectCollaborationPanel project={project} onRefresh={refresh} />
          </Suspense>
        ) : null}
      </Modal>

      <Modal
        isOpen={exportOpen}
        onClose={() => void closeExport()}
        title={t("novel.exportNovelTitle")}
        subtitle={t("novel.exportNovelSubtitle")}
        icon={<FileText size={15} />}
        maxWidthClassName="max-w-4xl"
        bodyClassName="px-4 py-4"
        closeOnBackdrop={!exportBusy}
        closeOnEscape={!exportBusy}
        showCloseButton={!exportBusy}
      >
        {exportOpen ? (
          <Suspense fallback={<LoadingSpinner size="md" text={t("novel.loadingExportOptions")} />}>
            <ExportPanel project={project} onBusyChange={setExportBusy} />
          </Suspense>
        ) : null}
      </Modal>

      {assetGraphOpen ? (
        <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
          <AssetDependencyGraphDialog
            open
            projectId={projectId}
            onClose={() => setAssetGraphOpen(false)}
          />
        </Suspense>
      ) : null}
      </div>
    </NovelDetailDirtyProvider>
  );
};

function formatRagProgress(
  current: number,
  total: number,
  percent: number,
  locale: Locale,
  t: MessageTranslator,
): React.ReactNode {
  return t("novel.ragProgress", {
    current: formatNumber(current, locale),
    total: formatNumber(total, locale),
    percent: formatNumber(percent, locale),
  });
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
