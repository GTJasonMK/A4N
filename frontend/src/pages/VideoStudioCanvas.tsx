import React, {
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
  type CanvasNodeRequestPreview,
} from "../api/canvas";
import { listImageConfigs } from "../api/image";
import { getProject, type NovelProject } from "../api/project";
import {
  cancelWorkflow,
  getWorkflowRunDetail,
  getWorkflowRunProgressEvents,
  listWorkflowRuns,
  resumeWorkflowRun,
  type WorkflowRunDetail,
  type WorkflowRun,
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
import { CanvasNodeContentPreviewDialog } from "./video-studio/CanvasNodeContentPreviewDialog";
import {
  EdgeInspector,
  NodeInspector,
} from "./video-studio/CanvasNodeInspector";
import { CanvasNodePreviewDialog } from "./video-studio/CanvasNodePreviewDialog";
import { CanvasWorkflowPanel } from "./video-studio/CanvasWorkflowPanel";
import {
  STUDIO_NODE_TYPES,
  buildNodeTemplateGroups,
  defaultRefTypeForNode,
  edgeRoleLabel,
  nodeDisplayTitle,
  nodeKindMeta,
  stateLabel,
} from "./video-studio/StudioCanvasNode";
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
  canvasConnectionError,
  canvasGraphSaveValidationError,
  inferCanvasEdgeContract,
} from "./video-studio/canvasEdgeContract";
import {
  canvasNodePosition,
  defaultConfigForNode,
  mergeCanvasDocumentRuntimeResults,
  mergeStudioNodeRuntimeResults,
  migrateLegacyCanvasGraph,
  normalizeLoadedCanvasEdges,
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

const CANVAS_WORKFLOW_KIND = "video_canvas_graph";

export const VideoStudioCanvas: React.FC = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const { addToast } = useToast();
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
    useState<WorkflowRunDetail | null>(null);
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
    useState<CanvasNodeRequestPreview | null>(null);
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
    () => buildNodeTemplateGroups(nodeContracts),
    [nodeContracts],
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
        addToast(extractErrorMessage(err, "刷新画布配置失败"), "error");
      }
    }
  }, [addToast, loadStudioConfigOptions]);

  const load = useCallback(async () => {
    const requestId = requestRef.current + 1;
    requestRef.current = requestId;
    if (!Number.isFinite(projectId) || projectId <= 0) {
      addToast("视频项目参数无效", "error");
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
        addToast("当前项目不是视频项目", "error");
        navigate("/", { replace: true });
        return;
      }
      const normalizedContracts =
        normalizeStudioNodeContracts(rawNodeContracts);
      if (normalizedContracts.length === 0) {
        throw new Error("画布节点合同为空");
      }
      const migratedDocument = migrateLegacyCanvasGraph(
        graphDocument,
        nextConfigOptions,
      );
      const migratedDocumentChanged =
        JSON.stringify({
          nodes: migratedDocument.nodes,
          edges: migratedDocument.edges,
        }) !==
        JSON.stringify({
          nodes: graphDocument.nodes,
          edges: graphDocument.edges,
        });
      const loadedContractsByKind = studioNodeContractMap(normalizedContracts);
      const flowNodes = migratedDocument.nodes.map(toFlowNode);
      const normalizedEdges = normalizeLoadedCanvasEdges(
        flowNodes,
        migratedDocument.edges.map(toFlowEdge),
        loadedContractsByKind,
      );
      setProject(projectRecord);
      setDocument(migratedDocument);
      setNodeContracts(normalizedContracts);
      setConfigOptions(nextConfigOptions);
      setNodes((current) => preserveSelectedItems(flowNodes, current));
      setEdges((current) =>
        preserveSelectedItems(normalizedEdges.edges, current),
      );
      if (
        migratedDocumentChanged ||
        normalizedEdges.dropped > 0 ||
        normalizedEdges.repaired > 0
      ) {
        addToast(
          `已修复旧画布结构，连线修复 ${normalizedEdges.repaired} 条，移除 ${normalizedEdges.dropped} 条，请检查后保存。`,
          normalizedEdges.dropped > 0 ? "info" : "success",
        );
      }
      setDirty(
        migratedDocumentChanged ||
          normalizedEdges.dropped > 0 ||
          normalizedEdges.repaired > 0,
      );
    } catch (err) {
      if (requestRef.current === requestId) {
        addToast(extractErrorMessage(err, "加载视频画布失败"), "error");
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
      );
    },
    [canvasMutationDisabled, flowInstance, nodeContractsByKind],
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
        nodeKindMeta(kind).label;
      addNode(
        kind,
        label,
        flowInstance.screenToFlowPosition({
          x: event.clientX,
          y: event.clientY,
        }),
      );
    },
    [addNode, canvasMutationDisabled, flowInstance, nodeContractsByKind],
  );

  const onConnect = useCallback(
    (connection: Connection) => {
      if (canvasMutationDisabled) return;
      const contract = inferCanvasEdgeContract(
        connection,
        nodes,
        nodeContractsByKind,
        edges,
      );
      if (!contract) {
        addToast("当前节点类型不能直接连接", "error");
        return;
      }
      const connectionError = canvasConnectionError(
        connection,
        nodes,
        edges,
        nodeContractsByKind,
      );
      if (connectionError) {
        addToast(connectionError, "error");
        return;
      }
      const config =
        contract.role === "assembly_clip" && connection.target
          ? { order_index: nextAssembleClipOrder(edges, connection.target) }
          : {};
      setEdges((items) =>
        addEdge(
          {
            ...connection,
            sourceHandle: connection.sourceHandle ?? contract.sourcePort,
            targetHandle: connection.targetHandle ?? contract.targetPort,
            type: "smoothstep",
            animated: false,
            label: edgeRoleLabel(contract.role),
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
    ],
  );

  const isValidCanvasConnection = useCallback<IsValidConnection>(
    (connection) =>
      !canvasMutationDisabled &&
      !canvasConnectionError(connection, nodes, edges, nodeContractsByKind),
    [canvasMutationDisabled, edges, nodeContractsByKind, nodes],
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
      addToast(`已删除节点：${nodeDisplayTitle(node.data)}`, "success");
    },
    [addToast, canvasMutationDisabled, setEdges, setNodes],
  );

  const deleteCanvasEdge = useCallback(
    (edge: StudioEdge) => {
      if (canvasMutationDisabled) return;
      setEdges((items) => items.filter((item) => item.id !== edge.id));
      setDirty(true);
      addToast("已删除连线", "success");
    },
    [addToast, canvasMutationDisabled, setEdges],
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
        ? patch.title.trim() || nodeKindMeta(selectedNode.data.nodeKind).label
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
      );
      const saved = await saveCanvasGraph(
        projectId,
        toCanvasGraphInput(
          document,
          nodesWithTextAssets,
          edges,
          nodeContractsByKind,
        ),
      );
      const savedEdges = saved.edges.map(toFlowEdge);
      setDocument(saved);
      setNodes((current) =>
        preserveSelectedItems(saved.nodes.map(toFlowNode), current),
      );
      setEdges((current) => preserveSelectedItems(savedEdges, current));
      setDirty(false);
      addToast("画布已保存", "success");
    } catch (err) {
      addToast(extractErrorMessage(err, "保存画布失败"), "error");
    } finally {
      setSaving(false);
    }
  };

  const importMedia = async (mediaKind: "image" | "video" | "audio") => {
    if (!document || canvasWriteBusy || !canEdit) return;
    if (mediaKind === "audio" && project?.source_mode !== "freeform") {
      addToast("音频资产导入只支持自由视频项目", "error");
      return;
    }
    const validationError = canvasGraphSaveValidationError(
      nodes,
      edges,
      nodeContractsByKind,
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
      const fallbackMime =
        mediaKind === "image"
          ? "application/octet-stream"
          : mediaKind === "video"
            ? "video/mp4"
            : "application/octet-stream";
      const imported = await importCanvasMedia({
        project_id: projectId,
        media_kind: mediaKind,
        file_name: file.name,
        mime_type: file.type || fallbackMime,
        data_base64: await readFileAsBase64(file),
        prompt: file.name,
      });
      const nextNodes = [
        ...nodes.map((node) => ({ ...node, selected: false })),
        imported.media_kind === "image"
          ? importedImageInputNode(imported.image, imported.asset, nodes.length)
          : imported.media_kind === "video"
            ? importedVideoInputNode(
                imported.video,
                imported.asset,
                nodes.length,
              )
            : importedAudioInputNode(imported.asset, nodes.length),
      ];
      const saved = await saveCanvasGraph(
        projectId,
        toCanvasGraphInput(document, nextNodes, edges, nodeContractsByKind),
      );
      const savedEdges = saved.edges.map(toFlowEdge);
      setDocument(saved);
      setNodes((current) =>
        preserveSelectedItems(saved.nodes.map(toFlowNode), current),
      );
      setEdges((current) => preserveSelectedItems(savedEdges, current));
      setDirty(false);
      addToast(
        imported.media_kind === "image"
          ? "图片已导入画布"
          : imported.media_kind === "video"
            ? "视频已导入画布"
            : "音频已导入画布",
        "success",
      );
    } catch (err) {
      addToast(
        extractErrorMessage(
          err,
          mediaKind === "image"
            ? "导入图片失败"
            : mediaKind === "video"
              ? "导入视频失败"
              : "导入音频失败",
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
                extractErrorMessage(err, "同步画布运行结果失败"),
                "error",
              );
            }
          }
        }
      } catch (err) {
        if (!options.silent) {
          addToast(extractErrorMessage(err, "加载画布运行记录失败"), "error");
        }
      } finally {
        setWorkflowLoading(false);
      }
    },
    [addToast, refreshCanvasRuntimeResults],
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
    ): Promise<WorkflowRun | null> => {
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
      addToast("请先保存画布再预览节点请求", "info");
      return;
    }
    const nodeId = numericId(node.id);
    if (!nodeId) {
      addToast("请先保存新节点再查看模型请求", "info");
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
      addToast(extractErrorMessage(err, "预览节点请求失败"), "error");
    } finally {
      setPreviewLoading(false);
    }
  };

  const runNode = async (node: StudioNode) => {
    if (!document || canvasWriteBusy || !canEdit || workflowSubmitting) return;
    if (dirty) {
      addToast("请先保存画布再运行", "info");
      return;
    }
    const contract = studioNodeContract(
      nodeContractsByKind,
      node.data.nodeKind,
    );
    if (!contract?.runnable) {
      addToast("输入节点不能单独运行，请运行下游功能节点或输出节点", "info");
      return;
    }
    const nodeId = numericId(node.id);
    if (!nodeId) {
      addToast("请先保存新节点再运行", "info");
      return;
    }
    const confirmation = [
      "确认只运行此节点？",
      "",
      `目标节点：${nodeDisplayTitle(node.data)}`,
      "系统只执行当前节点，不会自动运行上游节点。",
      "当前节点会读取已保存输入，或读取上游节点最近一次完成的运行结果。",
    ].join("\n");
    const confirmed = await confirmDialog({
      title: "只运行此节点",
      message: confirmation,
      confirmText: "运行",
      cancelText: "取消",
      dialogType: "warning",
    });
    if (!confirmed) {
      return;
    }
    setWorkflowSubmissionLabel(`正在只运行：${nodeDisplayTitle(node.data)}`);
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowRuntimeRefreshSignatureRef.current = null;
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, Date.now());
    try {
      const result = await runCanvasNode(projectId, graphId, nodeId);
      addToast(
        result.error_message ?? "已提交只运行此节点",
        result.status === "failed" ? "info" : "success",
      );
      await loadWorkflowStatus(result.workflow_run_id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, "只运行此节点失败"), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const runToNode = async (node: StudioNode) => {
    if (!document || canvasWriteBusy || !canEdit || workflowSubmitting) return;
    if (dirty) {
      addToast("请先保存画布再运行", "info");
      return;
    }
    if (
      studioNodeContract(nodeContractsByKind, node.data.nodeKind)?.category !==
      "output"
    ) {
      addToast(
        "运行到此节点只能锚定输出节点，请选择文本/图片/视频/音频输出节点",
        "info",
      );
      return;
    }
    const nodeId = numericId(node.id);
    if (!nodeId) {
      addToast("请先保存新节点再运行", "info");
      return;
    }
    const scope = computeRunToNodeScope(
      node,
      nodes,
      edges,
      nodeContractsByKind,
    );
    const confirmation = runToNodeConfirmationMessage(scope, node);
    const confirmed = await confirmDialog({
      title: "运行到此节点",
      message: confirmation,
      confirmText: "运行",
      cancelText: "取消",
      dialogType: "warning",
    });
    if (!confirmed) {
      return;
    }
    setWorkflowSubmissionLabel(`正在运行到：${nodeDisplayTitle(node.data)}`);
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowRuntimeRefreshSignatureRef.current = null;
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, Date.now());
    try {
      const result = await runCanvasGraphToNode(projectId, graphId, nodeId);
      addToast(
        result.error_message ?? "已提交运行到此节点",
        result.status === "failed" ? "info" : "success",
      );
      await loadWorkflowStatus(result.workflow_run_id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, "运行到此节点失败"), "error");
    } finally {
      setWorkflowSubmissionLabel(null);
      stopWorkflowDiscovery();
    }
  };

  const runGraph = async () => {
    if (!document || canvasWriteBusy || !canEdit || workflowSubmitting) return;
    if (dirty) {
      addToast("请先保存画布再运行", "info");
      return;
    }
    setWorkflowSubmissionLabel("正在运行画布");
    setLatestWorkflowRunId(null);
    setWorkflowDetail(null);
    setWorkflowEvents([]);
    workflowRuntimeRefreshSignatureRef.current = null;
    const graphId = document.graph.id;
    startWorkflowDiscovery(graphId, Date.now());
    try {
      const result = await runCanvasGraph(projectId, graphId);
      addToast(
        result.error_message ?? "画布运行已提交",
        result.status === "failed" ? "info" : "success",
      );
      await loadWorkflowStatus(result.workflow_run_id);
      await load();
    } catch (err) {
      addToast(extractErrorMessage(err, "运行画布失败"), "error");
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
          label: hasProviderRequest ? "模型请求" : "查看内容",
          icon: <Eye size={13} />,
          onClick: () => void previewNode(node),
        });
      }
      if (isRunnableNode) {
        items.push({
          label: "只运行此节点",
          icon: <Play size={13} />,
          disabled: canvasMutationDisabled || workflowSubmitting,
          onClick: () => void runNode(node),
        });
      }
      if (isOutputNode) {
        items.push({
          label: "运行到此节点",
          icon: <Play size={13} />,
          disabled: canvasMutationDisabled || workflowSubmitting,
          onClick: () => void runToNode(node),
        });
      }
      if (!canvasMutationDisabled) {
        if (items.length > 0) items.push({ divider: true });
        items.push({
          label: "删除节点",
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
    ],
  );

  const edgeContextMenuItems = useCallback(
    (edge: StudioEdge): ContextMenuEntry[] =>
      !canvasMutationDisabled
        ? [
            {
              label: "删除连线",
              icon: <Trash2 size={13} />,
              danger: true,
              shortcut: "Del",
              onClick: () => deleteCanvasEdge(edge),
            },
          ]
        : [],
    [canvasMutationDisabled, deleteCanvasEdge],
  );

  const mediaImportMenuItems = useCallback(
    (): ContextMenuEntry[] => [
      {
        label: "导入图片",
        icon: <Upload size={13} />,
        disabled: !document || canvasWriteBusy || !canEdit,
        onClick: () => void importMedia("image"),
      },
      {
        label: "导入视频",
        icon: <Upload size={13} />,
        disabled: !document || canvasWriteBusy || !canEdit,
        onClick: () => void importMedia("video"),
      },
      {
        label: "导入音频",
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
    ],
  );

  const paneContextMenuItems = useCallback(
    (): ContextMenuEntry[] => [
      {
        label: "运行画布",
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
      await cancelWorkflow({ taskId, workflowRunId });
      addToast("已请求取消画布任务", "info");
      if (workflowRunId) {
        await loadWorkflowStatus(workflowRunId);
      }
    } catch (err) {
      addToast(extractErrorMessage(err, "取消画布任务失败"), "error");
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
      addToast("已继续运行画布任务", "success");
      workflowRuntimeRefreshSignatureRef.current = null;
      await loadWorkflowStatus(run.id);
    } catch (err) {
      addToast(extractErrorMessage(err, "继续运行失败"), "error");
    } finally {
      setResumingWorkflow(false);
    }
  };

  const headerActionItems = useMemo<DropdownItem[]>(() => {
    const importDisabled = !document || canvasWriteBusy || !canEdit;
    const items: DropdownItem[] = [
      {
        label: "导入图片",
        icon: <Upload size={13} />,
        disabled: importDisabled,
        onClick: () => void importMedia("image"),
      },
      {
        label: "导入视频",
        icon: <Upload size={13} />,
        disabled: importDisabled,
        onClick: () => void importMedia("video"),
      },
    ];
    if (project?.source_mode === "freeform") {
      items.push({
        label: "导入音频",
        icon: <Upload size={13} />,
        disabled: importDisabled,
        onClick: () => void importMedia("audio"),
      });
    }
    if (selectedNode) {
      items.push({ type: "divider" });
      items.push({
        label: nodeHasProviderRequest(selectedNode) ? "查看模型请求" : "查看节点内容",
        icon: <Eye size={13} />,
        disabled:
          previewLoading ||
          (nodeHasProviderRequest(selectedNode) && (!document || dirty)),
        onClick: () => void previewNode(selectedNode),
      });
      if (selectedNodeRunToTarget || selectedNodeRunnable) {
        items.push({
          label: selectedNodeRunToTarget ? "运行到所选节点" : "只运行所选节点",
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
  ]);

  const renderNodeLibraryPanel = (mobile: boolean) => (
    <div className={mobile ? "space-y-3" : "flex h-full min-h-0 flex-col"}>
      {mobile ? (
        <p className="text-small text-book-text-muted">
          点击节点即可添加到当前画布视图中心
        </p>
      ) : (
        <div className="mb-3">
          <h2 className="text-sm font-semibold text-book-text-main">节点库</h2>
          <p className="mt-0.5 text-small text-book-text-muted">
            拖入画布编排 · {nodeTemplateCount} 种节点
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
                  title={mobile ? "添加到当前画布视图中心" : "拖入画布"}
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
            {selectedEdge ? "连线属性" : "节点属性"}
          </h2>
          <p className="mt-0.5 truncate text-small text-book-text-muted">
            {selectedNode
              ? `${selectedNode.data.label || nodeDisplayTitle(selectedNode.data)} · ${stateLabel(selectedNode.data.state)}`
              : selectedEdge
                ? edgeRoleLabel(selectedEdge.data?.role ?? "")
                : "选择节点或连线后编辑"}
          </p>
        </div>
      ) : selectedNode ? (
        <p className="text-small text-book-text-muted">
          {selectedNode.data.label || nodeDisplayTitle(selectedNode.data)} · {stateLabel(selectedNode.data.state)}
        </p>
      ) : null}
      <div className={mobile ? "space-y-3" : "min-h-0 flex-1 overflow-y-auto pr-1 custom-scrollbar"}>
        {selectedEdge ? (
          <EdgeInspector edge={selectedEdge} />
        ) : selectedNode ? (
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
        ) : (
          <div className="a4n-empty-state a4n-empty-state--compact text-xs">
            选择画布节点或连线后编辑属性
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
          leading={<BackButton iconOnly title="返回项目列表" onClick={() => navigate("/")} />}
          icon={<Film size={15} />}
          title={project?.title || "视频画布"}
          subtitle={
            selectedNode
              ? `${selectedNode.data.label || nodeDisplayTitle(selectedNode.data)} · ${stateLabel(selectedNode.data.state)}`
              : project?.source_mode === "freeform"
                ? "自由编排节点、媒体与生成流程"
                : "基于派生项目编排视频生成流程"
          }
          meta={
            <span className="text-small text-book-text-muted">
              {saving
                ? "正在保存画布…"
                : importing
                  ? "正在导入媒体…"
                  : project?.source_mode === "freeform"
                    ? "自由画布"
                    : "派生画布"}
              {!canvasWriteBusy && dirty ? " · 有未保存更改" : ""}
            </span>
          }
          actions={
            <>
              {isMobileLayout ? (
                <>
                  <BookIconButton
                    label="打开节点库"
                    onClick={() => setMobileNodeLibraryOpen(true)}
                    disabled={canvasMutationDisabled}
                    variant="secondary"
                    size="sm"
                  >
                    <ListPlus size={14} />
                  </BookIconButton>
                  <BookIconButton
                    label="打开属性面板"
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
                  {saving ? "保存中…" : "保存画布"}
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
                  title="运行完整画布工作流"
                >
                  {workflowSubmitting ? (
                    <LoadingSpinner size="sm" />
                  ) : (
                    <Play size={14} />
                  )}
                  {workflowSubmitting ? workflowSubmissionLabel ?? "正在运行" : "运行画布"}
                </BookButton>
              )}
            </>
          }
        />
        <section
          aria-label="视频画布编辑区"
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
                加载画布
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
                  <Controls className="a4n-studio-controls" />
                  <Background
                    color="rgb(var(--color-text-tertiary) / 0.24)"
                    gap={32}
                    size={1}
                  />
                </ReactFlow>
                {nodes.length === 0 ? (
                  <CanvasEmptyState disabled={!canEdit} />
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
              title="节点库"
              maxWidthClassName="max-w-lg"
              bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar p-3"
            >
              {renderNodeLibraryPanel(true)}
            </Modal>
            <Modal
              isOpen={mobileInspectorOpen}
              onClose={() => setMobileInspectorOpen(false)}
              title={selectedEdge ? "连线属性" : "节点属性"}
              maxWidthClassName="max-w-xl"
              bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar p-3"
            >
              {renderInspectorPanel(true)}
            </Modal>
          </>
        ) : null}
        <CanvasNodePreviewDialog
          open={previewDialogOpen}
          loading={previewLoading}
          preview={previewData}
          onClose={() => setPreviewDialogOpen(false)}
          onCopyError={(message) => addToast(message, "error")}
        />
        <CanvasNodeContentPreviewDialog
          node={contentPreviewNode}
          onClose={() => setContentPreviewNode(null)}
          onCopyError={(message) => addToast(message, "error")}
        />
      </div>
    </StudioNodeContractsProvider>
  );
};
