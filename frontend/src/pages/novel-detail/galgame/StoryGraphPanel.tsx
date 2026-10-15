import React, { memo, useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
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
  type Viewport,
} from "@xyflow/react";
import { ArrowRight, CircleDot, GitBranch, Play, Plus, Repeat2, Save, Trash2 } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import {
  advanceGalgameStoryRuntime,
  getGalgameStoryRuntimeState,
  saveGalgameStoryGraph,
  startGalgameStoryRuntime,
} from "../../../api/galgame";
import type {
  GalgameStoryRuntimeView,
  StoryEdge,
  StoryExpression,
  StoryGraph,
  StoryLoopGroup,
  StoryNode,
  StoryStateVariable,
  StoryValue,
} from "../../../api/generated/host";
import { BookButton } from "../../../components/ui/BookButton";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import { confirmDialog } from "../../../components/feedback/ConfirmDialog";
import { useDirty } from "../../../components/navigation/dirtyRegistry";
import { useI18n, type MessageTranslator } from "../../../i18n";
import { emptyGalgamePresentation, LINE_TYPES } from "./galgameTabShared";
import { containsUnrepresentableJsonNumber } from "./graphJson";

interface StoryGraphPanelProps {
  canEdit: boolean;
  graph: StoryGraph | null;
  graphStatus: string | null;
  projectId: number;
  onSaved: (graph: StoryGraph) => void;
}

type RuntimeBusyAction = "opening" | "advancing";

interface RuntimeSession {
  graphId: string;
  graphRevision: number;
  projectId: number;
  saveSlot: string;
  view: GalgameStoryRuntimeView;
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
const USER_CREATABLE_NODE_KINDS: StoryNode["kind"][] = NODE_KINDS.filter(
  (kind) => kind !== "loop_entry" && kind !== "loop_exit",
);
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

function combineStoryConditions(conditions: Array<StoryExpression | null | undefined>): StoryExpression | null {
  const present = conditions.filter((condition): condition is StoryExpression => condition != null);
  if (present.length === 0) return null;
  return present.length === 1 ? present[0] : { all: present };
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
  if (wouldCreateUnmarkedStoryCycle(graph, sourceId, targetId)) return false;

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

function wouldCreateUnmarkedStoryCycle(
  graph: StoryGraph,
  sourceId: string,
  targetId: string,
): boolean {
  const outgoing = new Map<string, string[]>();
  for (const edge of graph.edges) {
    if (edge.kind === "loop_back") continue;
    const targets = outgoing.get(edge.from_node_id) ?? [];
    targets.push(edge.to_node_id);
    outgoing.set(edge.from_node_id, targets);
  }
  const pending = [targetId];
  const visited = new Set<string>();
  while (pending.length > 0) {
    const nodeId = pending.pop();
    if (!nodeId || visited.has(nodeId)) continue;
    if (nodeId === sourceId) return true;
    visited.add(nodeId);
    pending.push(...(outgoing.get(nodeId) ?? []));
  }
  return false;
}

function hasUnmarkedStoryCycle(graph: StoryGraph): boolean {
  return graph.edges.some((edge) =>
    edge.kind !== "loop_back"
      && wouldCreateUnmarkedStoryCycle(graph, edge.from_node_id, edge.to_node_id),
  );
}

function canChangeStoryNodeKind(graph: StoryGraph, nodeId: string, kind: StoryNode["kind"]): boolean {
  if (kind === "loop_entry" || kind === "loop_exit") return false;
  const node = graph.nodes.find((item) => item.id === nodeId);
  if (!node) return false;
  const incoming = graph.edges.filter((edge) => edge.to_node_id === nodeId);
  const outgoing = graph.edges.filter((edge) => edge.from_node_id === nodeId);
  if (node.kind === "loop_entry" || node.kind === "loop_exit") return false;
  const isEntry = node.id === graph.entry_node_id;
  if (kind === "terminal") return outgoing.length === 0 && incoming.length === (isEntry ? 0 : 1);
  if (node.kind === "terminal" && kind === "narrative") {
    return outgoing.length === 0 && incoming.length === (isEntry ? 0 : 1);
  }
  if (kind === "join") return !isEntry && incoming.length >= 2 && outgoing.length === 1;
  if (kind === "fork" || kind === "choice") {
    return incoming.length <= 1 && outgoing.length >= 2 && outgoing.every((edge) => edge.kind === "choice");
  }
  return outgoing.length === 1
    && incoming.length === (isEntry ? 0 : 1)
    && !outgoing.some((edge) => edge.kind === "choice" || edge.kind === "loop_back");
}

function hasInvalidStoryInteger(value: unknown): boolean {
  if (Array.isArray(value)) return value.some(hasInvalidStoryInteger);
  if (value === null || typeof value !== "object") return false;
  const record = value as Record<string, unknown>;
  if (typeof record.int === "string" && !isValidI64Decimal(record.int)) return true;
  if (typeof record.amount === "string" && !isValidI64Decimal(record.amount)) return true;
  return Object.values(record).some(hasInvalidStoryInteger);
}

function hasInvalidStoryGraphInteger(graph: StoryGraph): boolean {
  const typedState = [
    ...graph.state_variables.map((variable) => variable.default_value),
    ...graph.nodes.flatMap((node) => [node.state_before, node.state_after, node.state_operations]),
    ...graph.edges.flatMap((edge) => [edge.guard, edge.state_operations]),
    ...graph.branch_groups.flatMap((group) => group.options.flatMap((option) => [option.condition, option.state_operations])),
    ...graph.loop_groups.flatMap((group) => [group.guard, group.state_operations]),
  ];
  return typedState.some(hasInvalidStoryInteger);
}

function findStoryGraphDraftError(graph: StoryGraph): string | null {
  if (graph.branch_groups.some((group) => group.options.some((option) => !option.label.trim()))) {
    return "galgame.graph.branchOptionLabelRequired";
  }
  if (hasInvalidStoryGraphInteger(graph)) return "galgame.graph.stateVariableIntInvalid";
  const adjacency = new Map<string, string[]>();
  for (const edge of graph.edges) {
    const next = adjacency.get(edge.from_node_id) ?? [];
    next.push(edge.to_node_id);
    adjacency.set(edge.from_node_id, next);
  }
  const reachable = new Set<string>();
  const pending = [graph.entry_node_id];
  while (pending.length > 0) {
    const nodeId = pending.pop();
    if (!nodeId || reachable.has(nodeId)) continue;
    reachable.add(nodeId);
    pending.push(...(adjacency.get(nodeId) ?? []));
  }
  if (reachable.size !== graph.nodes.length) return "galgame.graph.disconnectedNodes";
  if (hasUnmarkedStoryCycle(graph)) return "galgame.graph.unmarkedCycle";
  for (const node of graph.nodes) {
    const incoming = graph.edges.filter((edge) => edge.to_node_id === node.id);
    const outgoing = graph.edges.filter((edge) => edge.from_node_id === node.id);
    const isEntry = node.id === graph.entry_node_id;
    const isLoopBody = graph.loop_groups.some((group) => group.body_exit_node_id === node.id);
    if (node.kind === "terminal" && outgoing.length !== 0) return "galgame.graph.invalidNodeStructure";
    if ((node.kind === "narrative" || node.kind === "chapter_boundary")
      && outgoing.length !== (isLoopBody ? 2 : 1)) return "galgame.graph.invalidNodeStructure";
    if ((node.kind === "fork" || node.kind === "choice")
      && (outgoing.length < 2 || outgoing.some((edge) => edge.kind !== "choice"))) return "galgame.graph.invalidNodeStructure";
    if (node.kind === "join" && (incoming.length < 2 || outgoing.length !== 1)) return "galgame.graph.invalidNodeStructure";
    if (!isEntry && incoming.length === 0) return "galgame.graph.disconnectedNodes";
  }
  return null;
}

function hasStoryConnection(graph: StoryGraph, sourceId: string, targetId: string, kind: StoryEdge["kind"]): boolean {
  return graph.edges.some((edge) => edge.from_node_id === sourceId
    && edge.to_node_id === targetId
    && edge.kind === kind);
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

function collapseBranchGroups(
  graph: StoryGraph,
  removedEdgeIds: Set<string>,
): Pick<StoryGraph, "edges" | "branch_groups" | "nodes" | "terminal_node_ids"> {
  let edges = graph.edges.filter((edge) => !removedEdgeIds.has(edge.id));
  const branchGroups: StoryGraph["branch_groups"] = [];
  const removedGroupIds = new Set<string>();
  for (const group of graph.branch_groups) {
    const options = group.options.filter((option) => !removedEdgeIds.has(option.edge_id));
    if (options.length >= 2) {
      branchGroups.push({ ...group, options });
      continue;
    }
    removedGroupIds.add(group.id);
    if (options.length === 1) {
      const [option] = options;
      const branchEdge = edges.find((edge) => edge.id === option.edge_id);
      if (branchEdge) {
        edges = edges.map((edge) => edge.id === option.edge_id
          ? {
            ...edge,
            kind: "sequence",
            branch_group_id: null,
            guard: combineStoryConditions([edge.guard, option.condition]),
            state_operations: [...edge.state_operations, ...option.state_operations],
          }
          : edge);
      }
    }
  }
  edges = edges.filter((edge) => !edge.branch_group_id || !removedGroupIds.has(edge.branch_group_id));
  const outgoing = new Set(edges.map((edge) => edge.from_node_id));
  const incomingCount = new Map<string, number>();
  edges.forEach((edge) => incomingCount.set(edge.to_node_id, (incomingCount.get(edge.to_node_id) ?? 0) + 1));
  const nodes = graph.nodes.map((node) => {
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
  return { edges, branch_groups: branchGroups, nodes, terminal_node_ids: terminalIds(nodes) };
}

function loopForStructuralEdge(graph: StoryGraph, edgeId: string): StoryLoopGroup | null {
  const edge = graph.edges.find((item) => item.id === edgeId);
  if (!edge) return null;
  return graph.loop_groups.find((group) =>
    group.back_edge_id === edgeId
      || (edge.from_node_id === group.entry_node_id && edge.to_node_id === group.body_exit_node_id)
      || (edge.from_node_id === group.body_exit_node_id && edge.to_node_id === group.exit_node_id),
  ) ?? null;
}

function removeStoryNode(graph: StoryGraph, nodeId: string): StoryGraph {
  if (nodeId === graph.entry_node_id) throw new Error("galgame.graph.entryCannotDelete");
  if (graph.loop_groups.some((group) => group.body_exit_node_id === nodeId)) {
    throw new Error("galgame.graph.loopBodyCannotDelete");
  }
  let working = graph;
  for (const group of graph.loop_groups.filter((group) =>
    [group.entry_node_id, group.exit_node_id].includes(nodeId),
  )) working = removeStoryLoop(working, group);
  const node = working.nodes.find((item) => item.id === nodeId);
  if (!node) return working;
  const incoming = working.edges.filter((edge) => edge.to_node_id === nodeId);
  const outgoing = working.edges.filter((edge) => edge.from_node_id === nodeId);
  const incomingEdge = incoming.length === 1 ? incoming[0] : null;
  const outgoingEdge = outgoing.length === 1 ? outgoing[0] : null;
  const canBypass = Boolean(
    incomingEdge && outgoingEdge
      && incomingEdge.kind !== "loop_back"
      && outgoingEdge.kind !== "loop_back"
      && incomingEdge.branch_group_id == null
      && outgoingEdge.branch_group_id == null
      && incomingEdge.guard == null
      && outgoingEdge.guard == null
      && incomingEdge.state_operations.length === 0
      && outgoingEdge.state_operations.length === 0
      && node.state_operations.length === 0
      && Object.keys(node.state_before).length === 0
      && Object.keys(node.state_after).length === 0
      && (node.kind === "narrative" || node.kind === "chapter_boundary"),
  );
  const canRemoveBranchTarget = Boolean(
    incomingEdge?.kind === "choice"
      && outgoing.length === 0,
  );
  const predecessor = incomingEdge && working.nodes.find((item) => item.id === incomingEdge.from_node_id);
  const predecessorInLoop = predecessor && working.loop_groups.some((group) => group.body_exit_node_id === predecessor.id);
  const canRemoveTerminal = node.kind === "terminal"
    && working.nodes.length > 1
    && incoming.length === 1
    && outgoing.length === 0
    && predecessor != null
    && (predecessor.kind === "narrative" || predecessor.kind === "chapter_boundary")
    && !predecessorInLoop
    && incomingEdge?.kind !== "choice";
  if (!canBypass && !canRemoveBranchTarget && !canRemoveTerminal && (incoming.length > 0 || outgoing.length > 0)) {
    throw new Error("galgame.graph.removeNodeWouldBreakGraph");
  }
  const removedEdgeIds = new Set(
    working.edges
      .filter((edge) => edge.from_node_id === nodeId || edge.to_node_id === nodeId)
      .map((edge) => edge.id),
  );
  let edges = working.edges.filter((edge) => !removedEdgeIds.has(edge.id));
  if (canBypass && incomingEdge && outgoingEdge) {
    edges = [...edges, {
      ...incomingEdge,
      to_node_id: outgoingEdge.to_node_id,
      to_port: outgoingEdge.to_port,
      display_order: outgoingEdge.display_order,
    }];
  }
  const base: StoryGraph = {
    ...working,
    nodes: working.nodes.filter((item) => item.id !== nodeId),
    edges,
    branch_groups: working.branch_groups.map((group) => ({
      ...group,
      join_node_id: group.join_node_id === nodeId ? null : group.join_node_id,
    })),
  };
  const collapsed = collapseBranchGroups(base, canBypass ? new Set<string>() : removedEdgeIds);
  const nodes = collapsed.nodes.map((item) => {
    if (canRemoveTerminal && predecessor?.id === item.id && !collapsed.edges.some((edge) => edge.from_node_id === item.id)) {
      return { ...item, kind: "terminal" as const };
    }
    return item;
  });
  return {
    ...base,
    nodes,
    edges: collapsed.edges,
    branch_groups: collapsed.branch_groups,
    terminal_node_ids: terminalIds(nodes),
  };
}

function removeStoryEdge(graph: StoryGraph, edgeId: string): StoryGraph {
  const edge = graph.edges.find((item) => item.id === edgeId);
  if (!edge) return graph;
  const loop = loopForStructuralEdge(graph, edgeId);
  if (loop) {
    const withoutLoop = removeStoryLoop(graph, loop);
    return withoutLoop.edges.some((item) => item.id === edgeId)
      ? removeStoryEdge(withoutLoop, edgeId)
      : withoutLoop;
  }
  const source = graph.nodes.find((node) => node.id === edge.from_node_id);
  const target = graph.nodes.find((node) => node.id === edge.to_node_id);
  if (!source || !target) return graph;
  const sourceIncoming = graph.edges.filter((item) => item.to_node_id === source.id).length;
  const targetIncoming = graph.edges.filter((item) => item.to_node_id === target.id).length;
  const sourceCanBecomeTerminal = ["narrative", "chapter_boundary"].includes(source.kind)
    && sourceIncoming <= 1
    && !graph.loop_groups.some((group) => group.body_exit_node_id === source.id)
    && graph.edges.find((item) => item.to_node_id === source.id)?.kind !== "choice";
  if (edge.kind === "choice") {
    if (target.kind !== "terminal" || targetIncoming !== 1) {
      throw new Error("galgame.graph.removeConnectionWouldBreakGraph");
    }
    return removeStoryNode(graph, target.id);
  }
  const canRemove = target.kind === "join" && targetIncoming >= 3 && sourceCanBecomeTerminal;
  if (!canRemove) throw new Error("galgame.graph.removeConnectionWouldBreakGraph");
  const collapsed = collapseBranchGroups(graph, new Set([edgeId]));
  const nodes = collapsed.nodes.map((node) => {
    if (node.id === source.id && !collapsed.edges.some((item) => item.from_node_id === node.id)) {
      return { ...node, kind: "terminal" as const };
    }
    return node;
  });
  return { ...graph, ...collapsed, nodes, terminal_node_ids: terminalIds(nodes) };
}

function branchGroupsCollapsedByRemoval(
  graph: StoryGraph,
  removedEdgeIds: Set<string>,
  removedForkNodeId?: string,
): StoryGraph["branch_groups"] {
  return graph.branch_groups.filter((group) => {
    if (group.fork_node_id === removedForkNodeId) return false;
    const remaining = group.options.filter((option) => !removedEdgeIds.has(option.edge_id)).length;
    return remaining > 0 && remaining < 2;
  });
}

function isValidLoopLimit(value: string): boolean {
  const number = Number(value);
  return value.trim() !== "" && Number.isInteger(number) && number >= 1 && number <= 10_000;
}

function isValidI64Decimal(value: string): boolean {
  if (value !== value.trim() || !/^[+-]?\d+$/.test(value)) return false;
  try {
    const number = BigInt(value);
    return number >= -(1n << 63n) && number <= (1n << 63n) - 1n;
  } catch {
    return false;
  }
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
  const [selectedLoopLimitText, setSelectedLoopLimitText] = useState<string | null>(null);
  const [variableKey, setVariableKey] = useState("");
  const [variableKind, setVariableKind] = useState<StoryStateVariable["value_kind"]>("bool");
  const [saveSlot, setSaveSlot] = useState("main");
  const [runtimeSession, setRuntimeSession] = useState<RuntimeSession | null>(null);
  const [mobilePane, setMobilePane] = useState<MobileGraphPane>("details");
  const [graphViewport, setGraphViewport] = useState<Viewport | null>(null);
  const [saving, setSaving] = useState(false);
  const [runtimeBusyAction, setRuntimeBusyAction] = useState<RuntimeBusyAction | null>(null);
  const [dirtyJsonFieldIds, setDirtyJsonFieldIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const runtimeBusyRef = useRef(false);
  const runtimeRequestSequenceRef = useRef(0);
  const currentProjectIdRef = useRef(projectId);
  currentProjectIdRef.current = projectId;

  useEffect(() => {
    runtimeRequestSequenceRef.current += 1;
    runtimeBusyRef.current = false;
    setDraft(graph ? structuredClone(graph) : null);
    setSelectedNodeId(graph?.nodes[0]?.id ?? null);
    setSelectedEdgeId(null);
    setSelectedLoopLimitText(null);
    setGraphViewport(null);
    setRuntimeSession(null);
    setRuntimeBusyAction(null);
    setMobilePane("details");
    setError(null);
  }, [graph?.id, graph?.revision, projectId]);

  const selectedNode = draft?.nodes.find((node) => node.id === selectedNodeId) ?? null;
  const selectedEdge = draft?.edges.find((edge) => edge.id === selectedEdgeId) ?? null;
  const selectedLoop = draft?.loop_groups.find((group) => group.body_exit_node_id === selectedNodeId) ?? null;
  const selectedNodeTitleInvalid = selectedNode != null && !selectedNode.title.trim();
  const persistedLoopLimit = selectedLoop ? String(selectedLoop.max_iterations ?? "") : null;
  const loopLimitText = selectedLoopLimitText ?? persistedLoopLimit ?? "";
  const loopLimitInvalid = selectedLoop != null && !isValidLoopLimit(loopLimitText);
  const newLoopLimitInvalid = !isValidLoopLimit(loopMaxIterations);
  const hasEmptyNodeTitle = Boolean(draft?.nodes.some((node) => !node.title.trim()));
  const draftStructureError = draft ? findStoryGraphDraftError(draft) : null;
  const invalidStateVariable = draft?.state_variables.find((variable) =>
    variable.value_kind === "int"
      && "int" in variable.default_value
      && !isValidI64Decimal(variable.default_value.int),
  ) ?? null;
  const hasInvalidStateVariable = invalidStateVariable != null;
  useEffect(() => {
    setSelectedLoopLimitText(persistedLoopLimit);
  }, [selectedLoop?.body_exit_node_id, selectedLoop?.max_iterations]);
  const canEditGraph = canEdit && graphStatus !== "stale";
  const canMutateGraph = canEditGraph && !saving;
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
  const hasUnappliedJsonDrafts = dirtyJsonFieldIds.size > 0;
  const updateJsonDraftState = useCallback((id: string, dirty: boolean) => {
    setDirtyJsonFieldIds((current) => {
      if (current.has(id) === dirty) return current;
      const next = new Set(current);
      if (dirty) next.add(id);
      else next.delete(id);
      return next;
    });
  }, []);
  const blockForUnappliedJsonDraft = () => {
    return hasUnappliedJsonDrafts;
  };
  const normalizedSaveSlot = saveSlot.trim();
  const saveSlotError = !normalizedSaveSlot
    ? t("galgame.graph.saveSlotRequired")
    : new TextEncoder().encode(normalizedSaveSlot).length > 80
      ? t("galgame.graph.saveSlotTooLong")
      : null;
  const runtimeBusy = runtimeBusyAction != null;
  const runtimeView = runtimeSession
    && runtimeSession.projectId === projectId
    && runtimeSession.graphId === graph?.id
    && runtimeSession.graphRevision === graph?.revision
    && runtimeSession.saveSlot === normalizedSaveSlot
    && !hasUnsavedChanges
    && !hasUnappliedJsonDrafts
    && graphStatus !== "stale"
    ? runtimeSession.view
    : null;

  const discardGraphDraft = () => {
    if (saving) return false;
    setDraft(graph ? structuredClone(graph) : null);
    setSelectedNodeId(graph?.nodes[0]?.id ?? null);
    setSelectedEdgeId(null);
    setError(null);
    return true;
  };
  useDirty(`galgame-story-graph:${projectId}`, hasUnsavedChanges, discardGraphDraft, () => !saving);

  const hasUnappliedGraphInputs = Boolean(newNodeTitle || variableKey)
    || loopMaxIterations !== "3"
    || (selectedLoop != null && selectedLoopLimitText != null && selectedLoopLimitText !== persistedLoopLimit);
  useDirty(`galgame-story-graph-inputs:${projectId}`, hasUnappliedGraphInputs, () => {
    setNewNodeTitle("");
    setNewNodeKind("narrative");
    setConnectionKind("sequence");
    setLoopMaxIterations("3");
    setVariableKey("");
    setVariableKind("bool");
    setSelectedLoopLimitText(persistedLoopLimit);
  });

  const beginRuntimeRequest = (action: RuntimeBusyAction): number | null => {
    if (runtimeBusyRef.current) return null;
    runtimeBusyRef.current = true;
    const requestId = runtimeRequestSequenceRef.current + 1;
    runtimeRequestSequenceRef.current = requestId;
    setRuntimeBusyAction(action);
    return requestId;
  };
  const isCurrentRuntimeRequest = (requestId: number, requestProjectId: number) =>
    runtimeRequestSequenceRef.current === requestId
      && currentProjectIdRef.current === requestProjectId;
  const confirmStructuralRemoval = async (
    titleKey: "galgame.graph.removeNode" | "galgame.graph.removeConnection",
    removesLoop: boolean,
    collapsedBranchCount: number,
  ) => {
    const effects = [
      titleKey === "galgame.graph.removeNode"
        ? t("galgame.graph.removeNodeMessage")
        : t("galgame.graph.removeConnectionMessage"),
      removesLoop ? t("galgame.graph.removeLoopCascade") : null,
      collapsedBranchCount > 0 ? t("galgame.graph.removeBranchCollapse", { count: collapsedBranchCount }) : null,
    ].filter((effect): effect is string => effect != null);
    return confirmDialog({
      title: t(titleKey),
      message: effects.join("\n\n"),
      confirmText: t("common.delete"),
      cancelText: t("common.cancel"),
      dialogType: "warning",
    });
  };

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
    if (hasUnappliedJsonDrafts) return;
    if (!draft || !connection.source || !connection.target || connection.source === connection.target) return;
    const source = draft.nodes.find((node) => node.id === connection.source);
    const target = draft.nodes.find((node) => node.id === connection.target);
    if (!source || !target) return;
    if (hasStoryConnection(draft, source.id, target.id, connectionKind)) {
      setError(t("galgame.graph.connectionExists"));
      return;
    }
    if (!canConnectStoryNodes(draft, source.id, target.id, connectionKind)) {
      setError(wouldCreateUnmarkedStoryCycle(draft, source.id, target.id)
        ? t("galgame.graph.unmarkedCycle")
        : t("galgame.graph.connectionContract"));
      return;
    }
    setError(null);
    setDraft((current) => {
      if (!current) return current;
      if (!canConnectStoryNodes(current, source.id, target.id, connectionKind)
        || current.edges.some((edge) => edge.from_node_id === source.id
          && edge.to_node_id === target.id
          && edge.kind === connectionKind)) return current;
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
  }, [connectionKind, draft, hasUnappliedJsonDrafts, t]);

  const addNode = () => {
    if (blockForUnappliedJsonDraft()) return;
    if (!draft) return;
    const title = newNodeTitle.trim();
    if (!title) {
      setError(t("galgame.graph.nodeTitleRequired"));
      return;
    }
    if (newNodeKind === "loop_entry" || newNodeKind === "loop_exit") {
      setError(t("galgame.graph.loopBoundaryCreatedByLoop"));
      return;
    }
    const id = newId("node");
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
    setError(null);
  };

  const removeNode = async () => {
    if (blockForUnappliedJsonDraft()) return;
    if (!draft || !selectedNodeId) return;
    let next: StoryGraph;
    try {
      next = removeStoryNode(draft, selectedNodeId);
    } catch (cause) {
      const messageKey = cause instanceof Error && cause.message === "galgame.graph.entryCannotDelete"
        ? "galgame.graph.entryCannotDelete"
        : cause instanceof Error && cause.message === "galgame.graph.loopBodyCannotDelete"
          ? "galgame.graph.loopBodyCannotDelete"
          : cause instanceof Error && cause.message === "galgame.graph.removeNodeWouldBreakGraph"
            ? "galgame.graph.removeNodeWouldBreakGraph"
            : "galgame.graph.removeFailed";
      setError(t(messageKey));
      return;
    }
    const removedEdgeIds = new Set(draft.edges
      .filter((edge) => edge.from_node_id === selectedNodeId || edge.to_node_id === selectedNodeId)
      .map((edge) => edge.id));
    const removesLoop = draft.loop_groups.some((group) =>
      [group.entry_node_id, group.exit_node_id].includes(selectedNodeId),
    );
    const collapsedBranchCount = branchGroupsCollapsedByRemoval(draft, removedEdgeIds, selectedNodeId).length;
    if (!await confirmStructuralRemoval(
      "galgame.graph.removeNode",
      removesLoop,
      collapsedBranchCount,
    )) return;
    setDraft(next);
    setSelectedNodeId(next.nodes.find((node) => node.id === graph?.entry_node_id)?.id ?? next.nodes[0]?.id ?? null);
    setSelectedEdgeId(null);
    setError(null);
  };

  const removeEdge = async (edgeId: string) => {
    if (blockForUnappliedJsonDraft()) return;
    if (!draft) return;
    const loop = loopForStructuralEdge(draft, edgeId);
    const collapsedBranchCount = branchGroupsCollapsedByRemoval(draft, new Set([edgeId])).length;
    let next: StoryGraph;
    try {
      next = removeStoryEdge(draft, edgeId);
    } catch (cause) {
      setError(cause instanceof Error && cause.message === "galgame.graph.removeConnectionWouldBreakGraph"
        ? t("galgame.graph.removeConnectionWouldBreakGraph")
        : extractErrorMessage(cause, t("galgame.graph.removeFailed")));
      return;
    }
    if (!await confirmStructuralRemoval("galgame.graph.removeConnection", loop != null, collapsedBranchCount)) return;
    setDraft(next);
    setSelectedEdgeId(null);
    setError(null);
  };

  const addLoop = () => {
    if (blockForUnappliedJsonDraft()) return;
    if (!draft || !selectedNodeId) return;
    if (!isValidLoopLimit(loopMaxIterations)) {
      setError(t("galgame.graph.loopLimitInvalid"));
      return;
    }
    const maxIterations = Number(loopMaxIterations);
    try {
      setDraft(addStoryLoop(
        draft,
        selectedNodeId,
        maxIterations,
        t("galgame.graph.loopEntryNode"),
        t("galgame.graph.loopExitNode"),
      ));
      setLoopMaxIterations("3");
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
      : variableKind === "int" ? { int: "0" }
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
    setError(null);
  };

  const save = async () => {
    if (blockForUnappliedJsonDraft()) return;
    if (!draft || !graph) return;
    if (hasEmptyNodeTitle) {
      const invalidNode = draft.nodes.find((node) => !node.title.trim());
      if (invalidNode) setSelectedNodeId(invalidNode.id);
      setError(t("galgame.graph.nodeTitleRequired"));
      return;
    }
    if (loopLimitInvalid) {
      setError(t("galgame.graph.loopLimitInvalid"));
      return;
    }
    if (hasInvalidStateVariable) {
      setError(t("galgame.graph.stateVariableIntInvalid"));
      return;
    }
    if (draftStructureError) {
      setError(t(draftStructureError as Parameters<typeof t>[0]));
      return;
    }
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
    if (!graph || saveSlotError || hasUnsavedChanges || hasUnappliedJsonDrafts || graphStatus === "stale") return;
    const requestProjectId = projectId;
    const requestSaveSlot = normalizedSaveSlot;
    const requestGraphId = graph.id;
    const requestGraphRevision = graph.revision;
    const requestId = beginRuntimeRequest("opening");
    if (requestId == null) return;
    setRuntimeSession(null);
    setError(null);
    try {
      const existing = await getGalgameStoryRuntimeState(requestProjectId, { save_slot: requestSaveSlot });
      if (!isCurrentRuntimeRequest(requestId, requestProjectId)) return;
      const view = existing ?? await startGalgameStoryRuntime(requestProjectId, { save_slot: requestSaveSlot });
      if (!isCurrentRuntimeRequest(requestId, requestProjectId)) return;
      if (view.state.graph_id !== requestGraphId || view.state.graph_revision !== requestGraphRevision) {
        setError(t("galgame.graph.runtimeGraphChanged"));
        return;
      }
      setRuntimeSession({
        projectId: requestProjectId,
        saveSlot: requestSaveSlot,
        graphId: requestGraphId,
        graphRevision: view.state.graph_revision,
        view,
      });
    } catch (cause) {
      if (isCurrentRuntimeRequest(requestId, requestProjectId)) {
        setError(extractErrorMessage(cause, t("galgame.graph.runtimeFailed")));
      }
    } finally {
      if (isCurrentRuntimeRequest(requestId, requestProjectId)) {
        runtimeBusyRef.current = false;
        setRuntimeBusyAction(null);
      }
    }
  };

  const chooseRuntimeEdge = async (edgeId: string) => {
    if (!runtimeView || !runtimeSession || hasUnsavedChanges || hasUnappliedJsonDrafts || graphStatus === "stale") return;
    const requestProjectId = runtimeSession.projectId;
    const requestId = beginRuntimeRequest("advancing");
    if (requestId == null) return;
    setError(null);
    try {
      const next = await advanceGalgameStoryRuntime(requestProjectId, {
        save_slot: runtimeSession.saveSlot,
        expected_step_index: runtimeView.state.step_index,
        edge_id: edgeId,
      });
      if (!isCurrentRuntimeRequest(requestId, requestProjectId)) return;
      if (next.state.graph_id !== runtimeSession.graphId || next.state.graph_revision !== runtimeSession.graphRevision) {
        setError(t("galgame.graph.runtimeGraphChanged"));
        return;
      }
      setRuntimeSession((current) => current
        && current.projectId === runtimeSession.projectId
        && current.saveSlot === runtimeSession.saveSlot
        && current.graphId === runtimeSession.graphId
        && current.graphRevision === runtimeSession.graphRevision
        ? { ...current, view: next }
        : current);
    } catch (cause) {
      if (isCurrentRuntimeRequest(requestId, requestProjectId)) {
        setError(extractErrorMessage(cause, t("galgame.graph.runtimeFailed")));
      }
    } finally {
      if (isCurrentRuntimeRequest(requestId, requestProjectId)) {
        runtimeBusyRef.current = false;
        setRuntimeBusyAction(null);
      }
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
    <section className="a4n-flat-surface min-h-0 flex-none overflow-hidden border border-book-border/40">
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
          {canEditGraph ? <BookButton size="sm" variant="primary" onClick={() => void save()} disabled={saving || hasUnappliedJsonDrafts || !hasUnsavedChanges || hasEmptyNodeTitle || loopLimitInvalid || hasInvalidStateVariable || draftStructureError != null}>
            <Save size={14} /> {saving ? t("galgame.graph.saving") : t("galgame.graph.save")}
          </BookButton> : null}
        </div>
      </header>
      {graphStatus === "stale" ? <p role="status" className="border-b border-book-border/40 px-4 py-2 text-small text-warning">{t("galgame.graph.sourceChanged")}</p> : null}
      {draftStructureError ? <p role="alert" className="border-b border-book-border/40 px-4 py-2 text-small text-error">{t(draftStructureError as Parameters<typeof t>[0])}</p> : null}
      {hasUnappliedJsonDrafts ? <p role="status" className="border-b border-book-border/40 px-4 py-2 text-small text-warning">{t("galgame.graph.applyOrDiscardJsonBeforeContinue")}</p> : null}
      {error ? <p role="alert" className="border-b border-book-border/40 px-4 py-2 text-small text-error">{error}</p> : null}

      <div className="flex flex-wrap items-center gap-2 border-b border-book-border/40 px-3 py-2">
        {canEditGraph ? <div className="flex min-w-0 basis-full items-center gap-2 sm:basis-auto sm:flex-1">
            <input
              aria-label={t("galgame.graph.newNodeTitle")}
              value={newNodeTitle}
              onChange={(event) => setNewNodeTitle(event.target.value)}
              disabled={saving}
              placeholder={t("galgame.graph.newNodeTitle")}
              className="min-w-0 flex-1 border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs text-book-text-main"
            />
            <select aria-label={t("galgame.graph.newNodeKind")} value={newNodeKind}
              disabled={saving}
              onChange={(event) => setNewNodeKind(event.target.value as StoryNode["kind"])}
              className="max-w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs">
              {USER_CREATABLE_NODE_KINDS.map((kind) => <option key={kind} value={kind}>{nodeKindLabel(kind, t)}</option>)}
            </select>
            <BookButton size="sm" variant="secondary" onClick={addNode} disabled={saving}><Plus size={14} /> {t("galgame.graph.addNode")}</BookButton>
        </div> : null}
        {canEditGraph ? <div className="flex min-w-0 items-center gap-2">
            <select aria-label={t("galgame.graph.connectionKind")} value={connectionKind}
              disabled={saving}
              onChange={(event) => setConnectionKind(event.target.value as StoryEdge["kind"])}
              className="max-w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs">
              {EDGE_KINDS.map((kind) => <option key={kind} value={kind}>{edgeKindLabel(kind, t)}</option>)}
            </select>
        </div> : null}
        {graphStatus !== "stale" ? <div className="flex min-w-0 flex-wrap items-start gap-2">
          <div className="min-w-0">
            <input aria-label={t("galgame.graph.saveSlot")} aria-required="true"
              aria-invalid={saveSlotError != null}
              aria-describedby={saveSlotError ? `galgame-save-slot-${projectId}-error` : undefined}
              disabled={runtimeBusy}
              value={saveSlot}
              onChange={(event) => setSaveSlot(event.target.value)}
              className="w-20 border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs sm:w-24" />
            {saveSlotError ? <p id={`galgame-save-slot-${projectId}-error`} role="alert" className="mt-1 max-w-56 text-caption text-error">{saveSlotError}</p> : null}
          </div>
          <BookButton size="sm" variant="secondary" onClick={() => void loadOrStartRuntime()}
            disabled={runtimeBusy || saveSlotError != null || hasUnsavedChanges || hasUnappliedJsonDrafts}>
              <Play size={14} /> {runtimeBusyAction === "opening"
                ? t("galgame.graph.runtimeLoading")
                : runtimeBusyAction === "advancing"
                  ? t("galgame.graph.runtimeAdvancing")
                  : t("galgame.graph.startOrResume")}
            </BookButton>
        </div> : null}
        {hasUnsavedChanges ? <p role="status" className="basis-full text-caption text-warning">{t("galgame.graph.saveBeforePlaytest")}</p> : null}
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
            key={`${graph.id}:${graph.revision}:${mobilePane}`}
            nodes={visibleNodes}
            edges={visibleEdges}
            nodeTypes={nodeTypes}
            defaultViewport={graphViewport ?? undefined}
            fitView={graphViewport == null}
            onMoveEnd={(_event, viewport) => setGraphViewport(viewport)}
            onNodesChange={onNodesChange}
            onConnect={canMutateGraph ? onConnect : undefined}
            onNodeClick={(_event, node) => {
              if (blockForUnappliedJsonDraft()) return;
              setSelectedNodeId(node.id);
              setSelectedEdgeId(null);
              setMobilePane("details");
            }}
            onEdgeClick={(_event, edge) => {
              if (blockForUnappliedJsonDraft()) return;
              setSelectedEdgeId(edge.id);
              setSelectedNodeId(null);
              setMobilePane("details");
            }}
            onPaneClick={() => {
              if (blockForUnappliedJsonDraft()) return;
              setSelectedNodeId(null);
              setSelectedEdgeId(null);
            }}
            nodesDraggable={canMutateGraph}
            nodesConnectable={canMutateGraph && !hasUnappliedJsonDrafts}
            isValidConnection={(connection) => Boolean(
              draft
              && !hasUnappliedJsonDrafts
              && connection.source
              && connection.target
                && !hasStoryConnection(draft, connection.source, connection.target, connectionKind)
                && canConnectStoryNodes(draft, connection.source, connection.target, connectionKind),
            )}
            onConnectEnd={canMutateGraph ? (_event, connectionState) => {
              if (connectionState.toNode && connectionState.isValid === false) {
                if (hasUnappliedJsonDrafts) return;
                const sourceId = connectionState.fromNode.id;
                const targetId = connectionState.toNode.id;
                setError(hasStoryConnection(draft, sourceId, targetId, connectionKind)
                  ? t("galgame.graph.connectionExists")
                  : wouldCreateUnmarkedStoryCycle(draft, sourceId, targetId)
                    ? t("galgame.graph.unmarkedCycle")
                    : t("galgame.graph.connectionContract"));
              }
            } : undefined}
            deleteKeyCode={null}
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
                  aria-label={t("galgame.graph.nodeKindFor", { title: selectedNode.title })}
                  disabled={saving || selectedNode.kind === "loop_entry" || selectedNode.kind === "loop_exit" || Boolean(selectedLoop)}
                  onChange={(event) => {
                    const kind = event.target.value as StoryNode["kind"];
                    if (!canChangeStoryNodeKind(draft, selectedNode.id, kind)) {
                      setError(kind === "loop_entry" || kind === "loop_exit"
                        ? t("galgame.graph.loopBoundaryCreatedByLoop")
                        : t("galgame.graph.nodeKindChangeInvalid"));
                      return;
                    }
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
                  {NODE_KINDS.filter((kind) => USER_CREATABLE_NODE_KINDS.includes(kind) || kind === selectedNode.kind)
                    .map((kind) => <option key={kind} value={kind}>{nodeKindLabel(kind, t)}</option>)}
                </select>
                <button type="button" title={t("galgame.graph.removeNode")} aria-label={t("galgame.graph.removeNode")}
                  disabled={saving || selectedNode.id === draft.entry_node_id || selectedNode.kind === "loop_entry" || selectedNode.kind === "loop_exit" || Boolean(selectedLoop)}
                  onClick={() => void removeNode()}
                  className="inline-flex h-8 w-8 items-center justify-center border border-book-border/60 text-book-text-muted hover:text-error disabled:opacity-40">
                  <Trash2 size={14} />
                </button>
              </div>
              <label className="block space-y-1 text-caption text-book-text-muted">
                <span>{t("galgame.graph.nodeTitle")}</span>
                <input value={selectedNode.title}
                  aria-required="true"
                  aria-invalid={selectedNodeTitleInvalid}
                  aria-describedby={selectedNodeTitleInvalid ? `galgame-node-title-${projectId}-error` : undefined}
                  disabled={saving}
                  onChange={(event) => {
                    updateNode(selectedNode.id, (node) => ({ ...node, title: event.target.value }));
                    if (event.target.value.trim()) setError(null);
                  }}
                  className="w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs text-book-text-main" />
                {selectedNodeTitleInvalid ? <p id={`galgame-node-title-${projectId}-error`} role="alert" className="mt-1 text-caption text-error">{t("galgame.graph.nodeTitleRequired")}</p> : null}
              </label>
              <label className="block space-y-1 text-caption text-book-text-muted">
                <span>{t("galgame.graph.nodeSummary")}</span>
                <textarea rows={3} value={selectedNode.summary}
                  disabled={saving}
                  onChange={(event) => updateNode(selectedNode.id, (node) => ({ ...node, summary: event.target.value }))}
                  className="w-full resize-y border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs text-book-text-main" />
              </label>
              <div className="space-y-2">
                <div className="flex items-center justify-between gap-2">
                  <h4 className="text-caption font-semibold text-book-text-main">{t("galgame.graph.lines")}</h4>
                  <button type="button" title={t("galgame.graph.addLine")} aria-label={t("galgame.graph.addLine")}
                    disabled={saving}
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
                {selectedNode.lines.map((line, lineIndex) => <div key={line.id} className="space-y-1 border-l-2 border-book-border/50 pl-2">
                  <div className="flex items-center justify-between gap-2">
                    <select aria-label={t("galgame.graph.lineTypeFor", { number: lineIndex + 1 })} value={line.line_type} disabled={saving}
                      onChange={(event) => updateNode(selectedNode.id, (node) => ({
                        ...node,
                        lines: node.lines.map((item) => item.id === line.id ? { ...item, line_type: event.target.value } : item),
                      }))}
                      className="min-w-0 border border-book-border/60 bg-book-bg px-2 py-1 text-caption">
                      {LINE_TYPES.map(({ value, labelKey }) => <option key={value} value={value}>{t(labelKey)}</option>)}
                    </select>
                    <button type="button" title={t("galgame.graph.removeLineNumber", { number: lineIndex + 1 })} aria-label={t("galgame.graph.removeLineNumber", { number: lineIndex + 1 })}
                      disabled={saving}
                      onClick={() => updateNode(selectedNode.id, (node) => ({ ...node, lines: node.lines.filter((item) => item.id !== line.id) }))}
                      className="inline-flex h-7 w-7 items-center justify-center text-book-text-muted hover:text-error">
                      <Trash2 size={13} />
                    </button>
                  </div>
                  <textarea rows={2} aria-label={t("galgame.graph.lineTextFor", { number: lineIndex + 1 })} value={line.text}
                    disabled={saving}
                    onChange={(event) => updateNode(selectedNode.id, (node) => ({
                      ...node,
                      lines: node.lines.map((item) => item.id === line.id ? { ...item, text: event.target.value } : item),
                    }))}
                    className="w-full resize-y border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs" />
                </div>)}
              </div>
              {selectedBranch ? <div className="space-y-2 border-t border-book-border/40 pt-3">
                <h4 className="text-caption font-semibold text-book-text-main">{t("galgame.graph.branchOptions")}</h4>
                {selectedBranch.options.map((option, optionIndex) => <div key={option.id} className="space-y-1 text-caption text-book-text-muted">
                  <span>{option.edge_id}</span>
                  <input aria-label={t("galgame.graph.optionLabelFor", { number: optionIndex + 1 })} value={option.label}
                    aria-required="true"
                    aria-invalid={!option.label.trim()}
                    aria-describedby={!option.label.trim() ? `galgame-branch-label-${projectId}-${option.id}-error` : undefined}
                    disabled={saving}
                    onChange={(event) => setDraft((current) => current ? {
                      ...current,
                      branch_groups: current.branch_groups.map((group) => group.id === selectedBranch.id
                        ? { ...group, options: group.options.map((item) => item.id === option.id ? { ...item, label: event.target.value } : item) }
                        : group),
                    } : current)}
                    className="w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs text-book-text-main" />
                  {!option.label.trim()
                    ? <span id={`galgame-branch-label-${projectId}-${option.id}-error`} role="alert" className="text-caption text-error">{t("galgame.graph.branchOptionLabelRequired")}</span>
                    : null}
                  <JsonValueField key={`${option.id}:condition`} disabled={saving} label={t("galgame.graph.optionCondition")} value={option.condition} onDirtyChange={updateJsonDraftState}
                    onChange={(value) => setDraft((current) => current ? {
                      ...current,
                      branch_groups: current.branch_groups.map((group) => group.id === selectedBranch.id
                        ? { ...group, options: group.options.map((item) => item.id === option.id ? { ...item, condition: value as typeof item.condition } : item) }
                        : group),
                    } : current)} />
                  <JsonValueField key={`${option.id}:ops`} disabled={saving} label={t("galgame.graph.stateOperations")} value={option.state_operations} onDirtyChange={updateJsonDraftState}
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
                    disabled={saving}
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
                <JsonValueField key={`${selectedNode.id}:state-operations`} disabled={saving} label={t("galgame.graph.stateOperations")} value={selectedNode.state_operations} onDirtyChange={updateJsonDraftState}
                  onChange={(value) => updateNode(selectedNode.id, (node) => ({ ...node, state_operations: value as StoryNode["state_operations"] }))} />
                <JsonValueField key={`${selectedNode.id}:media-intents`} disabled={saving} label={t("galgame.graph.mediaIntents")} value={selectedNode.media_intents} onDirtyChange={updateJsonDraftState}
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
                  <input type="number" min={1} max={10_000} step={1}
                    aria-invalid={loopLimitInvalid}
                    disabled={saving}
                    value={loopLimitText}
                    onChange={(event) => {
                      const raw = event.target.value;
                      setSelectedLoopLimitText(raw);
                      if (!isValidLoopLimit(raw)) {
                        setError(t("galgame.graph.loopLimitInvalid"));
                        return;
                      }
                      setError(null);
                      setDraft((current) => current ? {
                        ...current,
                        loop_groups: current.loop_groups.map((group) => group.id === selectedLoop.id
                          ? { ...group, max_iterations: Number(raw) }
                          : group),
                      } : current);
                    }}
                    className="w-full border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs" />
                </label>
                <JsonValueField key={`${selectedLoop.id}:condition`} disabled={saving} label={t("galgame.graph.loopCondition")} value={selectedLoop.guard} onDirtyChange={updateJsonDraftState}
                  onChange={(value) => setDraft((current) => current ? {
                    ...current,
                    loop_groups: current.loop_groups.map((group) => group.id === selectedLoop.id
                      ? { ...group, guard: value as StoryLoopGroup["guard"] }
                      : group),
                  } : current)} />
                <JsonValueField key={`${selectedLoop.id}:operations`} disabled={saving} label={t("galgame.graph.stateOperations")} value={selectedLoop.state_operations} onDirtyChange={updateJsonDraftState}
                  onChange={(value) => setDraft((current) => current ? {
                    ...current,
                    loop_groups: current.loop_groups.map((group) => group.id === selectedLoop.id
                      ? { ...group, state_operations: value as StoryLoopGroup["state_operations"] }
                      : group),
                  } : current)} />
                <BookButton size="sm" variant="danger" disabled={saving} onClick={() => void (async () => {
                  if (blockForUnappliedJsonDraft()) return;
                  if (!await confirmDialog({
                    title: t("galgame.graph.removeLoop"),
                    message: t("galgame.graph.removeLoopCascade"),
                    confirmText: t("common.delete"),
                    cancelText: t("common.cancel"),
                    dialogType: "warning",
                  })) return;
                  setDraft(removeStoryLoop(draft, selectedLoop));
                  setError(null);
                })()}><Trash2 size={14} /> {t("galgame.graph.removeLoop")}</BookButton>
              </div> :
                <div className="flex items-center gap-2">
                  <input type="number" min={1} max={10_000} step={1} value={loopMaxIterations} disabled={saving}
                    aria-invalid={newLoopLimitInvalid}
                    aria-describedby={newLoopLimitInvalid ? `galgame-new-loop-limit-${projectId}-error` : undefined}
                    onChange={(event) => {
                      const raw = event.target.value;
                      setLoopMaxIterations(raw);
                      if (!isValidLoopLimit(raw)) setError(t("galgame.graph.loopLimitInvalid"));
                      else if (error === t("galgame.graph.loopLimitInvalid")) setError(null);
                    }}
                    aria-label={t("galgame.graph.loopLimit")}
                    className="w-24 border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs" />
                  {newLoopLimitInvalid
                    ? <span id={`galgame-new-loop-limit-${projectId}-error`} role="alert" className="text-caption text-error">{t("galgame.graph.loopLimitInvalid")}</span>
                    : null}
                  <BookButton size="sm" variant="secondary" onClick={addLoop} disabled={saving || newLoopLimitInvalid}>
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
              <JsonValueField key={`${selectedEdge.id}:condition`} disabled={saving} label={t("galgame.graph.edgeCondition")} value={selectedEdge.guard} onDirtyChange={updateJsonDraftState}
                onChange={(value) => updateEdge(selectedEdge.id, (edge) => ({ ...edge, guard: value as StoryEdge["guard"] }))} />
              <JsonValueField key={`${selectedEdge.id}:operations`} disabled={saving} label={t("galgame.graph.stateOperations")} value={selectedEdge.state_operations} onDirtyChange={updateJsonDraftState}
                onChange={(value) => updateEdge(selectedEdge.id, (edge) => ({ ...edge, state_operations: value as StoryEdge["state_operations"] }))} />
              <BookButton size="sm" variant="danger" disabled={saving} onClick={() => void removeEdge(selectedEdge.id)}>
                <Trash2 size={14} /> {t("galgame.graph.removeConnection")}
              </BookButton>
            </> : null}
          </div> : <p className="text-small text-book-text-muted">{t("galgame.graph.selectNode")}</p>}

          {canEditGraph ? <details className="border-t border-book-border/40 pt-3">
            <summary className="cursor-pointer text-caption font-semibold text-book-text-main">{t("galgame.graph.stateVariables")}</summary>
            <div className="mt-2 flex flex-wrap gap-2">
              <input aria-label={t("galgame.graph.variableKey")} value={variableKey}
                onChange={(event) => setVariableKey(event.target.value)} placeholder={t("galgame.graph.variableKey")}
                disabled={saving}
                className="min-w-0 flex-1 border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs" />
              <select aria-label={t("galgame.graph.variableType")} value={variableKind}
                disabled={saving}
                onChange={(event) => setVariableKind(event.target.value as StoryStateVariable["value_kind"])}
                className="border border-book-border/60 bg-book-bg px-2 py-1.5 text-xs">
                {["bool", "int", "string", "enum"].map((kind) => <option key={kind} value={kind}>{kind}</option>)}
              </select>
              <button type="button" title={t("galgame.graph.addVariable")} aria-label={t("galgame.graph.addVariable")}
                onClick={addVariable} disabled={saving} className="inline-flex h-8 w-8 items-center justify-center border border-book-border/60">
                <Plus size={14} />
              </button>
            </div>
            <div className="mt-2 space-y-2">
              {draft.state_variables.map((variable) => <div key={variable.key} className="flex min-w-0 items-center gap-2">
                <span className="min-w-0 flex-1 truncate text-xs text-book-text-main">{variable.key}</span>
                <select aria-label={t("galgame.graph.variableTypeFor", { key: variable.key })}
                  value={variable.value_kind}
                  disabled={saving}
                  onChange={(event) => {
                    const kind = event.target.value as StoryStateVariable["value_kind"];
                    const value: StoryValue = kind === "bool" ? { bool: false }
                      : kind === "int" ? { int: "0" }
                      : kind === "enum" ? { enum: "" }
                      : { string: "" };
                    updateVariable(variable.key, { ...variable, value_kind: kind, default_value: value });
                  }}
                  className="border border-book-border/60 bg-book-bg px-2 py-1 text-caption">
                  {["bool", "int", "string", "enum"].map((kind) => <option key={kind} value={kind}>{kind}</option>)}
                </select>
                {variable.value_kind === "bool" ? <input type="checkbox"
                  aria-label={t("galgame.graph.variableDefaultFor", { key: variable.key })}
                  disabled={saving}
                  checked={"bool" in variable.default_value && variable.default_value.bool}
                  onChange={(event) => updateVariable(variable.key, { ...variable, default_value: { bool: event.target.checked } })} />
                  : <input type="text"
                    aria-label={t("galgame.graph.variableDefaultFor", { key: variable.key })}
                    disabled={saving}
                    value={variable.value_kind === "int" && "int" in variable.default_value ? variable.default_value.int : storyValueText(variable.default_value)}
                    onChange={(event) => {
                      const value: StoryValue = variable.value_kind === "int"
                        ? { int: event.target.value }
                        : variable.value_kind === "enum" ? { enum: event.target.value } : { string: event.target.value };
                      updateVariable(variable.key, { ...variable, default_value: value });
                      if (variable.value_kind === "int" && !isValidI64Decimal(event.target.value)) {
                        setError(t("galgame.graph.stateVariableIntInvalid"));
                      } else {
                        setError(null);
                      }
                    }}
                    className="w-24 min-w-0 border border-book-border/60 bg-book-bg px-2 py-1 text-xs" />}
                {variable.value_kind === "int" && "int" in variable.default_value && !isValidI64Decimal(variable.default_value.int)
                  ? <span role="alert" className="basis-full text-caption text-error">{t("galgame.graph.stateVariableIntInvalid")}</span>
                  : null}
                <button type="button" title={t("galgame.graph.removeVariableNamed", { key: variable.key })}
                  aria-label={t("galgame.graph.removeVariableNamed", { key: variable.key })}
                  disabled={saving}
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
  disabled?: boolean;
  label: string;
  onChange: (value: unknown) => void;
  onDirtyChange: (id: string, dirty: boolean) => void;
  value: unknown;
}> = ({ disabled = false, label, onChange, onDirtyChange, value }) => {
  const { t } = useI18n();
  const id = useId();
  const [text, setText] = useState(JSON.stringify(value, null, 2));
  const [error, setError] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  useDirty(`galgame-story-json:${id}`, dirty, () => {
    setText(JSON.stringify(value, null, 2));
    setDirty(false);
    setError(null);
    onDirtyChange(id, false);
  });
  useEffect(() => () => onDirtyChange(id, false), [id, onDirtyChange]);
  useEffect(() => {
    if (!dirty) setText(JSON.stringify(value, null, 2));
  }, [dirty, value]);
  return (
    <div className="mt-2 space-y-1 text-caption text-book-text-muted">
      <label className="block" htmlFor={id}>{label}</label>
      <textarea id={id} rows={4} value={text} disabled={disabled} aria-invalid={error != null} aria-describedby={error ? `${id}-error` : undefined}
        onChange={(event) => { setText(event.target.value); setDirty(true); onDirtyChange(id, true); setError(null); }}
        className={`w-full resize-y border bg-book-bg px-2 py-1.5 font-mono text-caption text-book-text-main ${error ? "border-error" : "border-book-border/60"}`} />
      {dirty ? <div className="flex flex-wrap gap-2">
        <button type="button" disabled={disabled} onClick={() => {
          let parsed: unknown;
          try {
            parsed = JSON.parse(text);
          } catch {
            setError(t("galgame.graph.jsonInvalid"));
            return;
          }
          if (containsUnrepresentableJsonNumber(parsed)) {
            setError(t("galgame.graph.jsonUnsafeInteger"));
            return;
          }
          onChange(parsed);
          setDirty(false);
          onDirtyChange(id, false);
          setError(null);
        }} className="border border-book-border/60 px-2 py-1 text-caption text-book-text-main hover:bg-book-bg-tertiary">
          {t("galgame.graph.applyJson")}
        </button>
        <button type="button" disabled={disabled} onClick={() => {
          setText(JSON.stringify(value, null, 2));
          setDirty(false);
          onDirtyChange(id, false);
          setError(null);
        }} className="border border-book-border/60 px-2 py-1 text-caption text-book-text-main hover:bg-book-bg-tertiary">
          {t("galgame.graph.discardJson")}
        </button>
      </div> : null}
      {error ? <p id={`${id}-error`} role="alert" className="text-error">{error}</p> : null}
    </div>
  );
};
