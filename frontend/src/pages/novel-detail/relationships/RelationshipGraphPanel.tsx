import React, { useCallback, useEffect, useMemo, useState } from "react";
import "@xyflow/react/dist/style.css";
import {
  Background,
  MarkerType,
  Position,
  ReactFlow,
  useNodesState,
  type BezierPathOptions,
  type Edge,
  type Node,
  type NodeChange,
  type ReactFlowInstance,
} from "@xyflow/react";
import { ArrowRight, RefreshCcw, Users } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { FlowControls } from "../../../components/ui/FlowControls";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import { EmptyHint } from "../shared";
import {
  readPortableItem,
  removePortableItem,
  writePortableItem,
} from "../../../utils/portableStorage";
import { formatNumber, useI18n, type MessageTranslator } from "../../../i18n";

export interface CurrentRelationship {
  id: string;
  profile_id: number;
  character_a: string;
  character_b: string;
  description: string;
  last_synced_chapter: number;
  timeline_started: boolean;
  structured: boolean;
  editable: boolean;
}

interface RelationshipNode {
  id: string;
  name: string;
  degree: number;
}

interface RelationshipEdge {
  id: string;
  fromId: string;
  toId: string;
  relationship: CurrentRelationship;
  parallelIndex: number;
}

interface RelationshipGraphData {
  nodes: RelationshipNode[];
  edges: RelationshipEdge[];
  relationshipsByNodeId: Map<string, CurrentRelationship[]>;
}

interface RelationshipNodePosition {
  x: number;
  y: number;
}

type RelationshipNodePositions = Record<string, RelationshipNodePosition>;

interface RelationshipFlowNodeData extends Record<string, unknown> {
  label: React.ReactNode;
  name: string;
  degree: number;
}

interface RelationshipFlowEdgeData extends Record<string, unknown> {
  relationship: CurrentRelationship;
}

type RelationshipFlowNode = Node<RelationshipFlowNodeData, "default">;
type RelationshipFlowEdge = Edge<RelationshipFlowEdgeData, "default"> & {
  pathOptions?: BezierPathOptions;
};

const RELATIONSHIP_NODE_SIZE = 76;
const RELATIONSHIP_LAYOUT_MIN_WIDTH = 720;
const RELATIONSHIP_LAYOUT_MIN_HEIGHT = 480;
const RELATIONSHIP_RING_CAPACITY = 10;
const RELATIONSHIP_RING_GAP = 150;
const RELATIONSHIP_GRAPH_LAYOUT_STORAGE_PREFIX =
  "a4n:relationship-graph-layout:v1";

export const CurrentRelationshipsCard: React.FC<{
  projectId: number;
  relationships: CurrentRelationship[];
  onOpen: (relationship: CurrentRelationship) => void;
}> = ({ projectId, relationships, onOpen }) => {
  const { locale, t } = useI18n();
  const reportStorageError = useCallback((error: unknown) => {
    useToast.getState().addToast(extractErrorMessage(error, t("errors.saveFailed")), "error");
  }, [t]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [charactersOpen, setCharactersOpen] = useState(false);
  const [hoveredRelationship, setHoveredRelationship] =
    useState<CurrentRelationship | null>(null);
  const [fixedNodeCount, setFixedNodeCount] = useState(0);
  const [flowInstance, setFlowInstance] = useState<ReactFlowInstance<
    RelationshipFlowNode,
    RelationshipFlowEdge
  > | null>(null);
  const [nodes, setNodes, applyNodesChange] =
    useNodesState<RelationshipFlowNode>([]);
  const graph = useMemo(
    () => buildRelationshipGraph(relationships),
    [relationships],
  );
  const layoutSize = useMemo(
    () => relationshipLayoutSize(graph.nodes.length),
    [graph.nodes.length],
  );
  const edges = useMemo(() => buildRelationshipFlowEdges(graph, t), [graph, t]);

  useEffect(() => {
    const stored = loadRelationshipNodePositions(projectId);
    const nodeIds = new Set(graph.nodes.map((node) => node.id));
    const positions = pruneRelationshipNodePositions(stored, nodeIds);
    if (Object.keys(positions).length !== Object.keys(stored).length) {
      void saveRelationshipNodePositions(projectId, positions).catch(reportStorageError);
    }
    setFixedNodeCount(Object.keys(positions).length);
    setNodes((current) => preserveRelationshipMeasurements(current, buildRelationshipFlowNodes(graph, positions, layoutSize, t, locale)));
    setSelectedNodeId((current) =>
      current && nodeIds.has(current) ? current : null,
    );
  }, [graph, layoutSize, locale, projectId, reportStorageError, setNodes, t]);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      void flowInstance?.fitView({ padding: 0.22, maxZoom: 1.15 });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [flowInstance, graph, layoutSize]);

  useEffect(() => {
    setCharactersOpen(false);
  }, [projectId]);

  const flowNodes = useMemo(
    () =>
      nodes.map((node) => {
        const selected = node.id === selectedNodeId;
        return node.selected === selected ? node : { ...node, selected };
      }),
    [nodes, selectedNodeId],
  );
  const selectedNode = selectedNodeId
    ? (graph.nodes.find((node) => node.id === selectedNodeId) ?? null)
    : null;
  const selectedRelationships = selectedNodeId
    ? (graph.relationshipsByNodeId.get(selectedNodeId) ?? [])
    : [];

  const handleNodesChange = useCallback(
    (changes: NodeChange<RelationshipFlowNode>[]) => {
      applyNodesChange(
        changes.filter(
          (change) =>
            change.type === "position" || change.type === "dimensions",
        ),
      );
    },
    [applyNodesChange],
  );

  const persistDraggedNode = useCallback(
    (_event: MouseEvent | TouchEvent, node: RelationshipFlowNode) => {
      const positions = pruneRelationshipNodePositions(
        loadRelationshipNodePositions(projectId),
        new Set(graph.nodes.map((item) => item.id)),
      );
      positions[node.id] = relationshipNodePosition(node.position, layoutSize);
      void saveRelationshipNodePositions(projectId, positions).catch(reportStorageError);
      setFixedNodeCount(Object.keys(positions).length);
    },
    [graph.nodes, layoutSize, projectId, reportStorageError],
  );

  const resetNodeLayout = useCallback(() => {
    void clearRelationshipNodePositions(projectId).catch(reportStorageError);
    setFixedNodeCount(0);
    setNodes((current) => preserveRelationshipMeasurements(current, buildRelationshipFlowNodes(graph, {}, layoutSize, t, locale)));
    setSelectedNodeId(null);
    setHoveredRelationship(null);
    window.requestAnimationFrame(() => {
      void flowInstance?.fitView({ padding: 0.22, maxZoom: 1.15 });
    });
  }, [flowInstance, graph, layoutSize, locale, projectId, reportStorageError, setNodes, t]);

  if (relationships.length === 0) {
    return (
      <div className="a4n-flat-surface p-5">
        <EmptyHint text={t("novel.relationships.noRelationships")} />
      </div>
    );
  }

  return (
    <>
      <section className="a4n-flat-surface flex min-h-0 flex-col overflow-hidden p-0 lg:flex-1">
        <div className="flex min-h-12 items-center justify-between gap-2 border-b border-book-border/60 bg-book-bg-paper px-3 py-2">
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 items-baseline gap-2">
              <div className="truncate text-body font-semibold text-book-text-main">
                {t("novel.relationships.title")}
              </div>
              <div className="min-w-0 truncate text-small text-book-text-muted">
                {formatNumber(graph.nodes.length, locale)}{" "}
                {t("novel.characters")} ·{" "}
                {t("novel.relationships.relationshipCount", {
                  count: formatNumber(graph.edges.length, locale),
                })}
              </div>
            </div>
          </div>
          <BookButton
            variant="secondary"
            size="sm"
            className="shrink-0 lg:hidden"
            onClick={() => setCharactersOpen(true)}
            aria-label={t("novel.relationships.characters")}
            title={t("novel.relationships.characters")}
          >
            <Users size={14} /> {t("novel.relationships.characters")}
          </BookButton>
        </div>

        <div className="grid min-h-0 grid-cols-1 lg:flex-1 lg:grid-cols-[minmax(0,1fr)_288px]">
          <div className="relative h-[360px] min-h-0 min-w-0 bg-book-bg sm:h-[420px] lg:h-auto">
            {fixedNodeCount > 0 ? (
              <div className="absolute right-3 top-3 z-10">
                <Dropdown
                  items={[
                    {
                      label: t("novel.relationships.restoreLayout"),
                      icon: <RefreshCcw size={13} />,
                      onClick: resetNodeLayout,
                    },
                  ]}
                />
              </div>
            ) : null}
            <ReactFlow<RelationshipFlowNode, RelationshipFlowEdge>
              nodes={flowNodes}
              edges={edges}
              onNodesChange={handleNodesChange}
              onNodeClick={(_event, node) => {
                setSelectedNodeId((current) =>
                  current === node.id ? null : node.id,
                );
                if (window.matchMedia?.("(max-width: 1023px)")?.matches) {
                  setCharactersOpen(true);
                }
              }}
              onNodeDragStop={persistDraggedNode}
              onPaneClick={() => setSelectedNodeId(null)}
              onEdgeClick={(_event, edge) => {
                if (edge.data) onOpen(edge.data.relationship);
              }}
              onEdgeMouseEnter={(_event, edge) =>
                setHoveredRelationship(edge.data?.relationship ?? null)
              }
              onEdgeMouseLeave={() => setHoveredRelationship(null)}
              onInit={setFlowInstance}
              nodesConnectable={false}
              edgesReconnectable={false}
              deleteKeyCode={null}
              minZoom={0.45}
              maxZoom={2}
              fitView
              fitViewOptions={{ padding: 0.22, maxZoom: 1.15 }}
              nodeExtent={[
                [0, 0],
                [
                  layoutSize.width - RELATIONSHIP_NODE_SIZE,
                  layoutSize.height - RELATIONSHIP_NODE_SIZE,
                ],
              ]}
              className="a4n-relationship-flow"
              aria-label={t("novel.relationships.graphAria")}
            >
              <FlowControls showInteractive={false} />
              <Background
                color="rgb(var(--color-text-tertiary) / 0.2)"
                gap={28}
                size={1}
              />
            </ReactFlow>
          </div>

          <div className="order-first min-h-0 min-w-0 lg:order-none">
            <RelationshipGraphPanel
              graph={graph}
              hoveredRelationship={hoveredRelationship}
              selectedNode={selectedNode}
              selectedRelationships={selectedRelationships}
              selectedNodeId={selectedNodeId}
              className="max-h-72 min-h-0 overflow-y-auto border-b border-book-border bg-book-bg-paper custom-scrollbar lg:h-full lg:max-h-none lg:border-b-0 lg:border-l"
              onSelectNode={(id) => {
                setSelectedNodeId(id);
                if (window.matchMedia("(max-width: 1023px)").matches) setCharactersOpen(true);
              }}
              onOpen={onOpen}
            />
          </div>
        </div>
      </section>

      <Modal
        isOpen={charactersOpen}
        onClose={() => setCharactersOpen(false)}
        title={t("novel.relationships.allCharacters")}
        subtitle={`${formatNumber(graph.nodes.length, locale)} ${t("novel.characters")}`}
        icon={<Users size={15} />}
        maxWidthClassName="max-w-md"
        bodyClassName="min-h-0 flex-1 overflow-y-auto px-2 py-2 custom-scrollbar sm:px-3 sm:py-3"
      >
        <RelationshipGraphPanel
          graph={graph}
          hoveredRelationship={hoveredRelationship}
          selectedNode={selectedNode}
          selectedRelationships={selectedRelationships}
          selectedNodeId={selectedNodeId}
          className="min-w-0 bg-book-bg-paper"
          onSelectNode={setSelectedNodeId}
          onOpen={onOpen}
        />
      </Modal>
    </>
  );
};

const RelationshipGraphPanel: React.FC<{
  graph: RelationshipGraphData;
  hoveredRelationship: CurrentRelationship | null;
  selectedNode: RelationshipNode | null;
  selectedRelationships: CurrentRelationship[];
  selectedNodeId: string | null;
  className?: string;
  onSelectNode: (nodeId: string | null) => void;
  onOpen: (relationship: CurrentRelationship) => void;
}> = ({
  graph,
  hoveredRelationship,
  selectedNode,
  selectedRelationships,
  selectedNodeId,
  className = "",
  onSelectNode,
  onOpen,
}) => {
  const { locale, t } = useI18n();
  const prominentNodes = [...graph.nodes].sort(
    (a, b) => b.degree - a.degree || a.name.localeCompare(b.name),
  );

  return (
    <aside className={`min-w-0 ${className}`}>
      {hoveredRelationship ? (
        <section className="border-b border-book-primary/15 bg-book-primary/5 px-4 py-3">
          <div className="mb-1 flex min-w-0 items-center gap-1.5 text-small font-semibold text-book-text-main">
            <span className="truncate">{hoveredRelationship.character_a}</span>
            <span className="inline-flex shrink-0 items-center text-book-primary">
              <ArrowRight size={13} aria-hidden="true" />
              <span className="sr-only">{t("novel.relationships.directionTo")}</span>
            </span>
            <span className="truncate">{hoveredRelationship.character_b}</span>
          </div>
          <div className="line-clamp-2 text-small leading-relaxed text-book-text-muted">
            {hoveredRelationship.description}
          </div>
        </section>
      ) : null}

      {selectedNode ? (
        <section className="border-b border-book-border/70">
          <div className="flex items-center gap-2.5 px-4 py-3.5">
            <InitialBadge value={selectedNode.name} />
            <div className="min-w-0 flex-1">
              <div
                className="truncate text-sm font-semibold text-book-text-main"
                title={selectedNode.name}
              >
                {selectedNode.name}
              </div>
              <div className="mt-0.5 text-small text-book-text-muted">
                {t("novel.relationships.relationshipCount", {
                  count: formatNumber(selectedNode.degree, locale),
                })}
              </div>
            </div>
          </div>

          <div className="grid gap-1.5 border-t border-book-border/55 p-2">
            {selectedRelationships.map((relationship) => (
              <button
                key={relationship.id}
                type="button"
                onClick={() => onOpen(relationship)}
                className="block w-full rounded-xl border border-book-border/40 bg-book-bg-paper/45 px-3 py-2.5 text-left transition-colors hover:border-book-primary/35 hover:bg-book-bg-tertiary/55 focus-visible:border-book-primary/45 focus-visible:outline-none"
                title={t("novel.relationships.viewDetails")}
              >
                <div className="flex min-w-0 items-center gap-1.5 text-small font-semibold text-book-text-main">
                  <span className="truncate">{relationship.character_a}</span>
                  <span className="inline-flex shrink-0 items-center text-book-primary">
                    <ArrowRight size={13} aria-hidden="true" />
                    <span className="sr-only">{t("novel.relationships.directionTo")}</span>
                  </span>
                  <span className="truncate">{relationship.character_b}</span>
                </div>
                <div className="mt-0.5 line-clamp-2 text-small leading-relaxed text-book-text-muted">
                  {relationship.description}
                </div>
              </button>
            ))}
          </div>
        </section>
      ) : null}

      <section className="px-2 py-3">
        <div className="mb-1 px-2 text-small font-semibold text-book-text-main">
          {t("novel.relationships.allCharacters")}
        </div>
        <div className="grid gap-1.5">
          {prominentNodes.map((node) => {
            const selected = node.id === selectedNodeId;
            return (
              <button
                key={node.id}
                type="button"
                onClick={() => onSelectNode(selected ? null : node.id)}
                aria-pressed={selected}
                className={`grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2 rounded-xl border px-2 py-2 text-left transition-colors ${
                  selected
                    ? "border-book-primary/50 bg-book-primary/10 text-book-primary"
                    : "border-book-border/35 bg-book-bg-paper/40 text-book-text-muted hover:border-book-primary/30 hover:bg-book-bg-tertiary/65 hover:text-book-text-main"
                }`}
                title={node.name}
              >
                <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-book-bg-tertiary text-caption font-semibold text-book-text-muted">
                  {initialOf(node.name)}
                </span>
                <span className="min-w-0 truncate text-xs font-medium">
                  {node.name}
                </span>
                <span className="shrink-0 text-caption text-book-text-muted">
                  {formatNumber(node.degree, locale)}
                </span>
              </button>
            );
          })}
        </div>
      </section>
    </aside>
  );
};
const InitialBadge: React.FC<{ value: string }> = ({ value }) => (
  <div className="flex h-9 w-9 shrink-0 select-none items-center justify-center rounded-full bg-book-primary/10 text-xs font-semibold text-book-primary">
    {initialOf(value)}
  </div>
);
function buildRelationshipGraph(
  relationships: CurrentRelationship[],
): RelationshipGraphData {
  const nodeById = new Map<string, RelationshipNode>();
  const relationshipsByNodeId = new Map<string, CurrentRelationship[]>();
  const ensureNode = (name: string): RelationshipNode => {
    const id = relationshipNodeId(name);
    const existing = nodeById.get(id);
    if (existing) return existing;
    const node: RelationshipNode = {
      id,
      name: name.trim(),
      degree: 0,
    };
    nodeById.set(id, node);
    relationshipsByNodeId.set(id, []);
    return node;
  };

  const edges = relationships.map((relationship) => {
    const from = ensureNode(relationship.character_a);
    const to = ensureNode(relationship.character_b);
    from.degree += 1;
    relationshipsByNodeId.get(from.id)?.push(relationship);
    if (from.id !== to.id) {
      to.degree += 1;
      relationshipsByNodeId.get(to.id)?.push(relationship);
    }
    return {
      id: relationship.id,
      fromId: from.id,
      toId: to.id,
      relationship,
      parallelIndex: 0,
    };
  });

  const parallelGroups = new Map<string, RelationshipEdge[]>();
  edges.forEach((edge) => {
    const key = JSON.stringify([edge.fromId, edge.toId].sort());
    const group = parallelGroups.get(key) ?? [];
    group.push(edge);
    parallelGroups.set(key, group);
  });
  parallelGroups.forEach((group) => {
    group.forEach((edge, index) => {
      edge.parallelIndex = index;
    });
  });

  return {
    nodes: [...nodeById.values()],
    edges,
    relationshipsByNodeId,
  };
}

function relationshipLayoutSize(nodeCount: number): {
  width: number;
  height: number;
} {
  const ringCount = Math.max(
    1,
    Math.ceil(Math.max(0, nodeCount - 1) / RELATIONSHIP_RING_CAPACITY),
  );
  const diameter =
    ringCount * RELATIONSHIP_RING_GAP * 2 + RELATIONSHIP_NODE_SIZE * 2;
  return {
    width: Math.max(RELATIONSHIP_LAYOUT_MIN_WIDTH, diameter),
    height: Math.max(RELATIONSHIP_LAYOUT_MIN_HEIGHT, diameter),
  };
}

function automaticRelationshipNodeCenter(
  index: number,
  nodeCount: number,
  layout: { width: number; height: number },
): RelationshipNodePosition {
  if (nodeCount <= 1 || index === 0) {
    return { x: layout.width / 2, y: layout.height / 2 };
  }
  const ring = Math.floor((index - 1) / RELATIONSHIP_RING_CAPACITY) + 1;
  const ringStart = 1 + (ring - 1) * RELATIONSHIP_RING_CAPACITY;
  const ringCount = Math.min(RELATIONSHIP_RING_CAPACITY, nodeCount - ringStart);
  const ringIndex = index - ringStart;
  const angle =
    (Math.PI * 2 * ringIndex) / Math.max(1, ringCount) - Math.PI / 2;
  const radius = ring * RELATIONSHIP_RING_GAP;
  return {
    x: layout.width / 2 + Math.cos(angle) * radius,
    y: layout.height / 2 + Math.sin(angle) * radius,
  };
}

function preserveRelationshipMeasurements(current: RelationshipFlowNode[], next: RelationshipFlowNode[]): RelationshipFlowNode[] {
  const previous = new Map(current.map((node) => [node.id, node]));
  // 固定尺寸节点保留测量结果，刷新或恢复布局不应清空连线锚点。
  return next.map((node) => ({ ...previous.get(node.id), ...node }));
}

function buildRelationshipFlowNodes(
  graph: RelationshipGraphData,
  positions: RelationshipNodePositions,
  layout: { width: number; height: number },
  t: MessageTranslator,
  locale: Parameters<typeof formatNumber>[1],
): RelationshipFlowNode[] {
  const ordered = [...graph.nodes].sort(
    (left, right) =>
      right.degree - left.degree || left.name.localeCompare(right.name),
  );
  return ordered.map((node, index) => {
    const saved = positions[node.id];
    const center = saved
      ? { x: saved.x * layout.width, y: saved.y * layout.height }
      : automaticRelationshipNodeCenter(index, ordered.length, layout);
    const halfNode = RELATIONSHIP_NODE_SIZE / 2;
    return {
      id: node.id,
      type: "default",
      position: {
        x: center.x - halfNode,
        y: center.y - halfNode,
      },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
      data: {
        name: node.name,
        degree: node.degree,
        label: (
          <div className="flex h-full min-w-0 flex-col items-center justify-center px-1 text-center">
            <span className="text-sm font-bold text-book-primary">
              {initialOf(node.name)}
            </span>
            <span className="mt-0.5 line-clamp-2 max-w-full text-caption font-semibold leading-tight text-book-text-main">
              {node.name}
            </span>
            <span className="mt-0.5 text-[9px] text-book-text-muted">
              {t("novel.relationships.relationshipCount", {
                count: formatNumber(node.degree, locale),
              })}
            </span>
          </div>
        ),
      },
      style: {
        width: RELATIONSHIP_NODE_SIZE,
        height: RELATIONSHIP_NODE_SIZE,
      },
      className: "a4n-relationship-node",
      draggable: true,
      selectable: true,
      deletable: false,
      connectable: false,
      ariaLabel: t("novel.relationships.nodeAria", {
        name: node.name,
        count: formatNumber(node.degree, locale),
      }),
    };
  });
}

function buildRelationshipFlowEdges(
  graph: RelationshipGraphData,
  t: MessageTranslator,
): RelationshipFlowEdge[] {
  const showLabels = graph.edges.length <= 8;
  return graph.edges.map((edge) => ({
    id: edge.id,
    type: "default",
    source: edge.fromId,
    target: edge.toId,
    data: { relationship: edge.relationship },
    pathOptions: {
      curvature: Math.min(0.85, 0.2 + edge.parallelIndex * 0.18),
    },
    label: showLabels
      ? relationshipEdgeSummary(edge.relationship.description, t)
      : undefined,
    markerEnd: {
      type: MarkerType.ArrowClosed,
      color: "rgb(var(--color-text-secondary))",
    },
    interactionWidth: 20,
    className: "a4n-relationship-edge",
    selectable: true,
    deletable: false,
    focusable: true,
    ariaLabel: t("novel.relationships.edgeAria", {
      from: edge.relationship.character_a,
      to: edge.relationship.character_b,
      description: edge.relationship.description,
    }),
  }));
}

function relationshipNodePosition(
  position: { x: number; y: number },
  layout: { width: number; height: number },
): RelationshipNodePosition {
  return {
    x: (position.x + RELATIONSHIP_NODE_SIZE / 2) / layout.width,
    y: (position.y + RELATIONSHIP_NODE_SIZE / 2) / layout.height,
  };
}

function relationshipLayoutStorageKey(projectId: number): string {
  return RELATIONSHIP_GRAPH_LAYOUT_STORAGE_PREFIX + ":" + projectId;
}

function loadRelationshipNodePositions(
  projectId: number,
): RelationshipNodePositions {
  const raw = readPortableItem(relationshipLayoutStorageKey(projectId));
  if (!raw) return {};
  const value = JSON.parse(raw) as unknown;
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("关系图布局必须是对象");
  }
  const positions = (value as Record<string, unknown>).positions;
  if (!positions || typeof positions !== "object" || Array.isArray(positions)) {
    throw new Error("关系图布局缺少 positions 对象");
  }

  const parsed: RelationshipNodePositions = {};
  for (const [nodeId, rawPosition] of Object.entries(positions)) {
    if (!nodeId.trim()) {
      throw new Error("关系图布局包含空节点 ID");
    }
    if (
      !rawPosition ||
      typeof rawPosition !== "object" ||
      Array.isArray(rawPosition)
    ) {
      throw new Error("关系图节点位置无效: " + nodeId);
    }
    const record = rawPosition as Record<string, unknown>;
    if (
      typeof record.x !== "number" ||
      !Number.isFinite(record.x) ||
      record.x < 0 ||
      record.x > 1 ||
      typeof record.y !== "number" ||
      !Number.isFinite(record.y) ||
      record.y < 0 ||
      record.y > 1
    ) {
      throw new Error("关系图节点坐标无效: " + nodeId);
    }
    parsed[nodeId] = { x: record.x, y: record.y };
  }
  return parsed;
}

async function saveRelationshipNodePositions(
  projectId: number,
  positions: RelationshipNodePositions,
) {
  if (Object.keys(positions).length === 0) {
    return clearRelationshipNodePositions(projectId);
  }
  const saved = await writePortableItem(
    relationshipLayoutStorageKey(projectId),
    JSON.stringify({ positions }),
  );
  if (!saved) {
    throw new Error("关系图布局存储键无效");
  }
}

function clearRelationshipNodePositions(projectId: number) {
  return removePortableItem(relationshipLayoutStorageKey(projectId));
}

function pruneRelationshipNodePositions(
  positions: RelationshipNodePositions,
  nodeIds: Set<string>,
): RelationshipNodePositions {
  return Object.fromEntries(
    Object.entries(positions).filter(([nodeId]) => nodeIds.has(nodeId)),
  );
}

export function relationshipNodeId(name: string): string {
  return name.trim().toLowerCase();
}

function initialOf(value: string): string {
  return (value.trim().charAt(0) || "?").toUpperCase();
}

function relationshipEdgeSummary(
  description: string,
  t: MessageTranslator,
): string {
  const normalized = description.replace(/\s+/g, " ").trim();
  if (!normalized) return t("novel.relationships.relationship");
  const splitIndex = normalized.search(/[。；;，,]/);
  return splitIndex > 0 ? normalized.slice(0, splitIndex) : normalized;
}
