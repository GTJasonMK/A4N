import React, { useCallback } from "react";
import { addEdge, type Connection, type EdgeChange, type IsValidConnection, type NodeChange, type ReactFlowInstance, type Viewport } from "@xyflow/react";
import { extractErrorMessage } from "../../api";
import type { ToastType } from "../../components/feedback/Toast";
import { confirmDialog } from "../../components/feedback/ConfirmDialog";
import {
  getCanvasGraph,
  importCanvasMedia,
  saveCanvasGraph,
  type CanvasGraphDocument,
} from "../../api/canvas";
import type { NovelProject } from "../../api/project";
import { pickLocalFile, readFileAsBase64 } from "../../utils/fileUpload";
import type { MessageTranslator } from "../../i18n";
import type { StudioConfigOptionsByKind } from "./canvasConfig";
import {
  canvasNodePosition,
  changeCanvasNodeReference,
  invalidateCanvasRuntimeResults,
  defaultConfigForNode,
  mergeCanvasDocumentRuntimeResults,
  mergeStudioNodeRuntimeResults,
  preserveSelectedItems,
  toCanvasGraphInput,
  toFlowEdge,
  toFlowNode,
} from "./canvasGraphMapping";
import {
  nextAssembleClipOrder,
  renumberAssembleClipEdges,
  reorderAssembleClipEdges,
} from "./canvasAssemblyEdges";
import {
  NODE_DRAG_KIND,
  NODE_DRAG_LABEL,
  setNodeTemplateDragImage,
} from "./canvasDragPreview";
import {
  canvasGraphSaveValidationError,
  validateCanvasConnection,
} from "./canvasEdgeContract";
import {
  importedAudioInputNode,
  importedImageInputNode,
  importedVideoInputNode,
} from "./canvasImportNodes";
import { defaultRefTypeForNode } from "./canvasNodeRefs";
import { nodeDisplayTitle, nodeKindMeta } from "./StudioCanvasNode";
import {
  studioNodeContract,
  studioNodeInputPortsForData,
  type StudioNodeContractMap,
} from "./nodeContracts";
import type { StudioEdge, StudioNode } from "./types";

export interface CanvasGraphActionsOptions {
  projectId: number;
  project: NovelProject | null;
  document: CanvasGraphDocument | null;
  nodes: StudioNode[];
  edges: StudioEdge[];
  selectedNode: StudioNode | null;
  nodeContractsByKind: StudioNodeContractMap;
  configOptions: StudioConfigOptionsByKind;
  canvasWriteBusy: boolean;
  canvasMutationDisabled: boolean;
  flowInstance: ReactFlowInstance<StudioNode, StudioEdge> | null;
  canvasSectionRef: React.RefObject<HTMLElement | null>;
  t: MessageTranslator;
  addToast: (message: string, tone: ToastType) => void;
  setDocument: React.Dispatch<React.SetStateAction<CanvasGraphDocument | null>>;
  setNodes: React.Dispatch<React.SetStateAction<StudioNode[]>>;
  setEdges: React.Dispatch<React.SetStateAction<StudioEdge[]>>;
  applyNodesChange: (changes: NodeChange<StudioNode>[]) => void;
  applyEdgesChange: (changes: EdgeChange<StudioEdge>[]) => void;
  setDirty: React.Dispatch<React.SetStateAction<boolean>>;
  setSaving: React.Dispatch<React.SetStateAction<boolean>>;
  setImporting: React.Dispatch<React.SetStateAction<boolean>>;
  setMobileNodeLibraryOpen: React.Dispatch<React.SetStateAction<boolean>>;
}

function viewportChanged(before: Viewport | undefined, after: Viewport | undefined): boolean {
  return Boolean(before && after && (
    before.x !== after.x || before.y !== after.y || before.zoom !== after.zoom
  ));
}

export function useCanvasGraphActions({
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
}: CanvasGraphActionsOptions) {
  const translatorRef = React.useRef(t);
  translatorRef.current = t;
  const currentT = () => translatorRef.current;
  const actionScopeRef = React.useRef("");
  actionScopeRef.current = `${projectId}:${document?.graph.id ?? ""}:${document?.graph.version ?? ""}`;
  const latestDocumentRef = React.useRef(document);
  const latestNodesRef = React.useRef(nodes);
  const latestEdgesRef = React.useRef(edges);
  const latestSelectedNodeRef = React.useRef(selectedNode);
  latestDocumentRef.current = document;
  latestNodesRef.current = nodes;
  latestEdgesRef.current = edges;
  latestSelectedNodeRef.current = selectedNode;
  React.useEffect(() => () => {
    actionScopeRef.current = "";
  }, []);
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
      const removedTargetIds = new Set(
        allowedChanges
          .filter((change) => change.type === "remove")
          .map((change) => edges.find((edge) => edge.id === change.id)?.target)
          .filter((target): target is string => Boolean(target)),
      );
      if (removedTargetIds.size > 0) {
        setNodes((items) => invalidateCanvasRuntimeResults(items, edges, removedTargetIds));
      }
      if (allowedChanges.length > 0) applyEdgesChange(allowedChanges);
      if (removedTargetIds.size > 0) {
        setEdges((items) => renumberAssembleClipEdges(items, removedTargetIds));
      }
    },
    [applyEdgesChange, canvasWriteBusy, edges, setEdges, setNodes],
  );

  const addNode = useCallback(
    (kind: string, label: string, position?: { x: number; y: number }) => {
      if (canvasMutationDisabled) return;
      const id = `node-${Date.now()}-${Math.round(Math.random() * 10000)}`;
      const imageConfig =
        kind === "GenerateImageNode"
          ? configOptions.image.find((option) => option.isDefault)
          : undefined;
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
            config: defaultConfigForNode(kind, imageConfig),
          },
        },
      ]);
      setDirty(true);
    },
    [canvasMutationDisabled, configOptions.image, setNodes],
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
      const sourceIcon = event.currentTarget.querySelector<SVGSVGElement>("svg.lucide");
      if (!sourceIcon) {
        event.preventDefault();
        throw new Error("Canvas node template drag source is missing its Lucide icon");
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
        sourceIcon,
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
      const invalidationRoots = new Set(
        [connection.target].filter((id): id is string => Boolean(id)),
      );
      setEdges((items) =>
        addEdge(
          {
            ...connection,
            sourceHandle: contract.sourcePort,
            targetHandle: contract.targetPort,
            type: "smoothstep",
            animated: false,
            className: "a4n-studio-edge",
            labelBgPadding: [6, 3],
            labelBgBorderRadius: 4,
            data: { role: contract.role, config },
          },
          items,
        ),
      );
      setNodes((items) => invalidateCanvasRuntimeResults(items, edges, invalidationRoots));
      setDirty(true);
    },
    [
      addToast,
      canvasMutationDisabled,
      edges,
      nodeContractsByKind,
      nodes,
      setEdges,
      setNodes,
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
      const affectedTargets = new Set(edges.filter((edge) => edge.source === node.id).map((edge) => edge.target));
      setNodes((items) => invalidateCanvasRuntimeResults(
        items.filter((item) => item.id !== node.id),
        edges,
        affectedTargets,
      ));
      setEdges((items) =>
        renumberAssembleClipEdges(
          items.filter((edge) => edge.source !== node.id && edge.target !== node.id),
          affectedTargets,
        ),
      );
      setDirty(true);
      addToast(t("video.deletedNode", { name: nodeDisplayTitle(node.data, t) }), "success");
    },
    [addToast, canvasMutationDisabled, edges, setEdges, setNodes, t],
  );

  const deleteCanvasEdge = useCallback(
    (edge: StudioEdge) => {
      if (canvasMutationDisabled) return;
      const invalidationRoots = new Set([edge.target]);
      setEdges((items) => renumberAssembleClipEdges(items.filter((item) => item.id !== edge.id), new Set([edge.target])));
      setNodes((items) => invalidateCanvasRuntimeResults(items, edges, invalidationRoots));
      setDirty(true);
      addToast(t("video.deletedEdge"), "success");
    },
    [addToast, canvasMutationDisabled, edges, setEdges, setNodes, t],
  );

  const updateSelectedConfig = async (patch: Record<string, unknown>) => {
    if (!selectedNode || canvasMutationDisabled) return;
    const selectedNodeId = selectedNode.id;
    let activeInputIds: Set<string> | null = null;
    if (
      selectedNode.data.nodeKind === "GenerateVideoNode" &&
      Object.prototype.hasOwnProperty.call(patch, "input_mode")
    ) {
      const contract = studioNodeContract(
        nodeContractsByKind,
        selectedNode.data.nodeKind,
      );
      const nextData = {
        ...selectedNode.data,
        config: { ...selectedNode.data.config, ...patch },
      };
      activeInputIds = new Set(
        contract
          ? studioNodeInputPortsForData(nextData, contract).map((port) => port.id)
          : [],
      );
      const incompatibleEdges = edges.filter(
        (edge) => edge.target === selectedNodeId && !activeInputIds?.has(edge.targetHandle ?? ""),
      );
      if (incompatibleEdges.length > 0) {
        const requestScope = actionScopeRef.current;
        const confirmed = await confirmDialog({
          title: t("video.canvas.inputModeChangeTitle"),
          message: t("video.canvas.inputModeChangeMessage", {
            count: incompatibleEdges.length,
          }),
          confirmText: t("video.canvas.inputModeChangeConfirm"),
          dialogType: "warning",
        });
        if (
          !confirmed ||
          actionScopeRef.current !== requestScope ||
          latestSelectedNodeRef.current?.id !== selectedNodeId
        ) return;
      }
    }
    const invalidatesRuntimeResults = Object.keys(patch).some((key) => key !== "title");
    setNodes((items) => {
      const next = invalidatesRuntimeResults
        ? invalidateCanvasRuntimeResults(items, edges, [selectedNodeId])
        : items;
      return next.map((node) => {
        if (node.id !== selectedNodeId) return node;
        const nextConfig = { ...node.data.config, ...patch };
        for (const [key, value] of Object.entries(patch)) {
          if (value === undefined) delete nextConfig[key];
        }
        return {
          ...node,
          data: {
            ...node.data,
            label:
              typeof patch.title === "string"
                ? patch.title.trim() || nodeKindMeta(node.data.nodeKind, t).label
                : node.data.label,
            config: nextConfig,
          },
        };
      });
    });
    if (activeInputIds) {
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
    if (selectedNode.data.refType === refType && selectedNode.data.refId === refId) return;
    const selectedNodeId = selectedNode.id;
    setNodes((items) => {
      const next = invalidateCanvasRuntimeResults(items, edges, [selectedNodeId]);
      return next.map((node) =>
        node.id === selectedNodeId
          ? changeCanvasNodeReference(node, refType, refId)
          : node,
      );
    });
    setDirty(true);
  };

  const moveAssembleClip = (edgeId: string, direction: -1 | 1) => {
    if (canvasMutationDisabled) return;
    const next = reorderAssembleClipEdges(edges, edgeId, direction);
    if (next === edges) return;
    const targetNodeId = edges.find((edge) => edge.id === edgeId)?.target;
    if (!targetNodeId) return;
    setEdges(next);
    setNodes((items) => invalidateCanvasRuntimeResults(items, edges, [targetNodeId]));
    setDirty(true);
  };

  const persist = async () => {
    if (!document || canvasMutationDisabled) return;
    const requestScope = actionScopeRef.current;
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
    const viewportAtSave = flowInstance?.getViewport();
    try {
      const saved = await saveCanvasGraph(
        projectId,
        toCanvasGraphInput(
          viewportAtSave
            ? { ...document, graph: { ...document.graph, viewport_json: viewportAtSave } }
            : document,
          nodes,
          edges,
        ),
        document.graph.version,
      );
      if (actionScopeRef.current !== requestScope) return;
      const currentViewport = flowInstance?.getViewport();
      const viewportChangedDuringSave = viewportChanged(viewportAtSave, currentViewport);
      const savedEdges = saved.edges.map(toFlowEdge);
      setDocument(viewportChangedDuringSave && currentViewport
        ? { ...saved, graph: { ...saved.graph, viewport_json: currentViewport } }
        : saved);
      setNodes((current) =>
        preserveSelectedItems(saved.nodes.map(toFlowNode), current, saved.node_ids),
      );
      setEdges((current) => preserveSelectedItems(savedEdges, current));
      setDirty(viewportChangedDuringSave);
      addToast(currentT()("video.canvasSaved"), "success");
    } catch (err) {
      if (actionScopeRef.current !== requestScope) return;
      addToast(extractErrorMessage(err, currentT()("video.canvasSaveFailed")), "error");
    } finally {
      if (actionScopeRef.current === requestScope) setSaving(false);
    }
  };

  const importMedia = async (mediaKind: "image" | "video" | "audio") => {
    if (!document || canvasMutationDisabled) return;
    const requestScope = actionScopeRef.current;
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
    if (actionScopeRef.current !== requestScope) return;
    const currentDocument = latestDocumentRef.current;
    const currentNodes = latestNodesRef.current;
    const currentEdges = latestEdgesRef.current;
    if (!currentDocument) return;
    const currentValidationError = canvasGraphSaveValidationError(
      currentNodes,
      currentEdges,
      nodeContractsByKind,
      currentT(),
    );
    if (currentValidationError) {
      addToast(currentValidationError, "error");
      return;
    }
    setImporting(true);
    try {
      const dataBase64 = await readFileAsBase64(file);
      if (actionScopeRef.current !== requestScope) return;
      const latestDocument = latestDocumentRef.current;
      const latestNodes = latestNodesRef.current;
      const latestEdges = latestEdgesRef.current;
      if (!latestDocument) return;
      const latestValidationError = canvasGraphSaveValidationError(
        latestNodes,
        latestEdges,
        nodeContractsByKind,
        currentT(),
      );
      if (latestValidationError) {
        addToast(latestValidationError, "error");
        return;
      }
      const imported = await importCanvasMedia({
        project_id: projectId,
        media_kind: mediaKind,
        file_name: file.name,
        mime_type: file.type,
        data_base64: dataBase64,
      });
      let importedNode: StudioNode;
      let successMessageKey: Parameters<MessageTranslator>[0];
      switch (imported.media_kind) {
        case "image":
          importedNode = importedImageInputNode(
            imported.image,
            imported.asset,
            latestNodes.length,
            currentT(),
          );
          successMessageKey = "video.importedImage";
          break;
        case "video":
          importedNode = importedVideoInputNode(
            imported.video,
            imported.asset,
            latestNodes.length,
            currentT(),
          );
          successMessageKey = "video.importedVideo";
          break;
        case "audio":
          importedNode = importedAudioInputNode(imported.asset, latestNodes.length, currentT());
          successMessageKey = "video.importedAudio";
          break;
        default:
          throw new Error(currentT()("video.unsupportedImportedMedia"));
      }
      const nextNodes = [
        ...latestNodes.map((node) => ({ ...node, selected: false })),
        importedNode,
      ];
      const viewportAtSave = flowInstance?.getViewport();
      const saved = await saveCanvasGraph(
        projectId,
        toCanvasGraphInput(
          viewportAtSave
            ? { ...latestDocument, graph: { ...latestDocument.graph, viewport_json: viewportAtSave } }
            : latestDocument,
          nextNodes,
          latestEdges,
        ),
        latestDocument.graph.version,
      );
      if (actionScopeRef.current !== requestScope) return;
      const currentViewport = flowInstance?.getViewport();
      const viewportChangedDuringSave = viewportChanged(viewportAtSave, currentViewport);
      const savedEdges = saved.edges.map(toFlowEdge);
      setDocument(viewportChangedDuringSave && currentViewport
        ? { ...saved, graph: { ...saved.graph, viewport_json: currentViewport } }
        : saved);
      setNodes(preserveSelectedItems(saved.nodes.map(toFlowNode), nextNodes, saved.node_ids));
      setEdges((current) => preserveSelectedItems(savedEdges, current));
      setDirty(viewportChangedDuringSave);
      addToast(currentT()(successMessageKey), "success");
    } catch (err) {
      if (actionScopeRef.current !== requestScope) return;
      addToast(
        extractErrorMessage(
          err,
          mediaKind === "image"
            ? currentT()("video.importImageFailed")
            : mediaKind === "video"
              ? currentT()("video.importVideoFailed")
              : currentT()("video.importAudioFailed"),
        ),
        "error",
      );
    } finally {
      if (actionScopeRef.current === requestScope) setImporting(false);
    }
    };

  const refreshCanvasRuntimeResults = useCallback(
    async (graphId: number) => {
      const requestScope = actionScopeRef.current;
      const requestGraphVersion = latestDocumentRef.current?.graph.version ?? null;
      const freshDocument = await getCanvasGraph(projectId, graphId);
      if (
        actionScopeRef.current !== requestScope ||
        (requestGraphVersion != null && latestDocumentRef.current?.graph.version !== requestGraphVersion)
      ) return;
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


  return {
    onNodesChange,
    onEdgesChange,
    addNode,
    addNodeToVisibleCanvas,
    onNodeTemplateDragStart,
    onCanvasDragOver,
    onCanvasDrop,
    onConnect,
    isValidCanvasConnection,
    selectCanvasNode,
    selectCanvasEdge,
    deleteCanvasNode,
    deleteCanvasEdge,
    updateSelectedConfig,
    updateSelectedRef,
    moveAssembleClip,
    persist,
    importMedia,
    refreshCanvasRuntimeResults,
  };
}
