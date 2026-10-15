import * as workflowState from "../../src/pages/video-studio/canvasWorkflowState";
import { edgeRoleLabel } from "../../src/pages/video-studio/StudioCanvasNode";
import type { WorkflowRunDetailSummary } from "../../src/api/streaming";
import type { StudioEdge, StudioNode } from "../../src/pages/video-studio/types";

const state = workflowState as unknown as Record<string, (...args: any[]) => any>;
const hasStableFacts = typeof state.canvasWorkflowNodeRunStateFacts === "function";
const emptyScope = { targetNodeId: null, edgeIds: new Set<string>() } as any;
const translate = (key: string) => key;
const median = (values: number[]) => [...values].sort((left, right) => left - right)[Math.floor(values.length / 2)];

function projectionSignature(nodes: StudioNode[], edges: StudioEdge[]): string {
  let hash = 2166136261;
  const add = (value: unknown) => {
    const text = String(value ?? "");
    for (let index = 0; index < text.length; index += 1) {
      hash = Math.imul(hash ^ text.charCodeAt(index), 16777619) >>> 0;
    }
  };
  for (const node of nodes) {
    add(node.id);
    add(node.className);
    add(node.data.runStatus);
    add(node.data.runElapsedSeconds);
  }
  for (const edge of edges) {
    add(edge.id);
    add(edge.className);
    add(edge.animated);
    add(edge.label);
  }
  return hash.toString(16).padStart(8, "0");
}

function projectEdges(edges: StudioEdge[], scope: any): StudioEdge[] {
  return edges.map((edge) => ({
    ...state.edgeWithRuntimeClasses(edge, emptyScope, scope),
    label: edgeRoleLabel(edge.data!.role, translate),
  }));
}

function measureCanvasWorkflow(count: number) {
  const nodes = Array.from({ length: count }, (_, index) => ({
    id: String(index),
    className: "",
    position: { x: index, y: 0 },
    data: { nodeKind: "TextInputNode", state: "ready", runStatus: undefined, runElapsedSeconds: null },
  })) as unknown as StudioNode[];
  const edges = Array.from({ length: count - 1 }, (_, index) => ({
    id: `edge-${index}`,
    source: String(index),
    target: String(index + 1),
    animated: false,
    className: "",
    data: { role: "context", config: {} },
  })) as unknown as StudioEdge[];
  const stamp = "2026-10-15T12:00:00.000Z";
  const detail = {
    run: { current_step_key: `canvas_node:${count - 1}`, status: "running", updated_at: stamp, created_at: stamp },
    checkpoints: Array.from({ length: count }, (_, index) => ({
      id: index,
      step_key: `canvas_node:${index}`,
      status: index === count - 1 ? "running" : "completed",
      created_at: stamp,
      updated_at: stamp,
      completed_at: index === count - 1 ? null : stamp,
    })),
  } as unknown as WorkflowRunDetailSummary;
  let previousDetail = detail;
  let facts = hasStableFacts ? state.canvasWorkflowNodeRunStateFacts(detail, null, new Map()) : null;
  let stableScope = facts ? state.computeWorkflowActiveScope(nodes, edges, facts) : null;
  let stableEdges = stableScope ? projectEdges(edges, stableScope) : null;

  const nextPollDetail = (): WorkflowRunDetailSummary => ({
    ...detail,
    run: { ...detail.run },
    checkpoints: detail.checkpoints.map((checkpoint) => ({ ...checkpoint })),
  });
  const tick = (nowMs: number, pollDetail: WorkflowRunDetailSummary) => {
    if (hasStableFacts) {
      const nextFacts = state.canvasWorkflowNodeRunStateFacts(pollDetail, previousDetail, facts);
      previousDetail = pollDetail;
      if (nextFacts !== facts) {
        facts = nextFacts;
        stableScope = state.computeWorkflowActiveScope(nodes, edges, facts);
        stableEdges = projectEdges(edges, stableScope);
      }
    }
    const states = hasStableFacts
      ? state.canvasWorkflowNodeRunStates(facts, nowMs)
      : state.canvasWorkflowNodeRunStates(pollDetail, nowMs);
    const scope = stableScope ?? state.computeWorkflowActiveScope(nodes, edges, states);
    const projectedNodes = nodes.map((node) => state.nodeWithWorkflowRunClass(
      state.nodeWithRunScopeClass(node, emptyScope),
      states,
    ));
    const projectedEdges = stableEdges ?? projectEdges(edges, scope);
    return { states, scope, projectedNodes, projectedEdges };
  };

  const baseNowMs = Date.parse(stamp) + 60_000;
  for (let warmup = 0; warmup < 2; warmup += 1) {
    for (let tickIndex = 0; tickIndex < 8; tickIndex += 1) tick(baseNowMs + tickIndex * 1000, nextPollDetail());
  }

  const samplesMs: number[] = [];
  let finalProjection: ReturnType<typeof tick> | null = null;
  for (let round = 0; round < 7; round += 1) {
    const pollDetails = Array.from({ length: 8 }, nextPollDetail);
    const start = performance.now();
    for (let tickIndex = 0; tickIndex < 8; tickIndex += 1) {
      finalProjection = tick(baseNowMs + (round * 8 + tickIndex) * 1000, pollDetails[tickIndex]);
    }
    samplesMs.push((performance.now() - start) / 8);
  }

  if (!finalProjection || finalProjection.states.size !== count || finalProjection.projectedNodes.length !== count
    || finalProjection.projectedEdges.length !== count - 1 || finalProjection.scope.edgeIds.size !== count - 1) {
    throw new Error("Canvas workflow projection changed its output size");
  }
  const recomputedEdges = projectEdges(edges, state.computeWorkflowActiveScope(nodes, edges, facts ?? finalProjection.states));
  if (projectionSignature(finalProjection.projectedNodes, finalProjection.projectedEdges)
    !== projectionSignature(finalProjection.projectedNodes, recomputedEdges)) {
    throw new Error("Cached workflow edges differ from a full projection");
  }

  return {
    mode: hasStableFacts ? "stable-facts-reused-across-polls" : "full-recompute-per-poll",
    nodes: count,
    edges: count - 1,
    checkpoints: count,
    rounds: samplesMs.length,
    ticksPerRound: 8,
    samplesMs: samplesMs.map((sample) => Number(sample.toFixed(3))),
    medianMs: Number(median(samplesMs).toFixed(3)),
    projectionSignature: projectionSignature(finalProjection.projectedNodes, finalProjection.projectedEdges),
    edgeObjectsCreatedPerTick: hasStableFacts ? 0 : count - 1,
  };
}

Object.assign(window, { performanceReady: true, measureCanvasWorkflow });
