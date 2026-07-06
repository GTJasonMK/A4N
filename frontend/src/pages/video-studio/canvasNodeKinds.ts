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
