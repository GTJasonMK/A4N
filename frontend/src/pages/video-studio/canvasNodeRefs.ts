import type { StudioNodeData } from "./types";

export function nodeMediaAssetRefId(data: StudioNodeData): number | null {
  return nodeRefId(data, "media_asset");
}

export function nodeGeneratedImageRefId(data: StudioNodeData): number | null {
  return nodeRefId(data, "generated_image");
}

export function nodeGeneratedVideoRefId(data: StudioNodeData): number | null {
  return nodeRefId(data, "generated_video");
}

function nodeRefId(data: StudioNodeData, refType: string): number | null {
  return data.refType === refType && typeof data.refId === "number"
    ? data.refId
    : null;
}
