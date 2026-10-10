import React, { memo, useCallback, useEffect, useId, useMemo, useState } from "react";
import "@xyflow/react/dist/style.css";
import {
  applyNodeChanges,
  Background,
  Controls,
  Handle,
  MarkerType,
  Position,
  ReactFlow,
  type Connection,
  type Edge,
  type Node,
  type NodeChange,
  type NodeProps,
} from "@xyflow/react";
import { ArrowRight, CircleDot, GitBranch, Play, Plus, Repeat2, Save, Trash2 } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import { isPositiveSafeInteger } from "../../../utils/numberRange";
import {
  advanceGalgameStoryRuntime,
  getGalgameStoryRuntimeState,
  saveGalgameStoryGraph,
  startGalgameStoryRuntime,
} from "../../../api/galgame";
import type {
  GalgameStoryRuntimeView,
  StoryEdge,
  StoryGraph,
  StoryLoopGroup,
  StoryNode,
  StoryStateVariable,
  StoryValue,
} from "../../../api/generated/host";
import { BookButton } from "../../../components/ui/BookButton";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { useI18n, type MessageTranslator } from "../../../i18n";
import { emptyGalgamePresentation } from "./galgameTabShared";

interface StoryGraphPanelProps {
  canEdit: boolean;
  graph: StoryGraph | null;
  graphStatus: string | null;
  projectId: number;
  onSaved: (graph: StoryGraph) => void;
}

interface StoryFlowNodeData extends Record<string, unknown> {
  kindLabel: string;
  node: StoryNode;
}

interface StoryFlowEdgeData extends Record<string, unknown> {
  edge: StoryEdge;
}

type StoryFlowNode = Node<StoryFlowNodeData, "story">;
type StoryFlowEdge = Edge<StoryFlowEdgeData>;
type MobileGraphPane = "graph" | "details";

const NODE_KINDS: StoryNode["kind"][] = [
  "narrative",
  "choice",
  "fork",
  "join",
  "loop_entry",
  "loop_exit",
  "chapter_boundary",
  "terminal",
];
const EDGE_KINDS: StoryEdge["kind"][] = [
  "sequence",
  "choice",
  "chapter_transition",
  "interrupt",
];
const LINE_TYPES = ["narration", "dialogue", "monologue", "stage_direction", "choice"];

const nodeKindLabel = (kind: StoryNode["kind"], t: MessageTranslator): string => {
  switch (kind) {
    case "choice":
      return t("galgame.graph.choiceNode");
    case "fork":
      return t("galgame.graph.forkNode");
    case "join":
      return t("galgame.graph.joinNode");
    case "loop_entry":
      return t("galgame.graph.loopEntryNode");
    case "loop_exit":
      return t("galgame.graph.loopExitNode");
    case "chapter_boundary":
      return t("galgame.graph.chapterBoundaryNode");
    case "terminal":
      return t("galgame.graph.terminalNode");
    default:
      return t("galgame.graph.narrativeNode");
  }
};

const edgeKindLabel = (kind: StoryEdge["kind"], t: MessageTranslator): string => {
  switch (kind) {
    case "choice":
      return t("galgame.graph.choiceEdge");
    case "loop_back":
      return t("galgame.graph.loopEdge");
    case "chapter_transition":
      return t("galgame.graph.chapterEdge");
    case "interrupt":
      return t("galgame.graph.interruptEdge");
    default:
      return t("galgame.graph.sequenceEdge");
  }
};

const StoryGraphFlowNode = memo(({ data, selected }: NodeProps<StoryFlowNode>) => (
  <div className={`w-[220px] border bg-book-bg px-3 py-2 ${selected ? "border-book-primary" : "border-book-border/60"}`}>
    <Handle type="target" position={Position.Left} className="!h-2 !w-2 !border-book-bg !bg-book-primary" />
    <div className="flex items-center gap-2 text-caption text-book-text-muted">
      {data.node.kind === "fork" || data.node.kind === "choice" ? <GitBranch size={12} /> : null}
      {data.node.kind === "loop_entry" || data.node.kind === "loop_exit" ? <Repeat2 size={12} /> : null}
      <span className="truncate">{data.kindLabel}</span>
    </div>
    <strong className="mt-1 block truncate text-xs text-book-text-main">{data.node.title}</strong>
    <span className="mt-1 block text-caption tabular-nums text-book-text-muted">{data.node.lines.length}</span>
    <Handle type="source" position={Position.Right} className="!h-2 !w-2 !border-book-bg !bg-book-primary" />
  </div>
));

const nodeTypes = { story: StoryGraphFlowNode };

function newId(prefix: string): string {
  return `${prefix}:${crypto.randomUUID()}`;
}

function terminalIds(nodes: StoryNode[]): string[] {
  return nodes.filter((node) => node.kind === "terminal").map((node) => node.id);
}

function storyValueText(value: StoryValue): string {
  if ("int" in value) return String(value.int);
  if ("string" in value) return value.string;
  if ("enum" in value) return value.enum;
  return "";
}

function toFlowNodes(graph: StoryGraph, selectedNodeId: string | null, t: MessageTranslator): StoryFlowNode[] {
  return graph.nodes.map((node) => ({
    id: node.id,
    type: "story",
    position: { x: node.layout_x, y: node.layout_y },
    data: { node, kindLabel: nodeKindLabel(node.kind, t) },
    selected: node.id === selectedNodeId,
  }));
}

function branchOption(graph: StoryGraph, edgeId: string) {
  return graph.branch_groups.flatMap((group) => group.options).find((option) => option.edge_id === edgeId);
}

function canConnectStoryNodes(
  graph: StoryGraph,
  sourceId: string,
  targetId: string,
  edgeKind: StoryEdge["kind"],
): boolean {
  const source = graph.nodes.find((node) => node.id === sourceId);
  const target = graph.nodes.find((node) => node.id === targetId);
  if (!source || !target || source.id === target.id || source.kind === "terminal") return false;

  const isFork = source.kind === "fork" || source.kind === "choice";
  if (isFork !== (edgeKind === "choice")) return false;
  const sourceEdges = graph.edges.filter((edge) => edge.from_node_id === source.id);
  const loopBody = graph.loop_groups.some((group) => group.body_exit_node_id === source.id);
  if (sourceEdges.length >= (loopBody ? 2 : 1) && !isFork) return false;

  const targetEdges = graph.edges.filter((edge) => edge.to_node_id === target.id);
  if (target.kind === "join") return true;
  if (target.kind === "loop_entry") {
    const externalInputs = targetEdges.filter((edge) => edge.kind !== "loop_back").length;
    return externalInputs === 0 && graph.entry_node_id !== target.id;
  }
  return targetEdges.length === 0 && target.id !== graph.entry_node_id;
}

function toFlowEdges(
  graph: StoryGraph,
  selectedEdgeId: string | null,
  t: MessageTranslator,
): StoryFlowEdge[] {
  return graph.edges.map((edge) => {
    const option = branchOption(graph, edge.id);
    return {
      id: edge.id,
      source: edge.from_node_id,
      target: edge.to_node_id,
      type: "smoothstep",
      label: option?.label ?? (edge.kind === "sequence" ? "" : edgeKindLabel(edge.kind, t)),
      markerEnd: { type: MarkerType.ArrowClosed },
      selected: edge.id === selectedEdgeId,
      data: { edge },
    };
  });
}

function addStoryLoop(
  graph: StoryGraph,
  bodyNodeId: string,
  maxIterations: number,
  entryTitle: string,
  exitTitle: string,
): StoryGraph {
  const body = graph.nodes.find((node) => node.id === bodyNodeId);
  if (!body || !["narrative", "chapter_boundary"].includes(body.kind)) {
    throw new Error("galgame.graph.loopBodyInvalid");
  }
  if (
    graph.loop_groups.some((group) =>
      [group.entry_node_id, group.body_exit_node_id, group.exit_node_id].includes(bodyNodeId),
    ) || graph.edges.some((edge) =>
      edge.kind === "loop_back" && (edge.from_node_id === bodyNodeId || edge.to_node_id === bodyNodeId),
    )
  ) {
    throw new Error("galgame.graph.loopExists");
  }
  const outgoing = graph.edges.filter((edge) => edge.from_node_id === bodyNodeId);
  if (outgoing.length === 0) throw new Error("galgame.graph.loopNeedsNext");
  const incoming = graph.edges.filter((edge) => edge.to_node_id === bodyNodeId);
  if (incoming.length === 0 && graph.entry_node_id !== bodyNodeId) {
    throw new Error("galgame.graph.loopBodyDisconnected");
  }
  const loopEntryId = newId("loop_entry");
  const loopExitId = newId("loop_exit");
  const loopEntry: StoryNode = {
    id: loopEntryId,
    kind: "loop_entry",
    title: entryTitle,
    summary: "",
    primary_source: null,
    source_refs: [],
    lines: [],
    state_before: {},
    state_after: {},
    state_operations: [],
    media_intents: [],
    layout_x: body.layout_x - 250,
    layout_y: body.layout_y,
  };
  const loopExit: StoryNode = {
    id: loopExitId,
    kind: "loop_exit",
    title: exitTitle,
    summary: "",
    primary_source: null,
    source_refs: [],
    lines: [],
    state_before: {},
    state_after: {},
    state_operations: [],
    media_intents: [],
    layout_x: body.layout_x + 250,
    layout_y: body.layout_y,
  };
  const edges = graph.edges.map((edge) => ({
    ...edge,
    to_node_id: edge.to_node_id === bodyNodeId ? loopEntryId : edge.to_node_id,
    from_node_id: edge.from_node_id === bodyNodeId ? loopExitId : edge.from_node_id,
  }));
  const entryEdge: StoryEdge = {
    id: newId("loop_body"),
    from_node_id: loopEntryId,
    to_node_id: bodyNodeId,
    from_port: "out",
    to_port: "in",
    kind: "sequence",
    branch_group_id: null,
    guard: null,
    state_operations: [],
    display_order: 0,
  };
  const backEdge: StoryEdge = {
    id: newId("loop_back"),
    from_node_id: bodyNodeId,
    to_node_id: loopEntryId,
    from_port: "repeat",
    to_port: "loop",
    kind: "loop_back",
    branch_group_id: null,
    guard: null,
    state_operations: [],
    display_order: 0,
  };
  const exitEdge: StoryEdge = {
    id: newId("loop_exit_path"),
    from_node_id: bodyNodeId,
    to_node_id: loopExitId,
    from_port: "exit",
    to_port: "in",
    kind: "sequence",
    branch_group_id: null,
    guard: null,
    state_operations: [],
    display_order: 1,
  };
  const group: StoryLoopGroup = {
    id: newId("loop"),
    entry_node_id: loopEntryId,
    body_exit_node_id: bodyNodeId,
    back_edge_id: backEdge.id,
    back_to_node_id: loopEntryId,
    exit_node_id: loopExitId,
    guard: "true",
    state_operations: [],
    max_iterations: maxIterations,
  };
  return {
    ...graph,
    entry_node_id: graph.entry_node_id === bodyNodeId ? loopEntryId : graph.entry_node_id,
    nodes: [...graph.nodes, loopEntry, loopExit],
    edges: [...edges, entryEdge, backEdge, exitEdge],
    loop_groups: [...graph.loop_groups, group],
    terminal_node_ids: terminalIds([...graph.nodes, loopEntry, loopExit]),
  };
}

function removeStoryLoop(graph: StoryGraph, group: StoryLoopGroup): StoryGraph {
  const entryId = group.entry_node_id;
  const bodyId = group.body_exit_node_id;
  const exitId = group.exit_node_id;
  const incoming = graph.edges.filter((edge) => edge.to_node_id === entryId && edge.id !== group.back_edge_id);
  const afterLoop = graph.edges.filter((edge) => edge.from_node_id === exitId);
  const edges = graph.edges
    .filter((edge) => edge.from_node_id !== entryId
      && edge.to_node_id !== entryId
      && edge.from_node_id !== exitId
      && !(edge.from_node_id === bodyId && edge.to_node_id === exitId))
    .map((edge) => edge);
  edges.push(...incoming.map((edge) => ({ ...edge, to_node_id: bodyId })));
  edges.push(...afterLoop.map((edge) => ({ ...edge, from_node_id: bodyId })));
  const nodes = graph.nodes.filter((node) => node.id !== entryId && node.id !== exitId);
  return {
    ...graph,
    entry_node_id: graph.entry_node_id === entryId ? bodyId : graph.entry_node_id,
    nodes,
    edges,
    loop_groups: graph.loop_groups.filter((item) => item.id !== group.id),
    terminal_node_ids: terminalIds(nodes),
  };
}

function removeStoryNode(graph: StoryGraph, nodeId: string): StoryGraph {
  if (nodeId === graph.entry_node_id) throw new Error("galgame.graph.entryCannotDelete");
  let working = graph;
  for (const group of graph.loop_groups.filter((group) =>
    [group.entry_node_id, group.body_exit_node_id, group.exit_node_id].includes(nodeId),
  )) working = removeStoryLoop(working, group);
  if (!working.nodes.some((node) => node.id === nodeId)) return working;
  const removedEdgeIds = new Set<string>([
    ...working.edges.filter((edge) => edge.from_node_id === nodeId || edge.to_node_id === nodeId).map((edge) => edge.id),
  ]);
  let edges = working.edges.filter((edge) => !removedEdgeIds.has(edge.id));
  let branchGroups = working.branch_groups
    .filter((group) => group.fork_node_id !== nodeId)
    .map((group) => ({
      ...group,
      join_node_id: group.join_node_id === nodeId ? null : group.join_node_id,
      options: group.options.filter((option) => !removedEdgeIds.has(option.edge_id)),
    }))
    .filter((group) => group.options.length >= 2);
  const removedGroupIds = new Set(
    working.branch_groups.filter((group) => !branchGroups.some((valid) => valid.id === group.id)).map((group) => group.id),
  );
  edges = edges.filter((edge) => !edge.branch_group_id || !removedGroupIds.has(edge.branch_group_id));
  branchGroups = branchGroups.map((group) => ({
    ...group,
    options: group.options.filter((option) => edges.some((edge) => edge.id === option.edge_id)),
  }));
  const outgoing = new Set(edges.map((edge) => edge.from_node_id));
  const incomingCount = new Map<string, number>();
  edges.forEach((edge) => incomingCount.set(edge.to_node_id, (incomingCount.get(edge.to_node_id) ?? 0) + 1));
  const nodes = working.nodes
    .filter((node) => node.id !== nodeId)
    .map((node) => {
      if ((node.kind === "fork" || node.kind === "choice") && !branchGroups.some((group) => group.fork_node_id === node.id)) {
        return { ...node, kind: outgoing.has(node.id) ? "narrative" as const : "terminal" as const };
      }
      if (node.kind === "join" && ((incomingCount.get(node.id) ?? 0) < 2 || !outgoing.has(node.id))) {
        return { ...node, kind: outgoing.has(node.id) ? "narrative" as const : "terminal" as const };
      }
      if (!outgoing.has(node.id) && node.kind !== "terminal" && node.kind !== "join") {
        return { ...node, kind: "terminal" as const };
      }
      return node;
    });
  return {
    ...working,
    nodes,
    edges,
    branch_groups: branchGroups,
    loop_groups: working.loop_groups,
    terminal_node_ids: terminalIds(nodes),
  };
}

export const StoryGraphPanel: React.FC<StoryGraphPanelProps> = ({ canEdit, graph, graphStatus, projectId, onSaved }) => {
  const { t } = useI18n();
  const [draft, setDraft] = useState<StoryGraph | null>(null);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null);
  const [newNodeTitle, setNewNodeTitle] = useState("");
  const [newNodeKind, setNewNodeKind] = useState<StoryNode["kind"]>("narrative");
  const [connectionKind, setConnectionKind] = useState<StoryEdge["kind"]>("sequence");
  const [loopMaxIterations, setLoopMaxIterations] = useState("3");
  const [variableKey, setVariableKey] = useState("");
  const [variableKind, setVariableKind] = useState<StoryStateVariable["value_kind"]>("bool");
  const [saveSlot, setSaveSlot] = useState("main");
  const [runtimeView, setRuntimeView] = useState<GalgameStoryRuntimeView | null>(null);
  const [mobilePane, setMobilePane] = useState<MobileGraphPane>("details");
  const [saving, setSaving] = useState(false);
  const [runtimeBusy, setRuntimeBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDraft(graph ? structuredClone(graph) : null);
    setSelectedNodeId(graph?.nodes[0]?.id ?? null);
    setSelectedEdgeId(null);
    setRuntimeView(null);
    setMobilePane("details");
    setError(null);
  }, [graph?.id, graph?.revision, projectId]);

  const selectedNode = draft?.nodes.find((node) => node.id === selectedNodeId) ?? null;
  const selectedEdge = draft?.edges.find((edge) => edge.id === selectedEdgeId) ?? null;
  const selectedLoop = draft?.loop_groups.find((group) => group.body_exit_node_id === selectedNodeId) ?? null;
  const canEditGraph = canEdit && graphStatus !== "stale";
  const visibleNodes = useMemo(
    () => draft ? toFlowNodes(draft, selectedNodeId, t) : [],
    [draft, selectedNodeId, t],
  );
  const visibleEdges = useMemo(
    () => draft ? toFlowEdges(draft, selectedEdgeId, t) : [],
    [draft, selectedEdgeId, t],
  );
  const hasUnsavedChanges = useMemo(
    () => Boolean(graph && draft && JSON.stringify(graph) !== JSON.stringify(draft)),
    [draft, graph],
  );

  const updateNode = useCallback(
    (nodeId: string, update: (node: StoryNode) => StoryNode) => {
      setDraft((current) => current ? {
        ...current,
        nodes: current.nodes.map((node) => node.id === nodeId ? update(node) : node),
      } : current);
    },
    [],
  );

  const updateEdge = (edgeId: string, update: (edge: StoryEdge) => StoryEdge) => {
    setDraft((current) => {
      if (!current) return current;
      return {
        ...current,
        edges: current.edges.map((edge) => edge.id === edgeId ? update(edge) : edge),
      };
    });
  };

  const onNodesChange = useCallback((changes: NodeChange<StoryFlowNode>[]) => {
    if (!draft) return;
    const positionChanges = changes.filter((change) => change.type === "position");
    if (!positionChanges.length) return;
    const changed = applyNodeChanges(positionChanges, toFlowNodes(draft, selectedNodeId, t));
    const positions = new Map(changed.map((node) => [node.id, node.position]));
    setDraft((current) => current ? {
      ...current,
      nodes: current.nodes.map((node) => {
        const position = positions.get(node.id);
        return position
          ? { ...node, layout_x: position.x, layout_y: position.y }
          : node;
      }),
    } : current);
  }, [draft, selectedNodeId, t]);

  const onConnect = useCallback((connection: Connection) => {
    if (!draft || !connection.source || !connection.target || connection.source === connection.target) return;
    const source = draft.nodes.find((node) => node.id === connection.source);
    const target = draft.nodes.find((node) => node.id === connection.target);
    if (!source || !target) return;
    if (!canConnectStoryNodes(draft, source.id, target.id, connectionKind)) {
      setError(t("galgame.graph.connectionContract"));
      return;
    }
    if (draft.edges.some((edge) => edge.from_node_id === source.id && edge.to_node_id === target.id && edge.kind === connectionKind)) {
      return;
    }
    setError(null);
    setDraft((current) => {
      if (!current) return current;
      const group = connectionKind === "choice"
        ? current.branch_groups.find((item) => item.fork_node_id === source.id)
        : undefined;
      const groupId = group?.id ?? (connectionKind === "choice" ? newId("branch") : null);
      const edge: StoryEdge = {
        id: newId("edge"),
        from_node_id: source.id,
        to_node_id: target.id,
        from_port: connection.sourceHandle ?? "out",
        to_port: connection.targetHandle ?? "in",
        kind: connectionKind,
        branch_group_id: groupId,
        guard: null,
        state_operations: [],
        display_order: current.edges.filter((item) => item.from_node_id === source.id).length,
      };
      let branchGroups = current.branch_groups;
      if (connectionKind === "choice" && groupId) {
        const option = {
          id: newId("option"),
          label: target.title,
          edge_id: edge.id,
          condition: null,
          state_operations: [],
        };
        branchGroups = group
          ? current.branch_groups.map((item) => item.id === group.id ? { ...item, options: [...item.options, option] } : item)
          : [...current.branch_groups, { id: groupId, fork_node_id: source.id, options: [option], join_node_id: null }];
      }
        return {
          ...current,
          edges: [...current.edges, edge],
          branch_groups: branchGroups,
        };
    });
  }, [connectionKind, draft, t]);

  const addNode = () => {
    if (!draft) return;
    const id = newId("node");
    const title = newNodeTitle.trim() || t("galgame.graph.newNodeTitle");
    const node: StoryNode = {
      id,
      kind: newNodeKind,
      title,
      summary: "",
      primary_source: null,
      source_refs: [],
      lines: [],
      state_before: {},
      state_after: {},
      state_operations: [],
      media_intents: [],
      layout_x: draft.nodes.reduce((max, item) => Math.max(max, item.layout_x), 0) + 280,
      layout_y: 100,
    };
    setDraft((current) => current ? {
      ...current,
      nodes: [...current.nodes, node],
      terminal_node_ids: terminalIds([...current.nodes, node]),
    } : current);
    setSelectedNodeId(id);
    setSelectedEdgeId(null);
    setNewNodeTitle("");
  };

  const removeNode = () => {
    if (!draft || !selectedNodeId) return;
    try {
      const next = removeStoryNode(draft, selectedNodeId);
      setDraft(next);
      setSelectedNodeId(next.nodes.find((node) => node.id === graph?.entry_node_id)?.id ?? next.nodes[0]?.id ?? null);
      setSelectedEdgeId(null);
    } catch (cause) {
      setError(cause instanceof Error && cause.message === "galgame.graph.entryCannotDelete"
        ? t("galgame.graph.entryCannotDelete")
        : extractErrorMessage(cause, t("galgame.graph.removeFailed")));
    }
  };

  const removeEdge = (edgeId: string) => {
    if (!draft) return;
    setDraft((current) => {
      if (!current) return current;
      const originalEdge = current.edges.find((edge) => edge.id === edgeId);
      const loop = current.loop_groups.find((group) =>
        group.back_edge_id === edgeId
        || originalEdge?.from_node_id === group.entry_node_id
        || originalEdge?.to_node_id === group.entry_node_id
        || originalEdge?.from_node_id === group.exit_node_id
        || originalEdge?.to_node_id === group.exit_node_id
        || (originalEdge?.from_node_id === group.body_exit_node_id && originalEdge.to_node_id === group.exit_node_id),
      );
      const base = loop ? removeStoryLoop(current, loop) : current;
      const edgeToRemove = base.edges.find((edge) => edge.id === edgeId);
      if (!edgeToRemove) return base;
      const edges = base.edges.filter((edge) => edge.id !== edgeId);
      const branchGroups = base.branch_groups
        .map((group) => ({ ...group, options: group.options.filter((option) => option.edge_id !== edgeId) }))
        .filter((group) => group.options.length >= 2);
      const removedGroupIds = new Set(base.branch_groups
        .filter((group) => !branchGroups.some((valid) => valid.id === group.id))
        .map((group) => group.id));
      const remainingEdges = edges.filter((edge) => !edge.branch_group_id || !removedGroupIds.has(edge.branch_group_id));
      const outgoing = new Set(remainingEdges.map((edge) => edge.from_node_id));
      const incomingCount = new Map<string, number>();
      remainingEdges.forEach((edge) => incomingCount.set(edge.to_node_id, (incomingCount.get(edge.to_node_id) ?? 0) + 1));
      const nodes = base.nodes.map((node) => {
        if ((node.kind === "fork" || node.kind === "choice") && !branchGroups.some((group) => group.fork_node_id === node.id)) {
          return { ...node, kind: outgoing.has(node.id) ? "narrative" as const : "terminal" as const };
        }
        if (node.kind === "join" && ((incomingCount.get(node.id) ?? 0) < 2 || !outgoing.has(node.id))) {
          return { ...node, kind: outgoing.has(node.id) ? "narrative" as const : "terminal" as const };
        }
        if (!outgoing.has(node.id) && node.kind !== "terminal") {
          return { ...node, kind: "terminal" as const };
        }
        return node;
      });
      return {
        ...base,
        edges: remainingEdges,
        branch_groups: branchGroups,
        terminal_node_ids: terminalIds(nodes),
        nodes,
      };
    });
    setSelectedEdgeId(null);
  };

  const addLoop = () => {
    if (!draft || !selectedNodeId) return;
    const maxIterations = Number(loopMaxIterations);
    if (!isPositiveSafeInteger(maxIterations) || maxIterations > 10_000) {
      setError(t("galgame.graph.loopLimitInvalid"));
      return;
    }
    try {
      setDraft(addStoryLoop(
        draft,
        selectedNodeId,
        maxIterations,
        t("galgame.graph.loopEntryNode"),
        t("galgame.graph.loopExitNode"),
      ));
      setError(null);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : "";
      const knownErrors: Record<string, string> = {
        "galgame.graph.loopBodyInvalid": t("galgame.graph.loopBodyInvalid"),
        "galgame.graph.loopExists": t("galgame.graph.loopExists"),
        "galgame.graph.loopNeedsNext": t("galgame.graph.loopNeedsNext"),
        "galgame.graph.loopBodyDisconnected": t("galgame.graph.loopBodyDisconnected"),
      };
      setError(knownErrors[message] ?? t("galgame.graph.loopCreateFailed"));
    }
  };

  const addVariable = () => {
    if (!draft) return;
    const key = variableKey.trim();
    if (!key || draft.state_variables.some((variable) => variable.key === key)) {
      setError(t("galgame.graph.variableKeyInvalid"));
      return;
    }
    const defaultValue: StoryValue = variableKind === "bool" ? { bool: false }
      : variableKind === "int" ? { int: 0 }
      : variableKind === "enum" ? { enum: "" }
      : { string: "" };
    setDraft({
      ...draft,
      state_variables: [...draft.state_variables, { key, value_kind: variableKind, default_value: defaultValue }],
    });
    setVariableKey("");
    setError(null);
  };

  const updateVariable = (oldKey: string, variable: StoryStateVariable | null) => {
    if (!draft) return;
    const variables = variable
      ? draft.state_variables.map((item) => item.key === oldKey ? variable : item)
      : draft.state_variables.filter((item) => item.key !== oldKey);
    if (variable && variable.key !== oldKey && draft.state_variables.some((item) => item.key === variable.key && item.key !== oldKey)) {
      setError(t("galgame.graph.variableKeyInvalid"));
      return;
    }
    setDraft({
      ...draft,
      state_variables: variables,
    });
  };

  const save = async () => {
    if (!draft || !graph) return;
    setSaving(true);
    setError(null);
    try {
      const saved = await saveGalgameStoryGraph(projectId, { expected_revision: graph.revision, graph: draft });
      setDraft(structuredClone(saved));
      onSaved(saved);
    } catch (cause) {
      setError(extractErrorMessage(cause, t("galgame.graph.saveFailed")));
    } finally {
      setSaving(false);
    }
  };

  const loadOrStartRuntime = async () => {
    if (!graph || !saveSlot.trim()) return;
    setRuntimeBusy(true);
    setError(null);
    try {
      const existing = await getGalgameStoryRuntimeState(projectId, { save_slot: saveSlot.trim() });
      setRuntimeView(existing ?? await startGalgameStoryRuntime(projectId, { save_slot: saveSlot.trim() }));
    } catch (cause) {
      setError(extractErrorMessage(cause, t("galgame.graph.runtimeFailed")));
    } finally {
      setRuntimeBusy(false);
    }
  };

  const chooseRuntimeEdge = async (edgeId: string) => {
    if (!runtimeView) return;
    setRuntimeBusy(true);
    setError(null);
    try {
      const next = await advanceGalgameStoryRuntime(projectId, {
        save_slot: saveSlot.trim(),
        expected_step_index: runtimeView.state.step_index,
        edge_id: edgeId,
      });
      setRuntimeView(next);
    } catch (cause) {
      setError(extractErrorMessage(cause, t("galgame.graph.runtimeFailed")));
    } finally {
      setRuntimeBusy(false);
    }
  };

  const selectedBranch = draft?.branch_groups.find((group) => group.fork_node_id === selectedNodeId);
  const runtimeLabel = (edge: StoryEdge) => branchOption(draft ?? graph!, edge.id)?.label || edgeKindLabel(edge.kind, t);

  if (!graph || !draft) {
    return (
      <section className="a4n-flat-surface border border-book-border/40 px-4 py-3 text-small text-book-text-muted">
        {t("galgame.graph.notGenerated")}
      </section>
    );
  }

  return (
    <section className="a4n-flat-surface min-h-0 overflow-hidden border border-book-border/40">
      <header className="flex flex-wrap items-center justify-between gap-2 border-b border-book-border/40 px-4 py-3">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold text-book-text-main">{t("galgame.graph.title")}</h2>
          <p className="mt-1 text-caption text-book-text-muted">
            {t("galgame.graph.summary", {
              nodes: draft.nodes.length,
              edges: draft.edges.length,
              branches: draft.branch_groups.length,
              loops: draft.loop_groups.length,
              revision: graph.revision,
            })}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <span className={`inline-flex items-center gap-1.5 text-caption ${graphStatus === "stale" ? "text-warning" : "text-book-text-muted"}`}>
            <CircleDot size={13} /> {graphStatus === "stale" ? t("galgame.graph.stale") : graphStatus === "edited" ? t("galgame.graph.edited") : t("galgame.graph.generated")}
          </span>
          {canEditGraph ? <BookButton size="sm" variant="primary" onClick={() => void save()} disabled={saving || !hasUnsavedChanges}>
            <Save size={14} /> {saving ? t("galgame.graph.saving") : t("galgame.graph.save")}
          </BookButton> : null}
        </div>
      </header>
      {graphStatus === "stale" ? <p role="status" className="border-b border-book-border/40 px-4 py-2 text-small text-warning">{t("galgame.graph.sourceChanged")}</p> : null}
      {error ? <p role="alert" className="border-b border-book-border/40 px-4 py-2 text-small text-error">{error}</p> : null}

      <div className="flex flex-wrap items-center gap-2 border-b border-book-border/40 px-3 py-2">
        {canEditGraph ? <>
          <div className="flex min-w-0 basis-full items-center gap-2 sm:basis-auto sm:flex-1">
            <input
              aria-label={t("galgame.graph.newNodeTitle")}
              value={newNodeTitle}
              onChange={(event) => setNewNodeTitle(event.target.value)}
              placeholder={t("galgame.graph.newNodeTitle")}
              className="min-w-0 flex-1 border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs text-book-text-main"
            />
            <select aria-label={t("galgame.graph.nodeKind")} value={newNodeKind}
              onChange={(event) => setNewNodeKind(event.target.value as StoryNode["kind"])}
              className="max-w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs">
              {NODE_KINDS.map((kind) => <option key={kind} value={kind}>{nodeKindLabel(kind, t)}</option>)}
            </select>
            <BookButton size="sm" variant="secondary" onClick={addNode}><Plus size={14} /> {t("galgame.graph.addNode")}</BookButton>
          </div>
          <div className="flex min-w-0 items-center gap-2">
            <select aria-label={t("galgame.graph.connectionKind")} value={connectionKind}
              onChange={(event) => setConnectionKind(event.target.value as StoryEdge["kind"])}
              className="max-w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs">
              {EDGE_KINDS.map((kind) => <option key={kind} value={kind}>{edgeKindLabel(kind, t)}</option>)}
            </select>
            <input aria-label={t("galgame.graph.saveSlot")} value={saveSlot}
              onChange={(event) => setSaveSlot(event.target.value)}
              className="w-20 border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs sm:w-24" />
            <BookButton size="sm" variant="secondary" onClick={() => void loadOrStartRuntime()} disabled={runtimeBusy || graphStatus === "stale"}>
              <Play size={14} /> {runtimeBusy ? t("galgame.graph.runtimeLoading") : t("galgame.graph.startOrResume")}
            </BookButton>
          </div>
        </> : null}
      </div>

      <div className="border-b border-book-border/40 px-3 py-2 lg:hidden">
        <SegmentedControl
          ariaLabel={t("galgame.graph.mobilePane")}
          value={mobilePane}
          onChange={setMobilePane}
          options={[
            { id: "details", label: t("galgame.graph.detailsView") },
            { id: "graph", label: t("galgame.graph.flowView") },
          ]}
        />
      </div>

      <div className="grid min-h-0 grid-cols-1 lg:grid-cols-[minmax(0,1.35fr)_minmax(280px,0.85fr)]">
        <div className={`${mobilePane === "graph" ? "block" : "hidden"} h-[min(38vh,24rem)] min-h-[14rem] min-w-0 border-b border-book-border/40 sm:h-[min(44vh,30rem)] sm:min-h-[18rem] lg:block lg:h-[min(58vh,44rem)] lg:min-h-[26rem] lg:border-b-0 lg:border-r`}>
          <ReactFlow<StoryFlowNode, StoryFlowEdge>
            key={mobilePane}
            nodes={visibleNodes}
            edges={visibleEdges}
            nodeTypes={nodeTypes}
            onNodesChange={onNodesChange}
            onConnect={canEditGraph ? onConnect : undefined}
            onNodeClick={(_event, node) => { setSelectedNodeId(node.id); setSelectedEdgeId(null); setMobilePane("details"); }}
            onEdgeClick={(_event, edge) => { setSelectedEdgeId(edge.id); setSelectedNodeId(null); setMobilePane("details"); }}
            onPaneClick={() => { setSelectedNodeId(null); setSelectedEdgeId(null); }}
            nodesDraggable={canEditGraph}
            nodesConnectable={canEditGraph}
            isValidConnection={(connection) => Boolean(
              draft
                && connection.source
                && connection.target
                && canConnectStoryNodes(draft, connection.source, connection.target, connectionKind),
            )}
            deleteKeyCode={null}
            fitView
            fitViewOptions={{ padding: 0.35, maxZoom: 1 }}
            minZoom={0.2}
            maxZoom={1.5}
            className="bg-book-bg"
            aria-label={t("galgame.graph.canvas")}
          >
            <Controls position="top-right" orientation="horizontal" showInteractive={false} className="lg:!hidden" />
            <Controls position="bottom-right" className="!hidden lg:!flex" />
            <Background color="rgb(var(--color-text-tertiary) / 0.2)" gap={24} size={1} />
          </ReactFlow>
        </div>

        <aside className={`${mobilePane === "details" ? "block" : "hidden"} min-h-[20rem] min-w-0 space-y-4 overflow-auto p-4 lg:block`}>
          {selectedNode ? <>
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-caption text-book-text-muted">{nodeKindLabel(selectedNode.kind, t)}</p>
                <h3 className="mt-1 break-words text-sm font-semibold text-book-text-main">{selectedNode.title}</h3>
              </div>
              <span className="break-all text-caption text-book-text-subtle">{selectedNode.id}</span>
            </div>
            {canEditGraph ? <>
              <div className="flex flex-wrap gap-2">
                <select value={selectedNode.kind}
                  aria-label={t("galgame.graph.nodeKind")}
                  disabled={selectedNode.kind === "loop_entry" || selectedNode.kind === "loop_exit" || Boolean(selectedLoop)}
                  onChange={(event) => {
                    const kind = event.target.value as StoryNode["kind"];
                    const hasBranchGroup = draft?.branch_groups.some(
                      (group) => group.fork_node_id === selectedNode.id,
                    ) ?? false;
                    if (hasBranchGroup && kind !== "fork" && kind !== "choice") {
                      setError(t("galgame.graph.connectionContract"));
                      return;
                    }
                    setDraft((current) => current ? {
                      ...current,
                      nodes: current.nodes.map((node) => node.id === selectedNode.id ? { ...node, kind } : node),
                      terminal_node_ids: terminalIds(current.nodes.map((node) => node.id === selectedNode.id ? { ...node, kind } : node)),
                    } : current);
                  }}
                  className="min-w-0 flex-1 border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs">
                  {NODE_KINDS.map((kind) => <option key={kind} value={kind}>{nodeKindLabel(kind, t)}</option>)}
                </select>
                <button type="button" title={t("galgame.graph.removeNode")} aria-label={t("galgame.graph.removeNode")}
                  disabled={selectedNode.id === draft.entry_node_id || selectedNode.kind === "loop_entry" || selectedNode.kind === "loop_exit"}
                  onClick={removeNode}
                  className="inline-flex h-8 w-8 items-center justify-center border border-book-border/60 text-book-text-muted hover:text-error disabled:opacity-40">
                  <Trash2 size={14} />
                </button>
              </div>
              <label className="block space-y-1 text-caption text-book-text-muted">
                <span>{t("galgame.graph.nodeTitle")}</span>
                <input value={selectedNode.title}
                  onChange={(event) => updateNode(selectedNode.id, (node) => ({ ...node, title: event.target.value }))}
                  className="w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs text-book-text-main" />
              </label>
              <label className="block space-y-1 text-caption text-book-text-muted">
                <span>{t("galgame.graph.nodeSummary")}</span>
                <textarea rows={3} value={selectedNode.summary}
                  onChange={(event) => updateNode(selectedNode.id, (node) => ({ ...node, summary: event.target.value }))}
                  className="w-full resize-y border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs text-book-text-main" />
              </label>
              <div className="space-y-2">
                <div className="flex items-center justify-between gap-2">
                  <h4 className="text-caption font-semibold text-book-text-main">{t("galgame.graph.lines")}</h4>
                  <button type="button" title={t("galgame.graph.addLine")} aria-label={t("galgame.graph.addLine")}
                    onClick={() => updateNode(selectedNode.id, (node) => ({
                      ...node,
                      lines: [...node.lines, {
                        id: newId("line"),
                        display_order: node.lines.reduce((max, item) => Math.max(max, item.display_order), -1) + 1,
                        line_type: "narration",
                        speaker_profile_id: null,
                        text: "",
                        presentation: emptyGalgamePresentation(),
                      }],
                    }))}
                    className="inline-flex h-8 w-8 items-center justify-center border border-book-border/60 text-book-text-muted hover:bg-book-bg-tertiary">
                    <Plus size={14} />
                  </button>
                </div>
                {selectedNode.lines.map((line) => <div key={line.id} className="space-y-1 border-l-2 border-book-border/50 pl-2">
                  <div className="flex items-center justify-between gap-2">
                    <select aria-label={t("galgame.graph.lineType")} value={line.line_type}
                      onChange={(event) => updateNode(selectedNode.id, (node) => ({
                        ...node,
                        lines: node.lines.map((item) => item.id === line.id ? { ...item, line_type: event.target.value } : item),
                      }))}
                      className="min-w-0 border border-book-border/60 bg-book-bg px-2 py-1 text-caption">
                      {LINE_TYPES.map((type) => <option key={type} value={type}>{type}</option>)}
                    </select>
                    <button type="button" title={t("galgame.graph.removeLine")} aria-label={t("galgame.graph.removeLine")}
                      onClick={() => updateNode(selectedNode.id, (node) => ({ ...node, lines: node.lines.filter((item) => item.id !== line.id) }))}
                      className="inline-flex h-7 w-7 items-center justify-center text-book-text-muted hover:text-error">
                      <Trash2 size={13} />
                    </button>
                  </div>
                  <textarea rows={2} aria-label={t("galgame.graph.lineText")} value={line.text}
                    onChange={(event) => updateNode(selectedNode.id, (node) => ({
                      ...node,
                      lines: node.lines.map((item) => item.id === line.id ? { ...item, text: event.target.value } : item),
                    }))}
                    className="w-full resize-y border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs" />
                </div>)}
              </div>
              {selectedBranch ? <div className="space-y-2 border-t border-book-border/40 pt-3">
                <h4 className="text-caption font-semibold text-book-text-main">{t("galgame.graph.branchOptions")}</h4>
                {selectedBranch.options.map((option) => <div key={option.id} className="space-y-1 text-caption text-book-text-muted">
                  <span>{option.edge_id}</span>
                  <input value={option.label}
                    onChange={(event) => setDraft((current) => current ? {
                      ...current,
                      branch_groups: current.branch_groups.map((group) => group.id === selectedBranch.id
                        ? { ...group, options: group.options.map((item) => item.id === option.id ? { ...item, label: event.target.value } : item) }
                        : group),
                    } : current)}
                    className="w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs text-book-text-main" />
                  <JsonValueField key={`${option.id}:condition`} label={t("galgame.graph.optionCondition")} value={option.condition}
                    onChange={(value) => setDraft((current) => current ? {
                      ...current,
                      branch_groups: current.branch_groups.map((group) => group.id === selectedBranch.id
                        ? { ...group, options: group.options.map((item) => item.id === option.id ? { ...item, condition: value as typeof item.condition } : item) }
                        : group),
                    } : current)} />
                  <JsonValueField key={`${option.id}:ops`} label={t("galgame.graph.stateOperations")} value={option.state_operations}
                    onChange={(value) => setDraft((current) => current ? {
                      ...current,
                      branch_groups: current.branch_groups.map((group) => group.id === selectedBranch.id
                        ? { ...group, options: group.options.map((item) => item.id === option.id ? { ...item, state_operations: value as typeof item.state_operations } : item) }
                        : group),
                    } : current)} />
                </div>)}
                <label className="block space-y-1 text-caption text-book-text-muted">
                  <span>{t("galgame.graph.joinNode")}</span>
                  <select value={selectedBranch.join_node_id ?? ""}
                    onChange={(event) => setDraft((current) => current ? {
                      ...current,
                      branch_groups: current.branch_groups.map((group) => group.id === selectedBranch.id
                        ? { ...group, join_node_id: event.target.value || null }
                        : group),
                    } : current)}
                    className="w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs">
                    <option value="">{t("galgame.graph.noJoin")}</option>
                    {draft.nodes.filter((node) => node.kind === "join").map((node) => <option key={node.id} value={node.id}>{node.title}</option>)}
                  </select>
                </label>
              </div> : null}
              <details className="border-t border-book-border/40 pt-3">
                <summary className="cursor-pointer text-caption font-semibold text-book-text-main">{t("galgame.graph.advancedNodeData")}</summary>
                <JsonValueField label={t("galgame.graph.stateOperations")} value={selectedNode.state_operations}
                  onChange={(value) => updateNode(selectedNode.id, (node) => ({ ...node, state_operations: value as StoryNode["state_operations"] }))} />
                <JsonValueField label={t("galgame.graph.mediaIntents")} value={selectedNode.media_intents}
                  onChange={(value) => updateNode(selectedNode.id, (node) => ({ ...node, media_intents: value as StoryNode["media_intents"] }))} />
              </details>
            </> : <p className="whitespace-pre-wrap text-xs leading-relaxed text-book-text-sub">{selectedNode.summary}</p>}

            <dl className="grid grid-cols-2 gap-x-4 gap-y-2 border-t border-book-border/40 pt-3 text-caption">
              <dt className="text-book-text-muted">{t("galgame.graph.sources")}</dt><dd>{selectedNode.source_refs.length}</dd>
              <dt className="text-book-text-muted">{t("galgame.graph.mediaIntents")}</dt><dd>{selectedNode.media_intents.length}</dd>
            </dl>

            {canEditGraph ? <div className="space-y-2 border-t border-book-border/40 pt-3">
              <h4 className="text-caption font-semibold text-book-text-main">{t("galgame.graph.addLoop")}</h4>
              {selectedLoop ? <div className="space-y-2">
                <label className="block space-y-1 text-caption text-book-text-muted">
                  <span>{t("galgame.graph.loopLimit")}</span>
                  <input type="number" min={1} max={10_000}
                    value={selectedLoop.max_iterations ?? ""}
                    onChange={(event) => setDraft((current) => current ? {
                      ...current,
                      loop_groups: current.loop_groups.map((group) => group.id === selectedLoop.id
                        ? { ...group, max_iterations: event.target.value === "" ? null : Number(event.target.value) }
                        : group),
                    } : current)}
                    className="w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs" />
                </label>
                <JsonValueField label={t("galgame.graph.loopCondition")} value={selectedLoop.guard}
                  onChange={(value) => setDraft((current) => current ? {
                    ...current,
                    loop_groups: current.loop_groups.map((group) => group.id === selectedLoop.id
                      ? { ...group, guard: value as StoryLoopGroup["guard"] }
                      : group),
                  } : current)} />
                <JsonValueField label={t("galgame.graph.stateOperations")} value={selectedLoop.state_operations}
                  onChange={(value) => setDraft((current) => current ? {
                    ...current,
                    loop_groups: current.loop_groups.map((group) => group.id === selectedLoop.id
                      ? { ...group, state_operations: value as StoryLoopGroup["state_operations"] }
                      : group),
                  } : current)} />
                <BookButton size="sm" variant="danger" onClick={() => {
                  setDraft(removeStoryLoop(draft, selectedLoop));
                  setError(null);
                }}><Trash2 size={14} /> {t("galgame.graph.removeLoop")}</BookButton>
              </div> :
                <div className="flex items-center gap-2">
                  <input type="number" min={1} max={10_000} value={loopMaxIterations}
                    onChange={(event) => setLoopMaxIterations(event.target.value)}
                    aria-label={t("galgame.graph.loopLimit")}
                    className="w-24 border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs" />
                  <BookButton size="sm" variant="secondary" onClick={addLoop}>
                    <Repeat2 size={14} /> {t("galgame.graph.addLoop")}
                  </BookButton>
                </div>}
            </div> : null}
          </> : selectedEdge ? <div className="space-y-3">
            <div>
              <p className="text-caption text-book-text-muted">{edgeKindLabel(selectedEdge.kind, t)}</p>
              <h3 className="mt-1 break-all text-sm font-semibold text-book-text-main">{selectedEdge.id}</h3>
            </div>
            <p className="break-all text-caption text-book-text-muted">{selectedEdge.from_node_id} → {selectedEdge.to_node_id}</p>
            {canEditGraph ? <>
              <JsonValueField label={t("galgame.graph.edgeCondition")} value={selectedEdge.guard}
                onChange={(value) => updateEdge(selectedEdge.id, (edge) => ({ ...edge, guard: value as StoryEdge["guard"] }))} />
              <JsonValueField label={t("galgame.graph.stateOperations")} value={selectedEdge.state_operations}
                onChange={(value) => updateEdge(selectedEdge.id, (edge) => ({ ...edge, state_operations: value as StoryEdge["state_operations"] }))} />
              <BookButton size="sm" variant="danger" onClick={() => removeEdge(selectedEdge.id)}>
                <Trash2 size={14} /> {t("galgame.graph.removeConnection")}
              </BookButton>
            </> : null}
          </div> : <p className="text-small text-book-text-muted">{t("galgame.graph.selectNode")}</p>}

          {canEditGraph ? <details className="border-t border-book-border/40 pt-3">
            <summary className="cursor-pointer text-caption font-semibold text-book-text-main">{t("galgame.graph.stateVariables")}</summary>
            <div className="mt-2 flex flex-wrap gap-2">
              <input aria-label={t("galgame.graph.variableKey")} value={variableKey}
                onChange={(event) => setVariableKey(event.target.value)} placeholder={t("galgame.graph.variableKey")}
                className="min-w-0 flex-1 border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs" />
              <select aria-label={t("galgame.graph.variableType")} value={variableKind}
                onChange={(event) => setVariableKind(event.target.value as StoryStateVariable["value_kind"])}
                className="border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs">
                {["bool", "int", "string", "enum"].map((kind) => <option key={kind} value={kind}>{kind}</option>)}
              </select>
              <button type="button" title={t("galgame.graph.addVariable")} aria-label={t("galgame.graph.addVariable")}
                onClick={addVariable} className="inline-flex h-8 w-8 items-center justify-center border border-book-border/60">
                <Plus size={14} />
              </button>
            </div>
            <div className="mt-2 space-y-2">
              {draft.state_variables.map((variable) => <div key={variable.key} className="flex min-w-0 items-center gap-2">
                <span className="min-w-0 flex-1 truncate text-xs text-book-text-main">{variable.key}</span>
                <select value={variable.value_kind}
                  onChange={(event) => {
                    const kind = event.target.value as StoryStateVariable["value_kind"];
                    const value: StoryValue = kind === "bool" ? { bool: false }
                      : kind === "int" ? { int: 0 }
                      : kind === "enum" ? { enum: "" }
                      : { string: "" };
                    updateVariable(variable.key, { ...variable, value_kind: kind, default_value: value });
                  }}
                  className="border border-book-border/60 bg-book-bg px-2 py-1 text-caption">
                  {["bool", "int", "string", "enum"].map((kind) => <option key={kind} value={kind}>{kind}</option>)}
                </select>
                {variable.value_kind === "bool" ? <input type="checkbox" checked={"bool" in variable.default_value && variable.default_value.bool}
                  onChange={(event) => updateVariable(variable.key, { ...variable, default_value: { bool: event.target.checked } })} />
                  : <input type={variable.value_kind === "int" ? "number" : "text"}
                    value={variable.value_kind === "int" && "int" in variable.default_value ? variable.default_value.int : storyValueText(variable.default_value)}
                    onChange={(event) => {
                      const value: StoryValue = variable.value_kind === "int"
                        ? { int: Number(event.target.value) }
                        : variable.value_kind === "enum" ? { enum: event.target.value } : { string: event.target.value };
                      updateVariable(variable.key, { ...variable, default_value: value });
                    }}
                    className="w-24 min-w-0 border border-book-border/60 bg-book-bg px-2 py-1 text-xs" />}
                <button type="button" title={t("galgame.graph.removeVariable")} aria-label={t("galgame.graph.removeVariable")}
                  onClick={() => updateVariable(variable.key, null)}
                  className="inline-flex h-7 w-7 items-center justify-center text-book-text-muted hover:text-error">
                  <Trash2 size={13} />
                </button>
              </div>)}
            </div>
          </details> : null}

          {runtimeView ? <section className="space-y-2 border-t border-book-border/40 pt-3" aria-live="polite">
            <div className="flex items-center justify-between gap-2">
              <h3 className="text-sm font-semibold text-book-text-main">{t("galgame.graph.playtest")}</h3>
              <span className="text-caption tabular-nums text-book-text-muted">{runtimeView.state.step_index}</span>
            </div>
            <h4 className="text-xs font-semibold text-book-text-main">{runtimeView.current_node.title}</h4>
            {runtimeView.current_node.lines.map((line) => <p key={line.id} className="whitespace-pre-wrap text-xs text-book-text-sub">{line.text}</p>)}
            {runtimeView.available_edges.length ? <div className="flex flex-wrap gap-2">
              {runtimeView.available_edges.map((edge) => <BookButton key={edge.id} size="sm" variant="secondary" disabled={runtimeBusy}
                onClick={() => void chooseRuntimeEdge(edge.id)}><ArrowRight size={13} /> {runtimeLabel(edge)}</BookButton>)}
            </div> : <p className="text-caption text-book-text-muted">{t("galgame.graph.runComplete")}</p>}
          </section> : null}
        </aside>
      </div>
    </section>
  );
};

const JsonValueField: React.FC<{
  label: string;
  onChange: (value: unknown) => void;
  value: unknown;
}> = ({ label, onChange, value }) => {
  const { t } = useI18n();
  const id = useId();
  const [text, setText] = useState(JSON.stringify(value, null, 2));
  const [invalid, setInvalid] = useState(false);
  const [dirty, setDirty] = useState(false);
  useEffect(() => {
    if (!dirty) setText(JSON.stringify(value, null, 2));
  }, [dirty, value]);
  return (
    <div className="mt-2 space-y-1 text-caption text-book-text-muted">
      <label className="block" htmlFor={id}>{label}</label>
      <textarea id={id} rows={4} value={text} aria-invalid={invalid}
        onChange={(event) => { setText(event.target.value); setDirty(true); setInvalid(false); }}
        className={`w-full resize-y border bg-book-bg px-2 py-1.5 font-mono text-caption text-book-text-main ${invalid ? "border-error" : "border-book-border/60"}`} />
      {dirty ? <button type="button" onClick={() => {
        try {
          const value = JSON.parse(text);
          onChange(value);
          setDirty(false);
          setInvalid(false);
        } catch {
          setInvalid(true);
        }
      }} className="border border-book-border/60 px-2 py-1 text-caption text-book-text-main hover:bg-book-bg-tertiary">
        {t("galgame.graph.applyJson")}
      </button> : null}
    </div>
  );
};
