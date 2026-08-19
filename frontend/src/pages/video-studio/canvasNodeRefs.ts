import type { StudioNodeData } from "./types";

export const CANVAS_REF_TYPE_MEDIA_ASSET = "media_asset" as const;
export const CANVAS_REF_TYPE_GENERATED_IMAGE = "generated_image" as const;
export const CANVAS_REF_TYPE_GENERATED_VIDEO = "generated_video" as const;

const CANVAS_INPUT_REF_TYPES: Record<string, string> = {
  ImageInputNode: CANVAS_REF_TYPE_GENERATED_IMAGE,
  VideoInputNode: CANVAS_REF_TYPE_GENERATED_VIDEO,
  AudioInputNode: CANVAS_REF_TYPE_MEDIA_ASSET,
};

export function defaultRefTypeForNode(kind: string): string | null {
  return CANVAS_INPUT_REF_TYPES[kind] ?? null;
}

export function nodeMediaAssetRefId(data: StudioNodeData): number | null {
  return nodeRefId(data, CANVAS_REF_TYPE_MEDIA_ASSET);
}

export function nodeGeneratedImageRefId(data: StudioNodeData): number | null {
  return nodeRefId(data, CANVAS_REF_TYPE_GENERATED_IMAGE);
}

export function nodeGeneratedVideoRefId(data: StudioNodeData): number | null {
  return nodeRefId(data, CANVAS_REF_TYPE_GENERATED_VIDEO);
}

function nodeRefId(data: StudioNodeData, refType: string): number | null {
  return data.refType === refType && typeof data.refId === "number"
    ? data.refId
    : null;
}
