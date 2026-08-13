import { nodeDisplayTitle } from "./StudioCanvasNode";
import {
  dataTypesCompatible,
  studioNodeContract,
  studioNodeInputPortsForData,
  studioNodeOutputPortsForData,
  type StudioNodeContractMap,
} from "./nodeContracts";
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
  const targetPorts = targetHandle
    ? activeTargetInputs.filter((port) => port.id === targetHandle)
    : activeTargetInputs.filter(
        (port) =>
          port.multiple ||
          !existingEdges.some((edge) => edge.target === target.id && edge.targetHandle === port.id),
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
): CanvasEdgeContract | string {
  if (!connection.source || !connection.target) return "连线缺少源节点或目标节点";
  if (connection.source === connection.target) return "画布不允许节点自连";
  const contract = inferCanvasEdgeContract(connection, nodes, contractsByKind, edges);
  if (!contract) return "当前节点类型或端口不能直接连接";
  if (
    edges.some(
      (edge) =>
        edge.source === connection.source &&
        edge.target === connection.target &&
        (edge.sourceHandle ?? "") === contract.sourcePort &&
        (edge.targetHandle ?? "") === contract.targetPort,
    )
  ) {
    return "相同端口之间已经存在连线";
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
    if (hasExistingInput) return `${targetPort.label} 输入只允许一个来源`;
  }
  return contract;
}

export function canvasGraphSaveValidationError(
  nodes: StudioNode[],
  edges: StudioEdge[],
  contractsByKind: StudioNodeContractMap,
): string | null {
  const nodesById = new Map(nodes.map((node) => [node.id, node]));
  const singleInputEdges = new Set<string>();
  for (const edge of edges) {
    if (!edge.source || !edge.target) return "连线缺少源节点或目标节点";
    const contract = inferCanvasEdgeContract(edge, nodesById, contractsByKind);
    const source = nodesById.get(edge.source);
    const target = nodesById.get(edge.target);
    if (!contract || !source || !target) {
      return canvasEdgeMismatchMessage(edge, source, target);
    }
    if (
      edge.sourceHandle !== contract.sourcePort ||
      edge.targetHandle !== contract.targetPort ||
      edge.data?.role !== contract.role
    ) {
      return canvasEdgeMismatchMessage(edge, source, target);
    }
    const targetContract = studioNodeContract(contractsByKind, target.data.nodeKind);
    const targetPort = targetContract
      ? studioNodeInputPortsForData(target.data, targetContract).find((port) => port.id === contract.targetPort)
      : null;
    if (!targetPort) {
      return `连线与当前节点端口配置不匹配：${nodeDisplayTitle(source.data)} -> ${nodeDisplayTitle(target.data)}`;
    }
    if (!targetPort.multiple) {
      const key = `${target.id}:${contract.targetPort}`;
      if (singleInputEdges.has(key)) {
        return `${nodeDisplayTitle(target.data)} 的 ${targetPort.label} 输入只允许一个来源`;
      }
      singleInputEdges.add(key);
    }
  }
  return null;
}

function canvasEdgeMismatchMessage(edge: StudioEdge, source?: StudioNode, target?: StudioNode): string {
  const sourceName = source ? nodeDisplayTitle(source.data) : `节点 ${edge.source}`;
  const targetName = target ? nodeDisplayTitle(target.data) : `节点 ${edge.target}`;
  const sourceKind = source?.data.nodeKind ?? "未知类型";
  const targetKind = target?.data.nodeKind ?? "未知类型";
  const sourcePort = edge.sourceHandle ?? "-";
  const targetPort = edge.targetHandle ?? "-";
  return `连线与当前节点端口配置不匹配：${sourceName}（${sourceKind}.${sourcePort}） -> ${targetName}（${targetKind}.${targetPort}）。请删除该连线后按当前端口重新连接。`;
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
