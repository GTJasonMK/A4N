import React, { useCallback } from "react";
import { addEdge, type Connection, type EdgeChange, type IsValidConnection, type NodeChange, type ReactFlowInstance } from "@xyflow/react";
import { extractErrorMessage } from "../../api";
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
  CANVAS_RUNTIME_CONFIG_KEYS,
  canvasNodePosition,
  clearCanvasNodeRuntimeResult,
  defaultConfigForNode,
  mergeCanvasDocumentRuntimeResults,
  mergeStudioNodeRuntimeResults,
  persistCanvasTextAssets,
  preserveSelectedItems,
  toCanvasGraphInput,
  toFlowEdge,
  toFlowNode,
} from "./canvasGraphMapping";
import {
  nextAssembleClipOrder,
  reorderAssembleClipEdges,
} from "./canvasAssemblyEdges";
import {
  NODE_DRAG_KIND,
  NODE_DRAG_LABEL,
  setNodeTemplateDragImage,
} from "./canvasDragPreview";
import { validateCanvasConnection } from "./canvasEdgeContract";
import { defaultRefTypeForNode } from "./canvasNodeRefs";
import { edgeRoleLabel, nodeDisplayTitle, nodeKindMeta } from "./StudioCanvasNode";
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
  canEdit: boolean;
  canvasWriteBusy: boolean;
  canvasMutationDisabled: boolean;
  flowInstance: ReactFlowInstance<StudioNode, StudioEdge> | null;
  canvasSectionRef: React.RefObject<HTMLElement | null>;
  t: MessageTranslator;
  addToast: (message: string, tone: "success" | "error" | "info" | "warning") => void;
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

export function useCanvasGraphActions({
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
}: CanvasGraphActionsOptions) {
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
      const invalidatedNodeIds = downstreamNodeIds(removedTargetIds, edges);
      if (invalidatedNodeIds.size > 0) {
        setNodes((items) =>
          items.map((node) =>
            invalidatedNodeIds.has(node.id)
              ? clearCanvasNodeRuntimeResult(node)
              : node,
          ),
        );
      }
      if (allowedChanges.length > 0) applyEdgesChange(allowedChanges);
    },
    [applyEdgesChange, canvasWriteBusy, edges, setNodes],
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
      const invalidatedNodeIds = downstreamNodeIds(
        new Set([connection.target].filter((id): id is string => Boolean(id))),
        edges,
      );
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
      setNodes((items) =>
        items.map((item) =>
          invalidatedNodeIds.has(item.id)
            ? clearCanvasNodeRuntimeResult(item)
            : item,
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
      const invalidatedNodeIds = downstreamNodeIds(new Set([edge.target]), edges);
      setEdges((items) => items.filter((item) => item.id !== edge.id));
      setNodes((items) =>
        items.map((item) =>
          invalidatedNodeIds.has(item.id)
            ? clearCanvasNodeRuntimeResult(item)
            : item,
        ),
      );
      setDirty(true);
      addToast(t("video.deletedEdge"), "success");
    },
    [addToast, canvasMutationDisabled, edges, setEdges, setNodes, t],
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
    const invalidatesRuntimeResult =
      selectedNode.data.config.canvas_runtime_result === true &&
      Object.keys(patch).some((key) => key !== "title");
    if (invalidatesRuntimeResult) {
      for (const key of CANVAS_RUNTIME_CONFIG_KEYS) delete nextConfig[key];
    }
    const nextLabel =
      typeof patch.title === "string"
        ? patch.title.trim() || nodeKindMeta(selectedNode.data.nodeKind, t).label
        : selectedNode.data.label;
    const nextData = {
      ...selectedNode.data,
      label: nextLabel,
      refType: invalidatesRuntimeResult ? null : selectedNode.data.refType,
      refId: invalidatesRuntimeResult ? null : selectedNode.data.refId,
      state: invalidatesRuntimeResult ? "ready" : selectedNode.data.state,
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
        document.graph.version,
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
        document.graph.version,
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

function downstreamNodeIds(
  startNodeIds: Set<string>,
  edges: StudioEdge[],
): Set<string> {
  const visited = new Set(startNodeIds);
  const queue = [...startNodeIds];
  for (let index = 0; index < queue.length; index += 1) {
    const source = queue[index];
    for (const edge of edges) {
      if (edge.source !== source || visited.has(edge.target)) continue;
      visited.add(edge.target);
      queue.push(edge.target);
    }
  }
  return visited;
}
