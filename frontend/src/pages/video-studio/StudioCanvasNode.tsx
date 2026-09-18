import React from "react";
import { Handle, Position, useUpdateNodeInternals, type NodeProps } from "@xyflow/react";
import {
  CANVAS_TEXT_OUTPUT_KIND_DIALOGUE,
  CANVAS_TEXT_OUTPUT_KIND_IMAGE_PROMPT,
  CANVAS_TEXT_OUTPUT_KIND_RHYTHM_PLAN,
  CANVAS_TEXT_OUTPUT_KIND_SHOT_SCRIPT,
  CANVAS_TEXT_OUTPUT_KIND_STORYBOARD,
  CANVAS_TEXT_OUTPUT_KIND_VIDEO_PROMPT,
  CANVAS_TEXT_OUTPUT_KIND_VISUAL_DESCRIPTION,
} from "../../api/canvas";
import { loadVideoScrubPlaybackData } from "../../api/video";
import {
  AlertTriangle,
  Download,
  Eye,
  Film,
  Image,
  Layers3,
  Music,
  Scissors,
  Type,
  Wand2,
} from "lucide-react";
import { toMediaSrc } from "../../utils/mediaSrc";
import { formatNumber, useI18n, type Locale, type MessageTranslator } from "../../i18n";
import type { StudioNode, StudioNodeData, StudioTemplateIcon } from "./types";
import { configNumber, configObject, configPositiveNumber, configText } from "./canvasConfig";
import {
  STUDIO_NODE_CATEGORY_ORDER,
  generateVideoInputModeLabel,
  parseGenerateVideoInputMode,
  studioCategoryLabel,
  studioCategoryTitle,
  studioPortLabel,
  studioNodeInputPortsForData,
  studioNodeOutputPortsForData,
  type StudioNodeContract,
  useStudioNodeContract,
} from "./nodeContracts";
import {
  defaultRefTypeForNode,
  nodeGeneratedVideoRefId,
} from "./canvasNodeRefs";
import { videoPlaybackDataToObjectUrl } from "../../utils/videoPlayback";

type StudioNodeTemplate = {
  kind: string;
  label: string;
  icon: StudioTemplateIcon;
};

export function buildNodeTemplateGroups(contracts: StudioNodeContract[], t: MessageTranslator): Array<{ title: string; items: StudioNodeTemplate[] }> {
  return STUDIO_NODE_CATEGORY_ORDER.map((category) => ({
    title: studioCategoryTitle(category, t),
    items: contracts
      .filter((contract) => contract.category === category)
      .map((contract) => ({
        kind: contract.kind,
        label: nodeKindMeta(contract.kind, t).label,
        icon: nodeKindMeta(contract.kind, t).icon,
      })),
  })).filter((group) => group.items.length > 0);
}

const StudioCanvasNode: React.FC<NodeProps<StudioNode>> = ({ id, data, selected, isConnectable }) => {
  const { locale, t } = useI18n();
  const updateNodeInternals = useUpdateNodeInternals();
  const meta = nodeKindMeta(data.nodeKind, t);
  const contract = useStudioNodeContract(data.nodeKind);
  const Icon = meta.icon;
  const previewSrc = nodePreviewSrc(data);
  const videoPreview = previewSrc ? null : nodeVideoPreview(data, t);
  const issues = nodeValidationIssues(data, t);
  const inputPorts = contract
    ? studioNodeInputPortsForData(data, contract).map((port) => ({ id: port.id, label: studioPortLabel(data.nodeKind, port, t) }))
    : [];
  const outputPorts = contract
    ? studioNodeOutputPortsForData(data, contract).map((port) => ({ id: port.id, label: studioPortLabel(data.nodeKind, port, t) }))
    : [];
  const handleSignature = `${inputPorts.map((port) => port.id).join(",")}|${outputPorts.map((port) => port.id).join(",")}`;

  React.useEffect(() => {
    // React Flow caches handle bounds; refresh them when input_mode changes the handle set.
    updateNodeInternals(id);
  }, [handleSignature, id, updateNodeInternals]);

  return (
    <div
      className={[
        "a4n-studio-node group relative w-[200px] overflow-visible rounded-xl border bg-book-bg-paper shadow-[var(--shadow-card)] transition",
        meta.tone,
        selected ? "a4n-studio-node-selected" : "",
        data.state === "failed" || data.state === "missing" ? "a4n-studio-node-problem" : "",
        data.runStatus ? `a4n-studio-node-run-${data.runStatus}` : "",
      ].join(" ")}
    >
      {inputPorts.map((port, index) => (
        <Handle
          key={`in-${port.id}`}
          type="target"
          id={port.id}
          position={Position.Left}
          isConnectable={isConnectable}
          style={{ top: portOffset(index, inputPorts.length) }}
          className="a4n-studio-port a4n-studio-port-input"
          title={port.label}
        />
      ))}
      {outputPorts.map((port, index) => (
        <Handle
          key={`out-${port.id}`}
          type="source"
          id={port.id}
          position={Position.Right}
          isConnectable={isConnectable}
          style={{ top: portOffset(index, outputPorts.length) }}
          className="a4n-studio-port a4n-studio-port-output"
          title={port.label}
        />
      ))}

      <div className="flex items-center gap-2 border-b border-book-border/70 px-2 py-1.5">
        <div className="a4n-studio-node-icon flex h-6 w-6 shrink-0 items-center justify-center rounded border">
          <Icon size={14} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate text-xs font-semibold text-book-text-main" title={nodeDisplayTitle(data, t)}>
            {nodeDisplayTitle(data, t)}
          </div>
          <div className="truncate text-caption text-book-text-muted">
            {contract ? studioCategoryLabel(contract.category, t) : t("video.canvas.unknownNode")}
          </div>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-1">
          <span className={`a4n-studio-state-pill ${nodeStateClass(data.state)}`}>
            {stateLabel(data.state, t)}
          </span>
          {data.runStatus ? (
            <span className={`a4n-studio-run-pill a4n-studio-run-pill-${data.runStatus}`}>
              {nodeRunStatusLabel(data.runStatus, t)}
              {typeof data.runElapsedSeconds === "number" ? ` ${formatNodeRunElapsed(data.runElapsedSeconds, locale, t)}` : ""}
            </span>
          ) : null}
        </div>
      </div>

      {previewSrc ? (
        <div className="h-[76px] bg-book-bg">
          <img src={previewSrc} alt="" className="h-full w-full object-cover" />
        </div>
      ) : videoPreview ? (
        <StudioNodeVideoPreview preview={videoPreview} />
      ) : (
        <div className="flex min-h-[58px] flex-col gap-2 px-2 py-2">
          <p className="line-clamp-2 flex-1 text-small leading-4 text-book-text-sub">
            {nodeBodyPreview(data, t)}
          </p>
          <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 text-caption text-book-text-muted">
            <span className="min-w-0 truncate">{nodeSubtitle(data, t)}</span>
            <span className="font-mono">{nodeDimensionLabel(data)}</span>
          </div>
        </div>
      )}

      {issues.length > 0 ? (
        <div className="border-t border-warning/45 bg-warning-bg px-2 py-1 text-caption text-warning">
          <div className="flex items-center gap-1">
            <AlertTriangle size={10} />
            <span className="truncate" title={issues.join(t("common.summarySeparator"))}>
              {issues[0]}
            </span>
          </div>
        </div>
      ) : null}
    </div>
  );
};

export const STUDIO_NODE_TYPES = { studioNode: StudioCanvasNode };

type StudioNodeVideoPreviewValue = {
  filePath: string;
  generatedVideoId: number | null;
  title: string;
};

const StudioNodeVideoPreview: React.FC<{ preview: StudioNodeVideoPreviewValue }> = ({ preview }) => {
  const { t } = useI18n();
  const [src, setSrc] = React.useState("");
  const [loading, setLoading] = React.useState(false);
  const [failed, setFailed] = React.useState(false);

  React.useEffect(() => {
    let cancelled = false;
    let objectUrl: string | null = null;
    setFailed(false);
    setSrc("");

    if (!preview.filePath) {
      setLoading(false);
      return () => {};
    }

    if (!preview.generatedVideoId) {
      setSrc(toMediaSrc(preview.filePath));
      setLoading(false);
      return () => {};
    }

    setLoading(true);
    void loadVideoScrubPlaybackData(preview.generatedVideoId, true)
      .then((data) => {
        if (cancelled) return;
        objectUrl = videoPlaybackDataToObjectUrl(data);
        setSrc(objectUrl);
      })
      .catch(() => {
        if (cancelled) return;
        setFailed(true);
        setLoading(false);
      });

    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [preview.filePath, preview.generatedVideoId]);

  return (
    <div className="relative h-[76px] overflow-hidden bg-black">
      {src && !failed ? (
        <video
          key={src}
          src={src}
          muted
          playsInline
          preload="metadata"
          onLoadedData={() => setLoading(false)}
          onLoadedMetadata={() => setLoading(false)}
          onError={() => {
            setFailed(true);
            setLoading(false);
          }}
          className="pointer-events-none h-full w-full object-cover"
          title={preview.title}
        />
      ) : null}
      {loading ? (
        <div className="absolute inset-0 flex items-center justify-center bg-black/70 text-caption text-book-text-muted">
          {t("video.canvas.preview.loadingVideo")}
        </div>
      ) : null}
      {failed || (!src && !loading) ? (
        <div className="flex h-full flex-col justify-center gap-1 px-2 text-caption text-book-text-muted">
          <span className="truncate">{t("video.canvas.preview.videoGenerated")}</span>
          <span className="truncate font-mono" title={preview.filePath}>
            {preview.filePath.split("/").pop() || preview.filePath}
          </span>
        </div>
      ) : null}
    </div>
  );
};

export function edgeRoleLabel(role: string, t: MessageTranslator): string {
  switch (role) {
    case "context":
      return t("video.canvas.edgeRole.context");
    case "first_frame":
      return t("video.canvas.edgeRole.firstFrame");
    case "end_frame":
      return t("video.canvas.edgeRole.endFrame");
    case "style_reference":
      return t("video.canvas.edgeRole.styleReference");
    case "source_video":
      return t("video.canvas.edgeRole.sourceVideo");
    case "assembly_clip":
      return t("video.canvas.edgeRole.assemblyClip");
    case "final_video":
      return t("video.canvas.edgeRole.finalVideo");
    default:
      return role || t("video.canvas.edgeRole.connection");
  }
}

export function nodeKindMeta(kind: string, t: MessageTranslator): {
  label: string;
  icon: StudioTemplateIcon;
  tone: string;
} {
  switch (kind) {
    case "TextInputNode":
      return { label: t("video.canvas.node.textInput"), icon: Type, tone: "a4n-studio-node-text" };
    case "ImageInputNode":
      return { label: t("video.canvas.node.imageInput"), icon: Image, tone: "a4n-studio-node-media" };
    case "VideoInputNode":
      return { label: t("video.canvas.node.videoInput"), icon: Film, tone: "a4n-studio-node-video" };
    case "AudioInputNode":
      return { label: t("video.canvas.node.audioInput"), icon: Music, tone: "a4n-studio-node-audio" };
    case "RenderTextNode":
      return { label: t("video.canvas.node.renderText"), icon: Wand2, tone: "a4n-studio-node-action" };
    case "LlmTextNode":
      return { label: t("video.canvas.node.llmText"), icon: Wand2, tone: "a4n-studio-node-action" };
    case "GenerateImageNode":
      return { label: t("video.canvas.node.generateImage"), icon: Image, tone: "a4n-studio-node-generate" };
    case "GenerateVideoNode":
      return { label: t("video.canvas.node.generateVideo"), icon: Film, tone: "a4n-studio-node-generate" };
    case "ExtractFrameNode":
      return { label: t("video.canvas.node.extractFrame"), icon: Image, tone: "a4n-studio-node-action" };
    case "TrimVideoNode":
      return { label: t("video.canvas.node.trimVideo"), icon: Scissors, tone: "a4n-studio-node-action" };
    case "AssembleVideoNode":
      return { label: t("video.canvas.node.assembleVideo"), icon: Film, tone: "a4n-studio-node-assemble" };
    case "TextProbeNode":
      return { label: t("video.canvas.node.textProbe"), icon: Eye, tone: "a4n-studio-node-review" };
    case "ImageProbeNode":
      return { label: t("video.canvas.node.imageProbe"), icon: Eye, tone: "a4n-studio-node-review" };
    case "VideoProbeNode":
      return { label: t("video.canvas.node.videoProbe"), icon: Eye, tone: "a4n-studio-node-review" };
    case "AudioProbeNode":
      return { label: t("video.canvas.node.audioProbe"), icon: Eye, tone: "a4n-studio-node-review" };
    case "TextOutputNode":
      return { label: t("video.canvas.node.textOutput"), icon: Type, tone: "a4n-studio-node-assemble" };
    case "ImageOutputNode":
      return { label: t("video.canvas.node.imageOutput"), icon: Download, tone: "a4n-studio-node-assemble" };
    case "VideoOutputNode":
      return { label: t("video.canvas.node.videoOutput"), icon: Download, tone: "a4n-studio-node-assemble" };
    case "AudioOutputNode":
      return { label: t("video.canvas.node.audioOutput"), icon: Download, tone: "a4n-studio-node-assemble" };
    default:
      return { label: kind, icon: Layers3, tone: "a4n-studio-node-generic" };
  }
}

export function nodeDisplayTitle(data: StudioNodeData, t: MessageTranslator): string {
  const title = configText(data.config, "title").trim();
  if (title) return title;
  if (data.label && data.label !== data.nodeKind) return data.label;
  return nodeKindMeta(data.nodeKind, t).label;
}

export function canvasDataTypeLabel(value: string, t: MessageTranslator): string {
  const normalized = value.trim().toLowerCase();
  switch (normalized) {
    case "text": return t("video.canvas.dataType.text");
    case "image": return t("video.canvas.dataType.image");
    case "video": return t("video.canvas.dataType.video");
    case "audio": return t("video.canvas.dataType.audio");
    case "media": return t("video.canvas.dataType.media");
    case "json": return t("video.canvas.dataType.json");
    case "any": return t("video.canvas.dataType.any");
    case CANVAS_TEXT_OUTPUT_KIND_IMAGE_PROMPT: return t("video.canvas.dataType.imagePrompt");
    case CANVAS_TEXT_OUTPUT_KIND_VIDEO_PROMPT: return t("video.canvas.dataType.videoPrompt");
    case CANVAS_TEXT_OUTPUT_KIND_SHOT_SCRIPT: return t("video.canvas.dataType.shotScript");
    case CANVAS_TEXT_OUTPUT_KIND_STORYBOARD: return t("video.canvas.dataType.storyboard");
    case CANVAS_TEXT_OUTPUT_KIND_VISUAL_DESCRIPTION: return t("video.canvas.inspector.visualDescription");
    case CANVAS_TEXT_OUTPUT_KIND_DIALOGUE: return t("video.canvas.inspector.dialogue");
    case CANVAS_TEXT_OUTPUT_KIND_RHYTHM_PLAN: return t("video.canvas.inspector.rhythmPlan");
    default: return normalized || t("video.canvas.dataType.unknown");
  }
}

function canvasReferenceTypeLabel(value: string, t: MessageTranslator): string {
  switch (value) {
    case "generated_image": return t("video.canvas.reference.image");
    case "generated_video": return t("video.canvas.reference.video");
    case "media_asset": return t("video.canvas.reference.media");
    case "canvas_text_asset": return t("video.canvas.reference.text");
    default: return t("video.canvas.reference.resource");
  }
}

function nodeSubtitle(data: StudioNodeData, t: MessageTranslator): string {
  const fileName = configText(data.config, "file_path").split("/").pop();
  if (fileName) return fileName;
  const dimension = nodeDimensionLabel(data);
  if (dimension !== "-") return dimension;
  if (data.refType) return t("video.canvas.body.bound", { type: canvasReferenceTypeLabel(data.refType, t) });
  const prompt = configText(data.config, "prompt").trim();
  if (prompt) return prompt;
  const observedType = configText(data.config, "observed_data_type").trim();
  if (observedType) return t("video.canvas.body.observed", { type: canvasDataTypeLabel(observedType, t) });
  return nodeKindMeta(data.nodeKind, t).label;
}

export function nodePreviewSrc(data: StudioNodeData): string | null {
  if (nodeKindIsOperation(data.nodeKind)) return null;
  const thumbnailPath = configText(data.config, "thumbnail_path").trim();
  if (thumbnailPath) return toMediaSrc(thumbnailPath);
  const filePath = configText(data.config, "file_path").trim();
  if (!filePath) return null;
  if (data.nodeKind === "ImageInputNode" || data.nodeKind === "ImageOutputNode" || /\.(png|jpe?g|webp|gif|avif)$/i.test(filePath)) {
    return toMediaSrc(filePath);
  }
  return null;
}

function nodeVideoPreview(data: StudioNodeData, t: MessageTranslator): StudioNodeVideoPreviewValue | null {
  if (!nodeKindCanShowVideoPreview(data.nodeKind)) return null;
  const output = configObject(data.config, "output");
  const filePath = firstNonEmptyString([
    configText(data.config, "file_path"),
    configText(output, "file_path"),
  ]);
  if (!filePath || !/\.(mp4|mov|webm|m4v|mkv)$/i.test(filePath)) return null;
  const generatedVideoId =
    configPositiveNumber(data.config, "generated_video_id") ??
    configPositiveNumber(output, "generated_video_id") ??
    nodeGeneratedVideoRefId(data);
  return {
    filePath,
    generatedVideoId,
    title: nodeDisplayTitle(data, t),
  };
}

function nodeKindCanShowVideoPreview(kind: string): boolean {
  return kind === "VideoInputNode" || kind === "VideoOutputNode" || kind === "VideoProbeNode";
}

function frameKindLabel(value: string, t: MessageTranslator): string {
  switch (value) {
    case "first":
      return t("video.canvas.inspector.firstFrame");
    case "tail":
      return t("video.canvas.inspector.tailFrame");
    case "time":
      return t("video.canvas.inspector.specifiedTime");
    default:
      return value || t("video.canvas.dataType.unknown");
  }
}

function nodeBodyPreview(data: StudioNodeData, t: MessageTranslator): string {
  const operationPreview = operationNodeBodyPreview(data, t);
  if (operationPreview) return operationPreview;
  const observedText = configText(data.config, "observed_text").trim();
  if (observedText) return observedText;
  const renderedPrompt = configText(data.config, "prompt_text").trim();
  if (renderedPrompt) return renderedPrompt;
  const content = configText(data.config, "content").trim();
  if (content) return content;
  const prompt = configText(data.config, "prompt").trim();
  if (prompt) return prompt;
  const instruction = configText(data.config, "instruction").trim();
  if (instruction) return instruction;
  const filePath = configText(data.config, "file_path").trim();
  if (filePath) return filePath;
  return t("video.canvas.body.waitingConfig");
}

function operationNodeBodyPreview(data: StudioNodeData, t: MessageTranslator): string | null {
  switch (data.nodeKind) {
    case "RenderTextNode":
      return t("video.canvas.body.renderText");
    case "LlmTextNode": {
      const outputKind = configText(data.config, "output_kind");
      const instruction = configText(data.config, "instruction").trim();
      return instruction || (outputKind
        ? t("video.canvas.body.inputMode", { mode: canvasDataTypeLabel(outputKind, t) })
        : t("video.canvas.body.missingOutputType"));
    }
    case "GenerateImageNode":
      return t("video.canvas.body.generateImage");
    case "GenerateVideoNode":
      {
        const mode = parseGenerateVideoInputMode(data.config.input_mode);
        return mode
          ? t("video.canvas.body.inputMode", { mode: generateVideoInputModeLabel(mode, t) })
          : t("video.canvas.body.missingInputMode");
      }
    case "ExtractFrameNode":
      return t("video.canvas.body.extractFrame", { kind: frameKindLabel(configText(data.config, "frame_kind"), t) });
    case "TrimVideoNode": {
      const start = configNumber(data.config, "start_secs");
      const end = configNumber(data.config, "end_secs");
      return t("video.canvas.body.trim", { start, end });
    }
    case "AssembleVideoNode":
      return t("video.canvas.body.assembleVideo");
    case "TextProbeNode":
      return configText(data.config, "observed_text").trim() || t("video.canvas.body.textProbe");
    case "ImageProbeNode":
      return t("video.canvas.body.imageProbe");
    case "VideoProbeNode":
      return t("video.canvas.body.videoProbe");
    case "AudioProbeNode":
      return t("video.canvas.body.audioProbe");
    default:
      return null;
  }
}

function nodeDimensionLabel(data: StudioNodeData): string {
  const width = configNumber(data.config, "width");
  const height = configNumber(data.config, "height");
  const duration = configNumber(data.config, "duration_secs");
  const parts: string[] = [];
  if (width && height) parts.push(`${width}x${height}`);
  if (duration) parts.push(`${duration}s`);
  return parts.length ? parts.join(" / ") : "-";
}

export function nodeValidationIssues(data: StudioNodeData, t: MessageTranslator): string[] {
  const issues: string[] = [];
  if (data.state === "missing") {
    issues.push(configText(data.config, "missing_reason") || t("video.canvas.issue.missingAsset"));
  }
  if (defaultRefTypeForNode(data.nodeKind) && !data.refId) {
    issues.push(t("video.canvas.issue.missingMediaReference"));
  }
  return issues;
}

export function stateLabel(state: string, t: MessageTranslator): string {
  switch (state) {
    case "ready":
      return t("video.canvas.state.ready");
    case "running":
      return t("video.canvas.state.running");
    case "completed":
      return t("video.canvas.state.completed");
    case "failed":
      return t("video.canvas.state.failed");
    case "missing":
      return t("video.canvas.state.missing");
    default:
      return state || "-";
  }
}

export function nodeStateClass(state: string): string {
  switch (state) {
    case "completed":
      return "a4n-studio-state-success";
    case "running":
      return "a4n-studio-state-info";
    case "failed":
    case "missing":
      return "a4n-studio-state-error";
    default:
      return "a4n-studio-state-muted";
  }
}

function nodeRunStatusLabel(status: NonNullable<StudioNodeData["runStatus"]>, t: MessageTranslator): string {
  switch (status) {
    case "running":
      return t("video.canvas.state.runRunning");
    case "completed":
      return t("video.canvas.state.runCompleted");
    case "failed":
      return t("video.canvas.state.failed");
  }
}

function formatNodeRunElapsed(seconds: number, locale: Locale, t: MessageTranslator): string {
  if (!Number.isFinite(seconds) || seconds < 0) return t("video.postprocess.seconds", { count: formatNumber(0, locale) });
  if (seconds < 60) return t("video.postprocess.seconds", { count: formatNumber(Math.floor(seconds), locale) });
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.floor(seconds % 60);
  return t("video.postprocess.minutesSeconds", {
    minutes: formatNumber(minutes, locale),
    seconds: formatNumber(remainingSeconds, locale, { minimumIntegerDigits: 2 }),
  });
}

function portOffset(index: number, total: number): string {
  if (total <= 1) return "50%";
  return `${Math.round(((index + 1) / (total + 1)) * 100)}%`;
}

function firstNonEmptyString(values: string[]): string {
  return values.map((value) => value.trim()).find((value) => value.length > 0) ?? "";
}

function nodeKindIsOperation(kind: string): boolean {
  return (
    kind === "RenderTextNode" ||
    kind === "LlmTextNode" ||
    kind === "GenerateImageNode" ||
    kind === "GenerateVideoNode" ||
    kind === "ExtractFrameNode" ||
    kind === "TrimVideoNode" ||
    kind === "AssembleVideoNode"
  );
}
