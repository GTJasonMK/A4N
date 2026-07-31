import type { StudioEdge, StudioNode } from "./types";

export function nextAssembleClipOrder(edges: StudioEdge[], targetNodeId: string): number {
  return assembleClipEdgesForTarget(edges, targetNodeId).length + 1;
}

export function assembleClipEdges(
  assembleNodeId: string,
  nodes: StudioNode[],
  edges: StudioEdge[],
): Array<{ edge: StudioEdge; source: StudioNode; orderIndex: number }> {
  const nodesById = new Map(nodes.map((node) => [node.id, node]));
  return assembleClipEdgesForTarget(edges, assembleNodeId)
    .map((edge, index) => {
      const source = nodesById.get(edge.source);
      if (!source) return null;
      return {
        edge,
        source,
        orderIndex: edgeOrderIndex(edge) ?? index + 1,
      };
    })
    .filter((item): item is { edge: StudioEdge; source: StudioNode; orderIndex: number } => item !== null)
    .sort((left, right) => left.orderIndex - right.orderIndex);
}

function assembleClipEdgesForTarget(edges: StudioEdge[], targetNodeId: string): StudioEdge[] {
  return edges.filter(
    (edge) =>
      edge.target === targetNodeId &&
      edge.sourceHandle === "video" &&
      edge.targetHandle === "clip" &&
      edge.data?.role === "assembly_clip",
  );
}

export function reorderAssembleClipEdges(edges: StudioEdge[], edgeId: string, direction: -1 | 1): StudioEdge[] {
  const moving = edges.find((edge) => edge.id === edgeId);
  if (!moving) return edges;
  const clips = assembleClipEdgesForTarget(edges, moving.target)
    .slice()
    .sort((left, right) => (edgeOrderIndex(left) ?? 0) - (edgeOrderIndex(right) ?? 0));
  const currentIndex = clips.findIndex((edge) => edge.id === edgeId);
  const nextIndex = currentIndex + direction;
  if (currentIndex < 0 || nextIndex < 0 || nextIndex >= clips.length) return edges;
  const reordered = clips.slice();
  const [item] = reordered.splice(currentIndex, 1);
  if (!item) return edges;
  reordered.splice(nextIndex, 0, item);
  const nextOrderByEdgeId = new Map(reordered.map((edge, index) => [edge.id, index + 1]));
  return edges.map((edge) => {
    const orderIndex = nextOrderByEdgeId.get(edge.id);
    if (!orderIndex) return edge;
    return {
      ...edge,
      data: {
        role: edge.data?.role ?? "assembly_clip",
        config: {
          ...(edge.data?.config ?? {}),
          order_index: orderIndex,
        },
      },
    };
  });
}

export function edgeOrderIndex(edge: StudioEdge): number | null {
  const value = edge.data?.config?.order_index;
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0 ? value : null;
}
