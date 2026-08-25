import { configObject, configPositiveNumber, configText } from "./canvasConfig";
import {
  nodeGeneratedImageRefId,
  nodeGeneratedVideoRefId,
  nodeMediaAssetRefId,
} from "./canvasNodeRefs";
import type { StudioNode } from "./types";

export function isProbeNodeKind(kind: string): boolean {
  return kind === "TextProbeNode" || kind === "ImageProbeNode" || kind === "VideoProbeNode" || kind === "AudioProbeNode";
}

export function isOutputNodeKind(kind: string): boolean {
  return kind === "TextOutputNode" || kind === "ImageOutputNode" || kind === "VideoOutputNode" || kind === "AudioOutputNode";
}

export function nodeHasProviderRequest(node: StudioNode): boolean {
  return node.data.nodeKind === "GenerateVideoNode" || node.data.nodeKind === "GenerateImageNode" || node.data.nodeKind === "LlmTextNode";
}

export function nodeHasContentPreview(node: StudioNode): boolean {
  const config = node.data.config;
  const output = configObject(config, "output");
  const textKeys = [
    "observed_text",
    "text",
    "content",
    "prompt_text",
    "prompt",
    "instruction",
  ];
  const hasText = textKeys.some((key) => configText(config, key).trim()) || Boolean(configText(output, "text").trim());
  const hasFile = Boolean(configText(config, "file_path").trim() || configText(output, "file_path").trim());
  const hasThumbnail = Boolean(configText(config, "thumbnail_path").trim() || configText(output, "thumbnail_path").trim());
  const hasGeneratedMedia = Boolean(
    configPositiveNumber(config, "generated_image_id") ??
      nodeGeneratedImageRefId(node.data) ??
      configPositiveNumber(config, "generated_video_id") ??
      configPositiveNumber(output, "generated_video_id") ??
      nodeGeneratedVideoRefId(node.data) ??
      configPositiveNumber(config, "media_asset_id") ??
      nodeMediaAssetRefId(node.data),
  );
  return hasText || hasFile || hasThumbnail || hasGeneratedMedia;
}
