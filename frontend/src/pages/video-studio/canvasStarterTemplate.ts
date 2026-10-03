import { getCanvasGraph, saveCanvasGraph, type CanvasGraphDocument, type CanvasGraphInput } from "../../api/canvas";

export function buildVideoStarterGraph(
  document: CanvasGraphDocument,
  rewriteInstruction: string,
): CanvasGraphInput {
  return {
    id: document.graph.id,
    name: document.graph.name,
    graph_kind: document.graph.graph_kind,
    viewport_json: document.graph.viewport_json ?? undefined,
    nodes: [
      { client_id: "source_text", node_kind: "TextInputNode", config_json: { content: "" }, position_json: { x: 0, y: 80 }, state: "ready" },
      { client_id: "rewrite_text", node_kind: "LlmTextNode", config_json: { output_kind: "shot_script", instruction: rewriteInstruction }, position_json: { x: 300, y: 80 }, state: "ready" },
      { client_id: "generate_video", node_kind: "GenerateVideoNode", config_json: { input_mode: "text_only" }, position_json: { x: 600, y: 80 }, state: "ready" },
      { client_id: "video_output", node_kind: "VideoOutputNode", config_json: {}, position_json: { x: 900, y: 80 }, state: "ready" },
    ],
    edges: [
      { source_client_id: "source_text", source_port: "text", target_client_id: "rewrite_text", target_port: "input", role: "context", config_json: {} },
      { source_client_id: "rewrite_text", source_port: "text", target_client_id: "generate_video", target_port: "text", role: "context", config_json: {} },
      { source_client_id: "generate_video", source_port: "video", target_client_id: "video_output", target_port: "video", role: "final_video", config_json: {} },
    ],
  };
}

export async function initializeVideoStarterGraph(
  projectId: number,
  rewriteInstruction: string,
): Promise<boolean> {
  const document = await getCanvasGraph(projectId);
  if (document.nodes.length > 0) return false;
  try {
    await saveCanvasGraph(
      projectId,
      buildVideoStarterGraph(document, rewriteInstruction),
      document.graph.version,
    );
    return true;
  } catch (error) {
    const current = await getCanvasGraph(projectId, document.graph.id);
    if (current.nodes.length === 0) throw error;
    return false;
  }
}
