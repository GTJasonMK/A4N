import type { WorkflowRunDetailSummary } from "../../src/api/streaming";
import {
  canvasWorkflowNodeRunStates,
  canvasWorkflowNodeRunStatusSignature,
  computeWorkflowActiveScope,
  edgeWithRuntimeClasses,
  nodeWithWorkflowRunClass,
  type RunToNodeScope,
} from "../../src/pages/video-studio/canvasWorkflowState";
import type { StudioEdge, StudioNode } from "../../src/pages/video-studio/types";

const nodeCount = 1000;
const tickCount = 30;
const startedAt = "2026-01-01T00:00:00.000Z";
const nodes: StudioNode[] = Array.from({ length: nodeCount }, (_, index) => ({
  id: String(index + 1),
  type: "studioNode",
  position: { x: index, y: 0 },
  data: {
    label: `Node ${index + 1}`,
    nodeKind: "LlmTextNode",
    refType: null,
    refId: null,
    state: "ready",
    config: {},
  },
}));
const edges: StudioEdge[] = [];
for (let index = 1; index <= nodeCount; index += 1) {
  edges.push({
    id: `edge-next-${index}`,
    source: String(index),
    target: String(index === nodeCount ? 1 : index + 1),
    data: { role: "context", config: {} },
  });
  edges.push({
    id: `edge-skip-${index}`,
    source: String(index),
    target: String(index > nodeCount - 2 ? index - nodeCount + 2 : index + 2),
    data: { role: "context", config: {} },
  });
}
const detail = {
  run: {
    status: "running",
    current_step_key: `canvas_node:${nodeCount}`,
    created_at: startedAt,
    updated_at: startedAt,
  },
  checkpoints: Array.from({ length: nodeCount }, (_, index) => ({
    status: index + 1 === nodeCount ? "running" : "completed",
    step_key: `canvas_node:${index + 1}`,
    created_at: startedAt,
    updated_at: startedAt,
    completed_at: index + 1 === nodeCount ? null : startedAt,
  })),
} as unknown as WorkflowRunDetailSummary;
const emptySelectedScope: RunToNodeScope = {
  targetNodeId: null,
  nodeIds: new Set(),
  edgeIds: new Set(),
  inputStartNodes: [],
  runnableStartNodes: [],
  runnableNodes: [],
};

function projectionSignature(flowNodes: StudioNode[], flowEdges: StudioEdge[]): string {
  return JSON.stringify({
    nodes: flowNodes.map((node) => [
      node.id,
      node.className,
      node.data.runStatus,
      node.data.runElapsedSeconds,
    ]),
    edges: flowEdges.map((edge) => [edge.id, edge.className, edge.animated, edge.label]),
  });
}

function runTicks(reuseScopeByStatus: boolean, baseNowMs: number) {
  let previousSignature = "";
  let workflowScope: ReturnType<typeof computeWorkflowActiveScope> | null = null;
  let projectedEdges: StudioEdge[] = [];
  let projectedNodes: StudioNode[] = [];
  let states = new Map<string, { status: string; elapsedSeconds: number | null }>();

  for (let tick = 0; tick < tickCount; tick += 1) {
    states = canvasWorkflowNodeRunStates(detail, baseNowMs + tick * 1000);
    const statusSignature = reuseScopeByStatus
      ? canvasWorkflowNodeRunStatusSignature(states)
      : `${tick}`;
    if (!workflowScope || statusSignature !== previousSignature) {
      workflowScope = computeWorkflowActiveScope(nodes, edges, states);
      projectedEdges = edges.map((edge) => ({
        ...edgeWithRuntimeClasses(edge, emptySelectedScope, workflowScope!),
        label: edge.data?.role,
      }));
      previousSignature = statusSignature;
    }
    projectedNodes = nodes.map((node) => nodeWithWorkflowRunClass(node, states));
  }

  return { states, projectedNodes, projectedEdges };
}

async function measureCanvasWorkflowTimer() {
  const warmupNow = Date.parse(startedAt) + 100_000;
  for (let warmup = 0; warmup < 2; warmup += 1) {
    runTicks(false, warmupNow);
    runTicks(true, warmupNow);
  }

  const fullSamples: number[] = [];
  const statusOnlySamples: number[] = [];
  let before;
  let after;
  const comparisonNow = Date.parse(startedAt) + 200_000;
  for (let sample = 0; sample < 15; sample += 1) {
    const measure = (reuseScopeByStatus: boolean) => {
      const started = performance.now();
      const result = runTicks(reuseScopeByStatus, comparisonNow);
      return { elapsedMs: performance.now() - started, result };
    };
    if (sample % 2 === 0) {
      const old = measure(false);
      fullSamples.push(old.elapsedMs);
      before = old.result;
      const optimized = measure(true);
      statusOnlySamples.push(optimized.elapsedMs);
      after = optimized.result;
    } else {
      const optimized = measure(true);
      statusOnlySamples.push(optimized.elapsedMs);
      after = optimized.result;
      const old = measure(false);
      fullSamples.push(old.elapsedMs);
      before = old.result;
    }
  }

  const summarize = (samples: number[]) => {
    const sorted = [...samples].sort((left, right) => left - right);
    return { samplesMs: samples, medianMs: sorted[Math.floor(sorted.length / 2)] };
  };
  const oldSignature = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(projectionSignature(before!.projectedNodes, before!.projectedEdges)),
  );
  const optimizedSignature = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(projectionSignature(after!.projectedNodes, after!.projectedEdges)),
  );
  return {
    nodeCount,
    edgeCount: edges.length,
    checkpoints: detail.checkpoints.length,
    timerTicksPerSample: tickCount,
    fullRecompute: summarize(fullSamples),
    statusOnlyReuse: summarize(statusOnlySamples),
    outputEqual: Array.from(new Uint8Array(oldSignature)).join(",") === Array.from(new Uint8Array(optimizedSignature)).join(","),
    resultSha256: Array.from(new Uint8Array(optimizedSignature), (byte) => byte.toString(16).padStart(2, "0")).join(""),
  };
}

Object.assign(window, { measureCanvasWorkflowTimer, performanceReady: true });
