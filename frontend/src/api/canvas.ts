import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type { GeneratedImage } from "./image/types";
import type { GeneratedVideo } from "./video/types";

const currentUserId = () => readCurrentUserId();

export interface CanvasGraphRecord {
  id: number;
  project_id: number;
  name: string;
  graph_kind: "video_studio" | "derived_video" | "template" | string;
  version: number;
  viewport_json: unknown | null;
  created_at: string;
  updated_at: string;
}

export interface CanvasNodeRecord {
  id: number;
  graph_id: number;
  node_kind: string;
  ref_type: string | null;
  ref_id: number | null;
  config_json: Record<string, unknown>;
  position_json: Record<string, unknown>;
  state: string;
  created_at: string;
  updated_at: string;
}

export interface CanvasEdgeRecord {
  id: number;
  graph_id: number;
  source_node_id: number;
  source_port: string;
  target_node_id: number;
  target_port: string;
  role: string;
  config_json: Record<string, unknown>;
}

export interface CanvasTextAssetRecord {
  id: number;
  project_id: number;
  title: string;
  text_kind: string;
  content: string;
  content_hash: string;
  created_at: string;
  updated_at: string;
}

export interface MediaAssetRecord {
  id: number;
  project_id: number;
  root_project_id: number;
  source_chapter_id: number | null;
  source_snapshot_chapter_id: number | null;
  media_kind: string;
  origin: string;
  file_path: string | null;
  thumbnail_path: string | null;
  width: number | null;
  height: number | null;
  duration_secs: number | null;
  fps: number | null;
  file_size: number | null;
  checksum: string | null;
  status: string;
  error_message: string | null;
  created_at: string;
  updated_at: string;
}

export interface CanvasGraphDocument {
  graph: CanvasGraphRecord;
  nodes: CanvasNodeRecord[];
  edges: CanvasEdgeRecord[];
  text_assets: CanvasTextAssetRecord[];
}

export interface CanvasPortContract {
  id: string;
  label: string;
  data_type: "text" | "image" | "video" | "audio" | string;
  direction: "input" | "output" | string;
  required: boolean;
  multiple: boolean;
}

export interface CanvasNodeContract {
  kind: string;
  label: string;
  category: "input" | "operation" | "probe" | "output" | string;
  runnable: boolean;
  inputs: CanvasPortContract[];
  outputs: CanvasPortContract[];
}

export interface CanvasNodeInput {
  client_id: string;
  id?: number | null;
  node_kind: string;
  ref_type?: string | null;
  ref_id?: number | null;
  config_json?: Record<string, unknown>;
  position_json: Record<string, unknown>;
  state?: string | null;
}

export interface CanvasEdgeInput {
  source_client_id: string;
  target_client_id: string;
  source_node_id?: number | null;
  target_node_id?: number | null;
  source_port: string;
  target_port: string;
  role: string;
  config_json?: Record<string, unknown>;
}

export interface CanvasGraphInput {
  id?: number | null;
  name?: string | null;
  graph_kind?: string | null;
  viewport_json?: unknown | null;
  nodes: CanvasNodeInput[];
  edges: CanvasEdgeInput[];
}

export interface CanvasTextAssetInput {
  id?: number | null;
  title: string;
  text_kind: string;
  content: string;
}

export interface CanvasNodeRequestPreview {
  project_id: number;
  graph_id: number;
  node_id: number;
  node_kind: string;
  status: string;
  note: string;
  request: unknown;
}

export interface CanvasRunSummary {
  workflow_run_id: number;
  project_id: number;
  graph_id: number;
  mode: string;
  status: string;
  error_message: string | null;
}

export interface ImportCanvasMediaInput {
  project_id: number;
  media_kind: "image" | "video" | "audio";
  file_path?: string | null;
  file_name?: string | null;
  mime_type?: string | null;
  data_base64?: string | null;
  prompt?: string | null;
}

export type ImportedCanvasMedia =
  | { media_kind: "image"; image: GeneratedImage; asset: MediaAssetRecord }
  | { media_kind: "video"; video: GeneratedVideo; asset: MediaAssetRecord }
  | { media_kind: "audio"; asset: MediaAssetRecord };

export async function getCanvasGraph(
  projectId: number,
  graphId?: number | null,
) {
  return apiInvoke<CanvasGraphDocument>("get_canvas_graph", {
    userId: currentUserId(),
    input: { project_id: projectId, graph_id: graphId ?? null },
  });
}

export async function getCanvasNodeContracts() {
  return apiInvoke<CanvasNodeContract[]>("get_canvas_node_contracts", {
    userId: currentUserId(),
    input: {},
  });
}

export async function saveCanvasGraph(
  projectId: number,
  graph: CanvasGraphInput,
) {
  return apiInvoke<CanvasGraphDocument>("save_canvas_graph", {
    userId: currentUserId(),
    input: { project_id: projectId, graph },
  });
}

export async function saveCanvasTextAsset(
  projectId: number,
  textAsset: CanvasTextAssetInput,
) {
  return apiInvoke<CanvasTextAssetRecord>("save_canvas_text_asset", {
    userId: currentUserId(),
    input: { project_id: projectId, text_asset: textAsset },
  });
}

export async function previewCanvasNodeRequest(
  projectId: number,
  graphId: number,
  nodeId: number,
  inputOverrides?: unknown,
) {
  return apiInvoke<CanvasNodeRequestPreview>("preview_canvas_node_request", {
    userId: currentUserId(),
    input: {
      project_id: projectId,
      graph_id: graphId,
      node_id: nodeId,
      input_overrides: inputOverrides ?? null,
    },
  });
}

export async function runCanvasGraph(projectId: number, graphId: number) {
  return apiInvoke<CanvasRunSummary>("run_canvas_graph", {
    userId: currentUserId(),
    input: { project_id: projectId, graph_id: graphId, target_node_id: null },
  });
}

export async function runCanvasGraphToNode(
  projectId: number,
  graphId: number,
  nodeId: number,
) {
  return apiInvoke<CanvasRunSummary>("run_canvas_graph", {
    userId: currentUserId(),
    input: { project_id: projectId, graph_id: graphId, target_node_id: nodeId },
  });
}

export async function runCanvasNode(
  projectId: number,
  graphId: number,
  nodeId: number,
) {
  return apiInvoke<CanvasRunSummary>("run_canvas_node", {
    userId: currentUserId(),
    input: { project_id: projectId, graph_id: graphId, node_id: nodeId },
  });
}

export async function importCanvasMedia(input: ImportCanvasMediaInput) {
  return apiInvoke<ImportedCanvasMedia>("import_canvas_media", {
    userId: currentUserId(),
    input,
  });
}
