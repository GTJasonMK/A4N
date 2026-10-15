import React, {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  Background,
  MiniMap,
  ReactFlow,
  useEdgesState,
  useNodesState,
  type ReactFlowInstance,
  type Viewport,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
  Bot,
  Eye,
  Film,
  FileText,
  ListPlus,
  Play,
  RefreshCw,
  Save,
  SlidersHorizontal,
  Upload,
} from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../api";
import { isPositiveSafeInteger } from "../utils/numberRange";
import { listLlmConfig } from "../api/config/llm";
import type { ModelRequestPreview } from "../api/requestPreview";
import {
  getCanvasGraph,
  getCanvasNodeContracts,
  listCanvasTextAssets,
  type CanvasGraphDocument,
  type CanvasTextAssetSummary,
} from "../api/canvas";
import { listImageConfigs } from "../api/image";
import { canEditProjectMaterials, getProject, type NovelProject } from "../api/project";
import { useProjectLifecycleSync } from "../hooks/useProjectLifecycleSync";
import type {
  WorkflowRunDetailSummary,
  WorkflowRunSummary,
} from "../api/streaming";
import { listVideoConfigs } from "../api/video";
import type { ProgressEventRecord } from "../api/progress";
import { useToast } from "../components/feedback/Toast";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { UnsavedNavigationGuard } from "../components/navigation/UnsavedNavigationGuard";
import { BookButton, BookIconButton } from "../components/ui/BookButton";
import { FlowControls } from "../components/ui/FlowControls";
import { BackButton } from "../components/ui/BackButton";
import { Dropdown, type DropdownItem } from "../components/ui/Dropdown";
import { Modal } from "../components/ui/Modal";
import { PageHeader } from "../components/shell/PageHeader";
import { useNovelDetailDirty, useNovelDetailDirtyRegistry, useNovelDetailDirtyVersion } from "./novel-detail/novelDetailDirty";
import { DomainAgentConsole } from "../components/agent/DomainAgentConsole";
import { useContextMenu } from "../components/ui/ContextMenu";
import { CanvasEmptyState } from "./video-studio/CanvasEmptyState";
import { ProjectTextMaterialsPanel } from "./derived/ProjectTextMaterialsPanel";
import { CanvasWorkflowPanel } from "./video-studio/CanvasWorkflowPanel";
import { initializeVideoStarterGraph } from "./video-studio/canvasStarterTemplate";
import {
  STUDIO_NODE_TYPES,
  buildNodeTemplateGroups,
  edgeRoleLabel,
  nodeDisplayTitle,
  stateLabel,
} from "./video-studio/StudioCanvasNode";
import {
  EMPTY_CONFIG_OPTIONS,
  buildStudioConfigOptions,
  type StudioConfigKind,
  type StudioConfigLoadStatusByKind,
  type StudioConfigOptionsByKind,
} from "./video-studio/canvasConfig";
import { canvasGraphSaveValidationError } from "./video-studio/canvasEdgeContract";
import {
  canvasGraphViewport,
  preserveSelectedItems,
  toFlowEdge,
  toFlowNode,
} from "./video-studio/canvasGraphMapping";
import {
  CanvasSidebarResizeHandle,
  useVideoCanvasSidebarLayout,
} from "./video-studio/canvasLayout";
import { nodeHasProviderRequest } from "./video-studio/canvasNodeKinds";
import {
  canvasWorkflowNodeRunStateFacts,
  canvasWorkflowNodeRunStates,
  computeRunToNodeScope,
  computeWorkflowActiveScope,
  edgeWithRuntimeClasses,
  nodeWithRunScopeClass,
  nodeWithWorkflowRunClass,
} from "./video-studio/canvasWorkflowState";
import {
  StudioNodeContractsProvider,
  normalizeStudioNodeContracts,
  studioNodeContract,
  studioNodeContractMap,
  type StudioNodeContract,
} from "./video-studio/nodeContracts";
import type { StudioEdge, StudioNode } from "./video-studio/types";
import { formatNumber, useI18n } from "../i18n";
import { getProjectStudioBackRoute } from "../utils/projectRouting";
import { getProjectLifecycleBlockedReason } from "../utils/projectStatus";
import {
  CANVAS_WORKFLOW_KIND,
  type CanvasWorkflowSubmissionLabel,
  useCanvasWorkflowActions,
} from "./video-studio/useCanvasWorkflowActions";
import { useCanvasGraphActions } from "./video-studio/useCanvasGraphActions";

const RequestPreviewModal = lazy(() =>
  import("../components/feedback/RequestPreviewModal").then((module) => ({
    default: module.RequestPreviewModal,
  })),
);
const CanvasNodeContentPreviewDialog = lazy(() =>
  import("./video-studio/CanvasNodeContentPreviewDialog").then((module) => ({
    default: module.CanvasNodeContentPreviewDialog,
  })),
);
const NodeInspector = lazy(() =>
  import("./video-studio/CanvasNodeInspector").then((module) => ({
    default: module.NodeInspector,
  })),
);
const EdgeInspector = lazy(() =>
  import("./video-studio/CanvasNodeInspector").then((module) => ({
    default: module.EdgeInspector,
  })),
);

export const VideoStudioCanvas: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const dirtyRegistry = useNovelDetailDirtyRegistry();
  useNovelDetailDirtyVersion();
  const translatorRef = useRef(t);
  translatorRef.current = t;
  const contextMenu = useContextMenu();
  const projectId = Number(id);
  const [project, setProject] = useState<NovelProject | null>(null);
  const [document, setDocument] = useState<CanvasGraphDocument | null>(null);
  const initialViewport = canvasGraphViewport(document?.graph.viewport_json);
  const [nodeContracts, setNodeContracts] = useState<StudioNodeContract[]>([]);
  const [nodes, setNodes, applyNodesChange] = useNodesState<StudioNode>([]);
  const [edges, setEdges, applyEdgesChange] = useEdgesState<StudioEdge>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [importing, setImporting] = useState(false);
  const [latestWorkflowRunId, setLatestWorkflowRunId] = useState<number | null>(
    null,
  );
  const [workflowDetail, setWorkflowDetail] =
    useState<WorkflowRunDetailSummary | null>(null);
  const [workflowEvents, setWorkflowEvents] = useState<ProgressEventRecord[]>(
    [],
  );
  const [workflowStatusError, setWorkflowStatusError] = useState<string | null>(null);
  const [workflowEventsError, setWorkflowEventsError] = useState<string | null>(null);
  const [workflowLoading, setWorkflowLoading] = useState(false);
  const [workflowSubmissionLabel, setWorkflowSubmissionLabel] = useState<
    CanvasWorkflowSubmissionLabel | null
  >(null);
  const [workflowNowMs, setWorkflowNowMs] = useState(() => Date.now());
  const [cancellingWorkflow, setCancellingWorkflow] = useState(false);
  const [resumingWorkflow, setResumingWorkflow] = useState(false);
  const [configOptions, setConfigOptions] =
    useState<StudioConfigOptionsByKind>(EMPTY_CONFIG_OPTIONS);
  const [configOptionsStatus, setConfigOptionsStatus] =
    useState<StudioConfigLoadStatusByKind>({ llm: "loading", image: "loading", video: "loading" });
  const [configLoadError, setConfigLoadError] = useState<string | null>(null);
  const [textAssets, setTextAssets] = useState<CanvasTextAssetSummary[]>([]);
  const [flowInstance, setFlowInstance] = useState<ReactFlowInstance<
    StudioNode,
    StudioEdge
  > | null>(null);
  const [previewDialogOpen, setPreviewDialogOpen] = useState(false);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewData, setPreviewData] =
    useState<ModelRequestPreview | null>(null);
  const [contentPreviewNode, setContentPreviewNode] =
    useState<StudioNode | null>(null);
  const [mobileNodeLibraryOpen, setMobileNodeLibraryOpen] = useState(false);
  const [mobileInspectorOpen, setMobileInspectorOpen] = useState(false);
  const [textMaterialsOpen, setTextMaterialsOpen] = useState(false);
  const [textMaterialsDirty, setTextMaterialsDirty] = useState(false);
  const [agentConsoleOpen, setAgentConsoleOpen] = useState(false);
  const [miniMapVisible, setMiniMapVisible] = useState(false);
  const requestRef = useRef(0);
  const canvasSectionRef = useRef<HTMLElement | null>(null);
  const workflowDiscoveryTimerRef = useRef<number | null>(null);
  const workflowDiscoveryInFlightRef = useRef(false);
  const workflowRuntimeRefreshSignatureRef = useRef<string | null>(null);
  const configRefreshRef = useRef(0);
  const textAssetsRequestRef = useRef(0);
  const previousFlowNodeCountRef = useRef(0);
  const {
    leftSidebarWidth,
    rightSidebarWidth,
    leftSidebarMaxWidth,
    rightSidebarMaxWidth,
    isMobileLayout,
    leftSidebarVisible,
    rightSidebarVisible,
    canvasGridTemplateColumns,
    isResizingLeftSidebar,
    isResizingRightSidebar,
    handleSidebarResizeStart,
    handleSidebarResizeMove,
    finishSidebarResize,
    setSidebarWidth,
    resetSidebarWidth,
  } = useVideoCanvasSidebarLayout();

  const selectedNode = useMemo(
    () => nodes.find((node) => node.selected) ?? null,
    [nodes],
  );
  const selectedEdge = useMemo(
    () => edges.find((edge) => edge.selected) ?? null,
    [edges],
  );
  const nodeContractsByKind = useMemo(
    () => studioNodeContractMap(nodeContracts),
    [nodeContracts],
  );
  const nodeTemplateGroups = useMemo(
    () => buildNodeTemplateGroups(nodeContracts, t),
    [nodeContracts, t],
  );
  const nodeTemplateCount = useMemo(
    () =>
      nodeTemplateGroups.reduce((sum, group) => sum + group.items.length, 0),
    [nodeTemplateGroups],
  );
  const selectedNodeRunToTarget = selectedNode
    ? studioNodeContract(nodeContractsByKind, selectedNode.data.nodeKind)
        ?.category === "output"
    : false;
  const selectedNodeRunnable = selectedNode
    ? Boolean(
        studioNodeContract(nodeContractsByKind, selectedNode.data.nodeKind)
          ?.runnable,
      )
    : false;
  const selectedRunScope = useMemo(
    () =>
      computeRunToNodeScope(
        selectedNodeRunToTarget ? selectedNode : null,
        nodes,
        edges,
        nodeContractsByKind,
      ),
    [edges, nodeContractsByKind, nodes, selectedNode, selectedNodeRunToTarget],
  );
  const workflowNodeRunStateCacheRef = useRef({
    detail: workflowDetail,
    facts: new Map() as ReturnType<typeof canvasWorkflowNodeRunStateFacts>,
  });
  const workflowNodeRunStateFacts = useMemo(
    () => canvasWorkflowNodeRunStateFacts(
      workflowDetail,
      workflowNodeRunStateCacheRef.current.detail,
      workflowNodeRunStateCacheRef.current.facts,
    ),
    [workflowDetail],
  );
  useEffect(() => {
    workflowNodeRunStateCacheRef.current = { detail: workflowDetail, facts: workflowNodeRunStateFacts };
  }, [workflowDetail, workflowNodeRunStateFacts]);
  const workflowNodeRunStates = useMemo(
    () => canvasWorkflowNodeRunStates(workflowNodeRunStateFacts, workflowNowMs),
    [workflowNodeRunStateFacts, workflowNowMs],
  );
  const workflowActiveScope = useMemo(
    () => computeWorkflowActiveScope(nodes, edges, workflowNodeRunStateFacts),
    [edges, nodes, workflowNodeRunStateFacts],
  );
  const flowNodes = useMemo(
    () =>
      nodes.map((node) =>
        nodeWithWorkflowRunClass(
          nodeWithRunScopeClass(node, selectedRunScope),
          workflowNodeRunStates,
        ),
      ),
    [nodes, selectedRunScope, workflowNodeRunStates],
  );
  const flowEdges = useMemo(
    () =>
      edges.map((edge) => ({
        ...edgeWithRuntimeClasses(edge, selectedRunScope, workflowActiveScope),
        label: edgeRoleLabel(edge.data!.role, t),
      })),
    [edges, selectedRunScope, workflowActiveScope, t],
  );
  const workflowSubmitting = workflowSubmissionLabel !== null;
  const workflowSubmissionText = workflowSubmissionLabel
    ? t(workflowSubmissionLabel.key, workflowSubmissionLabel.params)
    : null;
  const canResumeCurrentCanvasRun = Boolean(
    document &&
      workflowDetail?.run.can_resume &&
      workflowDetail.run.status !== "running" &&
      workflowDetail.run.status !== "pending" &&
      workflowDetail.run.workflow_kind === CANVAS_WORKFLOW_KIND &&
      workflowDetail.run.scope_table === "canvas_graph" &&
      workflowDetail.run.scope_id === document.graph.id &&
      workflowDetail.run.mode === "full_graph" &&
      workflowRunGraphVersion(workflowDetail.run) === document.graph.version,
  );
  const workflowLive =
    workflowSubmitting ||
    resumingWorkflow ||
    (latestWorkflowRunId !== null && workflowLoading && workflowDetail === null) ||
    workflowDetail?.run.status === "running" ||
    workflowDetail?.run.status === "pending" ||
    (latestWorkflowRunId !== null && workflowStatusError !== null && workflowDetail === null);
  const lifecycleBlockedReason = project
    ? getProjectLifecycleBlockedReason(project, t)
    : null;
  const canEdit = project ? canEditProjectMaterials(project) : false;
  const canvasWriteBusy = saving || importing;
  const canvasReady = !loading && loadError == null && document != null && nodeContracts.length > 0;
  const hasRunnableNodes = useMemo(
    () => nodes.some((node) => studioNodeContract(nodeContractsByKind, node.data.nodeKind)?.runnable),
    [nodeContractsByKind, nodes],
  );
  const canvasMutationDisabled = !canEdit || canvasWriteBusy || !canvasReady || workflowLive;
  useNovelDetailDirty(`video-canvas:${projectId}`, dirty);

  const recordCanvasViewport = useCallback((viewport: Viewport) => {
    const previousViewport = canvasGraphViewport(document?.graph.viewport_json);
    if (
      previousViewport &&
      previousViewport.x === viewport.x &&
      previousViewport.y === viewport.y &&
      previousViewport.zoom === viewport.zoom
    ) return;
    setDocument((current) => current
      ? { ...current, graph: { ...current.graph, viewport_json: viewport } }
      : current);
    setDirty(true);
  }, [document?.graph.viewport_json, setDirty, setDocument]);

  useEffect(() => {
    if (leftSidebarVisible) setMobileNodeLibraryOpen(false);
    if (rightSidebarVisible) setMobileInspectorOpen(false);
  }, [leftSidebarVisible, rightSidebarVisible]);

  useEffect(() => {
    if (!workflowLive) return;
    setWorkflowNowMs(Date.now());
    const timer = window.setInterval(() => setWorkflowNowMs(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [workflowLive]);

  const refreshStudioConfigOptions = useCallback(async () => {
    const requestId = configRefreshRef.current + 1;
    configRefreshRef.current = requestId;
    setConfigLoadError(null);
    setConfigOptionsStatus((current) => ({
      llm: current.llm === "ready" ? "ready" : "loading",
      image: current.image === "ready" ? "ready" : "loading",
      video: current.video === "ready" ? "ready" : "loading",
    }));
    const failures: string[] = [];
    const loadOptions = async (
      kind: StudioConfigKind,
      label: string,
      request: () => Promise<StudioConfigOptionsByKind[StudioConfigKind]>,
    ) => {
      try {
        const options = await request();
        if (configRefreshRef.current !== requestId) return;
        setConfigOptions((current) => ({ ...current, [kind]: options }));
        setConfigOptionsStatus((current) => ({ ...current, [kind]: "ready" }));
      } catch (err) {
        if (configRefreshRef.current !== requestId) return;
        setConfigOptionsStatus((current) => ({ ...current, [kind]: "failed" }));
        failures.push(`${label}: ${extractErrorMessage(err, translatorRef.current("video.refreshCanvasConfigFailed"))}`);
      }
    };
    try {
      await Promise.all([
        loadOptions("llm", translatorRef.current("video.canvas.inspector.llmConfig"), async () =>
          buildStudioConfigOptions(await listLlmConfig(), [], []).llm,
        ),
        loadOptions("image", translatorRef.current("video.canvas.inspector.imageConfig"), async () =>
          buildStudioConfigOptions([], await listImageConfigs(), []).image,
        ),
        loadOptions("video", translatorRef.current("video.canvas.inspector.videoConfig"), async () =>
          buildStudioConfigOptions([], [], await listVideoConfigs()).video,
        ),
      ]);
      if (configRefreshRef.current === requestId && failures.length > 0) {
        setConfigLoadError(failures.join("; "));
        addToast(failures.join("; "), "error");
      } else if (configRefreshRef.current === requestId) {
        setConfigLoadError(null);
      }
    } catch (err) {
      if (configRefreshRef.current === requestId) {
        const message = extractErrorMessage(err, translatorRef.current("video.refreshCanvasConfigFailed"));
        setConfigLoadError(message);
        addToast(message, "error");
      }
    }
  }, [addToast]);

  const refreshCanvasTextAssets = useCallback(async () => {
    const requestId = textAssetsRequestRef.current + 1;
    textAssetsRequestRef.current = requestId;
    const assets = await listCanvasTextAssets(projectId);
    if (textAssetsRequestRef.current !== requestId) return [];
    setTextAssets(assets);
    return assets;
  }, [projectId]);

  useEffect(() => {
    textAssetsRequestRef.current += 1;
    setTextAssets([]);
    return () => {
      textAssetsRequestRef.current += 1;
    };
  }, [projectId]);

  useEffect(() => {
    if (!isPositiveSafeInteger(projectId)) return;
    const requestId = textAssetsRequestRef.current + 1;
    void refreshCanvasTextAssets().catch((error) => {
      if (textAssetsRequestRef.current !== requestId) return;
      addToast(extractErrorMessage(error, translatorRef.current("novel.textMaterialsLoadFailed")), "error");
    });
  }, [addToast, projectId, refreshCanvasTextAssets]);

  const load = useCallback(async () => {
    const requestId = requestRef.current + 1;
    requestRef.current = requestId;
    if (!isPositiveSafeInteger(projectId)) {
      setLoadError(translatorRef.current("video.projectInvalid"));
      setLoading(false);
      return;
    }
    setLoadError(null);
    setLoading(true);
    try {
      const [projectResult, graphResult, contractsResult] = await Promise.allSettled([
        getProject(projectId),
        getCanvasGraph(projectId),
        getCanvasNodeContracts(),
      ]);
      if (requestRef.current !== requestId) return;
      const failures: string[] = [];
      const projectRecord = projectResult.status === "fulfilled" ? projectResult.value : null;
      const graphDocument = graphResult.status === "fulfilled" ? graphResult.value : null;
      let normalizedContracts: StudioNodeContract[] | null = null;

      if (projectResult.status === "rejected") {
        failures.push(extractErrorMessage(projectResult.reason, translatorRef.current("video.canvasLoadFailed")));
      } else if (projectResult.value.project_kind !== "canvas") {
        addToast(translatorRef.current("video.notCanvasProject"), "error");
        navigate("/", { replace: true });
        return;
      }
      if (projectRecord && graphResult.status === "fulfilled"
          && graphResult.value.graph.graph_kind !== "video_studio") {
        failures.push(translatorRef.current("video.canvasWrongGraphKind"));
      }
      if (graphResult.status === "rejected") {
        failures.push(extractErrorMessage(graphResult.reason, translatorRef.current("video.canvasLoadFailed")));
      }
      if (contractsResult.status === "rejected") {
        setNodeContracts([]);
        failures.push(extractErrorMessage(contractsResult.reason, translatorRef.current("video.canvasLoadFailed")));
      } else {
        normalizedContracts = normalizeStudioNodeContracts(contractsResult.value);
        if (normalizedContracts.length === 0) {
          normalizedContracts = null;
          setNodeContracts([]);
          failures.push(translatorRef.current("video.emptyNodeContract"));
        } else {
          setNodeContracts(normalizedContracts);
        }
      }

      if (projectRecord) setProject(projectRecord);
      if (graphDocument) {
        const flowNodes = graphDocument.nodes.map(toFlowNode);
        const flowEdges = graphDocument.edges.map(toFlowEdge);
        if (normalizedContracts) {
          const loadedEdgeError = canvasGraphSaveValidationError(
            flowNodes,
            flowEdges,
            studioNodeContractMap(normalizedContracts),
            translatorRef.current,
          );
          if (loadedEdgeError) {
            setNodeContracts([]);
            failures.push(loadedEdgeError);
          }
        }
        setDocument(graphDocument);
        setNodes((current) => preserveSelectedItems(flowNodes, current));
        setEdges((current) => preserveSelectedItems(flowEdges, current));
        setDirty(false);
      }
      setLoadError(failures.length > 0 ? failures.join("; ") : null);
      void refreshStudioConfigOptions();
    } catch (err) {
      if (requestRef.current === requestId) {
        setLoadError(extractErrorMessage(err, translatorRef.current("video.canvasLoadFailed")));
      }
    } finally {
      if (requestRef.current === requestId) setLoading(false);
    }
  }, [
    addToast,
    navigate,
    projectId,
    refreshStudioConfigOptions,
    setEdges,
    setNodes,
  ]);

  const handleLifecycleGateChange = useCallback((
    nextProject: NovelProject,
    previousGate: NovelProject["lifecycle_gate"] | null | undefined,
    previousContentRevision: number | undefined,
  ) => {
    setProject(nextProject);
    if (
      (previousGate !== "active" && nextProject.lifecycle_gate === "active")
      || (previousContentRevision != null && previousContentRevision !== nextProject.content_revision)
    ) {
      void load();
    }
  }, [load]);

  useProjectLifecycleSync(
    projectId,
    project?.lifecycle_gate,
    handleLifecycleGateChange,
    {
      enabled: project != null && !loading,
      contentRevision: project?.content_revision,
      updatedAt: project?.updated_at,
      onReadError: () => { void load(); },
    },
  );

  const handleRetry = async () => {
    if (loading || saving || importing) return;
    if (dirty && !(await confirmDialog({
      title: t("common.unsavedChangesTitle"),
      message: t("common.unsavedChangesMessage", { name: t("video.canvasProject"), action: t("common.retry") }),
      confirmText: t("common.discardDraft"),
      dialogType: "warning",
    }))) return;
    await load();
  };

  const handleUseTextToVideoTemplate = async () => {
    if (
      !project ||
      !document ||
      document.nodes.length > 0 ||
      canvasMutationDisabled ||
      dirty ||
      saving ||
      importing
    ) return;
    setSaving(true);
    try {
      const created = await initializeVideoStarterGraph(
        projectId,
        t("video.canvas.defaultRewriteInstruction"),
      );
      addToast(
        t(created ? "video.canvas.templateAdded" : "video.canvas.templateAlreadyExists"),
        created ? "success" : "info",
      );
      await load();
    } catch (error) {
      addToast(extractErrorMessage(error, t("video.canvas.templateFailed")), "error");
    } finally {
      setSaving(false);
    }
  };

  useEffect(() => {
    const refreshWhenVisible = () => {
      if (window.document.visibilityState === "visible") {
        void refreshStudioConfigOptions();
      }
    };
    window.addEventListener("focus", refreshWhenVisible);
    window.document.addEventListener("visibilitychange", refreshWhenVisible);
    return () => {
      window.removeEventListener("focus", refreshWhenVisible);
      window.document.removeEventListener(
        "visibilitychange",
        refreshWhenVisible,
      );
    };
  }, [refreshStudioConfigOptions]);

  useEffect(() => {
    void load();
    return () => {
      requestRef.current += 1;
      configRefreshRef.current += 1;
    };
  }, [load]);


  const graphActions = useCanvasGraphActions({
    projectId,
    project,
    document,
    nodes,
    edges,
    selectedNode,
    nodeContractsByKind,
    configOptions,
    canvasWriteBusy,
    canvasMutationDisabled,
    flowInstance,
    canvasSectionRef,
    t,
    addToast,
    setDocument,
    setNodes,
    setEdges,
    applyNodesChange,
    applyEdgesChange,
    setDirty,
    setSaving,
    setImporting,
    setMobileNodeLibraryOpen,
  });

  const workflowActions = useCanvasWorkflowActions({
    projectId,
    project,
    document,
    nodes,
    edges,
    nodeContractsByKind,
    canResumeCurrentCanvasRun,
    canvasMutationDisabled,
    dirty,
    workflowSubmitting,
    resumingWorkflow,
    cancellingWorkflow,
    latestWorkflowRunId,
    workflowDetail,
    contextMenu,
    workflowDiscoveryTimerRef,
    workflowDiscoveryInFlightRef,
    workflowRuntimeRefreshSignatureRef,
    t,
    addToast,
    setLatestWorkflowRunId,
    setWorkflowDetail,
    setWorkflowEvents,
    setWorkflowStatusError,
    setWorkflowEventsError,
    setWorkflowLoading,
    setWorkflowSubmissionLabel,
    setCancellingWorkflow,
    setResumingWorkflow,
    setPreviewDialogOpen,
    setPreviewLoading,
    setPreviewData,
    setContentPreviewNode,
    refreshCanvasRuntimeResults: graphActions.refreshCanvasRuntimeResults,
    selectCanvasNode: graphActions.selectCanvasNode,
    selectCanvasEdge: graphActions.selectCanvasEdge,
    deleteCanvasNode: graphActions.deleteCanvasNode,
    deleteCanvasEdge: graphActions.deleteCanvasEdge,
    importMedia: graphActions.importMedia,
  });
  const {
    onNodesChange,
    onEdgesChange,
    onConnect,
    isValidCanvasConnection,
    addNodeToVisibleCanvas,
    onNodeTemplateDragStart,
    onCanvasDragOver,
    onCanvasDrop,
    updateSelectedConfig,
    updateSelectedRef,
    moveAssembleClip,
    persist,
    importMedia,
    selectCanvasNode,
  } = graphActions;
  const {
    loadWorkflowStatus,
    previewNode,
    closePreview,
    runNode,
    runToNode,
    runGraph,
    onNodeContextMenu,
    onEdgeContextMenu,
    onPaneContextMenu,
    cancelActiveWorkflow,
    resumeActiveWorkflow,
  } = workflowActions;

  const locateWorkflowNode = useCallback((nodeId: string) => {
    selectCanvasNode(nodeId);
    void flowInstance?.fitView({
      nodes: [{ id: nodeId }],
      padding: 0.35,
      duration: 300,
    });
  }, [flowInstance, selectCanvasNode]);

  useEffect(() => {
    if (!flowInstance) return;
    const previousCount = previousFlowNodeCountRef.current;
    previousFlowNodeCountRef.current = nodes.length;
    if (previousCount !== 0 || nodes.length === 0 || initialViewport !== undefined) return;
    const frame = window.requestAnimationFrame(() => {
      void flowInstance.fitView({
        padding: isMobileLayout ? 0.2 : 0.25,
        maxZoom: isMobileLayout ? 0.8 : 1,
        duration: 180,
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [flowInstance, initialViewport, isMobileLayout, nodes.length]);

  const headerActionItems = useMemo<DropdownItem[]>(() => {
    const importDisabled = !canvasReady || canvasWriteBusy || !canEdit;
    const items: DropdownItem[] = [];
    if (!rightSidebarVisible) {
      items.push({
        label: t("video.openProperties"),
        icon: <SlidersHorizontal size={13} />,
        onClick: () => setMobileInspectorOpen(true),
      });
      items.push({ type: "divider" });
    }
    items.push(
      {
        label: t("video.importImage"),
        icon: <Upload size={13} />,
        disabled: importDisabled,
        onClick: () => void importMedia("image"),
      },
      {
        label: t("video.importVideo"),
        icon: <Upload size={13} />,
        disabled: importDisabled,
        onClick: () => void importMedia("video"),
      },
    );
    items.push({
      label: t("video.importAudio"),
      icon: <Upload size={13} />,
      disabled: importDisabled,
      onClick: () => void importMedia("audio"),
    });
    if (selectedNode) {
      items.push({ type: "divider" });
      items.push({
        label: nodeHasProviderRequest(selectedNode) ? t("video.viewModelRequest") : t("video.viewNodeContent"),
        icon: <Eye size={13} />,
        disabled:
          previewLoading ||
          (nodeHasProviderRequest(selectedNode) && (!document || dirty)),
        onClick: () => void previewNode(selectedNode),
      });
      if (selectedNodeRunToTarget || selectedNodeRunnable) {
        items.push({
          label: selectedNodeRunToTarget ? t("video.runToSelectedNode") : t("video.runSelectedNode"),
          icon: <Play size={13} />,
          disabled:
            canvasMutationDisabled || dirty || workflowSubmitting,
          onClick: () =>
            void (selectedNodeRunToTarget
              ? runToNode(selectedNode)
              : runNode(selectedNode)),
        });
      }
    }
    return items;
  }, [
    canEdit,
    canvasReady,
    canvasMutationDisabled,
    canvasWriteBusy,
    dirty,
    document,
    importMedia,
    previewLoading,
    previewNode,
    runNode,
    runToNode,
    rightSidebarVisible,
    selectedNode,
    selectedNodeRunToTarget,
    selectedNodeRunnable,
    workflowSubmitting,
    t,
  ]);

  const renderNodeLibraryPanel = (mobile: boolean) => (
    <div className={mobile ? "space-y-3" : "flex h-full min-h-0 flex-col"}>
      {mobile ? (
        <p className="text-small text-book-text-muted">
          {t("video.nodeLibraryMobileHint")}
        </p>
      ) : (
        <div className="mb-3">
          <h2 className="text-sm font-semibold text-book-text-main">{t("video.nodeLibraryTitle")}</h2>
          <p className="mt-0.5 text-small text-book-text-muted">
            {t("video.nodeLibraryCount", { count: formatNumber(nodeTemplateCount, locale) })}
          </p>
        </div>
      )}
      <div className={mobile ? "space-y-4" : "min-h-0 flex-1 space-y-4 overflow-y-auto pr-1 custom-scrollbar"}>
        {nodeTemplateGroups.map((group) => (
          <section key={group.title} className="space-y-1.5">
            <h3 className="a4n-kicker px-1">
              {group.title}
            </h3>
            <div className="divide-y divide-book-border/35">
              {group.items.map(({ kind, label, icon: Icon }) => (
                <button
                  key={kind}
                  type="button"
                  draggable={!mobile && !canvasMutationDisabled}
                  onClick={() => addNodeToVisibleCanvas(kind, label)}
                  onDragStart={
                    mobile
                      ? undefined
                      : (event) => onNodeTemplateDragStart(event, kind, label)
                  }
                  disabled={canvasMutationDisabled}
                  title={mobile ? t("video.addToCanvas") : t("video.dragToCanvas")}
                  className={`flex w-full select-none items-center gap-2 rounded-xl bg-transparent px-2 text-left text-xs text-book-text-sub transition-colors hover:bg-book-bg-tertiary hover:text-book-text-main disabled:cursor-not-allowed disabled:opacity-50 ${
                    mobile
                      ? "h-10"
                      : "h-8 cursor-grab active:cursor-grabbing"
                  }`}
                >
                  <span className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-lg bg-book-primary/10 text-book-primary">
                    <Icon size={13} />
                  </span>
                  <span className="truncate">{label}</span>
                </button>
              ))}
            </div>
          </section>
        ))}
      </div>
    </div>
  );

  const renderInspectorPanel = (mobile = false) => (
    <div className={mobile ? "space-y-3" : "flex h-full min-h-0 flex-col"}>
      {!mobile ? (
        <div className="mb-3">
          <h2 className="text-sm font-semibold text-book-text-main">
            {selectedEdge ? t("video.edgeProperties") : t("video.nodeProperties")}
          </h2>
          <p className="mt-0.5 truncate text-small text-book-text-muted">
            {selectedNode
              ? `${selectedNode.data.label || nodeDisplayTitle(selectedNode.data, t)} · ${stateLabel(selectedNode.data.state, t)}`
              : selectedEdge
                ? edgeRoleLabel(selectedEdge.data?.role ?? "", t)
                : t("video.editAfterSelect")}
          </p>
        </div>
      ) : selectedNode ? (
        <p className="text-small text-book-text-muted">
          {selectedNode.data.label || nodeDisplayTitle(selectedNode.data, t)} · {stateLabel(selectedNode.data.state, t)}
        </p>
      ) : null}
      <div className={mobile ? "space-y-3" : "min-h-0 flex-1 overflow-y-auto pr-1 custom-scrollbar"}>
        {selectedEdge ? (
          <Suspense fallback={<CanvasInspectorFallback />}>
            <EdgeInspector edge={selectedEdge} nodes={nodes} />
          </Suspense>
        ) : selectedNode ? (
          <Suspense fallback={<CanvasInspectorFallback />}>
            <NodeInspector
              node={selectedNode}
              nodes={nodes}
              edges={edges}
              disabled={canvasMutationDisabled}
              configOptions={configOptions}
              configOptionsStatus={configOptionsStatus}
              textAssets={textAssets}
              onChange={updateSelectedConfig}
              onRefChange={updateSelectedRef}
              onMoveAssembleClip={moveAssembleClip}
              onPreviewContent={(node) => setContentPreviewNode(node)}
            />
          </Suspense>
        ) : (
          <div className="a4n-empty-state a4n-empty-state--compact text-xs">
            {t("video.selectCanvasItem")}
          </div>
        )}
        <CanvasWorkflowPanel
          latestWorkflowRunId={latestWorkflowRunId}
          detail={workflowDetail}
          events={workflowEvents}
          statusError={workflowStatusError}
          eventsError={workflowEventsError}
          loading={workflowLoading}
          submittingLabel={workflowSubmissionText}
          nodes={nodes}
          draftChanged={dirty}
          cancelling={cancellingWorkflow}
          resuming={resumingWorkflow}
          onRefresh={() =>
            latestWorkflowRunId && void loadWorkflowStatus(latestWorkflowRunId)
          }
          onLocateNode={locateWorkflowNode}
          onCancel={() => void cancelActiveWorkflow()}
          onResume={() => void resumeActiveWorkflow()}
        />
      </div>
    </div>
  );

  return (
    <StudioNodeContractsProvider contracts={nodeContractsByKind}>
      <div className="h-full min-h-0 flex flex-col bg-book-bg">
        <UnsavedNavigationGuard dirty={dirty || textMaterialsDirty} name={t("video.canvasProject")} action={t("common.leavePage")} />
        <PageHeader
          className="shrink-0 a4n-video-canvas-header"
          leading={<BackButton iconOnly title={t("a11y.back")} onClick={() => navigate(project ? getProjectStudioBackRoute(project) : "/")} />}
          icon={<Film size={15} />}
          title={project?.title || t("video.canvasProject")}
          meta={
            <span className="text-small text-book-text-muted">
              {saving
                ? t("video.canvasSaving")
                : importing
                  ? t("video.canvasImporting")
                  : t("video.freeCanvas")}
              <span
                aria-hidden={canvasWriteBusy || !dirty}
                className={canvasWriteBusy || !dirty ? "invisible" : undefined}
              >
                {t("video.canvasDirty")}
              </span>
            </span>
          }
          actions={
            <>
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => setAgentConsoleOpen(true)}
                disabled={!project || !canEdit || workflowLive}
                title={t("agent.domain.openCanvas")}
              >
                <Bot size={14} />
                <span className="hidden sm:inline">{t("agent.domain.domainAgent")}</span>
              </BookButton>
              <BookIconButton
                label={t("novel.textMaterialsTitle")}
                onClick={() => setTextMaterialsOpen(true)}
                disabled={!project || !canEdit}
                variant="secondary"
                size="sm"
              >
                <FileText size={14} />
              </BookIconButton>
              {!leftSidebarVisible ? (
                  <BookIconButton
                    label={t("video.openNodeLibrary")}
                    onClick={() => setMobileNodeLibraryOpen(true)}
                    disabled={canvasMutationDisabled}
                    variant="secondary"
                    size="sm"
                  >
                    <ListPlus size={14} />
                  </BookIconButton>
              ) : null}
              <Dropdown items={headerActionItems} />
              {dirty || saving ? (
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={persist}
                  disabled={canvasMutationDisabled}
                >
                  {saving ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Save size={14} />
                  )}
                  {saving ? t("video.canvasSaving") : t("video.saveCanvas")}
                </BookButton>
              ) : (
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={() => void runGraph()}
                  disabled={
                    !document ||
                    !hasRunnableNodes ||
                    canvasMutationDisabled
                  }
                  title={t("video.runFullCanvas")}
                >
                  {workflowSubmitting || resumingWorkflow || workflowDetail?.run.status === "running" || workflowDetail?.run.status === "pending" ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Play size={14} />
                  )}
                  {workflowSubmitting
                    ? workflowSubmissionText ?? t("video.running")
                    : resumingWorkflow || workflowDetail?.run.status === "running" || workflowDetail?.run.status === "pending"
                      ? t("video.running")
                    : canResumeCurrentCanvasRun
                      ? t("video.canvas.workflow.resume")
                      : t("video.runCanvas")}
                </BookButton>
              )}
            </>
          }
        />
        {lifecycleBlockedReason ? (
          <div role="status" className="shrink-0 border-b border-warning/30 bg-warning-bg px-4 py-2 text-small text-book-text-main">
            {lifecycleBlockedReason}
          </div>
        ) : null}
        {loadError && document ? (
          <div role="alert" className="flex flex-wrap items-center gap-3 px-4 py-2 text-sm text-error">
            <p className="min-w-0 flex-1">{loadError}</p>
            <BookButton variant="secondary" size="sm" onClick={() => void handleRetry()} disabled={loading || saving || importing}>
              <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
              {t("common.retry")}
            </BookButton>
          </div>
        ) : null}
        {configLoadError ? (
          <div role="alert" className="flex flex-wrap items-center gap-3 px-4 py-2 text-sm text-error">
            <p className="min-w-0 flex-1">{configLoadError}</p>
            <BookButton variant="secondary" size="sm" onClick={() => void refreshStudioConfigOptions()}>
              <RefreshCw size={12} /> {t("common.retry")}
            </BookButton>
          </div>
        ) : null}
        <section
          aria-label={t("video.canvasEditor")}
          aria-busy={canvasWriteBusy}
          className="grid flex-1 min-h-0 overflow-hidden"
          style={{ gridTemplateColumns: canvasGridTemplateColumns }}
        >
          {leftSidebarVisible ? (
            <aside className="min-h-0 overflow-hidden border-r border-book-border/80 bg-book-bg-paper p-2.5">
              {renderNodeLibraryPanel(false)}
            </aside>
          ) : null}
          {leftSidebarVisible ? (
            <CanvasSidebarResizeHandle
              side="left"
              width={leftSidebarWidth}
              maxWidth={leftSidebarMaxWidth}
              isResizing={isResizingLeftSidebar}
              onResizeStart={handleSidebarResizeStart}
              onResizeMove={handleSidebarResizeMove}
              onResizeEnd={finishSidebarResize}
              onSetWidth={setSidebarWidth}
              onResetWidth={resetSidebarWidth}
            />
          ) : null}
          <section ref={canvasSectionRef} className="relative min-h-0 bg-book-bg">
            {loading ? (
              <div className="flex h-full items-center justify-center text-xs text-book-text-muted">
                <LoadingSpinner size="sm" className="mr-2" />
                {t("video.canvasLoading")}
              </div>
            ) : loadError && !document ? (
              <div role="alert" className="flex flex-wrap items-center gap-3 p-6 text-sm text-error">
                <p className="min-w-0 flex-1">{loadError}</p>
                <BookButton variant="secondary" size="sm" onClick={() => void handleRetry()} disabled={loading || saving || importing}>
                  <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
                  {t("common.retry")}
                </BookButton>
              </div>
            ) : (
              <>
                <ReactFlow
                  nodes={flowNodes}
                  edges={flowEdges}
                  nodeTypes={STUDIO_NODE_TYPES}
                  onNodesChange={onNodesChange}
                  onEdgesChange={onEdgesChange}
                  onConnect={onConnect}
                  onMoveEnd={(event, viewport) => {
                    if (!event) return;
                    recordCanvasViewport(viewport);
                  }}
                  isValidConnection={isValidCanvasConnection}
                  nodesDraggable={!canvasMutationDisabled}
                  nodesConnectable={!canvasMutationDisabled}
                  onNodeContextMenu={onNodeContextMenu}
                  onEdgeContextMenu={onEdgeContextMenu}
                  onPaneContextMenu={onPaneContextMenu}
                  onInit={setFlowInstance}
                  onDragOver={onCanvasDragOver}
                  onDrop={onCanvasDrop}
                  fitView={initialViewport === undefined}
                  fitViewOptions={{ padding: 0.25, maxZoom: 1 }}
                  defaultViewport={initialViewport}
                  minZoom={isMobileLayout ? 0.25 : 0.35}
                  maxZoom={1.4}
                  deleteKeyCode={
                    canvasMutationDisabled ? null : ["Delete", "Backspace"]
                  }
                  className="a4n-studio-flow"
                >
                  {!isMobileLayout && miniMapVisible ? (
                    <MiniMap
                      pannable
                      zoomable
                      className="a4n-studio-minimap"
                      nodeColor="rgb(var(--color-bg-tertiary))"
                      maskColor="rgb(0 0 0 / 0.18)"
                      style={{
                        background: "rgb(var(--color-bg-secondary) / 0.94)",
                      }}
                    />
                  ) : null}
                  <FlowControls
                    position="top-left"
                    orientation="horizontal"
                    miniMapVisible={miniMapVisible}
                    miniMapLabel={!isMobileLayout ? t(miniMapVisible ? "video.hideCanvasMiniMap" : "video.showCanvasMiniMap") : undefined}
                    onToggleMiniMap={!isMobileLayout ? () => setMiniMapVisible((visible) => !visible) : undefined}
                    onViewportChange={() => {
                      const viewport = flowInstance?.getViewport();
                      if (viewport) recordCanvasViewport(viewport);
                    }}
                  />
                  <Background
                    color="rgb(var(--color-text-tertiary) / 0.24)"
                    gap={32}
                    size={1}
                  />
                </ReactFlow>
                {nodes.length === 0 ? (
                  <CanvasEmptyState
                    disabled={canvasMutationDisabled}
                    showTextToVideoTemplate={Boolean(project)}
                    templateDisabled={saving || importing || dirty}
                    mobile={isMobileLayout}
                    onOpenNodeLibrary={() => setMobileNodeLibraryOpen(true)}
                    onUseTextToVideoTemplate={() => void handleUseTextToVideoTemplate()}
                  />
                ) : null}
              </>
            )}
          </section>
          {rightSidebarVisible ? (
            <CanvasSidebarResizeHandle
              side="right"
              width={rightSidebarWidth}
              maxWidth={rightSidebarMaxWidth}
              isResizing={isResizingRightSidebar}
              onResizeStart={handleSidebarResizeStart}
              onResizeMove={handleSidebarResizeMove}
              onResizeEnd={finishSidebarResize}
              onSetWidth={setSidebarWidth}
              onResetWidth={resetSidebarWidth}
            />
          ) : null}
          {rightSidebarVisible ? (
            <aside className="min-h-0 overflow-hidden border-l border-book-border/80 bg-book-bg-paper p-2.5">
              {renderInspectorPanel(false)}
            </aside>
          ) : null}
        </section>
        {!leftSidebarVisible ? (
            <Modal
              isOpen={mobileNodeLibraryOpen}
              onClose={() => setMobileNodeLibraryOpen(false)}
              title={t("video.nodeLibrary")}
              maxWidthClassName="max-w-lg"
              bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar p-3"
            >
              {renderNodeLibraryPanel(true)}
            </Modal>
        ) : null}
        {!rightSidebarVisible ? (
            <Modal
              isOpen={mobileInspectorOpen}
              onClose={() => setMobileInspectorOpen(false)}
              title={selectedEdge ? t("video.edgeProperties") : t("video.nodeProperties")}
              maxWidthClassName="max-w-xl"
              bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar p-3"
            >
              {renderInspectorPanel(true)}
            </Modal>
        ) : null}
        {previewDialogOpen ? (
          <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
            <RequestPreviewModal
              open
              loading={previewLoading}
              data={previewData}
              onClose={closePreview}
            />
          </Suspense>
        ) : null}
        <Modal
          isOpen={textMaterialsOpen}
          onClose={() => {
            void (async () => {
              if (!dirtyRegistry.canDiscardDirty()) {
                addToast(t("common.operationInProgress"), "info");
                return;
              }
              if (textMaterialsDirty && !await confirmDialog({
                title: t("novel.textMaterialsDiscardTitle"),
                message: t("novel.textMaterialsDiscardChanges"),
              })) return;
              setTextMaterialsDirty(false);
              setTextMaterialsOpen(false);
              void refreshCanvasTextAssets().catch((error) =>
                addToast(extractErrorMessage(error, t("novel.textMaterialsLoadFailed")), "error"),
              );
            })();
          }}
          title={t("novel.textMaterialsTitle")}
          subtitle={project?.title}
          maxWidthClassName="max-w-5xl"
          bodyClassName="flex min-h-0 flex-1 overflow-hidden p-3"
          closeOnBackdrop={!textMaterialsDirty && dirtyRegistry.canDiscardDirty()}
          closeOnEscape={!textMaterialsDirty && dirtyRegistry.canDiscardDirty()}
        >
          {project ? (
            <ProjectTextMaterialsPanel
              projectId={project.id}
              canEdit={canEdit}
              onDirtyChange={setTextMaterialsDirty}
            />
          ) : null}
        </Modal>
        {contentPreviewNode ? (
          <Suspense fallback={<LoadingSpinner size="md" text={t("common.loading")} />}>
            <CanvasNodeContentPreviewDialog
              node={contentPreviewNode}
              onClose={() => setContentPreviewNode(null)}
              onCopyError={(message) => addToast(message, "error")}
            />
          </Suspense>
        ) : null}
        <Modal
          isOpen={agentConsoleOpen}
          onClose={() => setAgentConsoleOpen(false)}
          title={t("agent.domain.canvasTitle")}
          subtitle={project?.title ?? undefined}
          icon={<Bot size={14} />}
          maxWidthClassName="max-w-2xl"
          bodyClassName="min-h-0 flex-1 overflow-hidden p-0"
          closeOnBackdrop={false}
          closeOnEscape={false}
          showCloseButton={false}
        >
          <DomainAgentConsole
            capability="canvas"
            projectId={projectId}
            composerDirtyKey={`domain-agent-composer:canvas:${projectId}`}
            onClose={() => setAgentConsoleOpen(false)}
            resourceLabel={project?.title}
            onResourceChanged={() => {
              if (dirty) {
                addToast(t("agent.domain.canvasLocalChanges"), "info");
                return;
              }
              return load();
            }}
            fill
            className="min-h-0 rounded-none border-0 shadow-none"
          />
        </Modal>
      </div>
    </StudioNodeContractsProvider>
  );
};

const CanvasInspectorFallback = () => {
  const { t } = useI18n();
  return (
    <div className="inline-flex items-center gap-2 py-2 text-small text-book-text-muted">
      <LoadingSpinner size="sm" />
      {t("video.inspectorLoading")}
    </div>
  );
};

function workflowRunGraphVersion(run: WorkflowRunSummary): number | null {
  const fingerprint = run.input_fingerprint_json;
  if (!isRecord(fingerprint) || typeof fingerprint.graph_version !== "number") {
    return null;
  }
  return fingerprint.graph_version;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
