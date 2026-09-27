import {
  CANVAS_MEDIA_INPUT_METADATA_KEYS,
  CANVAS_PROMPT_KIND,
  CANVAS_TEXT_OUTPUT_KIND_SHOT_SCRIPT,
  type CanvasGraphDocument,
  type CanvasGraphInput,
} from "../../api/canvas";
import {
  type StudioConfigOption,
} from "./canvasConfig";
import type { StudioEdge, StudioNode } from "./types";
import type { Viewport } from "@xyflow/react";
import { isInputNodeKind } from "./canvasNodeKinds";
import { defaultRefTypeForNode } from "./canvasNodeRefs";

export function canvasGraphViewport(value: unknown): Viewport | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  const { x, y, zoom } = value as Record<string, unknown>;
  if (typeof x !== "number" || typeof y !== "number" || typeof zoom !== "number"
    || !Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(zoom) || zoom <= 0) return undefined;
  return { x, y, zoom };
}

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

export const CANVAS_RUNTIME_CONFIG_KEYS = [
  "canvas_runtime_result",
  "data_type",
  "status",
  "generation_spec_id",
  "prompt_artifact_id",
  "prompt_kind",
  "llm_finish_reason",
  "llm_usage",
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
    nodes: current.nodes.map((node) => {
      if (isInputNodeKind(node.node_kind)) return node;
      const freshNode = freshNodesById.get(node.id);
      if (!freshNode) return node;
      return {
        ...node,
        ref_type: freshNode.ref_type,
        ref_id: freshNode.ref_id,
        state: freshNode.state,
        config_json: mergeRuntimeConfig(node.node_kind, node.config_json, freshNode.config_json),
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
    if (isInputNodeKind(node.data.nodeKind)) return node;
    const freshNode = freshNodesById.get(node.id);
    if (!freshNode) return node;
    const config = mergeRuntimeConfig(node.data.nodeKind, node.data.config, freshNode.config_json);
    if (
      node.data.refType === freshNode.ref_type &&
      node.data.refId === freshNode.ref_id &&
      node.data.state === freshNode.state &&
      node.data.config === config
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

export function clearCanvasNodeRuntimeResult(node: StudioNode): StudioNode {
  if (node.data.config.canvas_runtime_result !== true) return node;
  const config = { ...node.data.config };
  for (const key of runtimeConfigKeysForNode(node.data.nodeKind)) delete config[key];
  return {
    ...node,
    data: {
      ...node.data,
      refType: null,
      refId: null,
      state: "ready",
      config,
    },
  };
}

export function invalidateCanvasRuntimeResults(
  nodes: StudioNode[],
  edges: StudioEdge[],
  sourceNodeIds: Iterable<string>,
): StudioNode[] {
  const invalidated = new Set(sourceNodeIds);
  const queue = [...invalidated];
  for (let index = 0; index < queue.length; index += 1) {
    const source = queue[index];
    for (const edge of edges) {
      if (edge.source !== source || invalidated.has(edge.target)) continue;
      invalidated.add(edge.target);
      queue.push(edge.target);
    }
  }

  let changed = false;
  const next = nodes.map((node) => {
    if (!invalidated.has(node.id)) return node;
    const cleared = clearCanvasNodeRuntimeResult(node);
    changed ||= cleared !== node;
    return cleared;
  });
  return changed ? next : nodes;
}

export function changeCanvasNodeReference(node: StudioNode, refType: string | null, refId: number | null): StudioNode {
  if (node.data.refType === refType && node.data.refId === refId) return node;
  const config = { ...node.data.config };
  if (defaultRefTypeForNode(node.data.nodeKind)) {
    for (const key of CANVAS_MEDIA_INPUT_METADATA_KEYS) delete config[key];
    delete config.missing_reason;
    delete config.missing_ref_type;
    delete config.missing_ref_id;
  }
  return { ...node, data: { ...node.data, refType, refId, state: "ready", config } };
}

function mergeRuntimeConfig(
  nodeKind: string,
  current: Record<string, unknown>,
  fresh: Record<string, unknown>,
): Record<string, unknown> {
  let changed = false;
  const next = { ...current };
  for (const key of runtimeConfigKeysForNode(nodeKind)) {
    if (!Object.prototype.hasOwnProperty.call(fresh, key)) continue;
    const value = fresh[key];
    if (JSON.stringify(next[key]) !== JSON.stringify(value)) {
      next[key] = value;
      changed = true;
    }
  }
  return changed ? next : current;
}

function runtimeConfigKeysForNode(nodeKind: string) {
  return CANVAS_RUNTIME_CONFIG_KEYS.filter(key => nodeKind !== "GenerateVideoNode" || key !== "duration_secs");
}

export function defaultConfigForNode(
  kind: string,
  imageConfig?: Pick<StudioConfigOption, "id" | "supportedSizes">,
): Record<string, unknown> {
  if (kind === "RenderTextNode") {
    return { output_kind: CANVAS_PROMPT_KIND };
  }
  if (kind === "LlmTextNode") {
    return { output_kind: CANVAS_TEXT_OUTPUT_KIND_SHOT_SCRIPT };
  }
  if (kind === "GenerateVideoNode") {
    return { input_mode: "text_only" };
  }
  if (kind === "GenerateImageNode") {
    const size = imageConfig?.supportedSizes?.[0];
    return size
      ? {
          ...(imageConfig.id > 0 ? { image_config_id: imageConfig.id } : {}),
          width: size.width,
          height: size.height,
        }
      : {};
  }
  if (kind === "ExtractFrameNode") {
    return { frame_kind: "first" };
  }
  if (kind === "TrimVideoNode") {
    return { start_secs: 0 };
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
    className: "a4n-studio-edge",
    labelBgPadding: [6, 3],
    labelBgBorderRadius: 4,
    data: { role: edge.role, config: edge.config_json },
  };
}

export function preserveSelectedItems<T extends { id: string; selected?: boolean }>(next: T[], current: T[], nodeIds?: Readonly<Record<string, number>>): T[] {
  const selectedIds = new Set(current.filter((item) => item.selected).map((item) => nodeIds ? String(nodeIds[item.id]) : item.id));
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
      ref_type: hasInlineTextInput(node) ? null : node.data.refType,
      ref_id: hasInlineTextInput(node) ? null : node.data.refId,
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

function hasInlineTextInput(node: StudioNode): boolean {
  return node.data.nodeKind === "TextInputNode" && typeof node.data.config.content === "string";
}

function canvasNodeConfigForSave(node: StudioNode): Record<string, unknown> {
  const config = { ...node.data.config };
  const hasRuntimeResult = config.canvas_runtime_result === true;
  if (hasInlineTextInput(node)) delete config.canvas_text_asset_id;
  if (node.data.nodeKind === "RenderTextNode") {
    delete config.prompt;
    delete config.template_text;
    delete config.render_mode;
    delete config.prompt_kind;
  }
  if (["LlmTextNode", "GenerateImageNode", "GenerateVideoNode"].includes(node.data.nodeKind)) {
    delete config.config_id;
    delete config.input;
    delete config.content;
    delete config.prompt;
    if (!hasRuntimeResult) delete config.text;
  }
  if (node.data.nodeKind === "LlmTextNode") {
    delete config.text_kind;
  }
  if (node.data.nodeKind === "GenerateVideoNode") {
    delete config.first_frame_image_id;
    delete config.last_frame_image_id;
    delete config.end_frame_image_id;
    delete config.reference_image_ids;
  }
  if (["TrimVideoNode", "ExtractFrameNode"].includes(node.data.nodeKind)) {
    delete config.video_id;
  }
  if (node.data.nodeKind === "ExtractFrameNode" && config.frame_kind !== "time") {
    delete config.at_secs;
  }
  return config;
}
