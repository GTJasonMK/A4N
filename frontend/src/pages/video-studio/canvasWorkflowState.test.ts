import { describe, expect, it } from "vitest";
import {
  canvasNodeIdFromStepKey,
  computeRunToNodeScope,
  computeWorkflowActiveScope,
  findSubmittedCanvasWorkflowRun,
} from "./canvasWorkflowState";
import type { WorkflowRunSummary } from "../../api/streaming";
import type { StudioEdge, StudioNode } from "./types";

const nodes: StudioNode[] = [1, 2, 3].map((id) => ({
  id: String(id),
  position: { x: 0, y: 0 },
  data: {
    label: String(id),
    nodeKind: "TextInputNode",
    refType: null,
    refId: null,
    state: "ready",
    config: {},
  },
}));

const edges: StudioEdge[] = [
  { id: "1-2", source: "1", target: "2", data: { role: "context", config: {} } },
  { id: "2-3", source: "2", target: "3", data: { role: "context", config: {} } },
];

describe("canvas workflow scope traversal", () => {
  it("collects the full upstream chain for both scope modes", () => {
    const runScope = computeRunToNodeScope(nodes[2], nodes, edges, new Map());
    expect([...runScope.nodeIds]).toEqual(["3", "2", "1"]);
    expect([...runScope.edgeIds]).toEqual(["2-3", "1-2"]);
    expect(runScope.inputStartNodes.map((node) => node.id)).toEqual(["1"]);

    const activeScope = computeWorkflowActiveScope(
      nodes,
      edges,
      new Map([["3", { status: "running", elapsedSeconds: 0 }]]),
    );
    expect([...activeScope.nodeIds]).toEqual(["3", "2", "1"]);
    expect([...activeScope.edgeIds]).toEqual(["2-3", "1-2"]);
  });
});

describe("canvas workflow discovery", () => {
  it("ignores older active runs updated after the new submission", () => {
    const submittedAt = Date.parse("2026-09-27T10:00:00Z");
    const olderActiveRun: WorkflowRunSummary = {
      active_task_id: "old-task",
      business_key: "old-run",
      can_resume: true,
      chapter_id: null,
      completed_at: null,
      created_at: "2026-09-27T09:50:00Z",
      current_step_key: "canvas_node:1",
      error_message: null,
      id: 1,
      input_fingerprint_json: { graph_version: 3 },
      mode: "full_graph",
      progress: null,
      project_id: 7,
      resume_block_reason: null,
      root_project_id: 7,
      scope_id: 11,
      scope_table: "canvas_graph",
      source_snapshot_id: null,
      status: "running",
      updated_at: "2026-09-27T10:00:02Z",
      workflow_kind: "video_canvas_graph",
    };
    const newRun = {
      ...olderActiveRun,
      active_task_id: "new-task",
      business_key: "new-run",
      created_at: "2026-09-27T10:00:01Z",
      id: 2,
      updated_at: "2026-09-27T10:00:01Z",
    };

    expect(
      findSubmittedCanvasWorkflowRun([olderActiveRun, newRun], 11, 3, submittedAt)?.id,
    ).toBe(2);
    expect(
      findSubmittedCanvasWorkflowRun([olderActiveRun], 11, 3, submittedAt),
    ).toBeNull();
    expect(
      findSubmittedCanvasWorkflowRun([newRun], 11, 4, submittedAt),
    ).toBeNull();
  });
});

describe("canvas workflow checkpoint projection", () => {
  it("keeps only canvas node step keys addressable", () => {
    expect(canvasNodeIdFromStepKey("canvas_node:42")).toBe("42");
    expect(canvasNodeIdFromStepKey("node:42")).toBe("42");
    expect(canvasNodeIdFromStepKey("video_provider")).toBeNull();
    expect(canvasNodeIdFromStepKey(null)).toBeNull();
  });
});
