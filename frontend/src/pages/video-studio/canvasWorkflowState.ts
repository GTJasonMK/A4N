import type { WorkflowRunDetailSummary } from "../../api/streaming";
import { nodeDisplayTitle } from "./StudioCanvasNode";
import { studioNodeContract, type StudioNodeContractMap } from "./nodeContracts";
import type { StudioEdge, StudioNode } from "./types";

const CANVAS_WORKFLOW_KIND = "video_canvas_graph";

type CanvasWorkflowNodeRunStatus = "running" | "completed" | "failed";

type CanvasWorkflowNodeRunState = {
  status: CanvasWorkflowNodeRunStatus;
  elapsedSeconds: number | null;
};

type WorkflowActiveScope = {
  targetNodeIds: Set<string>;
  nodeIds: Set<string>;
  edgeIds: Set<string>;
};

const EMPTY_WORKFLOW_ACTIVE_SCOPE: WorkflowActiveScope = {
  targetNodeIds: new Set(),
  nodeIds: new Set(),
  edgeIds: new Set(),
};

export type RunToNodeScope = {
  targetNodeId: string | null;
  nodeIds: Set<string>;
  edgeIds: Set<string>;
  inputStartNodes: StudioNode[];
  runnableStartNodes: StudioNode[];
  runnableNodes: StudioNode[];
};

const EMPTY_RUN_SCOPE: RunToNodeScope = {
  targetNodeId: null,
  nodeIds: new Set(),
  edgeIds: new Set(),
  inputStartNodes: [],
  runnableStartNodes: [],
  runnableNodes: [],
};

export function computeWorkflowActiveScope(
  nodes: StudioNode[],
  edges: StudioEdge[],
  states: Map<string, CanvasWorkflowNodeRunState>,
): WorkflowActiveScope {
  const runningNodeIds = new Set(
    Array.from(states.entries())
      .filter(([, state]) => state.status === "running")
      .map(([nodeId]) => nodeId),
  );
  if (runningNodeIds.size === 0) return EMPTY_WORKFLOW_ACTIVE_SCOPE;
  const existingNodeIds = new Set(nodes.map((node) => node.id));
  const targetNodeIds = new Set(Array.from(runningNodeIds).filter((nodeId) => existingNodeIds.has(nodeId)));
  if (targetNodeIds.size === 0) return EMPTY_WORKFLOW_ACTIVE_SCOPE;

  const nodeIds = new Set<string>(targetNodeIds);
  const edgeIds = new Set<string>();
  const incomingByTarget = new Map<string, StudioEdge[]>();
  for (const edge of edges) {
    if (!edge.source || !edge.target) continue;
    const list = incomingByTarget.get(edge.target) ?? [];
    list.push(edge);
    incomingByTarget.set(edge.target, list);
  }

  const queue = Array.from(targetNodeIds);
  while (queue.length > 0) {
    const targetId = queue.shift()!;
    for (const edge of incomingByTarget.get(targetId) ?? []) {
      edgeIds.add(edge.id);
      if (!nodeIds.has(edge.source)) {
        nodeIds.add(edge.source);
        queue.push(edge.source);
      }
    }
  }
  return { targetNodeIds, nodeIds, edgeIds };
}

export function computeRunToNodeScope(
  selectedNode: StudioNode | null,
  nodes: StudioNode[],
  edges: StudioEdge[],
  contractsByKind: StudioNodeContractMap,
): RunToNodeScope {
  if (!selectedNode) return EMPTY_RUN_SCOPE;
  const nodeIds = new Set<string>([selectedNode.id]);
  const edgeIds = new Set<string>();
  const incomingByTarget = new Map<string, StudioEdge[]>();
  for (const edge of edges) {
    if (!edge.source || !edge.target) continue;
    const list = incomingByTarget.get(edge.target) ?? [];
    list.push(edge);
    incomingByTarget.set(edge.target, list);
  }
  const queue = [selectedNode.id];
  while (queue.length > 0) {
    const targetId = queue.shift()!;
    for (const edge of incomingByTarget.get(targetId) ?? []) {
      edgeIds.add(edge.id);
      if (!nodeIds.has(edge.source)) {
        nodeIds.add(edge.source);
        queue.push(edge.source);
      }
    }
  }
  const scopedNodes = nodes.filter((node) => nodeIds.has(node.id));
  const scopedIncomingCount = new Map<string, number>();
  for (const edge of edges) {
    if (!edgeIds.has(edge.id)) continue;
    scopedIncomingCount.set(edge.target, (scopedIncomingCount.get(edge.target) ?? 0) + 1);
  }
  const runnableNodes = scopedNodes.filter((node) => studioNodeContract(contractsByKind, node.data.nodeKind)?.runnable ?? false);
  const runnableNodeIds = new Set(runnableNodes.map((node) => node.id));
  const runnableIncomingCount = new Map<string, number>();
  for (const edge of edges) {
    if (!edgeIds.has(edge.id)) continue;
    if (runnableNodeIds.has(edge.source) && runnableNodeIds.has(edge.target)) {
      runnableIncomingCount.set(edge.target, (runnableIncomingCount.get(edge.target) ?? 0) + 1);
    }
  }
  return {
    targetNodeId: selectedNode.id,
    nodeIds,
    edgeIds,
    inputStartNodes: scopedNodes.filter((node) => (scopedIncomingCount.get(node.id) ?? 0) === 0),
    runnableStartNodes: runnableNodes.filter((node) => (runnableIncomingCount.get(node.id) ?? 0) === 0),
    runnableNodes,
  };
}

export function nodeWithRunScopeClass(node: StudioNode, scope: RunToNodeScope): StudioNode {
  if (!scope.targetNodeId || !scope.nodeIds.has(node.id)) return node;
  return {
    ...node,
    className: [
      node.className,
      "a4n-studio-run-scope-node",
      node.id === scope.targetNodeId ? "a4n-studio-run-scope-target" : "",
    ]
      .filter(Boolean)
      .join(" "),
  };
}

export function nodeWithWorkflowRunClass(
  node: StudioNode,
  states: Map<string, CanvasWorkflowNodeRunState>,
): StudioNode {
  const runState = states.get(node.id);
  const className = replaceNodeClassTokens(
    node.className,
    WORKFLOW_NODE_RUN_CLASS_TOKENS,
    runState ? [`a4n-studio-workflow-node-${runState.status}`] : [],
  );
  const data = runState
    ? {
        ...node.data,
        runStatus: runState.status,
        runElapsedSeconds: runState.elapsedSeconds,
      }
    : node.data.runStatus
      ? {
          ...node.data,
          runStatus: undefined,
          runElapsedSeconds: null,
        }
      : node.data;
  if (className === (node.className ?? "") && data === node.data) return node;
  return { ...node, className, data };
}

const WORKFLOW_NODE_RUN_CLASS_TOKENS = [
  "a4n-studio-workflow-node-running",
  "a4n-studio-workflow-node-completed",
  "a4n-studio-workflow-node-failed",
];

export function canvasWorkflowNodeRunStates(
  detail: WorkflowRunDetailSummary | null,
  nowMs: number,
): Map<string, CanvasWorkflowNodeRunState> {
  const states = new Map<string, CanvasWorkflowNodeRunState>();
  if (!detail) return states;
  const currentNodeId = canvasNodeIdFromStepKey(detail.run.current_step_key);
  if ((detail.run.status === "running" || detail.run.status === "pending") && currentNodeId) {
    states.set(currentNodeId, {
      status: "running",
      elapsedSeconds: workflowRunCurrentStepElapsedSeconds(detail, nowMs),
    });
  }
  for (const checkpoint of detail.checkpoints) {
    const nodeId = canvasNodeIdFromStepKey(checkpoint.step_key);
    if (!nodeId) continue;
    const status = canvasWorkflowCheckpointNodeStatus(checkpoint.status);
    if (!status) continue;
    const current = states.get(nodeId);
    const next = canvasWorkflowCheckpointRunState(checkpoint, status, nowMs);
    if (!current || canvasWorkflowNodeRunStatusPriority(status) >= canvasWorkflowNodeRunStatusPriority(current.status)) {
      states.set(nodeId, next);
    }
  }
  return states;
}

export function canvasWorkflowRuntimeRefreshSignature(detail: WorkflowRunDetailSummary): string | null {
  if (detail.run.workflow_kind !== CANVAS_WORKFLOW_KIND || detail.run.scope_table !== "canvas_graph") {
    return null;
  }
  const checkpointSignature = detail.checkpoints
    .filter((checkpoint) => canvasNodeIdFromStepKey(checkpoint.step_key))
    .map((checkpoint) => `${checkpoint.id}:${checkpoint.status}:${checkpoint.updated_at}:${checkpoint.completed_at ?? ""}`)
    .join("|");
  return `${detail.run.id}:${detail.run.status}:${detail.run.updated_at}:${checkpointSignature}`;
}

function workflowRunCurrentStepElapsedSeconds(
  detail: WorkflowRunDetailSummary,
  nowMs: number,
): number | null {
  const updatedAtMs = parseWorkflowTimestampMs(detail.run.updated_at);
  const createdAtMs = parseWorkflowTimestampMs(detail.run.created_at);
  const startedAtMs = updatedAtMs ?? createdAtMs;
  return startedAtMs === null ? null : Math.max(0, Math.floor((nowMs - startedAtMs) / 1000));
}

function canvasWorkflowCheckpointRunState(
  checkpoint: WorkflowRunDetailSummary["checkpoints"][number],
  status: CanvasWorkflowNodeRunStatus,
  nowMs: number,
): CanvasWorkflowNodeRunState {
  const startedAtMs = parseWorkflowTimestampMs(checkpoint.created_at);
  const finishedAtMs =
    status === "running"
      ? null
      : parseWorkflowTimestampMs(checkpoint.completed_at) ?? parseWorkflowTimestampMs(checkpoint.updated_at);
  const elapsedSeconds =
    startedAtMs === null ? null : Math.max(0, Math.floor(((finishedAtMs ?? nowMs) - startedAtMs) / 1000));
  return {
    status,
    elapsedSeconds,
  };
}

function canvasWorkflowCheckpointNodeStatus(status: string): CanvasWorkflowNodeRunStatus | null {
  if (status === "running") return "running";
  if (status === "completed") return "completed";
  if (status === "failed") return "failed";
  return null;
}

function canvasWorkflowNodeRunStatusPriority(status: CanvasWorkflowNodeRunStatus): number {
  switch (status) {
    case "running":
      return 3;
    case "failed":
      return 2;
    case "completed":
      return 1;
  }
}

function canvasNodeIdFromStepKey(stepKey: string | null | undefined): string | null {
  const match = /^canvas_node:(\d+)$/.exec(stepKey ?? "");
  return match?.[1] ?? null;
}

export function parseWorkflowTimestampMs(value: string | null | undefined): number | null {
  if (!value) return null;
  const trimmed = value.trim();
  if (!trimmed) return null;
  const hasTimezone = /(?:Z|[+-]\d{2}:?\d{2})$/.test(trimmed);
  const normalized = trimmed.includes("T") ? trimmed : trimmed.replace(" ", "T");
  const parsed = Date.parse(hasTimezone ? normalized : `${normalized}Z`);
  return Number.isFinite(parsed) ? parsed : null;
}

function replaceNodeClassTokens(
  className: string | undefined,
  removeTokens: string[],
  addTokens: string[],
): string {
  const remove = new Set(removeTokens);
  const tokens = (className ?? "")
    .split(/\s+/)
    .filter((token) => token && !remove.has(token));
  tokens.push(...addTokens);
  return Array.from(new Set(tokens)).join(" ");
}

const CANVAS_EDGE_RUNTIME_CLASS_TOKENS = [
  "a4n-studio-run-scope-edge",
  "a4n-studio-workflow-active-edge",
];

export function edgeWithRuntimeClasses(
  edge: StudioEdge,
  selectedScope: RunToNodeScope,
  workflowScope: WorkflowActiveScope,
): StudioEdge {
  const addTokens: string[] = [];
  const selected = Boolean(selectedScope.targetNodeId && selectedScope.edgeIds.has(edge.id));
  const active = workflowScope.edgeIds.has(edge.id);
  if (selected) addTokens.push("a4n-studio-run-scope-edge");
  if (active) addTokens.push("a4n-studio-workflow-active-edge");
  const className = replaceNodeClassTokens(edge.className, CANVAS_EDGE_RUNTIME_CLASS_TOKENS, addTokens);
  const animated = selected || active ? true : edge.animated;
  if (className === (edge.className ?? "") && animated === edge.animated) return edge;
  return {
    ...edge,
    animated,
    className,
  };
}

export function runToNodeConfirmationMessage(scope: RunToNodeScope, targetNode: StudioNode): string {
  const target = nodeDisplayTitle(targetNode.data);
  const runnableStarts = scope.runnableStartNodes.map((node) => nodeDisplayTitle(node.data));
  const inputStarts = scope.inputStartNodes.map((node) => nodeDisplayTitle(node.data));
  const startLine =
    runnableStarts.length > 0
      ? `实际执行起点：${joinNodeNames(runnableStarts)}`
      : `上游起点：${joinNodeNames(inputStarts)}`;
  const inputLine = inputStarts.length > 0 ? `依赖输入：${joinNodeNames(inputStarts)}` : "依赖输入：无上游输入";
  return [
    "确认运行到此节点？",
    "",
    startLine,
    `终止节点：${target}`,
    `将执行节点数：${scope.runnableNodes.length}`,
    inputLine,
    "",
    "系统会执行该节点的全部上游依赖，并停在当前节点。",
  ].join("\n");
}

function joinNodeNames(names: string[]): string {
  if (names.length === 0) return "无";
  if (names.length <= 3) return names.join("、");
  return `${names.slice(0, 3).join("、")} 等 ${names.length} 个节点`;
}
