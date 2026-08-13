import { saveCanvasTextAsset, type CanvasGraphDocument, type CanvasGraphInput } from "../../api/canvas";
import { edgeRoleLabel } from "./StudioCanvasNode";
import {
  configText,
  firstLine,
} from "./canvasConfig";
import { normalizeGenerateVideoInputMode } from "./nodeContracts";
import type { StudioEdge, StudioNode } from "./types";

export function toFlowNode(node: CanvasGraphDocument["nodes"][number]): StudioNode {
  return {
    id: String(node.id),
    type: "studioNode",
    position: node.position_json as { x: number; y: number },
    data: {
      label: nodeLabel(node),
      nodeKind: node.node_kind,
      refType: node.ref_type,
      refId: node.ref_id,
      state: node.state,
      config: node.config_json,
    },
  };
}

const CANVAS_RUNTIME_CONFIG_KEYS = [
  "data_type",
  "observed_data_type",
  "observed_text",
  "generated_image_id",
  "generated_video_id",
  "media_asset_id",
  "canvas_text_asset_id",
  "file_path",
  "thumbnail_path",
  "duration_secs",
  "text",
  "output",
  "source_node_ids",
] as const;

export function mergeCanvasDocumentRuntimeResults(
  current: CanvasGraphDocument,
  fresh: CanvasGraphDocument,
): CanvasGraphDocument {
  const freshNodesById = new Map(fresh.nodes.map((node) => [node.id, node]));
  return {
    ...current,
    graph: fresh.graph,
    nodes: current.nodes.map((node) => {
      const freshNode = freshNodesById.get(node.id);
      if (!freshNode) return node;
      return {
        ...node,
        ref_type: freshNode.ref_type,
        ref_id: freshNode.ref_id,
        state: freshNode.state,
        config_json: mergeRuntimeConfig(node.config_json, freshNode.config_json),
      };
    }),
  };
}

export function mergeStudioNodeRuntimeResults(
  current: StudioNode[],
  freshNodes: CanvasGraphDocument["nodes"],
): StudioNode[] {
  const freshNodesById = new Map(freshNodes.map((node) => [String(node.id), node]));
  return current.map((node) => {
    const freshNode = freshNodesById.get(node.id);
    if (!freshNode) return node;
    const config = mergeRuntimeConfig(node.data.config, freshNode.config_json);
    if (
      node.data.refType === freshNode.ref_type &&
      node.data.refId === freshNode.ref_id &&
      node.data.state === freshNode.state &&
      shallowRuntimeConfigEqual(node.data.config, config)
    ) {
      return node;
    }
    return {
      ...node,
      data: {
        ...node.data,
        refType: freshNode.ref_type,
        refId: freshNode.ref_id,
        state: freshNode.state,
        config,
      },
    };
  });
}

function mergeRuntimeConfig(
  current: Record<string, unknown>,
  fresh: Record<string, unknown>,
): Record<string, unknown> {
  let changed = false;
  const next = { ...current };
  for (const key of CANVAS_RUNTIME_CONFIG_KEYS) {
    if (!Object.prototype.hasOwnProperty.call(fresh, key)) continue;
    const value = fresh[key];
    if (JSON.stringify(next[key]) !== JSON.stringify(value)) {
      next[key] = value;
      changed = true;
    }
  }
  return changed ? next : current;
}

function shallowRuntimeConfigEqual(left: Record<string, unknown>, right: Record<string, unknown>): boolean {
  if (left === right) return true;
  for (const key of CANVAS_RUNTIME_CONFIG_KEYS) {
    if (JSON.stringify(left[key]) !== JSON.stringify(right[key])) return false;
  }
  return true;
}

export function defaultConfigForNode(kind: string): Record<string, unknown> {
  if (kind === "GenerateVideoNode") {
    return { input_mode: "text_only" };
  }
  return {};
}

export function toFlowEdge(edge: CanvasGraphDocument["edges"][number]): StudioEdge {
  return {
    id: String(edge.id),
    source: String(edge.source_node_id),
    target: String(edge.target_node_id),
    sourceHandle: edge.source_port,
    targetHandle: edge.target_port,
    type: "smoothstep",
    label: edgeRoleLabel(edge.role),
    className: "a4n-studio-edge",
    labelBgPadding: [6, 3],
    labelBgBorderRadius: 4,
    data: { role: edge.role, config: edge.config_json },
  };
}

export function preserveSelectedItems<T extends { id: string; selected?: boolean }>(next: T[], current: T[]): T[] {
  const selectedIds = new Set(current.filter((item) => item.selected).map((item) => item.id));
  if (selectedIds.size === 0) return next;
  return next.map((item) => ({ ...item, selected: selectedIds.has(item.id) }));
}

function nodeLabel(node: CanvasGraphDocument["nodes"][number]): string {
  const title = node.config_json?.title;
  return typeof title === "string" && title.trim() ? title : node.node_kind;
}

export function numericId(id: string): number | null {
  if (!/^\d+$/.test(id)) return null;
  const value = Number(id);
  return Number.isSafeInteger(value) && value > 0 ? value : null;
}

export function canvasNodePosition(index: number): { x: number; y: number } {
  const column = index % 4;
  const row = Math.floor(index / 4);
  return {
    x: 140 + column * 300,
    y: 120 + row * 190,
  };
}

export async function persistCanvasTextAssets(projectId: number, nodes: StudioNode[]): Promise<StudioNode[]> {
  let nextNodes = nodes;
  for (const node of nodes) {
    if (node.data.nodeKind !== "TextInputNode") continue;
    const content = configText(node.data.config, "content").trim();
    if (!content) continue;
    const textKind = configText(node.data.config, "text_kind") || "freeform";
    const title =
      configText(node.data.config, "title") ||
      firstLine(content) ||
      node.data.label ||
      "文本";
    const existingId = node.data.refType === "canvas_text_asset" ? node.data.refId : null;
    const asset = await saveCanvasTextAsset(projectId, {
      id: existingId,
      title,
      text_kind: textKind,
      content,
    });
    nextNodes = nextNodes.map((item) =>
      item.id === node.id
        ? {
            ...item,
            data: {
              ...item.data,
              label: title,
              refType: "canvas_text_asset",
              refId: asset.id,
              config: {
                ...item.data.config,
                title,
                text_kind: asset.text_kind,
                content: asset.content,
              },
            },
          }
        : item,
    );
  }
  return nextNodes;
}

export function toCanvasGraphInput(
  document: CanvasGraphDocument,
  nodes: StudioNode[],
  edges: StudioEdge[],
): CanvasGraphInput {
  return {
    id: document.graph.id,
    name: document.graph.name,
    graph_kind: document.graph.graph_kind,
    viewport_json: document.graph.viewport_json,
    nodes: nodes.map((node) => ({
      client_id: node.id,
      node_kind: node.data.nodeKind,
      ref_type: node.data.refType,
      ref_id: node.data.refId,
      config_json: canvasNodeConfigForSave(node),
      position_json: node.position,
      state: node.data.state,
    })),
    edges: edges.map((edge) => ({
      source_client_id: edge.source,
      target_client_id: edge.target,
      source_port: edge.sourceHandle!,
      target_port: edge.targetHandle!,
      role: edge.data!.role,
      config_json: edge.data!.config,
    })),
  };
}

function canvasNodeConfigForSave(node: StudioNode): Record<string, unknown> {
  const config = { ...node.data.config };
  if (node.data.nodeKind === "RenderTextNode") {
    delete config.prompt;
    delete config.render_mode;
    delete config.prompt_kind;
  }
  if (["LlmTextNode", "GenerateImageNode", "GenerateVideoNode"].includes(node.data.nodeKind)) {
    delete config.config_id;
    delete config.input;
    delete config.content;
    delete config.text;
  }
  if (["GenerateImageNode", "GenerateVideoNode"].includes(node.data.nodeKind)) {
    delete config.prompt;
  }
  if (node.data.nodeKind === "GenerateVideoNode") {
    delete config.first_frame_image_id;
    delete config.last_frame_image_id;
    delete config.end_frame_image_id;
    delete config.reference_image_ids;
    config.input_mode = normalizeGenerateVideoInputMode(config.input_mode);
  }
  if (["TrimVideoNode", "ExtractFrameNode"].includes(node.data.nodeKind)) {
    delete config.video_id;
  }
  if (node.data.nodeKind === "ExtractFrameNode" && config.frame_kind !== "time") {
    delete config.at_secs;
  }
  return config;
}
