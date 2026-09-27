import { describe, expect, it } from "vitest";
import type { StudioEdge, StudioNode } from "./types";

import { canvasGraphViewport, changeCanvasNodeReference, clearCanvasNodeRuntimeResult, invalidateCanvasRuntimeResults, mergeCanvasDocumentRuntimeResults, mergeStudioNodeRuntimeResults, preserveSelectedItems, toCanvasGraphInput, toFlowNode } from "./canvasGraphMapping";
import type { CanvasGraphDocument } from "../../api/canvas";

it("restores a finite saved viewport and leaves absent or invalid values to fit view", () => {
  expect(canvasGraphViewport({ x: 40, y: 60, zoom: 0.75 })).toEqual({ x: 40, y: 60, zoom: 0.75 });
  for (const value of [null, {}, { x: 0, y: 0, zoom: 0 }, { x: NaN, y: 0, zoom: 1 }]) {
    expect(canvasGraphViewport(value)).toBeUndefined();
  }
});

const textNode = (id: string, content: string): StudioNode =>
  ({
    id,
    position: { x: 0, y: 0 },
    data: {
      label: id,
      nodeKind: "TextInputNode",
      refType: null,
      refId: null,
      state: "ready",
      config: { content },
    },
  }) as StudioNode;

it("preserves selection across assigned node IDs without matching node order or labels", () => {
  const first = { ...textNode("draft", "same text"), selected: true };
  const second = textNode("4", "same text");
  const saved = [textNode("4", "same text"), textNode("17", "same text")];
  const selected = preserveSelectedItems(saved, [first, second], { draft: 17, "4": 4 });
  expect(selected.map(node => [node.id, node.selected])).toEqual([["4", false], ["17", true]]);
});

it("changing a media input reference discards the old resource's metadata and missing diagnosis", () => {
  for (const [nodeKind, refType] of [["ImageInputNode", "generated_image"], ["VideoInputNode", "generated_video"], ["AudioInputNode", "media_asset"]]) {
    const node = textNode("7", "");
    node.data = {
      ...node.data, nodeKind, refType, refId: 1, state: "missing",
      config: {
        title: "authored title", generated_image_id: 1, generated_video_id: 1, media_asset_id: 1,
        file_path: "old.mp4", thumbnail_path: "old.png", width: 160, height: 96, duration_secs: 2,
        fps: 24, file_size: 1024, status: "completed", error_message: "old error",
        output: { file_path: "old.mp4" }, canvas_runtime_result: true,
        missing_reason: "deleted", missing_ref_type: refType, missing_ref_id: 1,
      },
    };
    expect(changeCanvasNodeReference(node, refType, 1)).toBe(node);
    const changed = changeCanvasNodeReference(node, refType, 4);
    expect(changed.data).toMatchObject({ refType, refId: 4, state: "ready", config: { title: "authored title" } });
    expect(Object.keys(changed.data.config)).toEqual(["title"]);
    expect(node.data.config.file_path).toBe("old.mp4");
  }
});

describe("toCanvasGraphInput", () => {
  it("keeps authored text, including an empty edit, entirely inside the versioned graph write", () => {
    const nodes = [textNode("4", "edited"), textNode("5", "")];
    for (const node of nodes) {
      node.data.refType = "canvas_text_asset";
      node.data.refId = 12;
      node.data.config.canvas_text_asset_id = 12;
    }
    const bound = textNode("6", "");
    bound.data.refType = "canvas_text_asset";
    bound.data.refId = 13;
    delete bound.data.config.content;
    const input = toCanvasGraphInput({
      graph: { id: 3, name: "画布", graph_kind: "video_studio", version: 1, viewport_json: null },
      nodes: [], edges: [],
    }, [...nodes, bound], []);

    for (const [index, content] of ["edited", ""].entries()) {
      expect(input.nodes[index]).toMatchObject({ ref_type: null, ref_id: null, config_json: { content } });
      expect(input.nodes[index].config_json).not.toHaveProperty("canvas_text_asset_id");
    }
    expect(input.nodes[2]).toMatchObject({ ref_type: "canvas_text_asset", ref_id: 13 });
  });

  it("preserves a generated text result when saving the canvas again", () => {
    const node = {
      id: "7",
      position: { x: 0, y: 0 },
      data: {
        label: "LLM",
        nodeKind: "LlmTextNode",
        refType: "prompt_artifact",
        refId: 12,
        state: "completed",
        config: {
          canvas_runtime_result: true,
          text: "已生成的提示词",
          prompt_artifact_id: 12,
        },
      },
    } as StudioNode;
    const input = toCanvasGraphInput(
      {
        graph: {
          id: 3,
          name: "画布",
          graph_kind: "video_studio",
          version: 1,
          viewport_json: null,
        },
        nodes: [],
        edges: [],
      },
      [node],
      [],
    );

    expect(input.nodes[0]?.config_json).toMatchObject({
      canvas_runtime_result: true,
      text: "已生成的提示词",
    });
  });
});

it("merges execution results without replacing input edits or their graph revision", () => {
  const current: CanvasGraphDocument = {
    graph: { id: 3, name: "my graph", graph_kind: "video_studio", version: 1, viewport_json: null },
    nodes: [
      { id: 4, node_kind: "TextInputNode", ref_type: null, ref_id: null, state: "ready", config_json: { content: "saved input" }, position_json: { x: 0, y: 0 } },
      { id: 5, node_kind: "RenderTextNode", ref_type: null, ref_id: null, state: "ready", config_json: { template: "my template" }, position_json: { x: 300, y: 0 } },
    ],
    edges: [],
  };
  const local = current.nodes.map(toFlowNode);
  local[0].data.config = { content: "unsaved input" };
  local[1].data.config = { template: "unsaved template" };
  const fresh: CanvasGraphDocument = {
    ...current,
    graph: { ...current.graph, name: "another editor", version: 2 },
    nodes: [
      { ...current.nodes[0], ref_type: "canvas_text_asset", ref_id: 9, config_json: { content: "another editor's input" } },
      { ...current.nodes[1], state: "completed", ref_type: "prompt_artifact", ref_id: 10, config_json: { template: "saved template", canvas_runtime_result: true, text: "new result" } },
    ],
  };

  const merged = mergeStudioNodeRuntimeResults(local, fresh.nodes);
  expect(merged[0]).toBe(local[0]);
  expect(merged[1].data).toMatchObject({ state: "completed", refId: 10, config: { template: "unsaved template", text: "new result" } });
  const document = mergeCanvasDocumentRuntimeResults(current, fresh);
  expect(document.graph).toBe(current.graph);
  expect(document.nodes[0]).toBe(current.nodes[0]);
  expect(document.nodes[1].config_json).toMatchObject({ template: "my template", text: "new result" });
});

it("preserves requested video duration when clearing or receiving execution results", () => {
  const node = textNode("13", "");
  node.data.nodeKind = "GenerateVideoNode";
  node.data.refType = "generated_video";
  node.data.refId = 91;
  node.data.state = "completed";
  node.data.config = { duration_secs: 6, canvas_runtime_result: true, generated_video_id: 91 };
  const cleared = clearCanvasNodeRuntimeResult(node);
  expect(cleared.data).toMatchObject({ refType: null, refId: null, state: "ready", config: { duration_secs: 6 } });
  expect(cleared.data.config).not.toHaveProperty("canvas_runtime_result");
  expect(cleared.data.config).not.toHaveProperty("generated_video_id");
  const merged = mergeStudioNodeRuntimeResults([node], [{
    id: 13, node_kind: "GenerateVideoNode", ref_type: "generated_video", ref_id: 92,
    state: "completed", position_json: { x: 0, y: 0 },
    config_json: { duration_secs: 2, canvas_runtime_result: true, generated_video_id: 92 },
  }]);
  expect(merged[0].data.config).toMatchObject({ duration_secs: 6, generated_video_id: 92 });
});

it("clears completed runtime results downstream of a changed canvas node", () => {
  const source = textNode("1", "new source");
  const makeResult = (id: string): StudioNode => ({
    ...textNode(id, ""),
    data: {
      ...textNode(id, "").data,
      nodeKind: "LlmTextNode",
      refType: "prompt_artifact",
      refId: Number(id) + 100,
      state: "completed",
      config: { canvas_runtime_result: true, text: `result-${id}` },
    },
  });
  const firstOutput = makeResult("2");
  const secondOutput = makeResult("3");
  const unrelated = makeResult("4");
  const edges = [
    { id: "a", source: "1", target: "2" },
    { id: "b", source: "2", target: "3" },
  ] as StudioEdge[];

  const next = invalidateCanvasRuntimeResults(
    [source, firstOutput, secondOutput, unrelated],
    edges,
    [source.id],
  );

  expect(next[0]).toBe(source);
  expect(next[1]?.data).toMatchObject({ state: "ready", refType: null, refId: null });
  expect(next[1]?.data.config).not.toHaveProperty("canvas_runtime_result");
  expect(next[2]?.data).toMatchObject({ state: "ready", refType: null, refId: null });
  expect(next[3]).toBe(unrelated);
});
