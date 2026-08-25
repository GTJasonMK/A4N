import React, { createContext, useContext } from "react";
import type { CanvasNodeContract, CanvasPortContract } from "../../api/canvas";
import type { MessageTranslator } from "../../i18n";
import { configText } from "./canvasConfig";

export type StudioNodeCategory = "input" | "operation" | "probe" | "output";

export type StudioNodeContract = CanvasNodeContract & {
  category: StudioNodeCategory;
  inputs: CanvasPortContract[];
  outputs: CanvasPortContract[];
};

export type StudioNodeContractMap = ReadonlyMap<string, StudioNodeContract>;
export type GenerateVideoInputMode = "text_only" | "first_frame" | "first_end_frame" | "reference_images";

const GENERATE_VIDEO_IMAGE_INPUTS_BY_MODE: Record<GenerateVideoInputMode, string[]> = {
  text_only: [],
  first_frame: ["first_frame"],
  first_end_frame: ["first_frame", "end_frame"],
  reference_images: ["reference_image"],
};

export const STUDIO_NODE_CATEGORY_ORDER: StudioNodeCategory[] = [
  "input",
  "operation",
  "probe",
  "output",
];

const StudioNodeContractsContext = createContext<StudioNodeContractMap>(new Map());

export const StudioNodeContractsProvider: React.FC<{
  contracts: StudioNodeContractMap;
  children: React.ReactNode;
}> = ({ contracts, children }) =>
  React.createElement(StudioNodeContractsContext.Provider, { value: contracts }, children);

function useStudioNodeContracts(): StudioNodeContractMap {
  return useContext(StudioNodeContractsContext);
}

export function useStudioNodeContract(kind: string): StudioNodeContract | null {
  return studioNodeContract(useStudioNodeContracts(), kind);
}

export function normalizeStudioNodeContracts(contracts: CanvasNodeContract[]): StudioNodeContract[] {
  const normalized: StudioNodeContract[] = [];
  const seen = new Set<string>();
  for (const contract of contracts) {
    const kind = contract.kind.trim();
    if (!kind || seen.has(kind)) continue;
    const category = normalizeCategory(contract.category);
    if (!category) continue;
    normalized.push({
      ...contract,
      kind,
      category,
      inputs: contract.inputs.filter((port) => port.direction === "input"),
      outputs: contract.outputs.filter((port) => port.direction === "output"),
    });
    seen.add(kind);
  }
  return normalized;
}

export function studioNodeContractMap(contracts: StudioNodeContract[]): StudioNodeContractMap {
  return new Map(contracts.map((contract) => [contract.kind, contract]));
}

export function studioNodeContract(
  contracts: StudioNodeContractMap,
  kind: string,
): StudioNodeContract | null {
  return contracts.get(kind) ?? null;
}

export function studioNodeOutputPortsForData(
  _data: { nodeKind: string; config: Record<string, unknown> },
  contract: StudioNodeContract,
): CanvasPortContract[] {
  return contract.outputs;
}

export function studioNodeInputPortsForData(
  data: { nodeKind: string; config: Record<string, unknown> },
  contract: StudioNodeContract,
): CanvasPortContract[] {
  if (data.nodeKind !== "GenerateVideoNode") return contract.inputs;
  const mode = parseGenerateVideoInputMode(configText(data.config, "input_mode"));
  const activeImageInputs = new Set(mode ? GENERATE_VIDEO_IMAGE_INPUTS_BY_MODE[mode] : []);
  return contract.inputs.filter((port) => port.data_type !== "image" || activeImageInputs.has(port.id));
}

export function parseGenerateVideoInputMode(value: unknown): GenerateVideoInputMode | null {
  const normalized = typeof value === "string" ? value.trim() : value;
  if (
    normalized === "first_frame" ||
    normalized === "first_end_frame" ||
    normalized === "reference_images" ||
    normalized === "text_only"
  ) {
    return normalized;
  }
  return null;
}

export function generateVideoInputModeLabel(mode: GenerateVideoInputMode, t: MessageTranslator): string {
  switch (mode) {
    case "first_frame":
      return t("video.canvas.inputMode.firstFrame");
    case "first_end_frame":
      return t("video.canvas.inputMode.firstEndFrame");
    case "reference_images":
      return t("video.canvas.inputMode.referenceImages");
    case "text_only":
      return t("video.canvas.inputMode.textOnly");
  }
}

export function studioCategoryLabel(category: string, t: MessageTranslator): string {
  switch (category) {
    case "input":
      return t("video.canvas.category.input");
    case "operation":
      return t("video.canvas.category.operation");
    case "probe":
      return t("video.canvas.category.probe");
    case "output":
      return t("video.canvas.category.output");
    default:
      return category || t("video.canvas.unknownNode");
  }
}

export function studioCategoryTitle(category: StudioNodeCategory, t: MessageTranslator): string {
  return studioCategoryLabel(category, t);
}

export function studioPortLabel(
  nodeKind: string,
  port: Pick<CanvasPortContract, "id" | "data_type">,
  t: MessageTranslator,
): string {
  if (port.id === "first_frame") return t("video.canvas.port.firstFrame");
  if (port.id === "end_frame") return t("video.canvas.port.endFrame");
  if (port.id === "reference_image") return t("video.canvas.port.referenceImage");
  if (port.id === "clip") return t("video.canvas.port.clip");
  if (nodeKind === "ExtractFrameNode" && port.id === "image") return t("video.canvas.dataType.frame");
  switch (port.data_type) {
    case "text":
      return t("video.canvas.dataType.text");
    case "image":
      return t("video.canvas.dataType.image");
    case "video":
      return t("video.canvas.dataType.video");
    case "audio":
      return t("video.canvas.dataType.audio");
    case "media":
      return t("video.canvas.dataType.media");
    case "json":
      return t("video.canvas.dataType.json");
    case "any":
      return t("video.canvas.dataType.any");
    default:
      return t("video.canvas.port.unknown", { id: port.id });
  }
}

export function dataTypesCompatible(sourceType: string, targetType: string): boolean {
  return sourceType === targetType && ["text", "image", "video", "audio"].includes(sourceType);
}

function normalizeCategory(value: string): StudioNodeCategory | null {
  return STUDIO_NODE_CATEGORY_ORDER.includes(value as StudioNodeCategory)
    ? (value as StudioNodeCategory)
    : null;
}
