import { describe, expect, it } from "vitest";
import { buildVideoStarterGraph } from "./canvasStarterTemplate";

describe("buildVideoStarterGraph", () => {
  it("creates a connected editable text-to-video workflow", () => {
    const graph = buildVideoStarterGraph({
      graph: {
        id: 7,
        name: "视频画布",
        graph_kind: "video_studio",
        version: 1,
        viewport_json: null,
      },
      nodes: [],
      edges: [],
    }, "rewrite as a shot script");

    expect(graph.nodes.map((node) => node.node_kind)).toEqual([
      "TextInputNode",
      "LlmTextNode",
      "GenerateVideoNode",
      "VideoOutputNode",
    ]);
    expect(graph.edges).toHaveLength(3);
    expect(graph.nodes[2].config_json).toEqual({ input_mode: "text_only" });
    expect(graph.nodes[1].config_json).toEqual({
      output_kind: "shot_script",
      instruction: "rewrite as a shot script",
    });
  });
});
