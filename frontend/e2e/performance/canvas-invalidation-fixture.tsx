import { invalidateCanvasRuntimeResults } from "../../src/pages/video-studio/canvasGraphMapping";
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

async function measureCanvasInvalidation() {
  const cases = [];
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
  }
  return { cases };
}

Object.assign(window, { measureCanvasInvalidation, performanceReady: true });
