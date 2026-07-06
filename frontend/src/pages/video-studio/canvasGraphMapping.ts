import { saveCanvasTextAsset, type CanvasGraphDocument, type CanvasGraphInput } from "../../api/canvas";
import { edgeRoleLabel } from "./StudioCanvasNode";
import {
  configPositiveNumber,
  configText,
  firstLine,
  studioConfigKind,
  type StudioConfigKind,
  type StudioConfigOptionsByKind,
} from "./canvasConfig";
import { edgeOrderIndex, nextAssembleClipOrder } from "./canvasAssemblyEdges";
import { inferCanvasEdgeContract, type CanvasEdgeContract } from "./canvasEdgeContract";
import { normalizeGenerateVideoInputMode, studioNodeContract, studioNodeInputPortsForData, type StudioNodeContractMap } from "./nodeContracts";
import type { StudioEdge, StudioNode } from "./types";

export function migrateLegacyCanvasGraph(
  document: CanvasGraphDocument,
  configOptions: StudioConfigOptionsByKind,
): CanvasGraphDocument {
  const configByNodeId = new Map<number, { type: StudioConfigKind; id: number; name: string }>();
  for (const node of document.nodes) {
    if (node.node_kind !== "ConfigNode") continue;
    const type = studioConfigKind(configText(node.config_json, "config_type"));
    const configId = configPositiveNumber(node.config_json, "config_id") ?? node.ref_id;
    if (!type || !configId) continue;
    const option = configOptions[type].find((item) => item.id === configId);
    configByNodeId.set(node.id, {
      type,
      id: configId,
      name: option?.name ?? configText(node.config_json, "config_name"),
    });
  }

  const configPatchByNodeId = new Map<number, Record<string, unknown>>();
  for (const edge of document.edges) {
    const config = configByNodeId.get(edge.source_node_id);
    if (!config) continue;
    const key =
      config.type === "llm"
        ? "llm_config_id"
        : config.type === "image"
          ? "image_config_id"
          : "video_config_id";
    configPatchByNodeId.set(edge.target_node_id, {
      ...(configPatchByNodeId.get(edge.target_node_id) ?? {}),
      [key]: config.id,
      [`${config.type}_config_name`]: config.name,
    });
  }

  const nodes = document.nodes
    .filter((node) => node.node_kind !== "ConfigNode")
    .map((node) => migrateLegacyCanvasNode(node, configPatchByNodeId.get(node.id)));
  const validNodeIds = new Set(nodes.map((node) => node.id));
  const edges = document.edges
    .filter((edge) => !configByNodeId.has(edge.source_node_id))
    .filter((edge) => validNodeIds.has(edge.source_node_id) && validNodeIds.has(edge.target_node_id))
    .map(migrateLegacyCanvasEdge)
    .filter((edge): edge is CanvasGraphDocument["edges"][number] => edge !== null);
  return { ...document, nodes, edges };
}

function migrateLegacyCanvasNode(
  node: CanvasGraphDocument["nodes"][number],
  configPatch: Record<string, unknown> | undefined,
): CanvasGraphDocument["nodes"][number] {
  const config = { ...node.config_json, ...(configPatch ?? {}) };
  let nodeKind = node.node_kind;
  let refType = node.ref_type;
  let refId = node.ref_id;
  if (nodeKind === "PromptNode") {
    nodeKind = "TextInputNode";
    config.content = configText(config, "prompt") || configText(config, "content");
    delete config.prompt;
  } else if (nodeKind === "TextNode") {
    nodeKind = "TextInputNode";
  } else if (nodeKind === "ImageNode" || nodeKind === "MediaAssetNode") {
    const mediaKind = mediaKindFromConfig(config);
    nodeKind = mediaKind === "video" ? "VideoInputNode" : "ImageInputNode";
    if (mediaKind === "video") {
      const generatedVideoId = configPositiveNumber(config, "generated_video_id");
      if (!refType && generatedVideoId) {
        refType = "generated_video";
        refId = generatedVideoId;
      } else {
        refType = refType ?? "media_asset";
      }
    } else {
      const generatedImageId = configPositiveNumber(config, "generated_image_id");
      if (!refType && generatedImageId) {
        refType = "generated_image";
        refId = generatedImageId;
      } else {
        refType = refType ?? "media_asset";
      }
    }
  } else if (nodeKind === "VideoNode" || nodeKind === "VideoSegmentNode") {
    nodeKind = "VideoInputNode";
    const generatedVideoId = configPositiveNumber(config, "generated_video_id");
    if ((!refType || refType === "video_segment") && generatedVideoId) {
      refType = "generated_video";
      refId = generatedVideoId;
    } else {
      refType = refType === "video_segment" ? "media_asset" : refType;
    }
  } else if (nodeKind === "AudioNode") {
    nodeKind = "AudioInputNode";
  } else if (nodeKind === "PromptArtifactNode") {
    nodeKind = "TextInputNode";
    config.content = configText(config, "prompt_text") || configText(config, "content");
  } else if (nodeKind === "RenderPromptNode") {
    nodeKind = "RenderTextNode";
  } else if (nodeKind === "LlmTransformNode") {
    nodeKind = "LlmTextNode";
  } else if (nodeKind === "AssembleNode") {
    nodeKind = "AssembleVideoNode";
  } else if (nodeKind === "ExportVideoNode") {
    nodeKind = "VideoOutputNode";
  }
  if (nodeKind === "ImageInputNode" && refType === "media_asset") {
    const generatedImageId = configPositiveNumber(config, "generated_image_id");
    if (generatedImageId) {
      refType = "generated_image";
      refId = generatedImageId;
    }
  }
  if (nodeKind === "VideoInputNode" && refType === "media_asset") {
    const generatedVideoId = configPositiveNumber(config, "generated_video_id");
    if (generatedVideoId) {
      refType = "generated_video";
      refId = generatedVideoId;
    }
  }
  return {
    ...node,
    node_kind: nodeKind,
    ref_type: refType,
    ref_id: refId,
    config_json: config,
  };
}

function migrateLegacyCanvasEdge(edge: CanvasGraphDocument["edges"][number]): CanvasGraphDocument["edges"][number] | null {
  let sourcePort = edge.source_port;
  let targetPort = edge.target_port;
  let role = edge.role;
  if (sourcePort === "prompt" || sourcePort === "context") sourcePort = "text";
  if (targetPort === "prompt") targetPort = "text";
  if (targetPort === "context") targetPort = "input";
  if (sourcePort === "timeline") sourcePort = "video";
  if (targetPort === "timeline") targetPort = "video";
  if (["character_reference", "object_reference", "scene_reference", "style_reference"].includes(targetPort)) {
    targetPort = "reference_image";
  }
  if (role === "style_reference" && targetPort === "image") targetPort = "reference_image";
  if (role === "timeline") role = "final_video";
  if (role === "config") return null;
  return {
    ...edge,
    source_port: sourcePort,
    target_port: targetPort,
    role,
  };
}

function mediaKindFromConfig(config: Record<string, unknown>): string {
  const mediaKind = configText(config, "media_kind");
  if (mediaKind) return mediaKind;
  const filePath = configText(config, "file_path");
  if (/\.(mp4|mov|webm|mkv)$/i.test(filePath)) return "video";
  if (/\.(mp3|wav|m4a|aac|flac)$/i.test(filePath)) return "audio";
  return "image";
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
    text_assets: fresh.text_assets,
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

export function normalizeLoadedCanvasEdges(
  nodes: StudioNode[],
  edges: StudioEdge[],
  contractsByKind: StudioNodeContractMap,
): { edges: StudioEdge[]; repaired: number; dropped: number } {
  const nodesById = new Map(nodes.map((node) => [node.id, node]));
  const normalized: StudioEdge[] = [];
  let repaired = 0;
  let dropped = 0;

  for (const edge of edges) {
    let contract = inferCanvasEdgeContract(edge, nodesById, contractsByKind, normalized);
    if (contract && !edgeTargetInputAvailable(edge.target, contract, nodesById, contractsByKind, normalized)) {
      contract = null;
    }
    contract ??= inferCanvasEdgeContract(
      { source: edge.source, target: edge.target },
      nodesById,
      contractsByKind,
      normalized,
    );
    if (!contract) {
      dropped += 1;
      continue;
    }

    const config =
      contract.role === "assembly_clip"
        ? {
            ...(edge.data?.config ?? {}),
            order_index: edgeOrderIndex(edge) ?? nextAssembleClipOrder(normalized, edge.target),
          }
        : edge.data?.config ?? {};
    const nextEdge: StudioEdge = {
      ...edge,
      sourceHandle: contract.sourcePort,
      targetHandle: contract.targetPort,
      label: edgeRoleLabel(contract.role),
      data: {
        ...(edge.data ?? {}),
        role: contract.role,
        config,
      },
    };
    if (
      edge.sourceHandle !== nextEdge.sourceHandle ||
      edge.targetHandle !== nextEdge.targetHandle ||
      edge.data?.role !== nextEdge.data?.role
    ) {
      repaired += 1;
    }
    normalized.push(nextEdge);
  }

  return { edges: normalized, repaired, dropped };
}

function edgeTargetInputAvailable(
  targetNodeId: string,
  contract: CanvasEdgeContract,
  nodesById: Map<string, StudioNode>,
  contractsByKind: StudioNodeContractMap,
  existingEdges: StudioEdge[],
): boolean {
  const target = nodesById.get(targetNodeId);
  const targetContract = target ? studioNodeContract(contractsByKind, target.data.nodeKind) : null;
  const targetPort =
    target && targetContract
      ? studioNodeInputPortsForData(target.data, targetContract).find((port) => port.id === contract.targetPort)
      : null;
  if (!targetPort || targetPort.multiple) return true;
  return !existingEdges.some((edge) => edge.target === targetNodeId && edge.targetHandle === contract.targetPort);
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
  contractsByKind: StudioNodeContractMap,
): CanvasGraphInput {
  const nodesById = new Map(nodes.map((node) => [node.id, node]));
  return {
    id: document.graph.id,
    name: document.graph.name,
    graph_kind: document.graph.graph_kind,
    viewport_json: document.graph.viewport_json,
    nodes: nodes.map((node) => ({
      client_id: node.id,
      id: numericId(node.id),
      node_kind: node.data.nodeKind,
      ref_type: node.data.refType,
      ref_id: node.data.refId,
      config_json: canvasNodeConfigForSave(node),
      position_json: node.position,
      state: node.data.state,
    })),
    edges: edges
      .filter((edge) => edge.source && edge.target)
      .map((edge) => {
        const contract = inferCanvasEdgeContract(edge, nodesById, contractsByKind);
        return {
          source_client_id: edge.source,
          target_client_id: edge.target,
          source_node_id: numericId(edge.source),
          target_node_id: numericId(edge.target),
          source_port: contract?.sourcePort ?? edge.sourceHandle ?? "",
          target_port: contract?.targetPort ?? edge.targetHandle ?? "",
          role: contract?.role ?? edge.data?.role ?? "",
          config_json: edge.data?.config ?? {},
        };
      }),
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
