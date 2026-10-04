import { apiInvoke } from "./index";
export { CANVAS_MEDIA_INPUT_METADATA_KEYS, CANVAS_MEDIA_INPUT_REF_TYPES } from "./generated/host";
import type { GeneratedImage } from "./image/types";
import type { ModelRequestPreview } from "./requestPreview";
import type { GeneratedVideo } from "./video/types";
import type {
  CanvasGraphDocument,
  CanvasGraphInput,
  CanvasGraphSaveResult,
  CanvasMediaAssetSummary,
  CanvasNodeContract,
  CanvasPortContract,
  CanvasRunSummary,
  CanvasTextAssetInput,
  CanvasTextAssetSummary,
  CanvasTextAssetVersionSummary,
  CanvasTextAssetDraftInput,
  CanvasTextAssetDraftSummary,
  GetCanvasGraphInput,
  ListCanvasTextAssetVersionsInput,
  ListCanvasTextAssetsInput,
  GetCanvasNodeContractsInput,
  ImportCanvasMediaInput as GeneratedImportCanvasMediaInput,
  ImportedCanvasMedia as GeneratedImportedCanvasMedia,
  PreviewCanvasNodeRequestInput,
  RunCanvasGraphInput,
  RunCanvasNodeInput,
  SaveCanvasGraphInput,
  SaveCanvasTextAssetInput,
} from "./generated/host";

export const CANVAS_PROMPT_KIND = "canvas_prompt" as const;
export const CANVAS_TEXT_OUTPUT_KIND_IMAGE_PROMPT = "image_prompt" as const;
export const CANVAS_TEXT_OUTPUT_KIND_VIDEO_PROMPT = "video_prompt" as const;
export const CANVAS_TEXT_OUTPUT_KIND_SHOT_SCRIPT = "shot_script" as const;
export const CANVAS_TEXT_OUTPUT_KIND_STORYBOARD = "storyboard" as const;
export const CANVAS_TEXT_OUTPUT_KIND_VISUAL_DESCRIPTION =
  "visual_description" as const;
export const CANVAS_TEXT_OUTPUT_KIND_DIALOGUE = "dialogue" as const;
export const CANVAS_TEXT_OUTPUT_KIND_RHYTHM_PLAN = "rhythm_plan" as const;

export type {
  CanvasGraphDocument,
  CanvasGraphInput,
  CanvasMediaAssetSummary,
  CanvasNodeContract,
  CanvasPortContract,
  CanvasRunSummary,
  CanvasTextAssetSummary,
  CanvasTextAssetVersionSummary,
};

type CanvasMediaUploadSource = {
  file_path?: never;
  file_name: string;
  mime_type: string;
  data_base64: string;
};

type CanvasMediaPathSource = {
  file_path: string;
  file_name?: never;
  mime_type?: never;
  data_base64?: never;
};

type CanvasMediaImportProject = Pick<GeneratedImportCanvasMediaInput, "project_id">;

export type ImportCanvasMediaInput = CanvasMediaImportProject &
  (
    | ({ media_kind: "image" } & CanvasMediaUploadSource)
    | ({ media_kind: "video" | "audio" } &
        (CanvasMediaUploadSource | CanvasMediaPathSource))
  );
export type ImportedCanvasMedia =
  | (Omit<GeneratedImportedCanvasMedia, "media_kind" | "image" | "video" | "asset"> & {
      media_kind: "image";
      image: GeneratedImage;
      asset: CanvasMediaAssetSummary;
    })
  | (Omit<GeneratedImportedCanvasMedia, "media_kind" | "image" | "video" | "asset"> & {
      media_kind: "video";
      video: GeneratedVideo;
      asset: CanvasMediaAssetSummary;
    })
  | (Omit<GeneratedImportedCanvasMedia, "media_kind" | "image" | "video" | "asset"> & {
      media_kind: "audio";
      asset: CanvasMediaAssetSummary;
    });

export async function getCanvasGraph(
  projectId: number,
  graphId?: number | null,
) {
  return apiInvoke<CanvasGraphDocument>("get_canvas_graph", {
    input: {
      project_id: projectId,
      graph_id: graphId ?? null,
    } satisfies GetCanvasGraphInput,
  });
}

export async function getCanvasNodeContracts() {
  return apiInvoke<CanvasNodeContract[]>("get_canvas_node_contracts", {
    input: {} satisfies GetCanvasNodeContractsInput,
  });
}

export async function saveCanvasGraph(
  projectId: number,
  graph: CanvasGraphInput,
  expectedVersion: number,
) {
  const saved = await apiInvoke<CanvasGraphSaveResult>("save_canvas_graph", {
    input: {
      project_id: projectId,
      expected_version: expectedVersion,
      graph,
    } satisfies SaveCanvasGraphInput,
  });
  const nodeIds = new Set(saved.nodes.map(node => node.id));
  if (!saved.node_ids || typeof saved.node_ids !== "object" || Array.isArray(saved.node_ids) || graph.nodes.some(node => {
    const id = saved.node_ids[node.client_id.trim()];
    return !Number.isSafeInteger(id) || id <= 0 || !nodeIds.has(id);
  })) {
    throw new Error("保存画布返回的节点标识映射不完整");
  }
  return saved;
}

export async function saveCanvasTextAsset(
  projectId: number,
  textAsset: CanvasTextAssetInput,
) {
  return apiInvoke<CanvasTextAssetSummary>("save_canvas_text_asset", {
    input: {
      project_id: projectId,
      text_asset: textAsset,
    } satisfies SaveCanvasTextAssetInput,
  });
}

export async function listCanvasTextAssets(projectId: number): Promise<CanvasTextAssetSummary[]> {
  return apiInvoke<CanvasTextAssetSummary[]>("list_canvas_text_assets", {
    input: { project_id: projectId } satisfies ListCanvasTextAssetsInput,
  });
}

export async function listCanvasTextAssetVersions(
  projectId: number,
  assetId: number,
): Promise<CanvasTextAssetVersionSummary[]> {
  return apiInvoke<CanvasTextAssetVersionSummary[]>("list_canvas_text_asset_versions", {
    input: { project_id: projectId, asset_id: assetId } satisfies ListCanvasTextAssetVersionsInput,
  });
}

export async function generateCanvasTextAssetDraft(
  input: CanvasTextAssetDraftInput,
): Promise<CanvasTextAssetDraftSummary> {
  return apiInvoke<CanvasTextAssetDraftSummary>("generate_canvas_text_asset_draft", { input });
}

export async function previewCanvasNodeRequest(
  projectId: number,
  graphId: number,
  nodeId: number,
) {
  return apiInvoke<ModelRequestPreview>("preview_canvas_node_request", {
    input: {
      project_id: projectId,
      graph_id: graphId,
      node_id: nodeId,
    } satisfies PreviewCanvasNodeRequestInput,
  });
}

export async function runCanvasGraph(projectId: number, graphId: number) {
  return apiInvoke<CanvasRunSummary>("run_canvas_graph", {
    input: {
      project_id: projectId,
      graph_id: graphId,
      target_node_id: null,
    } satisfies RunCanvasGraphInput,
  });
}

export async function runCanvasGraphToNode(
  projectId: number,
  graphId: number,
  nodeId: number,
) {
  return apiInvoke<CanvasRunSummary>("run_canvas_graph", {
    input: {
      project_id: projectId,
      graph_id: graphId,
      target_node_id: nodeId,
    } satisfies RunCanvasGraphInput,
  });
}

export async function runCanvasNode(
  projectId: number,
  graphId: number,
  nodeId: number,
) {
  return apiInvoke<CanvasRunSummary>("run_canvas_node", {
    input: {
      project_id: projectId,
      graph_id: graphId,
      node_id: nodeId,
    } satisfies RunCanvasNodeInput,
  });
}

export async function importCanvasMedia(input: ImportCanvasMediaInput) {
  return apiInvoke<ImportedCanvasMedia>("import_canvas_media", {
    input,
  });
}
