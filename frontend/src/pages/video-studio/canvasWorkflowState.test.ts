import { describe, expect, it } from "vitest";
import {
  computeRunToNodeScope,
  computeWorkflowActiveScope,
} from "./canvasWorkflowState";
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
