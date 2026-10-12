import { invalidateCanvasRuntimeResults } from "../../src/pages/video-studio/canvasGraphMapping";
import { renumberAssembleClipEdges } from "../../src/pages/video-studio/canvasAssemblyEdges";
import type { StudioEdge, StudioNode } from "../../src/pages/video-studio/types";

function graph(nodeCount: number) {
  const nodes: StudioNode[] = Array.from({ length: nodeCount }, (_, index) => ({
    id: `node-${index}`,
    type: "studioNode",
    position: { x: index, y: 0 },
    data: {
      label: `Node ${index}`,
      nodeKind: "LlmTextNode",
      refType: "prompt_artifact",
      refId: index + 1,
      state: "completed",
      config: { canvas_runtime_result: true, text: `result-${index}` },
    },
  }));
  const edges: StudioEdge[] = [];
  for (let index = 0; index < nodeCount; index += 1) {
    edges.push({
      id: `edge-next-${index}`,
      source: `node-${index}`,
      target: `node-${(index + 1) % nodeCount}`,
      data: { role: "dependency", config: {} },
    });
    if (nodeCount > 2) {
      edges.push({
        id: `edge-skip-${index}`,
        source: `node-${index}`,
        target: `node-${(index + 2) % nodeCount}`,
        data: { role: "dependency", config: {} },
      });
    }
  }
  return { nodes, edges };
}

function assemblyGraph(nodeCount: number) {
  const edges: StudioEdge[] = [];
  const targetsToRenumber = new Set<string>();
  for (let index = 1; index < nodeCount; index += 1) {
    const target = `node-${index}`;
    for (let order = 1; order <= 3; order += 1) {
      if (order === 2 && index % 2 === 1) {
        targetsToRenumber.add(target);
        continue;
      }
      edges.push({
        id: `clip-${index}-${order}`,
        source: `media-${index}-${order}`,
        target,
        sourceHandle: "video",
        targetHandle: "clip",
        data: { role: "assembly_clip", config: { order_index: order } },
      });
    }
  }
  return { edges, targetsToRenumber };
}

async function measureCanvasInvalidation() {
  const cases = [];
  const assemblyCases = [];
  for (const nodeCount of [32, 128, 512, 1000]) {
    const input = graph(nodeCount);
    const run = () => invalidateCanvasRuntimeResults(input.nodes, input.edges, ["node-0"]);
    for (let warmup = 0; warmup < 2; warmup += 1) run();
    const samplesMs = [];
    let finalNodes: StudioNode[] = [];
    for (let sample = 0; sample < 15; sample += 1) {
      const started = performance.now();
      finalNodes = run();
      samplesMs.push(performance.now() - started);
    }
    const result = finalNodes.map((node) => ({
      id: node.id,
      state: node.data.state,
      refType: node.data.refType,
      refId: node.data.refId,
      config: node.data.config,
    }));
    const clearedNodes = result.filter(({ config }) => config.canvas_runtime_result !== true).length;
    if (clearedNodes !== nodeCount) {
      throw new Error(`invalidated ${clearedNodes} of ${nodeCount} reachable nodes`);
    }
    const digest = await crypto.subtle.digest(
      "SHA-256",
      new TextEncoder().encode(JSON.stringify(result)),
    );
    const sorted = [...samplesMs].sort((left, right) => left - right);
    cases.push({
      nodeCount,
      edgeCount: input.edges.length,
      samplesMs,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      clearedNodes,
      resultSha256: Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join(""),
    });

    const assembly = assemblyGraph(nodeCount);
    for (let warmup = 0; warmup < 2; warmup += 1) {
      renumberAssembleClipEdges(assembly.edges, assembly.targetsToRenumber);
    }
    const renumberSamplesMs = [];
    let renumberedEdges: StudioEdge[] = [];
    for (let sample = 0; sample < 15; sample += 1) {
      const started = performance.now();
      renumberedEdges = renumberAssembleClipEdges(assembly.edges, assembly.targetsToRenumber);
      renumberSamplesMs.push(performance.now() - started);
    }
    const changedClipOrders = [...assembly.targetsToRenumber].filter((target) => {
      const orders = renumberedEdges
        .filter((edge) => edge.target === target && edge.data?.role === "assembly_clip")
        .map((edge) => edge.data?.config.order_index)
        .sort((left, right) => Number(left) - Number(right));
      return orders.length === 2 && orders[0] === 1 && orders[1] === 2;
    }).length;
    const renumberDigest = await crypto.subtle.digest(
      "SHA-256",
      new TextEncoder().encode(JSON.stringify(renumberedEdges)),
    );
    const sortedRenumberSamples = [...renumberSamplesMs].sort((left, right) => left - right);
    assemblyCases.push({
      nodeCount,
      edgeCount: assembly.edges.length,
      targetCount: assembly.targetsToRenumber.size,
      samplesMs: renumberSamplesMs,
      medianMs: sortedRenumberSamples[Math.floor(sortedRenumberSamples.length / 2)],
      changedClipOrders,
      resultSha256: Array.from(new Uint8Array(renumberDigest), (byte) => byte.toString(16).padStart(2, "0")).join(""),
    });
  }
  return { cases, assemblyCases };
}

Object.assign(window, { measureCanvasInvalidation, performanceReady: true });
