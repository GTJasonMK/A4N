import type { AssetDependencyGraph } from "../../src/api/project";
import type { MessageTranslator } from "../../src/i18n";
import { buildLayout } from "../../src/pages/novel-detail/asset-graph/graphCanvas";

const translate: MessageTranslator = (key, params) =>
  key === "novel.assetGraph.depth" ? `层 ${params?.depth}` : key;

function graphWithLayeredEdges(nodeCount: number): AssetDependencyGraph {
  const layerCount = 8;
  if (nodeCount % layerCount !== 0) throw new Error("nodeCount must divide into eight layers");
  const nodesPerLayer = nodeCount / layerCount;
  const nodes: AssetDependencyGraph["nodes"] = [];
  const nodeIdsByLayer: string[][] = [];

  for (let layer = 0; layer < layerCount; layer += 1) {
    const layerNodeIds: string[] = [];
    for (let index = 0; index < nodesPerLayer; index += 1) {
      const id = `chapter_${layer}_${index}`;
      layerNodeIds.push(id);
      nodes.push({
        asset_type: "chapter_content",
        chapter_id: layer * nodesPerLayer + index + 1,
        count: 1 + (index % 5),
        details: {},
        direct_downstream_count: layer < layerCount - 1 ? 3 : 0,
        downstream_count: (layerCount - layer - 1) * nodesPerLayer,
        id,
        kind: "text",
        label: `第 ${layer * nodesPerLayer + index + 1} 章正文`,
        layer,
        mutable: layer === layerCount - 1,
        mutation_policy: layer === layerCount - 1 ? "leaf_editable" : "requires_conflict_resolution",
        needs_visualization: false,
        profile_id: null,
        record_id: null,
        status: null,
        summary: `正文内容与章节事实、人物档案及派生媒体之间的引用关系 ${"相关设定与事件来源。".repeat(5)}`,
        visualized: true,
      });
    }
    nodeIdsByLayer.push(layerNodeIds);
  }

  const edges: AssetDependencyGraph["edges"] = [];
  for (let layer = 0; layer < layerCount - 1; layer += 1) {
    for (let index = 0; index < nodesPerLayer; index += 1) {
      for (let offset = 0; offset < 3; offset += 1) {
        edges.push({
          count: 1,
          from: nodeIdsByLayer[layer][index],
          relation: "derives",
          to: nodeIdsByLayer[layer + 1][(index * 31 + offset) % nodesPerLayer],
        });
      }
    }
  }

  return {
    edges,
    layers: nodeIdsByLayer.map((node_ids, index) => ({
      index,
      label: `层 ${index}`,
      node_ids,
    })),
    nodes,
    project_id: 1,
    warnings: [],
  };
}

async function measureAssetGraph(nodeCount: number) {
  const graph = graphWithLayeredEdges(nodeCount);
  const inputBytes = new TextEncoder().encode(JSON.stringify(graph)).byteLength;
  const warmupUntil = performance.now() + 250;
  while (performance.now() < warmupUntil) buildLayout(graph, translate);

  const samplesMs = [];
  let lastLayout: ReturnType<typeof buildLayout> | undefined;
  for (let sample = 0; sample < 5; sample += 1) {
    const started = performance.now();
    lastLayout = buildLayout(graph, translate);
    samplesMs.push(performance.now() - started);
  }
  const ordered = [...samplesMs].sort((a, b) => a - b);
  const digestValue = JSON.stringify({
    width: lastLayout?.width,
    height: lastLayout?.height,
    nodes: lastLayout?.nodes.map(({ id, layer, x, y, width, height }) => [id, layer, x, y, width, height]),
    layers: lastLayout?.layers.map(({ index, label, x, y, width, height, nodeCount, assetCount }) =>
      [index, label, x, y, width, height, nodeCount, assetCount]),
  });
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(digestValue));
  return {
    nodes: nodeCount,
    edges: graph.edges.length,
    inputBytes,
    samplesMs,
    medianMs: ordered[2],
    digestSha256: Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join(""),
  };
}

Object.assign(window, { measureAssetGraph, performanceReady: true });

if (new URLSearchParams(location.search).get("measure") === "1") {
  const results = [];
  for (const count of [1000, 5000, 10000]) results.push(await measureAssetGraph(count));
  const output = document.createElement("pre");
  output.id = "asset-graph-report";
  output.textContent = JSON.stringify(results);
  document.body.replaceChildren(output);
}
