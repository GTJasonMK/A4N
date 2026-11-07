import type {
  AssetDependencyGraph,
  AssetGraphEdge,
  AssetGraphNode,
} from "../../../api/project";
import type { MessageKey, MessageTranslator } from "../../../i18n";
import { readCssRgbVariable, rgb, rgba, type Rgb } from "../../../utils/cssColor";

export interface PositionedNode extends AssetGraphNode {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface PositionedLayer {
  index: number;
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
  nodeCount: number;
  assetCount: number;
}

interface GraphLayout {
  nodes: PositionedNode[];
  layers: PositionedLayer[];
  width: number;
  height: number;
}

interface ReadyNode {
  id: string;
  sequence: number;
}

interface NodeCluster {
  key: string;
  order: number;
}

interface Point {
  x: number;
  y: number;
}

interface GraphTheme {
  isDark: boolean;
  primary: Rgb;
  accent: Rgb;
  bg: Rgb;
  paper: Rgb;
  tertiary: Rgb;
  border: Rgb;
  borderDark: Rgb;
  text: Rgb;
  muted: Rgb;
  subtle: Rgb;
  success: Rgb;
  warning: Rgb;
  info: Rgb;
}

const NODE_WIDTH = 158;
const NODE_HEIGHT = 60;
const NETWORK_PADDING = 40;
const NETWORK_MIN_WIDTH = 820;
const NETWORK_MIN_HEIGHT = 560;
const NETWORK_SLOT_WIDTH = NODE_WIDTH + 18;
const NETWORK_SLOT_HEIGHT = NODE_HEIGHT + 16;

export function buildLayout(graph: AssetDependencyGraph | null, t: MessageTranslator): GraphLayout {
  if (!graph) return { nodes: [], layers: [], width: NETWORK_MIN_WIDTH, height: NETWORK_MIN_HEIGHT };
  const degreeById = nodeDegreeMap(graph.nodes, graph.edges);
  const visibleNodes = graph.nodes.filter((node) => node.id === "project" || node.count > 0);
  const visibleNodeIds = new Set(visibleNodes.map((node) => node.id));
  const visibleEdges = graph.edges.filter((edge) => visibleNodeIds.has(edge.from) && visibleNodeIds.has(edge.to));
  const layoutEdges = visibleEdges.filter(isPrimaryLayoutEdge);
  const nodesById = new Map(visibleNodes.map((node) => [node.id, node]));
  const clusterById = new Map(visibleNodes.map((node) => [node.id, nodeCluster(node)]));
  if (!visibleNodes.length) {
    return { nodes: [], layers: [], width: NETWORK_MIN_WIDTH, height: NETWORK_MIN_HEIGHT };
  }
  const incoming = new Map<string, string[]>();
  const outgoing = new Map<string, string[]>();
  layoutEdges.forEach((edge) => {
    const inList = incoming.get(edge.to) ?? [];
    inList.push(edge.from);
    incoming.set(edge.to, inList);
    const outList = outgoing.get(edge.from) ?? [];
    outList.push(edge.to);
    outgoing.set(edge.from, outList);
  });

  const layers = topologyDepthLayers(visibleNodes, layoutEdges, nodesById, degreeById, clusterById, t);
  const orderedLayerIds = orderLayerNodes(layers, nodesById, degreeById, incoming, outgoing, clusterById);
  const nodeCount = Math.max(1, visibleNodes.length);
  const maxLayerCount = Math.max(1, ...orderedLayerIds.map((ids) => ids.length));
  const columns = Math.max(
    4,
    Math.min(10, Math.max(Math.ceil(Math.sqrt(nodeCount * 1.55)), Math.min(maxLayerCount, 8))),
  );
  const width = Math.max(NETWORK_MIN_WIDTH, columns * NETWORK_SLOT_WIDTH + NETWORK_PADDING * 2);
  const positioned: PositionedNode[] = [];
  const positionedLayers: PositionedLayer[] = [];
  let cursorY = NETWORK_PADDING;

  layers.forEach((layer, layerIndex) => {
    const nodeIds = orderedLayerIds[layerIndex] ?? [];
    const rows = Math.max(1, Math.ceil(nodeIds.length / columns));
    const bandHeight = 28 + rows * NETWORK_SLOT_HEIGHT + 14;
    const nodeTop = cursorY + 28;
    nodeIds.forEach((nodeId, nodeIndex) => {
      const node = nodesById.get(nodeId);
      if (!node) return;
      const row = Math.floor(nodeIndex / columns);
      const itemsInRow = Math.min(columns, nodeIds.length - row * columns);
      const column = nodeIndex % columns;
      const rowOffset = ((columns - itemsInRow) * NETWORK_SLOT_WIDTH) / 2;
      positioned.push({
        ...node,
        layer: layer.index,
        x: NETWORK_PADDING + rowOffset + column * NETWORK_SLOT_WIDTH + (NETWORK_SLOT_WIDTH - NODE_WIDTH) / 2,
        y: nodeTop + row * NETWORK_SLOT_HEIGHT + (NETWORK_SLOT_HEIGHT - NODE_HEIGHT) / 2,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
      });
    });
    const layerNodes = nodeIds
      .map((nodeId) => nodesById.get(nodeId))
      .filter((node): node is AssetGraphNode => Boolean(node));
    positionedLayers.push({
      index: layer.index,
      label: layer.label,
      x: NETWORK_PADDING / 2,
      y: cursorY,
      width: width - NETWORK_PADDING,
      height: bandHeight,
      nodeCount: layerNodes.length,
      assetCount: layerNodes.reduce((sum, node) => sum + node.count, 0),
    });
    cursorY += bandHeight + 18;
  });
  const height = Math.max(NETWORK_MIN_HEIGHT, cursorY + NETWORK_PADDING - 18);

  return {
    nodes: positioned,
    layers: positionedLayers,
    width,
    height,
  };
}

function topologyDepthLayers(
  nodes: AssetGraphNode[],
  edges: AssetGraphEdge[],
  nodesById: Map<string, AssetGraphNode>,
  degreeById: Map<string, number>,
  clusterById: Map<string, NodeCluster>,
  t: MessageTranslator,
): AssetDependencyGraph["layers"] {
  const nodeIds = new Set(nodes.map((node) => node.id));
  const incomingCount = new Map(nodes.map((node) => [node.id, 0]));
  const outgoing = new Map<string, string[]>();
  edges.forEach((edge) => {
    if (!nodeIds.has(edge.from) || !nodeIds.has(edge.to)) return;
    incomingCount.set(edge.to, (incomingCount.get(edge.to) ?? 0) + 1);
    const list = outgoing.get(edge.from) ?? [];
    list.push(edge.to);
    outgoing.set(edge.from, list);
  });

  const depthById = new Map(nodes.map((node) => [node.id, 0]));
  const remainingIncoming = new Map(incomingCount);
  const ready: ReadyNode[] = [];
  let readySequence = 0;
  for (const node of nodes) {
    if ((incomingCount.get(node.id) ?? 0) !== 0) continue;
    pushReadyNode(ready, { id: node.id, sequence: readySequence++ }, nodesById, degreeById, clusterById);
  }
  const visited = new Set<string>();

  while (ready.length) {
    const nodeId = popReadyNode(ready, nodesById, degreeById, clusterById);
    if (!nodeId || visited.has(nodeId)) continue;
    visited.add(nodeId);
    (outgoing.get(nodeId) ?? []).forEach((childId) => {
      depthById.set(childId, Math.max(depthById.get(childId) ?? 0, (depthById.get(nodeId) ?? 0) + 1));
      const nextIncoming = (remainingIncoming.get(childId) ?? 0) - 1;
      remainingIncoming.set(childId, nextIncoming);
      if (nextIncoming === 0) {
        pushReadyNode(
          ready,
          { id: childId, sequence: readySequence++ },
          nodesById,
          degreeById,
          clusterById,
        );
      }
    });
  }

  nodes
    .filter((node) => !visited.has(node.id))
    .sort((a, b) => compareNodePriority(a, b))
    .forEach((node) => {
      depthById.set(node.id, Math.max(depthById.get(node.id) ?? 0, node.layer));
    });

  const idsByDepth = new Map<number, string[]>();
  nodes.forEach((node) => {
    const depth = depthById.get(node.id) ?? 0;
    const list = idsByDepth.get(depth) ?? [];
    list.push(node.id);
    idsByDepth.set(depth, list);
  });

  return [...idsByDepth.entries()]
    .sort(([a], [b]) => a - b)
    .map(([depth, ids]) => ({
      index: depth,
      label: depth === 0 ? t("novel.assetGraph.rootNode") : t("novel.assetGraph.depth", { depth }),
      node_ids: ids,
  }));
}

function pushReadyNode(
  ready: ReadyNode[],
  node: ReadyNode,
  nodesById: Map<string, AssetGraphNode>,
  degreeById: Map<string, number>,
  clusterById: Map<string, NodeCluster>,
): void {
  ready.push(node);
  let index = ready.length - 1;
  while (index > 0) {
    const parent = Math.floor((index - 1) / 2);
    if (!readyNodeComesFirst(ready[index], ready[parent], nodesById, degreeById, clusterById)) break;
    [ready[index], ready[parent]] = [ready[parent], ready[index]];
    index = parent;
  }
}

function popReadyNode(
  ready: ReadyNode[],
  nodesById: Map<string, AssetGraphNode>,
  degreeById: Map<string, number>,
  clusterById: Map<string, NodeCluster>,
): string | undefined {
  const first = ready[0];
  const last = ready.pop();
  if (ready.length > 0 && last) {
    ready[0] = last;
    let index = 0;
    while (true) {
      const left = index * 2 + 1;
      if (left >= ready.length) break;
      const right = left + 1;
      const child =
        right < ready.length &&
        readyNodeComesFirst(ready[right], ready[left], nodesById, degreeById, clusterById)
          ? right
          : left;
      if (!readyNodeComesFirst(ready[child], ready[index], nodesById, degreeById, clusterById)) break;
      [ready[index], ready[child]] = [ready[child], ready[index]];
      index = child;
    }
  }
  return first?.id;
}

function readyNodeComesFirst(
  left: ReadyNode,
  right: ReadyNode,
  nodesById: Map<string, AssetGraphNode>,
  degreeById: Map<string, number>,
  clusterById: Map<string, NodeCluster>,
): boolean {
  const priority = compareLayerNodeIds(left.id, right.id, nodesById, degreeById, clusterById);
  return priority < 0 || (priority === 0 && left.sequence < right.sequence);
}

function isPrimaryLayoutEdge(edge: AssetGraphEdge): boolean {
  return edge.relation !== "references";
}

function orderLayerNodes(
  layers: AssetDependencyGraph["layers"],
  nodesById: Map<string, AssetGraphNode>,
  degreeById: Map<string, number>,
  incoming: Map<string, string[]>,
  outgoing: Map<string, string[]>,
  clusterById: Map<string, NodeCluster>,
): string[][] {
  const ordered = layers.map((layer) =>
    [...layer.node_ids].sort((a, b) => compareLayerNodeIds(a, b, nodesById, degreeById, clusterById)),
  );

  for (let pass = 0; pass < 5; pass += 1) {
    const forwardOrder = orderIndexMap(ordered);
    for (let layerIndex = 1; layerIndex < ordered.length; layerIndex += 1) {
      ordered[layerIndex].sort((a, b) =>
        compareByBarycenter(a, b, incoming, forwardOrder, nodesById, degreeById, clusterById),
      );
    }

    const backwardOrder = orderIndexMap(ordered);
    for (let layerIndex = ordered.length - 2; layerIndex >= 0; layerIndex -= 1) {
      ordered[layerIndex].sort((a, b) =>
        compareByBarycenter(a, b, outgoing, backwardOrder, nodesById, degreeById, clusterById),
      );
    }
  }

  return ordered;
}

function orderIndexMap(layers: string[][]): Map<string, number> {
  const map = new Map<string, number>();
  layers.forEach((nodeIds) => {
    nodeIds.forEach((nodeId, index) => map.set(nodeId, index));
  });
  return map;
}

function compareByBarycenter(
  a: string,
  b: string,
  neighbors: Map<string, string[]>,
  orderMap: Map<string, number>,
  nodesById: Map<string, AssetGraphNode>,
  degreeById: Map<string, number>,
  clusterById: Map<string, NodeCluster>,
): number {
  const aCenter = neighborBarycenter(a, neighbors, orderMap);
  const bCenter = neighborBarycenter(b, neighbors, orderMap);
  if (aCenter != null && bCenter != null && Math.abs(aCenter - bCenter) > 0.001) {
    return aCenter - bCenter;
  }
  if (aCenter != null) return -1;
  if (bCenter != null) return 1;
  const clusterCompare = compareNodeClusters(a, b, clusterById);
  if (clusterCompare !== 0) return clusterCompare;
  return compareLayerNodeIds(a, b, nodesById, degreeById, clusterById);
}

function neighborBarycenter(
  nodeId: string,
  neighbors: Map<string, string[]>,
  orderMap: Map<string, number>,
): number | null {
  const positions = (neighbors.get(nodeId) ?? [])
    .map((id) => orderMap.get(id))
    .filter((value): value is number => value != null);
  if (!positions.length) return null;
  return positions.reduce((sum, value) => sum + value, 0) / positions.length;
}

function compareLayerNodeIds(
  a: string,
  b: string,
  nodesById: Map<string, AssetGraphNode>,
  degreeById: Map<string, number>,
  clusterById: Map<string, NodeCluster>,
): number {
  const clusterCompare = compareNodeClusters(a, b, clusterById);
  if (clusterCompare !== 0) return clusterCompare;
  const aDegree = degreeById.get(a) ?? 0;
  const bDegree = degreeById.get(b) ?? 0;
  if (aDegree !== bDegree) return bDegree - aDegree;
  return compareNodePriority(nodesById.get(a), nodesById.get(b));
}

function compareNodeClusters(
  a: string,
  b: string,
  clusterById: Map<string, NodeCluster>,
): number {
  const aCluster = clusterById.get(a) ?? { key: "unknown", order: 99 };
  const bCluster = clusterById.get(b) ?? { key: "unknown", order: 99 };
  if (aCluster.order !== bCluster.order) return aCluster.order - bCluster.order;
  return aCluster.key.localeCompare(bCluster.key, "zh-Hans-CN");
}

function nodeCluster(node?: AssetGraphNode): NodeCluster {
  if (!node) return { key: "unknown", order: 99 };
  const value = `${node.id} ${node.asset_type} ${node.label} ${node.summary ?? ""}`.toLowerCase();
  if (value.includes("project")) return { key: "project", order: 0 };
  if (value.includes("inspiration")) return { key: "inspiration", order: 1 };
  if (value.includes("blueprint")) return { key: "blueprint", order: 2 };
  if (value.includes("chapter_outline") || value.includes("part_outline") || value.includes("outline")) {
    return { key: "outline", order: 3 };
  }
  if (value.includes("chapter_analysis") || value.includes("chapter_content") || value.includes("chapter")) {
    return { key: "chapter", order: 4 };
  }
  if (value.includes("character_portrait") || value.includes("portrait") || value.includes("立绘")) {
    return { key: "portrait", order: 5 };
  }
  if (value.includes("character_profile") || value.includes("character") || value.includes("人物")) {
    return { key: "character", order: 6 };
  }
  if (value.includes("scene_profile") || value.includes("scene") || value.includes("场景")) {
    return { key: "scene", order: 7 };
  }
  if (value.includes("object_profile") || value.includes("object") || value.includes("物体")) {
    return { key: "object", order: 8 };
  }
  if (value.includes("manga") || value.includes("comic") || value.includes("漫画")) {
    return { key: "manga", order: 9 };
  }
  if (value.includes("image") || value.includes("图片")) return { key: "image", order: 10 };
  if (value.includes("video") || value.includes("视频")) return { key: "video", order: 11 };
  if (value.includes("profile")) return { key: "profile", order: 12 };
  return { key: node.kind || node.asset_type || "other", order: 50 };
}

function nodeDegreeMap(nodes: AssetGraphNode[], edges: AssetGraphEdge[]): Map<string, number> {
  const degree = new Map(nodes.map((node) => [node.id, 0]));
  edges.forEach((edge) => {
    const weight = Math.max(1, edge.count);
    degree.set(edge.from, (degree.get(edge.from) ?? 0) + weight);
    degree.set(edge.to, (degree.get(edge.to) ?? 0) + weight);
  });
  return degree;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function nodeCenter(node: PositionedNode): { x: number; y: number } {
  return {
    x: node.x + node.width / 2,
    y: node.y + node.height / 2,
  };
}

function hashUnit(value: string): number {
  let hash = 2166136261;
  for (let i = 0; i < value.length; i += 1) {
    hash ^= value.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0) / 4294967295;
}

function compareNodePriority(a?: AssetGraphNode, b?: AssetGraphNode): number {
  if (!a && !b) return 0;
  if (!a) return 1;
  if (!b) return -1;
  const aPending = a.needs_visualization && !a.visualized ? 1 : 0;
  const bPending = b.needs_visualization && !b.visualized ? 1 : 0;
  if (aPending !== bPending) return bPending - aPending;
  if (a.downstream_count !== b.downstream_count) return b.downstream_count - a.downstream_count;
  if (a.direct_downstream_count !== b.direct_downstream_count) {
    return b.direct_downstream_count - a.direct_downstream_count;
  }
  if (a.count !== b.count) return b.count - a.count;
  return a.label.localeCompare(b.label, "zh-Hans-CN");
}

export function drawGraph(
  canvas: HTMLCanvasElement,
  layout: GraphLayout,
  edges: AssetGraphEdge[],
  selectedId: string | null,
  t: MessageTranslator,
) {
  const theme = readGraphTheme();
  const ratio = window.devicePixelRatio || 1;
  canvas.width = Math.round(layout.width * ratio);
  canvas.height = Math.round(layout.height * ratio);
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.clearRect(0, 0, layout.width, layout.height);
  ctx.fillStyle = rgb(theme.bg);
  ctx.fillRect(0, 0, layout.width, layout.height);

  const nodeById = new Map(layout.nodes.map((node) => [node.id, node]));
  const layerOrderByIndex = new Map(layout.layers.map((layer, index) => [layer.index, index]));
  const relatedIds = selectedId ? relatedNodeIds(selectedId, edges) : new Set<string>();
  drawGraphBackdrop(ctx, layout, theme);
  drawLayerHeaders(ctx, layout, theme);

  const selectedEdges = edges.filter((edge) => selectedId === edge.from || selectedId === edge.to);
  const mutedEdges = edges.filter((edge) => selectedId !== edge.from && selectedId !== edge.to);
  mutedEdges.forEach((edge) => {
    const from = nodeById.get(edge.from);
    const to = nodeById.get(edge.to);
    if (!from || !to) return;
    drawEdge(ctx, layout, from, to, edge, false, theme, layerOrderByIndex);
  });
  selectedEdges.forEach((edge) => {
    const from = nodeById.get(edge.from);
    const to = nodeById.get(edge.to);
    if (!from || !to) return;
    drawEdge(ctx, layout, from, to, edge, true, theme, layerOrderByIndex);
  });

  layout.nodes.forEach((node) => {
    const selected = node.id === selectedId;
    const related = !selectedId || selected || relatedIds.has(node.id);
    drawNode(ctx, node, selected, related, theme, t);
  });
}

function drawGraphBackdrop(ctx: CanvasRenderingContext2D, layout: GraphLayout, theme: GraphTheme) {
  const gridSize = 24;
  ctx.save();
  ctx.strokeStyle = rgba(theme.border, theme.isDark ? 0.18 : 0.3);
  ctx.lineWidth = 1;
  for (let x = 0.5; x < layout.width; x += gridSize) {
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, layout.height);
    ctx.stroke();
  }
  for (let y = 0.5; y < layout.height; y += gridSize) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(layout.width, y);
    ctx.stroke();
  }
  ctx.restore();
}

function drawLayerHeaders(ctx: CanvasRenderingContext2D, layout: GraphLayout, theme: GraphTheme) {
  ctx.save();
  layout.layers.forEach((layer, index) => {
    const labelX = layer.x + 10;
    const labelY = layer.y + 7;

    ctx.beginPath();
    ctx.moveTo(layer.x + 170, labelY + 10.5);
    ctx.lineTo(layer.x + layer.width - 12, labelY + 10.5);
    ctx.strokeStyle = rgba(theme.borderDark, theme.isDark ? 0.24 : 0.32);
    ctx.lineWidth = 1;
    ctx.stroke();

    roundRect(ctx, labelX, labelY, 146, 20, 4);
    ctx.fillStyle = rgba(theme.paper, index % 2 === 0 ? (theme.isDark ? 0.72 : 0.88) : (theme.isDark ? 0.58 : 0.78));
    ctx.fill();
    ctx.strokeStyle = rgba(theme.borderDark, theme.isDark ? 0.42 : 0.5);
    ctx.stroke();

    ctx.fillStyle = rgb(theme.muted);
    ctx.font = "600 11px 'Noto Sans SC', sans-serif";
    drawText(ctx, layer.label, labelX + 8, labelY + 14, 92);
    ctx.font = "600 10px 'JetBrains Mono', monospace";
    ctx.textAlign = "right";
    ctx.fillStyle = rgba(theme.muted, 0.88);
    ctx.fillText(`${layer.nodeCount}/${layer.assetCount}`, labelX + 138, labelY + 14);
    ctx.textAlign = "left";
  });
  ctx.restore();
}

function drawEdge(
  ctx: CanvasRenderingContext2D,
  layout: GraphLayout,
  from: PositionedNode,
  to: PositionedNode,
  edge: AssetGraphEdge,
  selected: boolean,
  theme: GraphTheme,
  layerOrderByIndex: Map<number, number>,
) {
  const fromCenter = nodeCenter(from);
  const toCenter = nodeCenter(to);
  const start = rectBoundaryPoint(from, toCenter);
  const end = rectBoundaryPoint(to, fromCenter);
  const route = buildEdgeRoute(layout, from, to, edge, start, end, layerOrderByIndex);
  if (route.length > 2) {
    drawSteppedEdge(ctx, route, edge, selected, theme);
    return;
  }

  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const distance = Math.max(1, Math.hypot(dx, dy));
  const curveSign = hashUnit(`${edge.from}->${edge.to}`) > 0.5 ? 1 : -1;
  const bend = Math.min(46, distance * 0.18) * curveSign;
  const controlX = (start.x + end.x) / 2 - (dy / distance) * bend;
  const controlY = (start.y + end.y) / 2 + (dx / distance) * bend;
  const stroke = selected ? theme.primary : theme.borderDark;

  ctx.save();
  ctx.beginPath();
  ctx.moveTo(start.x, start.y);
  ctx.quadraticCurveTo(controlX, controlY, end.x, end.y);
  ctx.lineWidth = selected ? 2.6 : 1.15;
  ctx.strokeStyle = rgba(stroke, selected ? 0.86 : theme.isDark ? 0.34 : 0.42);
  if (!selected) ctx.setLineDash([8, 8]);
  ctx.stroke();
  ctx.setLineDash([]);

  const arrowAngle = Math.atan2(end.y - controlY, end.x - controlX);
  drawArrowHead(ctx, end.x, end.y, arrowAngle, selected ? 8 : 6, stroke, selected ? 0.9 : 0.48);

  if (selected && edge.count > 1) {
    const labelX = (start.x + 2 * controlX + end.x) / 4;
    const labelY = (start.y + 2 * controlY + end.y) / 4;
    const label = String(edge.count);
    ctx.font = "700 10px 'JetBrains Mono', monospace";
    const width = Math.max(20, ctx.measureText(label).width + 10);
    roundRect(ctx, labelX - width / 2, labelY - 8, width, 16, 8);
    ctx.fillStyle = rgb(theme.paper);
    ctx.fill();
    ctx.strokeStyle = rgba(theme.primary, 0.68);
    ctx.stroke();
    ctx.fillStyle = rgb(theme.primary);
    ctx.textAlign = "center";
    ctx.fillText(label, labelX, labelY + 3);
    ctx.textAlign = "left";
  }
  ctx.restore();
}

function buildEdgeRoute(
  layout: GraphLayout,
  from: PositionedNode,
  to: PositionedNode,
  edge: AssetGraphEdge,
  start: Point,
  end: Point,
  layerOrderByIndex: Map<number, number>,
): Point[] {
  const fromLayerOrder = layerOrderByIndex.get(from.layer) ?? -1;
  const toLayerOrder = layerOrderByIndex.get(to.layer) ?? -1;
  if (fromLayerOrder < 0 || toLayerOrder < 0 || Math.abs(toLayerOrder - fromLayerOrder) <= 1) {
    return [start, end];
  }

  const direction = toLayerOrder > fromLayerOrder ? 1 : -1;
  const span = Math.abs(toLayerOrder - fromLayerOrder);
  const route: Point[] = [start];
  const edgeOffset = (hashUnit(`${edge.from}->${edge.to}:route`) - 0.5) * 58;

  for (let order = fromLayerOrder; order !== toLayerOrder; order += direction) {
    const currentLayer = layout.layers[order];
    const nextLayer = layout.layers[order + direction];
    if (!currentLayer || !nextLayer) break;
    const gapY = direction > 0
      ? (currentLayer.y + currentLayer.height + nextLayer.y) / 2
      : (nextLayer.y + nextLayer.height + currentLayer.y) / 2;
    const progress = Math.abs(order + direction - fromLayerOrder) / span;
    const offsetStrength = 1 - Math.abs(progress - 0.5) * 1.35;
    route.push({
      x: clamp(
        start.x + (end.x - start.x) * progress + edgeOffset * Math.max(0.25, offsetStrength),
        NETWORK_PADDING,
        layout.width - NETWORK_PADDING,
      ),
      y: gapY,
    });
  }

  route.push(end);
  return route;
}

function drawSteppedEdge(
  ctx: CanvasRenderingContext2D,
  route: Point[],
  edge: AssetGraphEdge,
  selected: boolean,
  theme: GraphTheme,
) {
  if (route.length < 2) return;
  const stroke = selected ? theme.primary : theme.borderDark;
  ctx.save();
  ctx.beginPath();
  ctx.moveTo(route[0].x, route[0].y);
  route.slice(1).forEach((point) => ctx.lineTo(point.x, point.y));
  ctx.lineWidth = selected ? 2.4 : 1.1;
  ctx.lineJoin = "round";
  ctx.lineCap = "round";
  ctx.strokeStyle = rgba(stroke, selected ? 0.82 : theme.isDark ? 0.32 : 0.4);
  if (!selected) ctx.setLineDash([7, 7]);
  ctx.stroke();
  ctx.setLineDash([]);

  route.slice(1, -1).forEach((point) => {
    ctx.beginPath();
    ctx.arc(point.x, point.y, selected ? 3.4 : 2.5, 0, Math.PI * 2);
    ctx.fillStyle = rgb(theme.paper);
    ctx.fill();
    ctx.strokeStyle = rgba(stroke, selected ? 0.76 : 0.46);
    ctx.lineWidth = selected ? 1.4 : 1;
    ctx.stroke();
  });

  const previous = route[route.length - 2];
  const end = route[route.length - 1];
  const arrowAngle = Math.atan2(end.y - previous.y, end.x - previous.x);
  drawArrowHead(ctx, end.x, end.y, arrowAngle, selected ? 8 : 6, stroke, selected ? 0.9 : 0.48);

  if (selected && edge.count > 1) {
    const labelIndex = Math.max(1, Math.floor(route.length / 2));
    const labelPoint = route[labelIndex];
    drawEdgeCountLabel(ctx, labelPoint.x, labelPoint.y - 10, String(edge.count), theme);
  }
  ctx.restore();
}

function drawEdgeCountLabel(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  label: string,
  theme: GraphTheme,
) {
  ctx.font = "700 10px 'JetBrains Mono', monospace";
  const width = Math.max(20, ctx.measureText(label).width + 10);
  roundRect(ctx, x - width / 2, y - 8, width, 16, 8);
  ctx.fillStyle = rgb(theme.paper);
  ctx.fill();
  ctx.strokeStyle = rgba(theme.primary, 0.68);
  ctx.stroke();
  ctx.fillStyle = rgb(theme.primary);
  ctx.textAlign = "center";
  ctx.fillText(label, x, y + 3);
  ctx.textAlign = "left";
}

function rectBoundaryPoint(node: PositionedNode, toward: { x: number; y: number }): { x: number; y: number } {
  const center = nodeCenter(node);
  const dx = toward.x - center.x;
  const dy = toward.y - center.y;
  if (Math.abs(dx) < 0.01 && Math.abs(dy) < 0.01) return center;
  const halfWidth = node.width / 2;
  const halfHeight = node.height / 2;
  const scale = Math.min(
    halfWidth / Math.max(0.01, Math.abs(dx)),
    halfHeight / Math.max(0.01, Math.abs(dy)),
  );
  return {
    x: center.x + dx * scale,
    y: center.y + dy * scale,
  };
}

function drawArrowHead(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  angle: number,
  size: number,
  color: Rgb,
  alpha: number,
) {
  ctx.save();
  ctx.translate(x, y);
  ctx.rotate(angle);
  ctx.beginPath();
  ctx.moveTo(0, 0);
  ctx.lineTo(-size, -size * 0.55);
  ctx.lineTo(-size, size * 0.55);
  ctx.closePath();
  ctx.fillStyle = rgba(color, alpha);
  ctx.fill();
  ctx.restore();
}

function drawNode(
  ctx: CanvasRenderingContext2D,
  node: PositionedNode,
  selected: boolean,
  related: boolean,
  theme: GraphTheme,
  t: MessageTranslator,
) {
  const accent = kindAccent(node.kind, theme);
  const status = node.mutable
    ? { text: t("novel.assetGraph.editable"), color: theme.success }
    : node.downstream_count > 0
      ? { text: t("novel.assetGraph.impact", { count: node.downstream_count }), color: theme.warning }
      : { text: t("novel.assetGraph.readOnly"), color: theme.muted };

  ctx.save();
  ctx.globalAlpha = related ? 1 : 0.52;
  ctx.shadowColor = theme.isDark ? "rgba(0,0,0,0.32)" : "rgba(92,62,34,0.12)";
  ctx.shadowBlur = selected ? 12 : 4;
  ctx.shadowOffsetY = selected ? 3 : 1;
  roundRect(ctx, node.x, node.y, node.width, node.height, 6);
  ctx.fillStyle = selected ? rgba(theme.tertiary, theme.isDark ? 0.82 : 0.58) : rgb(theme.paper);
  ctx.fill();
  ctx.shadowColor = "transparent";
  ctx.lineWidth = selected ? 2 : 1;
  ctx.strokeStyle = selected ? rgb(theme.primary) : rgba(theme.borderDark, theme.isDark ? 0.78 : 0.66);
  ctx.stroke();

  roundRect(ctx, node.x, node.y, 6, node.height, 6);
  ctx.fillStyle = rgba(accent, selected ? 0.96 : 0.78);
  ctx.fill();

  ctx.fillStyle = rgb(theme.text);
  ctx.font = "700 13px 'Noto Sans SC', sans-serif";
  drawText(ctx, node.label, node.x + 16, node.y + 20, node.width - 66);

  ctx.font = "500 10px 'JetBrains Mono', 'Noto Sans SC', monospace";
  ctx.fillStyle = rgb(theme.muted);
  drawText(
    ctx,
    `${kindLabel(node.kind, t)} · ${assetTypeLabel(node.asset_type, t)}`,
    node.x + 16,
    node.y + 38,
    node.width - 34,
  );

  drawStatusPill(ctx, node.x + 16, node.y + 50, status.text, status.color, theme);
  if (node.needs_visualization) {
    const text = node.visualized
      ? t("novel.assetGraph.visualized")
      : t("novel.assetGraph.visualizationPending");
    const color = node.visualized ? theme.success : theme.warning;
    drawStatusPill(ctx, node.x + 90, node.y + 50, text, color, theme);
  }

  ctx.font = "700 13px 'JetBrains Mono', monospace";
  ctx.fillStyle = rgb(accent);
  ctx.textAlign = "right";
  ctx.fillText(String(node.count), node.x + node.width - 14, node.y + 21);
  ctx.textAlign = "left";
  ctx.restore();
}

function drawStatusPill(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  text: string,
  color: Rgb,
  theme: GraphTheme,
) {
  ctx.save();
  ctx.font = "600 10px 'Noto Sans SC', sans-serif";
  const width = Math.min(72, Math.max(42, ctx.measureText(text).width + 12));
  roundRect(ctx, x, y - 9, width, 16, 4);
  ctx.fillStyle = rgba(color, theme.isDark ? 0.16 : 0.1);
  ctx.fill();
  ctx.strokeStyle = rgba(color, 0.38);
  ctx.stroke();
  ctx.fillStyle = rgb(color);
  drawText(ctx, text, x + 6, y + 3, width - 12);
  ctx.restore();
}

export function hitTestNode(nodes: PositionedNode[], x: number, y: number): PositionedNode | undefined {
  for (let index = nodes.length - 1; index >= 0; index -= 1) {
    const node = nodes[index];
    if (
      x >= node.x &&
      x <= node.x + node.width &&
      y >= node.y &&
      y <= node.y + node.height
    ) {
      return node;
    }
  }
  return undefined;
}

function relatedNodeIds(selectedId: string, edges: AssetGraphEdge[]): Set<string> {
  const ids = new Set<string>([selectedId]);
  edges.forEach((edge) => {
    if (edge.from === selectedId) ids.add(edge.to);
    if (edge.to === selectedId) ids.add(edge.from);
  });
  return ids;
}

function kindAccent(kind: string, theme: GraphTheme): Rgb {
  if (kind === "prompt") return theme.info;
  if (kind === "image") return theme.success;
  if (kind === "video") return theme.accent;
  if (kind === "media") return theme.muted;
  return theme.primary;
}

function readGraphTheme(): GraphTheme {
  const style = getComputedStyle(document.documentElement);
  return {
    isDark: document.documentElement.classList.contains("dark"),
    primary: readCssRgbVariable(style, "--color-primary", [151, 72, 29]),
    accent: readCssRgbVariable(style, "--color-accent", [183, 116, 61]),
    bg: readCssRgbVariable(style, "--color-bg-primary", [241, 238, 232]),
    paper: readCssRgbVariable(style, "--color-bg-secondary", [255, 252, 245]),
    tertiary: readCssRgbVariable(style, "--color-bg-tertiary", [234, 228, 219]),
    border: readCssRgbVariable(style, "--color-border", [205, 194, 181]),
    borderDark: readCssRgbVariable(style, "--color-border-dark", [182, 166, 150]),
    text: readCssRgbVariable(style, "--color-text-primary", [44, 24, 16]),
    muted: readCssRgbVariable(style, "--color-text-tertiary", [109, 101, 96]),
    subtle: readCssRgbVariable(style, "--color-text-disabled", [176, 168, 160]),
    success: readCssRgbVariable(style, "--status-success", [38, 120, 73]),
    warning: readCssRgbVariable(style, "--status-warning", [132, 84, 18]),
    info: readCssRgbVariable(style, "--status-info", [38, 103, 139]),
  };
}

function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) {
  const safeRadius = Math.min(radius, width / 2, height / 2);
  ctx.beginPath();
  ctx.moveTo(x + safeRadius, y);
  ctx.lineTo(x + width - safeRadius, y);
  ctx.quadraticCurveTo(x + width, y, x + width, y + safeRadius);
  ctx.lineTo(x + width, y + height - safeRadius);
  ctx.quadraticCurveTo(x + width, y + height, x + width - safeRadius, y + height);
  ctx.lineTo(x + safeRadius, y + height);
  ctx.quadraticCurveTo(x, y + height, x, y + height - safeRadius);
  ctx.lineTo(x, y + safeRadius);
  ctx.quadraticCurveTo(x, y, x + safeRadius, y);
}

function drawText(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  maxWidth: number,
) {
  let value = text;
  while (value.length > 1 && ctx.measureText(value).width > maxWidth) {
    value = value.slice(0, -1);
  }
  ctx.fillText(value === text ? value : `${value.slice(0, -1)}…`, x, y);
}

const KIND_LABEL_KEYS: Record<string, MessageKey> = {
  text: "novel.assetGraph.kind.text",
  prompt: "novel.assetGraph.kind.prompt",
  image: "novel.assetGraph.kind.image",
  video: "novel.assetGraph.kind.video",
  media: "novel.assetGraph.kind.media",
  profile: "novel.assetGraph.kind.profile",
  task: "novel.assetGraph.kind.task",
};

export function kindLabel(kind: string, t: MessageTranslator): string {
  return t(KIND_LABEL_KEYS[kind] ?? "novel.assetGraph.kind.asset");
}

const ASSET_TYPE_LABELS: Record<string, MessageKey> = {
  project: "novel.assetGraph.assetTypes.project",
  inspiration_conversation: "novel.assetGraph.assetTypes.inspirationConversation",
  novel_blueprint: "novel.assetGraph.assetTypes.novelBlueprint",
  blueprint_version: "novel.assetGraph.assetTypes.blueprintVersion",
  part_outline: "novel.assetGraph.assetTypes.partOutline",
  chapter_outline: "novel.assetGraph.assetTypes.chapterOutline",
  chapter_content: "novel.assetGraph.assetTypes.chapterContent",
  chapter_analysis: "novel.assetGraph.assetTypes.chapterAnalysis",
  character_profile: "novel.assetGraph.assetTypes.characterProfile",
  object_profile: "novel.assetGraph.assetTypes.objectProfile",
  scene_profile: "novel.assetGraph.assetTypes.sceneProfile",
  chapter_manga_prompt: "novel.assetGraph.assetTypes.chapterMangaPrompt",
  manga_page_image: "novel.assetGraph.assetTypes.mangaPageImage",
  chapter_shooting_script: "novel.assetGraph.assetTypes.chapterShootingScript",
  video_segment_plan: "novel.assetGraph.assetTypes.videoSegmentPlan",
  video_frame_prompt: "novel.assetGraph.assetTypes.videoFramePrompt",
  video_frame_image: "novel.assetGraph.assetTypes.videoFrameImage",
  chapter_video_prompt: "novel.assetGraph.assetTypes.chapterVideoPrompt",
  generated_video: "novel.assetGraph.assetTypes.generatedVideo",
  generation_spec: "novel.assetGraph.assetTypes.generationSpec",
  prompt_artifact: "novel.assetGraph.assetTypes.promptArtifact",
  media_asset: "novel.assetGraph.assetTypes.mediaAsset",
  derived_project: "novel.assetGraph.assetTypes.derivedProject",
};

export function assetTypeLabel(value: string | null | undefined, t: MessageTranslator): string {
  const normalized = String(value ?? "").trim().toLowerCase();
  return t(ASSET_TYPE_LABELS[normalized] ?? "novel.assetGraph.assetTypes.relatedAsset");
}

export function isVideoChainNode(node: AssetGraphNode): boolean {
  return node.asset_type.includes("video");
}

export function primaryAssetIndexNodes<T extends AssetGraphNode>(
  visibleNodes: ReadonlyArray<T>,
  selectedId: string | null,
): T[] {
  const videoNodes = visibleNodes.filter(isVideoChainNode);
  if (!videoNodes.length) return visibleNodes.slice(0, 8);
  const selectedNode = visibleNodes.find((node) => node.id === selectedId) ?? null;
  const seen = new Set<string>();
  const result: T[] = [];
  for (const node of [selectedNode, ...videoNodes]) {
    if (node == null || seen.has(node.id)) continue;
    seen.add(node.id);
    result.push(node);
  }
  return result;
}

export function summarizeGraph(graph: AssetDependencyGraph | null): { text: number; image: number; video: number } {
  if (!graph) return { text: 0, image: 0, video: 0 };
  return graph.nodes.reduce(
    (acc, node) => {
      if (node.id === "project" || node.count <= 0) return acc;
      if (node.kind === "image") acc.image += node.count;
      else if (node.kind === "video") acc.video += node.count;
      else if (node.kind === "text" || node.kind === "prompt") acc.text += node.count;
      return acc;
    },
    { text: 0, image: 0, video: 0 },
  );
}

export function preferredInitialNodeId(graph: AssetDependencyGraph): string | null {
  let firstVisible: string | null = null;
  let shootingScript: string | null = null;
  let videoChain: string | null = null;
  let needsVisualization: string | null = null;
  let firstNonProject: string | null = null;
  for (const node of graph.nodes) {
    if (node.id !== "project" && node.count <= 0) continue;
    firstVisible ??= node.id;
    if (shootingScript == null && node.id === "chapter_shooting_script") {
      shootingScript = node.id;
    }
    if (videoChain == null && isVideoChainNode(node)) videoChain = node.id;
    if (node.id !== "project") {
      if (
        needsVisualization == null &&
        node.needs_visualization &&
        !node.visualized
      ) {
        needsVisualization = node.id;
      }
      firstNonProject ??= node.id;
    }
  }
  return shootingScript ?? videoChain ?? needsVisualization ?? firstNonProject ?? firstVisible;
}
