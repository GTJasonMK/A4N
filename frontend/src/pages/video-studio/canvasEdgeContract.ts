import { nodeDisplayTitle } from "./StudioCanvasNode";
import {
  dataTypesCompatible,
  studioPortLabel,
  studioNodeContract,
  studioNodeInputPortsForData,
  studioNodeOutputPortsForData,
  type StudioNodeContractMap,
} from "./nodeContracts";
import type { MessageTranslator } from "../../i18n";
import type { StudioEdge, StudioNode } from "./types";

export type CanvasEdgeContract = {
  sourcePort: string;
  targetPort: string;
  role: string;
};

export type CanvasConnectionLike = {
  source?: string | null;
  target?: string | null;
  sourceHandle?: string | null;
  targetHandle?: string | null;
};

export function inferCanvasEdgeContract(
  connection: CanvasConnectionLike,
  nodesOrMap: StudioNode[] | Map<string, StudioNode>,
  contractsByKind: StudioNodeContractMap,
  existingEdges: StudioEdge[] = [],
): CanvasEdgeContract | null {
  if (!connection.source || !connection.target) return null;
  const nodesById = Array.isArray(nodesOrMap)
    ? new Map(nodesOrMap.map((node) => [node.id, node]))
    : nodesOrMap;
  const source = nodesById.get(connection.source);
  const target = nodesById.get(connection.target);
  if (!source || !target) return null;
  if (source.id === target.id) return null;

  const sourceContract = studioNodeContract(contractsByKind, source.data.nodeKind);
  const targetContract = studioNodeContract(contractsByKind, target.data.nodeKind);
  if (!sourceContract || !targetContract) return null;
  if (sourceContract.category === "output" || targetContract.category === "input") return null;

  const sourceHandle = connection.sourceHandle?.trim();
  const targetHandle = connection.targetHandle?.trim();
  const activeSourceOutputs = studioNodeOutputPortsForData(source.data, sourceContract);
  const sourcePorts = sourceHandle
    ? activeSourceOutputs.filter((port) => port.id === sourceHandle)
    : activeSourceOutputs;
  const activeTargetInputs = studioNodeInputPortsForData(target.data, targetContract);
  const occupiedTargetPorts = targetHandle
    ? new Set<string>()
    : new Set(
        existingEdges
          .filter((edge) => edge.target === target.id && edge.targetHandle != null)
          .map((edge) => edge.targetHandle as string),
      );
  const targetPorts = targetHandle
    ? activeTargetInputs.filter((port) => port.id === targetHandle)
    : activeTargetInputs.filter(
        (port) =>
          port.multiple ||
          !occupiedTargetPorts.has(port.id),
      );

  for (const sourcePort of sourcePorts) {
    for (const targetPort of targetPorts) {
      if (!dataTypesCompatible(sourcePort.data_type, targetPort.data_type)) continue;
      return {
        sourcePort: sourcePort.id,
        targetPort: targetPort.id,
        role: canvasEdgeRoleForPorts(sourcePort.id, target.data.nodeKind, targetPort.id),
      };
    }
  }
  return null;
}

export function validateCanvasConnection(
  connection: CanvasConnectionLike,
  nodes: StudioNode[],
  edges: StudioEdge[],
  contractsByKind: StudioNodeContractMap,
  t: MessageTranslator,
): CanvasEdgeContract | string {
  if (!connection.source || !connection.target) return t("video.canvas.validation.missingEndpoints");
  if (connection.source === connection.target) return t("video.canvas.validation.selfConnection");
  const contract = inferCanvasEdgeContract(connection, nodes, contractsByKind, edges);
  if (!contract) return t("video.canvas.validation.incompatible");
  if (
    edges.some(
      (edge) =>
        edge.source === connection.source &&
        edge.target === connection.target &&
        (edge.sourceHandle ?? "") === contract.sourcePort &&
        (edge.targetHandle ?? "") === contract.targetPort,
    )
  ) {
    return t("video.canvas.validation.duplicate");
  }
  const targetNode = nodes.find((node) => node.id === connection.target);
  const targetContract = targetNode ? studioNodeContract(contractsByKind, targetNode.data.nodeKind) : null;
  const targetPort =
    targetNode && targetContract
      ? studioNodeInputPortsForData(targetNode.data, targetContract).find((port) => port.id === contract.targetPort)
      : null;
  if (targetPort && !targetPort.multiple) {
    const hasExistingInput = edges.some(
      (edge) => edge.target === connection.target && edge.targetHandle === contract.targetPort,
    );
    if (hasExistingInput) {
      const targetNode = nodes.find((node) => node.id === connection.target);
      const portLabel = targetNode
        ? studioPortLabel(targetNode.data.nodeKind, targetPort, t)
        : targetPort.id;
      return t("video.canvas.validation.singleInput", { port: portLabel });
    }
  }
  return contract;
}

export function canvasGraphSaveValidationError(
  nodes: StudioNode[],
  edges: StudioEdge[],
  contractsByKind: StudioNodeContractMap,
  t: MessageTranslator,
): string | null {
  const nodesById = new Map(nodes.map((node) => [node.id, node]));
  const singleInputEdges = new Set<string>();
  for (const edge of edges) {
    if (!edge.source || !edge.target) return t("video.canvas.validation.missingEndpoints");
    const contract = inferCanvasEdgeContract(edge, nodesById, contractsByKind);
    const source = nodesById.get(edge.source);
    const target = nodesById.get(edge.target);
    if (!contract || !source || !target) {
      return canvasEdgeMismatchMessage(edge, source, target, t);
    }
    if (
      edge.sourceHandle !== contract.sourcePort ||
      edge.targetHandle !== contract.targetPort ||
      edge.data?.role !== contract.role
    ) {
      return canvasEdgeMismatchMessage(edge, source, target, t);
    }
    const targetContract = studioNodeContract(contractsByKind, target.data.nodeKind);
    const targetPort = targetContract
      ? studioNodeInputPortsForData(target.data, targetContract).find((port) => port.id === contract.targetPort)
      : null;
    if (!targetPort) {
      return canvasEdgeMismatchMessage(edge, source, target, t);
    }
    if (!targetPort.multiple) {
      const key = `${target.id}:${contract.targetPort}`;
      if (singleInputEdges.has(key)) {
        return t("video.canvas.validation.singleInput", {
          port: studioPortLabel(target.data.nodeKind, targetPort, t),
        });
      }
      singleInputEdges.add(key);
    }
  }
  return null;
}

function canvasEdgeMismatchMessage(
  edge: StudioEdge,
  source: StudioNode | undefined,
  target: StudioNode | undefined,
  t: MessageTranslator,
): string {
  const sourceName = source ? nodeDisplayTitle(source.data, t) : edge.source;
  const targetName = target ? nodeDisplayTitle(target.data, t) : edge.target;
  const sourceKind = source?.data.nodeKind ?? t("video.canvas.unknownNode");
  const targetKind = target?.data.nodeKind ?? t("video.canvas.unknownNode");
  const sourcePort = edge.sourceHandle ?? "-";
  const targetPort = edge.targetHandle ?? "-";
  return t("video.canvas.validation.edgeMismatch", {
    source: sourceName,
    sourceKind,
    sourcePort,
    target: targetName,
    targetKind,
    targetPort,
  });
}

function canvasEdgeRoleForPorts(
  sourcePort: string,
  targetKind: string,
  targetPort: string,
): string {
  if (targetPort === "first_frame") return "first_frame";
  if (targetPort === "end_frame") return "end_frame";
  if (targetPort === "reference_image") return "style_reference";
  if (targetPort === "clip") return "assembly_clip";
  if (targetKind === "VideoOutputNode" && targetPort === "video") return "final_video";
  if (targetPort === "video" && sourcePort === "video") return "source_video";
  return "context";
}
