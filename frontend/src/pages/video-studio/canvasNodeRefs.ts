import type { StudioNodeData } from "./types";
import { CANVAS_MEDIA_INPUT_REF_TYPES } from "../../api/canvas";
import { isPositiveSafeInteger } from "../../utils/numberRange";

export const CANVAS_REF_TYPE_MEDIA_ASSET = "media_asset" as const;
export const CANVAS_REF_TYPE_TEXT_ASSET = "canvas_text_asset" as const;
export const CANVAS_REF_TYPE_GENERATED_IMAGE = "generated_image" as const;
export const CANVAS_REF_TYPE_GENERATED_VIDEO = "generated_video" as const;

export function defaultRefTypeForNode(kind: string): string | null {
  return CANVAS_MEDIA_INPUT_REF_TYPES[kind] ?? null;
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

export function nodeRefId(data: StudioNodeData, refType: string | null = data.refType): number | null {
  return refType && data.refType === refType && typeof data.refId === "number"
    && isPositiveSafeInteger(data.refId)
    ? data.refId
    : null;
}
