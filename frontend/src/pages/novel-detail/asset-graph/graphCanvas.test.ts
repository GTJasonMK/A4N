import { describe, expect, it } from "vitest";
import type {
  AssetDependencyGraph,
  AssetGraphNode,
} from "../../../api/project";
import { translateMessage, type MessageTranslator } from "../../../i18n";
import {
  buildLayout,
  hitTestNode,
  preferredInitialNodeId,
  primaryAssetIndexNodes,
} from "./graphCanvas";

const t: MessageTranslator = (key, params) =>
  translateMessage("zh-CN", key, params);

const node = (id: string): AssetGraphNode => ({
  asset_type: "test",
  chapter_id: null,
  count: 1,
  details: null,
  direct_downstream_count: 0,
  downstream_count: 0,
  id,
  kind: "test",
  label: id,
  layer: 0,
  mutable: false,
  mutation_policy: "none",
  needs_visualization: false,
  profile_id: null,
  record_id: null,
  status: null,
  summary: null,
  visualized: false,
});

describe("buildLayout", () => {
  it("lays out a large dependency chain without changing topological order", () => {
    const nodes = [node("project"), ...Array.from({ length: 200 }, (_, index) => node(`asset-${index}`))];
    const edges = nodes.slice(1).map((current, index) => ({
      count: 1,
      from: nodes[index].id,
      relation: "depends_on",
      to: current.id,
    }));
    const graph: AssetDependencyGraph = {
      edges,
      layers: [],
      nodes,
      project_id: 1,
      warnings: [],
    };

    const layout = buildLayout(graph, t);

    expect(layout.nodes).toHaveLength(nodes.length);
    expect(layout.nodes.map((item) => item.layer)).toEqual(
      nodes.map((_, index) => index),
    );
  });
});

it("returns the last overlapping node without copying the layout", () => {
  const first = { ...node("first"), x: 0, y: 0, width: 40, height: 40 };
  const last = { ...node("last"), x: 10, y: 10, width: 40, height: 40 };

  expect(hitTestNode([first, last], 20, 20)?.id).toBe("last");
  expect(hitTestNode([first, last], 100, 100)).toBeUndefined();
});

it("keeps initial-node priority during a single visibility scan", () => {
  const graph = {
    nodes: [
      { ...node("project"), count: 0 },
      { ...node("asset-1"), needs_visualization: true, visualized: false },
      { ...node("video_frame_image"), asset_type: "video_frame_image" },
      { ...node("chapter_shooting_script") },
    ],
  } as AssetDependencyGraph;

  expect(preferredInitialNodeId(graph)).toBe("chapter_shooting_script");
  expect(
    preferredInitialNodeId({
      nodes: [
        { ...node("project"), count: 0 },
        { ...node("asset-1"), needs_visualization: true, visualized: false },
      ],
    } as AssetDependencyGraph),
  ).toBe("asset-1");
});

it("keeps the selected video node first while deduplicating in one pass", () => {
  const nodes = [
    { ...node("video-1"), asset_type: "video_frame_image" },
    { ...node("video-2"), asset_type: "video_frame_image" },
    { ...node("video-1"), asset_type: "video_frame_image" },
    node("text-1"),
  ];

  expect(primaryAssetIndexNodes(nodes, "video-2").map((item) => item.id)).toEqual([
    "video-2",
    "video-1",
  ]);
  expect(primaryAssetIndexNodes([node("text-1"), node("text-2")], null)).toHaveLength(2);
});
