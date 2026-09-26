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
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
  Bot,
  Eye,
  Film,
  ListPlus,
  Play,
  Save,
  SlidersHorizontal,
  Upload,
} from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../api";
import { listLlmConfig } from "../api/config/llm";
import type { ModelRequestPreview } from "../api/requestPreview";
import {
  getCanvasGraph,
  getCanvasNodeContracts,
  type CanvasGraphDocument,
} from "../api/canvas";
import { listImageConfigs } from "../api/image";
import { getProject, type NovelProject } from "../api/project";
import type {
  WorkflowRunDetailSummary,
  WorkflowRunSummary,
} from "../api/streaming";
import { listVideoConfigs } from "../api/video";
import type { ProgressEventRecord } from "../api/progress";
import { useToast } from "../components/feedback/Toast";
import { BookButton, BookIconButton } from "../components/ui/BookButton";
import { FlowControls } from "../components/ui/FlowControls";
import { BackButton } from "../components/ui/BackButton";
import { Dropdown, type DropdownItem } from "../components/ui/Dropdown";
import { Modal } from "../components/ui/Modal";
import { PageHeader } from "../components/shell/PageHeader";
import { DomainAgentConsole } from "../components/agent/DomainAgentConsole";
import { useContextMenu } from "../components/ui/ContextMenu";
import { CanvasEmptyState } from "./video-studio/CanvasEmptyState";
import { CanvasWorkflowPanel } from "./video-studio/CanvasWorkflowPanel";
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
import {
  CANVAS_WORKFLOW_KIND,
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
  const [workflowLoading, setWorkflowLoading] = useState(false);
  const [workflowSubmissionLabel, setWorkflowSubmissionLabel] = useState<
    string | null
  >(null);
  const [workflowNowMs, setWorkflowNowMs] = useState(() => Date.now());
  const [cancellingWorkflow, setCancellingWorkflow] = useState(false);
  const [resumingWorkflow, setResumingWorkflow] = useState(false);
  const [configOptions, setConfigOptions] =
    useState<StudioConfigOptionsByKind>(EMPTY_CONFIG_OPTIONS);
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
  const [agentConsoleOpen, setAgentConsoleOpen] = useState(false);
  const requestRef = useRef(0);
  const canvasSectionRef = useRef<HTMLElement | null>(null);
  const workflowDiscoveryTimerRef = useRef<number | null>(null);
  const workflowDiscoveryInFlightRef = useRef(false);
  const workflowRuntimeRefreshSignatureRef = useRef<string | null>(null);
  const configRefreshRef = useRef(0);
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
  const workflowNodeRunStates = useMemo(
    () => canvasWorkflowNodeRunStates(workflowDetail, workflowNowMs),
    [workflowDetail, workflowNowMs],
  );
  const workflowActiveScope = useMemo(
    () => computeWorkflowActiveScope(nodes, edges, workflowNodeRunStates),
    [edges, nodes, workflowNodeRunStates],
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
  const canResumeCurrentCanvasRun = Boolean(
    document &&
      workflowDetail?.run.can_resume &&
      workflowDetail.run.workflow_kind === CANVAS_WORKFLOW_KIND &&
      workflowDetail.run.scope_table === "canvas_graph" &&
      workflowDetail.run.scope_id === document.graph.id &&
      workflowDetail.run.mode === "full_graph" &&
      workflowRunGraphVersion(workflowDetail.run) === document.graph.version,
  );
  const workflowLive =
    workflowSubmitting ||
    workflowDetail?.run.status === "running" ||
    workflowDetail?.run.status === "pending";
  const canEdit =
    project?.my_role === "owner" ||
    project?.my_role === "manager" ||
    project?.my_role === "editor";
  const canvasWriteBusy = saving || importing;
  const canvasMutationDisabled = !canEdit || canvasWriteBusy;

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

  const loadStudioConfigOptions = useCallback(async () => {
    const [llmConfigs, imageConfigs, videoConfigs] = await Promise.all([
      listLlmConfig(),
      listImageConfigs(),
      listVideoConfigs(),
    ]);
    return buildStudioConfigOptions(llmConfigs, imageConfigs, videoConfigs);
  }, []);

  const refreshStudioConfigOptions = useCallback(async () => {
    const requestId = configRefreshRef.current + 1;
    configRefreshRef.current = requestId;
    try {
      const nextConfigOptions = await loadStudioConfigOptions();
      if (configRefreshRef.current !== requestId) return;
      setConfigOptions(nextConfigOptions);
    } catch (err) {
      if (configRefreshRef.current === requestId) {
        addToast(extractErrorMessage(err, t("video.refreshCanvasConfigFailed")), "error");
      }
    }
  }, [addToast, loadStudioConfigOptions, t]);

  const load = useCallback(async () => {
    const requestId = requestRef.current + 1;
    requestRef.current = requestId;
    if (!Number.isFinite(projectId) || projectId <= 0) {
      setLoadError(translatorRef.current("video.projectInvalid"));
      setLoading(false);
      return;
    }
    setLoadError(null);
    setLoading(true);
    try {
      const [
        projectRecord,
        graphDocument,
        rawNodeContracts,
        nextConfigOptions,
      ] = await Promise.all([
        getProject(projectId),
        getCanvasGraph(projectId),
        getCanvasNodeContracts(),
        loadStudioConfigOptions(),
      ]);
      if (requestRef.current !== requestId) return;
      if (projectRecord.project_kind !== "video") {
        addToast(translatorRef.current("video.notVideoProject"), "error");
        navigate("/", { replace: true });
        return;
      }
      const normalizedContracts =
        normalizeStudioNodeContracts(rawNodeContracts);
      if (normalizedContracts.length === 0) {
        throw new Error(translatorRef.current("video.emptyNodeContract"));
      }
      const loadedContractsByKind = studioNodeContractMap(normalizedContracts);
      const flowNodes = graphDocument.nodes.map(toFlowNode);
      const flowEdges = graphDocument.edges.map(toFlowEdge);
      const loadedEdgeError = canvasGraphSaveValidationError(
        flowNodes,
        flowEdges,
        loadedContractsByKind,
        translatorRef.current,
      );
      if (loadedEdgeError) throw new Error(loadedEdgeError);
      setProject(projectRecord);
      setDocument(graphDocument);
      setNodeContracts(normalizedContracts);
      setConfigOptions(nextConfigOptions);
      setNodes((current) => preserveSelectedItems(flowNodes, current));
      setEdges((current) => preserveSelectedItems(flowEdges, current));
      setDirty(false);
    } catch (err) {
      if (requestRef.current === requestId) {
        setLoadError(extractErrorMessage(err, translatorRef.current("video.canvasLoadFailed")));
      }
    } finally {
      if (requestRef.current === requestId) setLoading(false);
    }
  }, [
    addToast,
    loadStudioConfigOptions,
    navigate,
    projectId,
    setEdges,
    setNodes,
  ]);

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
    canEdit,
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
    canEdit,
    canvasWriteBusy,
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
  } = graphActions;
  const {
    loadWorkflowStatus,
    previewNode,
    runNode,
    runToNode,
    runGraph,
    onNodeContextMenu,
    onEdgeContextMenu,
    onPaneContextMenu,
    cancelActiveWorkflow,
    resumeActiveWorkflow,
  } = workflowActions;

  const headerActionItems = useMemo<DropdownItem[]>(() => {
    const importDisabled = !document || canvasWriteBusy || !canEdit;
    const items: DropdownItem[] = [
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
    ];
    if (project?.source_mode === "freeform") {
      items.push({
        label: t("video.importAudio"),
        icon: <Upload size={13} />,
        disabled: importDisabled,
        onClick: () => void importMedia("audio"),
      });
    }
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
    canvasMutationDisabled,
    canvasWriteBusy,
    dirty,
    document,
    importMedia,
    previewLoading,
    previewNode,
    project?.source_mode,
    runNode,
    runToNode,
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
          loading={workflowLoading}
          submittingLabel={workflowSubmissionLabel}
          cancelling={cancellingWorkflow}
          resuming={resumingWorkflow}
          onRefresh={() =>
            latestWorkflowRunId && void loadWorkflowStatus(latestWorkflowRunId)
          }
          onCancel={() => void cancelActiveWorkflow()}
          onResume={() => void resumeActiveWorkflow()}
        />
      </div>
    </div>
  );

  return (
    <StudioNodeContractsProvider contracts={nodeContractsByKind}>
      <div className="h-full min-h-0 flex flex-col bg-book-bg">
        <PageHeader
          className="shrink-0"
          leading={<BackButton iconOnly title={t("a11y.back")} onClick={() => navigate("/")} />}
          icon={<Film size={15} />}
          title={project?.title || t("video.canvasProject")}
          subtitle={
            selectedNode
              ? `${selectedNode.data.label || nodeDisplayTitle(selectedNode.data, t)} · ${stateLabel(selectedNode.data.state, t)}`
              : project?.source_mode === "freeform"
                ? t("video.freeCanvasDescription")
                : t("video.derivedCanvasDescription")
          }
          meta={
            <span className="text-small text-book-text-muted">
              {saving
                ? t("video.canvasSaving")
                : importing
                  ? t("video.canvasImporting")
                  : project?.source_mode === "freeform"
                    ? t("video.freeCanvas")
                    : t("video.derivedCanvas")}
              {!canvasWriteBusy && dirty ? t("video.canvasDirty") : ""}
            </span>
          }
          actions={
            <>
              <BookButton
                variant="secondary"
                size="sm"
                onClick={() => setAgentConsoleOpen(true)}
                disabled={!project}
                title={t("agent.domain.openCanvas")}
              >
                <Bot size={14} />
                <span className="hidden sm:inline">{t("agent.domain.domainAgent")}</span>
              </BookButton>
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
              {!rightSidebarVisible ? (
                  <BookIconButton
                    label={t("video.openProperties")}
                    onClick={() => setMobileInspectorOpen(true)}
                    variant="secondary"
                    size="sm"
                  >
                    <SlidersHorizontal size={14} />
                  </BookIconButton>
              ) : null}
              <Dropdown items={headerActionItems} />
              {dirty || saving ? (
                <BookButton
                  variant="primary"
                  size="sm"
                  onClick={persist}
                  disabled={canvasWriteBusy || !canEdit}
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
                    canvasWriteBusy ||
                    !canEdit ||
                    workflowSubmitting ||
                    resumingWorkflow
                  }
                  title={t("video.runFullCanvas")}
                >
                  {workflowSubmitting ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Play size={14} />
                  )}
                  {workflowSubmitting
                    ? workflowSubmissionLabel ?? t("video.running")
                    : canResumeCurrentCanvasRun
                      ? t("video.canvas.workflow.resume")
                      : t("video.runCanvas")}
                </BookButton>
              )}
            </>
          }
        />
        {loadError && document ? <p role="alert" className="px-4 py-2 text-sm text-error">{loadError}</p> : null}
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
              <div role="alert" className="p-6 text-sm text-error">{loadError}</div>
            ) : (
              <>
                <ReactFlow
                  nodes={flowNodes}
                  edges={flowEdges}
                  nodeTypes={STUDIO_NODE_TYPES}
                  onNodesChange={onNodesChange}
                  onEdgesChange={onEdgesChange}
                  onConnect={onConnect}
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
                  minZoom={0.35}
                  maxZoom={1.4}
                  deleteKeyCode={
                    canvasMutationDisabled ? null : ["Delete", "Backspace"]
                  }
                  className="a4n-studio-flow"
                >
                  {!isMobileLayout ? (
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
                  <FlowControls />
                  <Background
                    color="rgb(var(--color-text-tertiary) / 0.24)"
                    gap={32}
                    size={1}
                  />
                </ReactFlow>
                {nodes.length === 0 ? (
                  <CanvasEmptyState
                    disabled={!canEdit}
                    mobile={isMobileLayout}
                    onOpenNodeLibrary={() => setMobileNodeLibraryOpen(true)}
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
              onClose={() => setPreviewDialogOpen(false)}
            />
          </Suspense>
        ) : null}
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
        >
          <DomainAgentConsole
            capability="canvas"
            projectId={projectId}
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
