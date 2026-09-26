import { expect, it } from "vitest";
import { nodeGeneratedImageRefId, nodeRefId } from "./canvasNodeRefs";
import { nodeHasContentPreview } from "./canvasNodeKinds";
import type { StudioNode } from "./types";

it("does not present an unbound or invalid resource as linked content", () => {
  const node: StudioNode = {
    id: "image-input", position: { x: 0, y: 0 },
    data: { label: "Image", nodeKind: "ImageInputNode", state: "ready", config: {}, refType: "generated_image", refId: null },
  };
  for (const refId of [null, 0, -1, 1.5, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1]) {
    node.data.refId = refId;
    expect(nodeRefId(node.data)).toBeNull();
    expect(nodeHasContentPreview(node)).toBe(false);
  }
  node.data.refId = 42;
  expect(nodeGeneratedImageRefId(node.data)).toBe(42);
  expect(nodeHasContentPreview(node)).toBe(true);
  node.data.refType = "generated_video";
  expect(nodeGeneratedImageRefId(node.data)).toBeNull();
  expect(nodeRefId({ ...node.data, refType: null })).toBeNull();
});
