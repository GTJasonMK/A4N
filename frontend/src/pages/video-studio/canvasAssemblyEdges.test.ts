import { expect, it } from "vitest";
import { renumberAssembleClipEdges, reorderAssembleClipEdges } from "./canvasAssemblyEdges";
import type { StudioEdge } from "./types";

const clip = (id: string, target: string, order: number): StudioEdge => ({
  id, source: id, target, sourceHandle: "video", targetHandle: "clip",
  data: { role: "assembly_clip", config: { order_index: order, caption: id } },
});

it("closes deleted clip gaps without reordering remaining clips or another assembly", () => {
  const remaining = [clip("a3", "a", 3), clip("b1", "b", 1), clip("a2", "a", 2)];
  const result = renumberAssembleClipEdges(remaining, new Set(["a"]));
  expect(result.map(edge => edge.data?.config.order_index)).toEqual([2, 1, 1]);
  expect(result[1]).toBe(remaining[1]);
  expect(result[2].data?.config.caption).toBe("a2");
  const moved = reorderAssembleClipEdges(result, "a3", -1);
  expect(moved.map(edge => edge.data?.config.order_index)).toEqual([1, 1, 2]);
});

it("rejects malformed order metadata instead of assigning a guessed order", () => {
  const edge = clip("bad", "a", 1);
  edge.data!.config.order_index = "1";
  expect(() => renumberAssembleClipEdges([edge], new Set(["a"]))).toThrow("order_index");
});
