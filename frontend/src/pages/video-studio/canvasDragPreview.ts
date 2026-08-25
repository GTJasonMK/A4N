import {
  canvasDataTypeLabel,
  nodeKindMeta,
  nodeStateClass,
  stateLabel,
} from "./StudioCanvasNode";
import { configText } from "./canvasConfig";
import { defaultConfigForNode } from "./canvasGraphMapping";
import { defaultRefTypeForNode } from "./canvasNodeRefs";
import {
  generateVideoInputModeLabel,
  parseGenerateVideoInputMode,
  studioCategoryLabel,
  studioNodeInputPortsForData,
  studioNodeOutputPortsForData,
  type StudioNodeContract,
} from "./nodeContracts";
import type { MessageTranslator } from "../../i18n";

export const NODE_DRAG_KIND = "application/a4n-studio-node-kind";
export const NODE_DRAG_LABEL = "application/a4n-studio-node-label";

export function setNodeTemplateDragImage(
  dataTransfer: DataTransfer,
  kind: string,
  label: string,
  zoom: number,
  contract: StudioNodeContract | null,
  t: MessageTranslator,
): void {
  const nodeWidth = 218;
  const nodeHeight = 132;
  const scale = Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
  const data = {
    label,
    nodeKind: kind,
    refType: defaultRefTypeForNode(kind),
    refId: null,
    state: "ready",
    config: defaultConfigForNode(kind),
  };
  const meta = nodeKindMeta(kind, t);
  const inputPorts = contract ? studioNodeInputPortsForData(data, contract) : [];
  const outputPorts = contract ? studioNodeOutputPortsForData(data, contract) : [];

  const wrapper = document.createElement("div");
  Object.assign(wrapper.style, {
    position: "fixed",
    left: "-10000px",
    top: "-10000px",
    width: `${nodeWidth * scale}px`,
    height: `${nodeHeight * scale}px`,
    boxSizing: "border-box",
    pointerEvents: "none",
    zIndex: "2147483647",
  });

  const preview = document.createElement("div");
  preview.className = `a4n-studio-node ${meta.tone}`;
  Object.assign(preview.style, {
    width: `${nodeWidth}px`,
    minHeight: `${nodeHeight}px`,
    transform: `scale(${scale})`,
    transformOrigin: "top left",
  });

  inputPorts.forEach((_, index) => {
    preview.appendChild(createDragPreviewPort("input", index, inputPorts.length));
  });
  outputPorts.forEach((_, index) => {
    preview.appendChild(createDragPreviewPort("output", index, outputPorts.length));
  });

  const header = document.createElement("div");
  header.className = "flex items-center gap-2 border-b border-book-border/70 px-2 py-1.5";

  const icon = document.createElement("div");
  icon.className = "a4n-studio-node-icon flex h-6 w-6 shrink-0 items-center justify-center rounded border";
  icon.textContent = nodeKindDragGlyph(kind);
  icon.style.fontSize = "14px";
  header.appendChild(icon);

  const titleBox = document.createElement("div");
  titleBox.className = "min-w-0 flex-1";
  const title = document.createElement("div");
  title.className = "truncate text-[11px] font-medium text-book-text-main";
  title.textContent = label;
  const category = document.createElement("div");
  category.className = "truncate text-[10px] text-book-text-muted";
  category.textContent = contract ? studioCategoryLabel(contract.category, t) : t("video.canvas.unknownNode");
  titleBox.append(title, category);
  header.appendChild(titleBox);

  const state = document.createElement("span");
  state.className = `a4n-studio-state-pill ${nodeStateClass("ready")}`;
  state.textContent = stateLabel("ready", t);
  header.appendChild(state);
  preview.appendChild(header);

  const body = document.createElement("div");
  body.className = "flex min-h-[58px] flex-col justify-between gap-2 px-2 py-2";
  const bodyText = document.createElement("p");
  bodyText.className = "line-clamp-2 text-[10px] leading-4 text-book-text-sub";
  bodyText.textContent = dragPreviewBodyText(kind, t);
  const footer = document.createElement("div");
  footer.className = "flex items-center justify-between gap-2 text-[10px] text-book-text-muted";
  const subtitle = document.createElement("span");
  subtitle.className = "truncate";
  subtitle.textContent = meta.label;
  const dimension = document.createElement("span");
  dimension.className = "font-mono";
  dimension.textContent = "-";
  footer.append(subtitle, dimension);
  body.append(bodyText, footer);
  preview.appendChild(body);

  wrapper.appendChild(preview);
  document.body.appendChild(wrapper);
  dataTransfer.setDragImage(
    wrapper,
    Math.round((nodeWidth * scale) / 2),
    Math.round((nodeHeight * scale) / 2),
  );
  window.setTimeout(() => wrapper.remove(), 0);
}

function createDragPreviewPort(kind: "input" | "output", index: number, total: number): HTMLSpanElement {
  const port = document.createElement("span");
  const top = total <= 1 ? 50 : 28 + (44 * index) / Math.max(total - 1, 1);
  Object.assign(port.style, {
    position: "absolute",
    top: `${top}%`,
    left: kind === "input" ? "-8px" : "auto",
    right: kind === "output" ? "-8px" : "auto",
    width: "12px",
    height: "12px",
    border: "2px solid rgb(var(--color-bg-secondary))",
    borderRadius: "999px",
    background: kind === "input" ? "rgb(var(--color-text-tertiary))" : "rgb(var(--color-primary))",
    boxShadow: "0 0 0 1px rgb(var(--color-primary) / 0.45), 0 2px 6px rgb(0 0 0 / 0.24)",
    transform: "translateY(-50%)",
  });
  return port;
}

function nodeKindDragGlyph(kind: string): string {
  if (kind.includes("Text")) return "T";
  if (kind.includes("Image")) return "I";
  if (kind.includes("Video")) return "V";
  if (kind.includes("Audio")) return "A";
  return "*";
}

function dragPreviewBodyText(kind: string, t: MessageTranslator): string {
  switch (kind) {
    case "RenderTextNode":
      return t("video.canvas.body.renderText");
    case "LlmTextNode": {
      const outputKind = configText(defaultConfigForNode(kind), "output_kind");
      return outputKind
        ? t("video.canvas.body.inputMode", { mode: canvasDataTypeLabel(outputKind, t) })
        : t("video.canvas.body.waitingConfig");
    }
    case "GenerateImageNode":
      return t("video.canvas.body.generateImage");
    case "GenerateVideoNode": {
      const mode = parseGenerateVideoInputMode(defaultConfigForNode(kind).input_mode);
      return mode
        ? t("video.canvas.body.inputMode", { mode: generateVideoInputModeLabel(mode, t) })
        : t("video.canvas.body.missingInputMode");
    }
    case "ExtractFrameNode":
      return t("video.canvas.node.extractFrame");
    case "TrimVideoNode":
      return t("video.canvas.body.trimDefault");
    case "AssembleVideoNode":
      return t("video.canvas.body.assembleVideo");
    default:
      return t("video.canvas.body.waitingConfig");
  }
}
