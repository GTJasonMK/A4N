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
  Controls,
  MiniMap,
  ReactFlow,
  addEdge,
  useEdgesState,
  useNodesState,
  type Connection,
  type EdgeChange,
  type IsValidConnection,
  type NodeChange,
  type ReactFlowInstance,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Eye,
  Film,
  ListPlus,
  Play,
  Save,
  SlidersHorizontal,
  Trash2,
  Upload, } from "lucide-react";
import { LoadingSpinner } from "../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../api";
import { listLlmConfig } from "../api/config/llm";
import type { ModelRequestPreview } from "../api/requestPreview";
import {
  getCanvasGraph,
  getCanvasNodeContracts,
  importCanvasMedia,
  previewCanvasNodeRequest,
  runCanvasGraph,
  runCanvasGraphToNode,
  runCanvasNode,
  saveCanvasGraph,
  type CanvasGraphDocument,
} from "../api/canvas";
import { listImageConfigs } from "../api/image";
import { getProject, type NovelProject } from "../api/project";
import {
  cancelWorkflow,
  getWorkflowRunDetail,
  getWorkflowRunProgressEvents,
  listWorkflowRuns,
  resumeWorkflowRun,
  type WorkflowRunDetailSummary,
  type WorkflowRunSummary,
} from "../api/streaming";
import { listVideoConfigs } from "../api/video";
import type { ProgressEventRecord } from "../api/progress";
import { confirmDialog } from "../components/feedback/ConfirmDialog";
import { useToast } from "../components/feedback/Toast";
import { BookButton, BookIconButton } from "../components/ui/BookButton";
import { BackButton } from "../components/ui/BackButton";
import { Dropdown, type DropdownItem } from "../components/ui/Dropdown";
import { Modal } from "../components/ui/Modal";
import { PageHeader } from "../components/shell/PageHeader";
import {
  useContextMenu,
  type ContextMenuEntry,
} from "../components/ui/ContextMenu";
import { pickLocalFile, readFileAsBase64 } from "../utils/fileUpload";
import { CanvasEmptyState } from "./video-studio/CanvasEmptyState";
import { CanvasWorkflowPanel } from "./video-studio/CanvasWorkflowPanel";
import {
  STUDIO_NODE_TYPES,
  buildNodeTemplateGroups,
  edgeRoleLabel,
  nodeDisplayTitle,
  nodeKindMeta,
  stateLabel,
} from "./video-studio/StudioCanvasNode";
import { defaultRefTypeForNode } from "./video-studio/canvasNodeRefs";
import {
  nextAssembleClipOrder,
  reorderAssembleClipEdges,
} from "./video-studio/canvasAssemblyEdges";
import {
  EMPTY_CONFIG_OPTIONS,
  buildStudioConfigOptions,
  type StudioConfigOptionsByKind,
} from "./video-studio/canvasConfig";
import {
  NODE_DRAG_KIND,
  NODE_DRAG_LABEL,
  setNodeTemplateDragImage,
} from "./video-studio/canvasDragPreview";
import {
  canvasGraphSaveValidationError,
  validateCanvasConnection,
} from "./video-studio/canvasEdgeContract";
import {
  canvasNodePosition,
  defaultConfigForNode,
  mergeCanvasDocumentRuntimeResults,
  mergeStudioNodeRuntimeResults,
  numericId,
  persistCanvasTextAssets,
  preserveSelectedItems,
  toCanvasGraphInput,
  toFlowEdge,
  toFlowNode,
} from "./video-studio/canvasGraphMapping";
import {
  importedAudioInputNode,
  importedImageInputNode,
  importedVideoInputNode,
} from "./video-studio/canvasImportNodes";
import {
  CanvasSidebarResizeHandle,
  useVideoCanvasSidebarLayout,
} from "./video-studio/canvasLayout";
import { nodeHasProviderRequest } from "./video-studio/canvasNodeKinds";
import {
  canvasWorkflowRuntimeRefreshSignature,
  canvasWorkflowNodeRunStates,
  computeRunToNodeScope,
  computeWorkflowActiveScope,
  edgeWithRuntimeClasses,
  nodeWithRunScopeClass,
  nodeWithWorkflowRunClass,
  parseWorkflowTimestampMs,
  runToNodeConfirmationMessage,
} from "./video-studio/canvasWorkflowState";
import {
  StudioNodeContractsProvider,
  normalizeStudioNodeContracts,
  studioNodeContract,
  studioNodeContractMap,
  studioNodeInputPortsForData,
  type StudioNodeContract,
} from "./video-studio/nodeContracts";
import type { StudioEdge, StudioNode } from "./video-studio/types";
import { formatNumber, useI18n } from "../i18n";

const CANVAS_WORKFLOW_KIND = "video_canvas_graph";

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
  const contextMenu = useContextMenu();
  const projectId = Number(id);
  const [project, setProject] = useState<NovelProject | null>(null);
  const [document, setDocument] = useState<CanvasGraphDocument | null>(null);
  const [nodeContracts, setNodeContracts] = useState<StudioNodeContract[]>([]);
  const [nodes, setNodes, applyNodesChange] = useNodesState<StudioNode>([]);
  const [edges, setEdges, applyEdgesChange] = useEdgesState<StudioEdge>([]);
  const [loading, setLoading] = useState(true);
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
      edges.map((edge) =>
        edgeWithRuntimeClasses(edge, selectedRunScope, workflowActiveScope),
      ),
    [edges, selectedRunScope, workflowActiveScope],
  );
  const workflowSubmitting = workflowSubmissionLabel !== null;
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
    if (isMobileLayout) return;
    setMobileNodeLibraryOpen(false);
    setMobileInspectorOpen(false);
  }, [isMobileLayout]);

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
      addToast(t("video.projectInvalid"), "error");
      setLoading(false);
      return;
    }
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
        addToast(t("video.notVideoProject"), "error");
        navigate("/", { replace: true });
        return;
      }
      const normalizedContracts =
        normalizeStudioNodeContracts(rawNodeContracts);
      if (normalizedContracts.length === 0) {
        throw new Error(t("video.emptyNodeContract"));
      }
      const loadedContractsByKind = studioNodeContractMap(normalizedContracts);
      const flowNodes = graphDocument.nodes.map(toFlowNode);
      const flowEdges = graphDocument.edges.map((edge) => toFlowEdge(edge, t));
      const loadedEdgeError = canvasGraphSaveValidationError(
        flowNodes,
        flowEdges,
        loadedContractsByKind,
        t,
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
        addToast(extractErrorMessage(err, t("video.canvasLoadFailed")), "error");
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
    t,
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

  const onNodesChange = useCallback(
    (changes: NodeChange<StudioNode>[]) => {
      const allowedChanges = canvasWriteBusy
        ? changes.filter(
            (change) =>
              change.type === "select" || change.type === "dimensions",
          )
        : changes;
      if (
        allowedChanges.some(
          (change) => change.type !== "select" && change.type !== "dimensions",
        )
      ) {
        setDirty(true);
      }
      if (allowedChanges.length > 0) applyNodesChange(allowedChanges);
    },
    [applyNodesChange, canvasWriteBusy],
  );

  const onEdgesChange = useCallback(
    (changes: EdgeChange<StudioEdge>[]) => {
      const allowedChanges = canvasWriteBusy
        ? changes.filter((change) => change.type === "select")
        : changes;
      if (allowedChanges.some((change) => change.type !== "select")) {
        setDirty(true);
      }
      if (allowedChanges.length > 0) applyEdgesChange(allowedChanges);
    },
    [applyEdgesChange, canvasWriteBusy],
  );

  const addNode = useCallback(
    (kind: string, label: string, position?: { x: number; y: number }) => {
      if (canvasMutationDisabled) return;
      const id = `node-${Date.now()}-${Math.round(Math.random() * 10000)}`;
      setNodes((items) => [
        ...items.map((item) => ({ ...item, selected: false })),
        {
          id,
          type: "studioNode",
          selected: true,
          position: position ?? canvasNodePosition(items.length),
          data: {
            label,
            nodeKind: kind,
            refType: defaultRefTypeForNode(kind),
            refId: null,
            state: "ready",
            config: defaultConfigForNode(kind),
          },
        },
      ]);
      setDirty(true);
    },
    [canvasMutationDisabled, setNodes],
  );

  const addNodeToVisibleCanvas = useCallback(
    (kind: string, label: string) => {
      const rect = canvasSectionRef.current?.getBoundingClientRect();
      const position =
        flowInstance && rect
          ? flowInstance.screenToFlowPosition({
              x: rect.left + rect.width / 2,
              y: rect.top + rect.height / 2,
            })
          : undefined;
      addNode(kind, label, position);
      setMobileNodeLibraryOpen(false);
    },
    [addNode, flowInstance],
  );

  const onNodeTemplateDragStart = useCallback(
    (
      event: React.DragEvent<HTMLButtonElement>,
      kind: string,
      label: string,
    ) => {
      if (canvasMutationDisabled) {
        event.preventDefault();
        return;
      }
      event.dataTransfer.effectAllowed = "copy";
      event.dataTransfer.setData(NODE_DRAG_KIND, kind);
      event.dataTransfer.setData(NODE_DRAG_LABEL, label);
      setNodeTemplateDragImage(
        event.dataTransfer,
        kind,
        label,
        flowInstance?.getZoom() ?? 1,
        studioNodeContract(nodeContractsByKind, kind),
        t,
      );
    },
    [canvasMutationDisabled, flowInstance, nodeContractsByKind, t],
  );

  const onCanvasDragOver = useCallback(
    (event: React.DragEvent) => {
      if (
        canvasMutationDisabled ||
        !Array.from(event.dataTransfer.types).includes(NODE_DRAG_KIND)
      )
        return;
      event.preventDefault();
      event.dataTransfer.dropEffect = "copy";
    },
    [canvasMutationDisabled],
  );

  const onCanvasDrop = useCallback(
    (event: React.DragEvent) => {
      if (canvasMutationDisabled || !flowInstance) return;
      const kind = event.dataTransfer.getData(NODE_DRAG_KIND);
      const contract = kind
        ? studioNodeContract(nodeContractsByKind, kind)
        : null;
      if (!kind || !contract) return;
      event.preventDefault();
      const label =
        event.dataTransfer.getData(NODE_DRAG_LABEL) ||
        contract.label ||
        nodeKindMeta(kind, t).label;
      addNode(
        kind,
        label,
        flowInstance.screenToFlowPosition({
          x: event.clientX,
          y: event.clientY,
        }),
      );
    },
    [addNode, canvasMutationDisabled, flowInstance, nodeContractsByKind, t],
  );

  const onConnect = useCallback(
    (connection: Connection) => {
      if (canvasMutationDisabled) return;
      const validation = validateCanvasConnection(
        connection,
        nodes,
        edges,
        nodeContractsByKind,
        t,
      );
      if (typeof validation === "string") {
        addToast(validation, "error");
        return;
      }
      const contract = validation;
      const config =
        contract.role === "assembly_clip" && connection.target
          ? { order_index: nextAssembleClipOrder(edges, connection.target) }
          : {};
      setEdges((items) =>
        addEdge(
          {
            ...connection,
            sourceHandle: contract.sourcePort,
            targetHandle: contract.targetPort,
            type: "smoothstep",
            animated: false,
            label: edgeRoleLabel(contract.role, t),
            className: "a4n-studio-edge",
            labelBgPadding: [6, 3],
            labelBgBorderRadius: 4,
            data: { role: contract.role, config },
          },
          items,
        ),
      );
      setDirty(true);
    },
    [
      addToast,
      canvasMutationDisabled,
      edges,
      nodeContractsByKind,
      nodes,
      setEdges,
      t,
    ],
  );

  const isValidCanvasConnection = useCallback<IsValidConnection>(
    (connection) =>
      !canvasMutationDisabled &&
      typeof validateCanvasConnection(connection, nodes, edges, nodeContractsByKind, t) !== "string",
    [canvasMutationDisabled, edges, nodeContractsByKind, nodes, t],
  );

  const selectCanvasNode = useCallback(
    (nodeId: string) => {
      setNodes((items) =>
        items.map((node) => ({ ...node, selected: node.id === nodeId })),
      );
      setEdges((items) => items.map((edge) => ({ ...edge, selected: false })));
    },
    [setEdges, setNodes],
  );

  const selectCanvasEdge = useCallback(
    (edgeId: string) => {
      setNodes((items) => items.map((node) => ({ ...node, selected: false })));
      setEdges((items) =>
        items.map((edge) => ({ ...edge, selected: edge.id === edgeId })),
      );
    },
    [setEdges, setNodes],
  );

  const deleteCanvasNode = useCallback(
    (node: StudioNode) => {
      if (canvasMutationDisabled) return;
      setNodes((items) => items.filter((item) => item.id !== node.id));
      setEdges((items) =>
        items.filter(
          (edge) => edge.source !== node.id && edge.target !== node.id,
        ),
      );
      setDirty(true);
      addToast(t("video.deletedNode", { name: nodeDisplayTitle(node.data, t) }), "success");
    },
    [addToast, canvasMutationDisabled, setEdges, setNodes, t],
  );

  const deleteCanvasEdge = useCallback(
    (edge: StudioEdge) => {
      if (canvasMutationDisabled) return;
      setEdges((items) => items.filter((item) => item.id !== edge.id));
      setDirty(true);
      addToast(t("video.deletedEdge"), "success");
    },
    [addToast, canvasMutationDisabled, setEdges, t],
  );

  const updateSelectedConfig = (patch: Record<string, unknown>) => {
    if (!selectedNode || canvasMutationDisabled) return;
    const nextConfig = {
      ...selectedNode.data.config,
      ...patch,
    };
    for (const [key, value] of Object.entries(patch)) {
      if (value === undefined) delete nextConfig[key];
    }
    const nextLabel =
      typeof patch.title === "string"
        ? patch.title.trim() || nodeKindMeta(selectedNode.data.nodeKind, t).label
        : selectedNode.data.label;
    const nextData = {
      ...selectedNode.data,
      label: nextLabel,
      config: nextConfig,
    };
    setNodes((items) =>
      items.map((node) =>
        node.id === selectedNode.id
          ? {
              ...node,
              data: nextData,
            }
          : node,
      ),
    );
    if (
      selectedNode.data.nodeKind === "GenerateVideoNode" &&
      Object.prototype.hasOwnProperty.call(patch, "input_mode")
    ) {
      const contract = studioNodeContract(
        nodeContractsByKind,
        selectedNode.data.nodeKind,
      );
      const activeInputIds = new Set(
        contract
          ? studioNodeInputPortsForData(nextData, contract).map(
              (port) => port.id,
            )
          : [],
      );
      setEdges((items) =>
        items.filter(
          (edge) =>
            edge.target !== selectedNode.id ||
            activeInputIds.has(edge.targetHandle ?? ""),
        ),
      );
    }
    setDirty(true);
  };

  const updateSelectedRef = (refType: string | null, refId: number | null) => {
    if (!selectedNode || canvasMutationDisabled) return;
    setNodes((items) =>
      items.map((node) =>
        node.id === selectedNode.id
          ? {
              ...node,
              data: {
                ...node.data,
                refType,
                refId,
              },
            }
          : node,
      ),
    );
    setDirty(true);
  };

  const moveAssembleClip = (edgeId: string, direction: -1 | 1) => {
    if (canvasMutationDisabled) return;
    setEdges((items) => reorderAssembleClipEdges(items, edgeId, direction));
    setDirty(true);
  };

  const persist = async () => {
    if (!document || canvasWriteBusy || !canEdit) return;
    const validationError = canvasGraphSaveValidationError(
      nodes,
      edges,
      nodeContractsByKind,
      t,
    );
    if (validationError) {
      addToast(validationError, "error");
      return;
    }
    setSaving(true);
    try {
      const nodesWithTextAssets = await persistCanvasTextAssets(
        projectId,
        nodes,
        t,
      );
      const saved = await saveCanvasGraph(
        projectId,
        toCanvasGraphInput(
          document,
          nodesWithTextAssets,
          edges,
        ),
      );
      const savedEdges = saved.edges.map((edge) => toFlowEdge(edge, t));
      setDocument(saved);
      setNodes((current) =>
        preserveSelectedItems(saved.nodes.map(toFlowNode), current),
      );
      setEdges((current) => preserveSelectedItems(savedEdges, current));
      setDirty(false);
      addToast(t("video.canvasSaved"), "success");
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.canvasSaveFailed")), "error");
    } finally {
      setSaving(false);
    }
  };

  const importMedia = async (mediaKind: "image" | "video" | "audio") => {
    if (!document || canvasWriteBusy || !canEdit) return;
    if (mediaKind === "audio" && project?.source_mode !== "freeform") {
      addToast(t("video.freeProjectAudioOnly"), "error");
      return;
    }
    const validationError = canvasGraphSaveValidationError(
      nodes,
      edges,
      nodeContractsByKind,
      t,
    );
    if (validationError) {
      addToast(validationError, "error");
      return;
    }
    const accept =
      mediaKind === "image"
        ? "image/*"
        : mediaKind === "video"
          ? "video/*"
          : "audio/*";
    const file = await pickLocalFile(accept);
    if (!file) return;
    setImporting(true);
    try {
      const imported = await importCanvasMedia({
        project_id: projectId,
        media_kind: mediaKind,
        file_name: file.name,
        mime_type: file.type,
        data_base64: await readFileAsBase64(file),
      });
      let importedNode: StudioNode;
      let successMessage: string;
      switch (imported.media_kind) {
        case "image":
          importedNode = importedImageInputNode(
            imported.image,
            imported.asset,
            nodes.length,
            t,
          );
          successMessage = t("video.importedImage");
          break;
        case "video":
          importedNode = importedVideoInputNode(
            imported.video,
            imported.asset,
            nodes.length,
            t,
          );
          successMessage = t("video.importedVideo");
          break;
        case "audio":
          importedNode = importedAudioInputNode(imported.asset, nodes.length, t);
          successMessage = t("video.importedAudio");
          break;
        default:
          throw new Error(t("video.unsupportedImportedMedia"));
      }
      const nextNodes = [
        ...nodes.map((node) => ({ ...node, selected: false })),
        importedNode,
      ];
      const saved = await saveCanvasGraph(
        projectId,
        toCanvasGraphInput(document, nextNodes, edges),
      );
      const savedEdges = saved.edges.map((edge) => toFlowEdge(edge, t));
      setDocument(saved);
      setNodes((current) =>
        preserveSelectedItems(saved.nodes.map(toFlowNode), current),
      );
      setEdges((current) => preserveSelectedItems(savedEdges, current));
      setDirty(false);
      addToast(successMessage, "success");
    } catch (err) {
      addToast(
        extractErrorMessage(
          err,
          mediaKind === "image"
            ? t("video.importImageFailed")
            : mediaKind === "video"
              ? t("video.importVideoFailed")
              : t("video.importAudioFailed"),
        ),
        "error",
      );
    } finally {
      setImporting(false);
    }
    };

  const refreshCanvasRuntimeResults = useCallback(
    async (graphId: number) => {
      const freshDocument = await getCanvasGraph(projectId, graphId);
      setDocument((current) =>
        current && current.graph.id === freshDocument.graph.id
          ? mergeCanvasDocumentRuntimeResults(current, freshDocument)
          : current,
      );
      setNodes((current) =>
        mergeStudioNodeRuntimeResults(current, freshDocument.nodes),
      );
    },
    [projectId, setNodes],
  );

  const loadWorkflowStatus = useCallback(
    async (workflowRunId: number, options: { silent?: boolean } = {}) => {
      setLatestWorkflowRunId(workflowRunId);
      setWorkflowLoading(true);
      try {
        const [detail, events] = await Promise.all([
          getWorkflowRunDetail(workflowRunId, true),
          getWorkflowRunProgressEvents(workflowRunId, 20),
        ]);
        setWorkflowDetail(detail);
        setWorkflowEvents(events);
        const refreshSignature = canvasWorkflowRuntimeRefreshSignature(detail);
        const graphId =
          detail.run.scope_table === "canvas_graph"
            ? detail.run.scope_id
            : null;
        if (
          refreshSignature &&
          graphId &&
          workflowRuntimeRefreshSignatureRef.current !== refreshSignature
        ) {
          workflowRuntimeRefreshSignatureRef.current = refreshSignature;
          try {
            await refreshCanvasRuntimeResults(graphId);
          } catch (err) {
            if (!options.silent) {
              addToast(
                extractErrorMessage(err, t("video.syncCanvasRuntimeFailed")),
                "error",
              );
            }
          }
        }
      } catch (err) {
        if (!options.silent) {
          addToast(extractErrorMessage(err, t("video.loadCanvasRunsFailed")), "error");
        }
      } finally {
        setWorkflowLoading(false);
      }
    },
    [addToast, refreshCanvasRuntimeResults, t],
  );

  const stopWorkflowDiscovery = useCallback(() => {
    if (workflowDiscoveryTimerRef.current !== null) {
      window.clearInterval(workflowDiscoveryTimerRef.current);
      workflowDiscoveryTimerRef.current = null;
    }
    workflowDiscoveryInFlightRef.current = false;
  }, []);

  const findActiveCanvasWorkflowRun = useCallback(
    async (
      graphId: number,
      submittedAtMs: number,
    ): Promise<WorkflowRunSummary | null> => {
      const runs = await listWorkflowRuns({
        project_id: projectId,
        workflow_kind: CANVAS_WORKFLOW_KIND,
        resumable_only: false,
        limit: 50,
      });
      const submittedFloorMs = submittedAtMs - 5000;
      return (
        runs.find((run) => {
          if (run.scope_table !== "canvas_graph" || run.scope_id !== graphId)
            return false;
          if (run.status === "running" || run.status === "pending") return true;
          const updatedAtMs = parseWorkflowTimestampMs(run.updated_at);
          const createdAtMs = parseWorkflowTimestampMs(run.created_at);
          return (
            (updatedAtMs !== null && updatedAtMs >= submittedFloorMs) ||
            (createdAtMs !== null && createdAtMs >= submittedFloorMs)
          );
        }) ?? null
      );
    },
    [projectId],
  );

  const startWorkflowDiscovery = useCallback(
    (graphId: number, submittedAtMs: number) => {
      stopWorkflowDiscovery();
      const tick = async () => {
        if (workflowDiscoveryInFlightRef.current) return;
        workflowDiscoveryInFlightRef.current = true;
        try {
          const run = await findActiveCanvasWorkflowRun(graphId, submittedAtMs);
          if (!run) return;
          await loadWorkflowStatus(run.id, { silent: true });
          stopWorkflowDiscovery();
        } catch (_err) {
          // Discovery is best-effort while the run submission request is still in flight.
        } finally {
          workflowDiscoveryInFlightRef.current = false;
        }
      };
      workflowDiscoveryTimerRef.current = window.setInterval(() => {
        void tick();
      }, 700);
      void tick();
    },
    [findActiveCanvasWorkflowRun, loadWorkflowStatus, stopWorkflowDiscovery],
  );

  useEffect(() => stopWorkflowDiscovery, [stopWorkflowDiscovery]);

  useEffect(() => {
    if (!latestWorkflowRunId) return;
    const status = workflowDetail?.run.status;
    if (status !== "running" && status !== "pending") return;
    const timer = window.setInterval(() => {
      void loadWorkflowStatus(latestWorkflowRunId, { silent: true });
    }, 1000);
    return () => window.clearInterval(timer);
  }, [latestWorkflowRunId, loadWorkflowStatus, workflowDetail?.run.status]);

  const previewNode = async (node: StudioNode) => {
    if (!nodeHasProviderRequest(node)) {
      setContentPreviewNode(node);
      return;
    }
    if (!document) return;
    if (dirty) {
      addToast(t("video.saveBeforePreview"), "info");
      return;
    }
    const nodeId = numericId(node.id);
    if (!nodeId) {
      addToast(t("video.saveNodeBeforeRequest"), "info");
      return;
    }
    setPreviewDialogOpen(true);
    setPreviewLoading(true);
    setPreviewData(null);
    try {
      const preview = await previewCanvasNodeRequest(
        projectId,
        document.graph.id,
        nodeId,
      );
      setPreviewData(preview);
    } catch (err) {
      setPreviewDialogOpen(false);
      addToast(extractErrorMessage(err, t("video.previewRequestFailed")), "error");
    } finally {
      setPreviewLoading(false);
    }
  };

  const runNode = async (node: StudioNode) => {
    if (!document || canvasWriteBusy || !canEdit || workflowSubmitting) return;
    if (dirty) {
      addToast(t("video.saveBeforeRun"), "info");
      return;
    }
    const contract = studioNodeContract(
      nodeContractsByKind,
      node.data.nodeKind,
    );
    if (!contract?.runnable) {
      addToast(t("video.inputNodeCannotRun"), "info");
      return;
    }
    const nodeId = numericId(node.id);
    if (!nodeId) {
      addToast(t("video.saveNodeBeforeRun"), "info");
      return;
    }
    const confirmation = [
      t("video.runNodeMessage"),
      "",
      t("video.runNodeTarget", { name: nodeDisplayTitle(node.data, t) }),
      t("video.runNodeConfirmDescription"),
      t("video.runNodeInputHint"),
    ].join("\n");
    const confirmed = await confirmDialog({
      title: t("video.runNodeTitle"),
      message: confirmation,
      confirmText: t("video.run"),
      cancelText: t("common.cancel"),
      dialogType: "warning",
    });
    if (!confirmed) {
      return;
    }
    setWorkflowSubmissionLabel(t("video.runNodeSubmitting", { name: nodeDisplayTitle(node.data, t) }));
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowRuntimeRefreshSignatureRef.current = null;
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, Date.now());
    try {
      const result = await runCanvasNode(projectId, graphId, nodeId);
      addToast(
        result.error_message ?? t("video.runOnlyNodeSubmitted"),
        result.status === "failed" ? "info" : "success",
      );
      await loadWorkflowStatus(result.workflow_run_id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.runOnlyNodeError")), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const runToNode = async (node: StudioNode) => {
    if (!document || canvasWriteBusy || !canEdit || workflowSubmitting) return;
    if (dirty) {
      addToast(t("video.saveBeforeRun"), "info");
      return;
    }
    if (
      studioNodeContract(nodeContractsByKind, node.data.nodeKind)?.category !==
      "output"
    ) {
      addToast(
        t("video.runToOutputOnly"),
        "info",
      );
      return;
    }
    const nodeId = numericId(node.id);
    if (!nodeId) {
      addToast(t("video.saveNodeBeforeRun"), "info");
      return;
    }
    const scope = computeRunToNodeScope(
      node,
      nodes,
      edges,
      nodeContractsByKind,
    );
    const confirmation = runToNodeConfirmationMessage(scope, node, t);
    const confirmed = await confirmDialog({
      title: t("video.runToNodeTitle"),
      message: confirmation,
      confirmText: t("video.run"),
      cancelText: t("common.cancel"),
      dialogType: "warning",
    });
    if (!confirmed) {
      return;
    }
    setWorkflowSubmissionLabel(t("video.runToNodeSubmitting", { name: nodeDisplayTitle(node.data, t) }));
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowRuntimeRefreshSignatureRef.current = null;
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, Date.now());
    try {
      const result = await runCanvasGraphToNode(projectId, graphId, nodeId);
      addToast(
        result.error_message ?? t("video.runToNodeSubmitted"),
        result.status === "failed" ? "info" : "success",
      );
      await loadWorkflowStatus(result.workflow_run_id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.runToNodeError")), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const runGraph = async () => {
    if (!document || canvasWriteBusy || !canEdit || workflowSubmitting) return;
    if (dirty) {
      addToast(t("video.saveBeforeRun"), "info");
      return;
    }
    setWorkflowSubmissionLabel(t("video.runCanvasSubmitting"));
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowRuntimeRefreshSignatureRef.current = null;
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, Date.now());
    try {
      const result = await runCanvasGraph(projectId, graphId);
      addToast(
        result.error_message ?? t("video.submittedRunCanvas"),
        result.status === "failed" ? "info" : "success",
      );
      await loadWorkflowStatus(result.workflow_run_id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.runCanvasError")), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const nodeContextMenuItems = useCallback(
    (node: StudioNode): ContextMenuEntry[] => {
      const contract = studioNodeContract(
        nodeContractsByKind,
        node.data.nodeKind,
      );
      const isOutputNode = contract?.category === "output";
      const isRunnableNode = Boolean(contract?.runnable);
      const hasProviderRequest = nodeHasProviderRequest(node);
      const items: ContextMenuEntry[] = [];
      if (!hasProviderRequest || document) {
        items.push({
          label: hasProviderRequest ? t("video.viewModelRequest") : t("video.viewNodeContent"),
          icon: <Eye size={13} />,
          onClick: () => void previewNode(node),
        });
      }
      if (isRunnableNode) {
        items.push({
          label: t("video.runOnlyNode"),
          icon: <Play size={13} />,
          disabled: canvasMutationDisabled || workflowSubmitting,
          onClick: () => void runNode(node),
        });
      }
      if (isOutputNode) {
        items.push({
          label: t("video.runToNode"),
          icon: <Play size={13} />,
          disabled: canvasMutationDisabled || workflowSubmitting,
          onClick: () => void runToNode(node),
        });
      }
      if (!canvasMutationDisabled) {
        if (items.length > 0) items.push({ divider: true });
        items.push({
          label: t("video.deleteNode"),
          icon: <Trash2 size={13} />,
          danger: true,
          shortcut: "Del",
          onClick: () => deleteCanvasNode(node),
        });
      }
      return items;
    },
    [
      canvasMutationDisabled,
      deleteCanvasNode,
      document,
      nodeContractsByKind,
      previewNode,
      runNode,
      runToNode,
      workflowSubmitting,
      t,
    ],
  );

  const edgeContextMenuItems = useCallback(
    (edge: StudioEdge): ContextMenuEntry[] =>
      !canvasMutationDisabled
        ? [
            {
              label: t("video.deleteEdge"),
              icon: <Trash2 size={13} />,
              danger: true,
              shortcut: "Del",
              onClick: () => deleteCanvasEdge(edge),
            },
          ]
        : [],
    [canvasMutationDisabled, deleteCanvasEdge, t],
  );

  const mediaImportMenuItems = useCallback(
    (): ContextMenuEntry[] => [
      {
        label: t("video.importImage"),
        icon: <Upload size={13} />,
        disabled: !document || canvasWriteBusy || !canEdit,
        onClick: () => void importMedia("image"),
      },
      {
        label: t("video.importVideo"),
        icon: <Upload size={13} />,
        disabled: !document || canvasWriteBusy || !canEdit,
        onClick: () => void importMedia("video"),
      },
      {
        label: t("video.importAudio"),
        icon: <Upload size={13} />,
        disabled:
          !document ||
          canvasWriteBusy ||
          !canEdit ||
          project?.source_mode !== "freeform",
        onClick: () => void importMedia("audio"),
      },
    ],
    [
      canEdit,
      document,
      importMedia,
      canvasWriteBusy,
      project?.source_mode,
      t,
    ],
  );

  const paneContextMenuItems = useCallback(
    (): ContextMenuEntry[] => [
      {
        label: t("video.runCanvasTitle"),
        icon: <Play size={13} />,
        disabled:
          !document || canvasWriteBusy || !canEdit || workflowSubmitting,
        onClick: () => void runGraph(),
      },
      { divider: true },
      ...mediaImportMenuItems(),
    ],
    [
      canEdit,
      canvasWriteBusy,
      document,
      mediaImportMenuItems,
      runGraph,
      workflowSubmitting,
      t,
    ],
  );

  const onNodeContextMenu = useCallback(
    (event: React.MouseEvent, node: StudioNode) => {
      event.preventDefault();
      event.stopPropagation();
      selectCanvasNode(node.id);
      contextMenu.open(
        event.clientX,
        event.clientY,
        nodeContextMenuItems(node),
      );
    },
    [contextMenu, nodeContextMenuItems, selectCanvasNode],
  );

  const onEdgeContextMenu = useCallback(
    (event: React.MouseEvent, edge: StudioEdge) => {
      event.preventDefault();
      event.stopPropagation();
      selectCanvasEdge(edge.id);
      contextMenu.open(
        event.clientX,
        event.clientY,
        edgeContextMenuItems(edge),
      );
    },
    [contextMenu, edgeContextMenuItems, selectCanvasEdge],
  );

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
      contextMenu.open(event.clientX, event.clientY, paneContextMenuItems());
    },
    [contextMenu, paneContextMenuItems],
  );

  const cancelActiveWorkflow = async () => {
    const taskId = workflowDetail?.run.active_task_id;
    const workflowRunId = workflowDetail?.run.id ?? latestWorkflowRunId;
    if ((!taskId && !workflowRunId) || cancellingWorkflow) return;
    setCancellingWorkflow(true);
    try {
      await cancelWorkflow({
        task_id: taskId,
        workflow_run_id: workflowRunId,
      });
      addToast(t("video.cancelRunRequested"), "info");
      if (workflowRunId) {
        await loadWorkflowStatus(workflowRunId);
      }
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.cancelRunFailed")), "error");
    } finally {
      setCancellingWorkflow(false);
    }
  };

  const resumeActiveWorkflow = async () => {
    const workflowRunId = workflowDetail?.run.id ?? latestWorkflowRunId;
    if (!workflowRunId || resumingWorkflow) return;
    setResumingWorkflow(true);
    try {
      const run = await resumeWorkflowRun(workflowRunId);
      addToast(t("video.resumeRunSuccess"), "success");
      workflowRuntimeRefreshSignatureRef.current = null;
      await loadWorkflowStatus(run.id);
    } catch (err) {
      addToast(extractErrorMessage(err, t("video.resumeRunFailed")), "error");
    } finally {
      setResumingWorkflow(false);
    }
  };

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
                  onClick={
                    mobile ? () => addNodeToVisibleCanvas(kind, label) : undefined
                  }
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
            <EdgeInspector edge={selectedEdge} />
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
              {isMobileLayout ? (
                <>
                  <BookIconButton
                    label={t("video.openNodeLibrary")}
                    onClick={() => setMobileNodeLibraryOpen(true)}
                    disabled={canvasMutationDisabled}
                    variant="secondary"
                    size="sm"
                  >
                    <ListPlus size={14} />
                  </BookIconButton>
                  <BookIconButton
                    label={t("video.openProperties")}
                    onClick={() => setMobileInspectorOpen(true)}
                    variant="secondary"
                    size="sm"
                  >
                    <SlidersHorizontal size={14} />
                  </BookIconButton>
                </>
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
                    workflowSubmitting
                  }
                  title={t("video.runFullCanvas")}
                >
                  {workflowSubmitting ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Play size={14} />
                  )}
                  {workflowSubmitting ? workflowSubmissionLabel ?? t("video.running") : t("video.runCanvas")}
                </BookButton>
              )}
            </>
          }
        />
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
                  fitView
                  fitViewOptions={{ padding: 0.25, maxZoom: 1 }}
                  defaultViewport={{ x: 0, y: 0, zoom: 1 }}
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
                  <Controls className="a4n-flow-controls" />
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
        {isMobileLayout ? (
          <>
            <Modal
              isOpen={mobileNodeLibraryOpen}
              onClose={() => setMobileNodeLibraryOpen(false)}
              title={t("video.nodeLibrary")}
              maxWidthClassName="max-w-lg"
              bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar p-3"
            >
              {renderNodeLibraryPanel(true)}
            </Modal>
            <Modal
              isOpen={mobileInspectorOpen}
              onClose={() => setMobileInspectorOpen(false)}
              title={selectedEdge ? t("video.edgeProperties") : t("video.nodeProperties")}
              maxWidthClassName="max-w-xl"
              bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar p-3"
            >
              {renderInspectorPanel(true)}
            </Modal>
          </>
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
