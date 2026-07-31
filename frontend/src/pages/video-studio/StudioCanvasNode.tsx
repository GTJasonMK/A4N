import React from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { loadVideoPlaybackData } from "../../api/video";
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
import type { StudioNode, StudioNodeData, StudioTemplateIcon } from "./types";
import { configNumber, configObject, configPositiveNumber, configText } from "./canvasConfig";
import {
  STUDIO_NODE_CATEGORY_ORDER,
  generateVideoInputModeLabel,
  normalizeGenerateVideoInputMode,
  studioCategoryLabel,
  studioCategoryTitle,
  studioNodeInputPortsForData,
  studioNodeOutputPortsForData,
  type StudioNodeContract,
  useStudioNodeContract,
} from "./nodeContracts";
import { nodeGeneratedVideoRefId } from "./canvasNodeRefs";
import { videoPlaybackDataToObjectUrl } from "../../utils/videoPlayback";

type StudioNodeTemplate = {
  kind: string;
  label: string;
  icon: StudioTemplateIcon;
};

export function buildNodeTemplateGroups(contracts: StudioNodeContract[]): Array<{ title: string; items: StudioNodeTemplate[] }> {
  return STUDIO_NODE_CATEGORY_ORDER.map((category) => ({
    title: studioCategoryTitle(category),
    items: contracts
      .filter((contract) => contract.category === category)
      .map((contract) => ({
        kind: contract.kind,
        label: contract.label,
        icon: nodeKindMeta(contract.kind).icon,
      })),
  })).filter((group) => group.items.length > 0);
}

const StudioCanvasNode: React.FC<NodeProps<StudioNode>> = ({ data, selected, isConnectable }) => {
  const meta = nodeKindMeta(data.nodeKind);
  const contract = useStudioNodeContract(data.nodeKind);
  const Icon = meta.icon;
  const previewSrc = nodePreviewSrc(data);
  const videoPreview = previewSrc ? null : nodeVideoPreview(data);
  const issues = nodeValidationIssues(data);
  const inputPorts = contract
    ? studioNodeInputPortsForData(data, contract).map((port) => ({ id: port.id, label: port.label }))
    : [];
  const outputPorts = contract
    ? studioNodeOutputPortsForData(data, contract).map((port) => ({ id: port.id, label: port.label }))
    : [];

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
          <div className="truncate text-xs font-semibold text-book-text-main" title={nodeDisplayTitle(data)}>
            {nodeDisplayTitle(data)}
          </div>
          <div className="truncate text-caption text-book-text-muted">
            {contract ? studioCategoryLabel(contract.category) : "未知节点"}
          </div>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-1">
          <span className={`a4n-studio-state-pill ${nodeStateClass(data.state)}`}>
            {stateLabel(data.state)}
          </span>
          {data.runStatus ? (
            <span className={`a4n-studio-run-pill a4n-studio-run-pill-${data.runStatus}`}>
              {nodeRunStatusLabel(data.runStatus)}
              {typeof data.runElapsedSeconds === "number" ? ` ${formatNodeRunElapsed(data.runElapsedSeconds)}` : ""}
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
            {nodeBodyPreview(data)}
          </p>
          <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 text-caption text-book-text-muted">
            <span className="min-w-0 truncate">{nodeSubtitle(data)}</span>
            <span className="font-mono">{nodeDimensionLabel(data)}</span>
          </div>
        </div>
      )}

      {issues.length > 0 ? (
        <div className="border-t border-warning/45 bg-warning-bg px-2 py-1 text-caption text-warning">
          <div className="flex items-center gap-1">
            <AlertTriangle size={10} />
            <span className="truncate" title={issues.join("；")}>
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
    void loadVideoPlaybackData(preview.generatedVideoId)
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
          加载视频预览
        </div>
      ) : null}
      {failed || (!src && !loading) ? (
        <div className="flex h-full flex-col justify-center gap-1 px-2 text-caption text-book-text-muted">
          <span className="truncate">视频已生成</span>
          <span className="truncate font-mono" title={preview.filePath}>
            {preview.filePath.split("/").pop() || preview.filePath}
          </span>
        </div>
      ) : null}
    </div>
  );
};

export function defaultRefTypeForNode(kind: string): string | null {
  switch (kind) {
    case "ImageInputNode":
      return "generated_image";
    case "VideoInputNode":
      return "generated_video";
    case "AudioInputNode":
      return "media_asset";
    default:
      return null;
  }
}

export function edgeRoleLabel(role: string): string {
  switch (role) {
    case "context":
      return "上下文";
    case "first_frame":
      return "首帧";
    case "end_frame":
      return "尾帧";
    case "style_reference":
      return "参考图";
    case "source_video":
      return "源视频";
    case "assembly_clip":
      return "片段";
    case "final_video":
      return "最终视频";
    default:
      return role || "连线";
  }
}

export function nodeKindMeta(kind: string): {
  label: string;
  icon: StudioTemplateIcon;
  tone: string;
} {
  switch (kind) {
    case "TextInputNode":
      return { label: "文本输入", icon: Type, tone: "a4n-studio-node-text" };
    case "ImageInputNode":
      return { label: "图片输入", icon: Image, tone: "a4n-studio-node-media" };
    case "VideoInputNode":
      return { label: "视频输入", icon: Film, tone: "a4n-studio-node-video" };
    case "AudioInputNode":
      return { label: "音频输入", icon: Music, tone: "a4n-studio-node-audio" };
    case "RenderTextNode":
      return { label: "文本模板", icon: Wand2, tone: "a4n-studio-node-action" };
    case "LlmTextNode":
      return { label: "LLM文本处理", icon: Wand2, tone: "a4n-studio-node-action" };
    case "GenerateImageNode":
      return { label: "生成图片", icon: Image, tone: "a4n-studio-node-generate" };
    case "GenerateVideoNode":
      return { label: "生成视频", icon: Film, tone: "a4n-studio-node-generate" };
    case "ExtractFrameNode":
      return { label: "抽取帧", icon: Image, tone: "a4n-studio-node-action" };
    case "TrimVideoNode":
      return { label: "裁剪视频", icon: Scissors, tone: "a4n-studio-node-action" };
    case "AssembleVideoNode":
      return { label: "组装视频", icon: Film, tone: "a4n-studio-node-assemble" };
    case "TextProbeNode":
      return { label: "文本探针", icon: Eye, tone: "a4n-studio-node-review" };
    case "ImageProbeNode":
      return { label: "图片探针", icon: Eye, tone: "a4n-studio-node-review" };
    case "VideoProbeNode":
      return { label: "视频探针", icon: Eye, tone: "a4n-studio-node-review" };
    case "AudioProbeNode":
      return { label: "音频探针", icon: Eye, tone: "a4n-studio-node-review" };
    case "TextOutputNode":
      return { label: "文本输出", icon: Type, tone: "a4n-studio-node-assemble" };
    case "ImageOutputNode":
      return { label: "图片输出", icon: Download, tone: "a4n-studio-node-assemble" };
    case "VideoOutputNode":
      return { label: "视频输出", icon: Download, tone: "a4n-studio-node-assemble" };
    case "AudioOutputNode":
      return { label: "音频输出", icon: Download, tone: "a4n-studio-node-assemble" };
    default:
      return { label: kind, icon: Layers3, tone: "a4n-studio-node-generic" };
  }
}

export function nodeDisplayTitle(data: StudioNodeData): string {
  const title = configText(data.config, "title").trim();
  if (title) return title;
  if (data.label && data.label !== data.nodeKind) return data.label;
  return nodeKindMeta(data.nodeKind).label;
}

export function canvasDataTypeLabel(value: string): string {
  const normalized = value.trim().toLowerCase();
  const labels: Record<string, string> = {
    text: "文本",
    image: "图片",
    video: "视频",
    audio: "音频",
    media: "媒体",
    json: "结构数据",
    any: "任意内容",
    canvas_text: "画布文本",
    image_prompt: "图片提示词",
    video_prompt: "视频提示词",
    shot_script: "镜头脚本",
    storyboard: "分镜",
  };
  return labels[normalized] ?? (normalized || "未知内容");
}

function canvasReferenceTypeLabel(value: string): string {
  const labels: Record<string, string> = {
    generated_image: "图片",
    generated_video: "视频",
    media_asset: "媒体",
    canvas_text_asset: "文本",
  };
  return labels[value] ?? "资源";
}

function nodeSubtitle(data: StudioNodeData): string {
  const fileName = configText(data.config, "file_path").split("/").pop();
  if (fileName) return fileName;
  const dimension = nodeDimensionLabel(data);
  if (dimension !== "-") return dimension;
  if (data.refType) return `已绑定${canvasReferenceTypeLabel(data.refType)}`;
  const prompt = configText(data.config, "prompt").trim();
  if (prompt) return prompt;
  const observedType = configText(data.config, "observed_data_type").trim();
  if (observedType) return `已观察：${canvasDataTypeLabel(observedType)}`;
  return nodeKindMeta(data.nodeKind).label;
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

function nodeVideoPreview(data: StudioNodeData): StudioNodeVideoPreviewValue | null {
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
    title: nodeDisplayTitle(data),
  };
}

function nodeKindCanShowVideoPreview(kind: string): boolean {
  return kind === "VideoInputNode" || kind === "VideoOutputNode" || kind === "VideoProbeNode";
}

function nodeBodyPreview(data: StudioNodeData): string {
  const operationPreview = operationNodeBodyPreview(data);
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
  if (data.nodeKind === "GenerateVideoNode") {
    return `输入模式：${generateVideoInputModeLabel(normalizeGenerateVideoInputMode(data.config.input_mode))}`;
  }
  return "等待配置";
}

function operationNodeBodyPreview(data: StudioNodeData): string | null {
  switch (data.nodeKind) {
    case "RenderTextNode":
      return "按模板生成文本";
    case "LlmTextNode": {
      const outputKind = configText(data.config, "output_kind") || "shot_script";
      const instruction = configText(data.config, "instruction").trim();
      return instruction || `输出类型：${outputKind}`;
    }
    case "GenerateImageNode":
      return "接收上游提示词生成图片";
    case "GenerateVideoNode":
      return `输入模式：${generateVideoInputModeLabel(normalizeGenerateVideoInputMode(data.config.input_mode))}`;
    case "ExtractFrameNode":
      return `抽帧：${configText(data.config, "frame_kind") || "first"}`;
    case "TrimVideoNode": {
      const start = configNumber(data.config, "start_secs");
      const end = configNumber(data.config, "end_secs");
      return `裁剪：${start === "" ? 0 : start}s - ${end === "" ? "?" : end}s`;
    }
    case "AssembleVideoNode":
      return "接收视频片段并输出视频";
    case "TextProbeNode":
      return configText(data.config, "observed_text").trim() || "观察并透传文本";
    case "ImageProbeNode":
      return "观察并透传图片";
    case "VideoProbeNode":
      return "观察并透传视频";
    case "AudioProbeNode":
      return "观察并透传音频";
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

export function nodeValidationIssues(data: StudioNodeData): string[] {
  const issues: string[] = [];
  if (data.state === "missing") {
    issues.push(configText(data.config, "missing_reason") || "引用资产缺失");
  }
  if ((data.nodeKind === "ImageInputNode" || data.nodeKind === "VideoInputNode" || data.nodeKind === "AudioInputNode") && !data.refId) {
    issues.push("缺少媒体资产引用");
  }
  return issues;
}

export function stateLabel(state: string): string {
  switch (state) {
    case "ready":
      return "就绪";
    case "running":
      return "运行";
    case "completed":
      return "完成";
    case "failed":
      return "失败";
    case "missing":
      return "缺失";
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

function nodeRunStatusLabel(status: NonNullable<StudioNodeData["runStatus"]>): string {
  switch (status) {
    case "running":
      return "运行中";
    case "completed":
      return "已完成";
    case "failed":
      return "失败";
  }
}

function formatNodeRunElapsed(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0s";
  if (seconds < 60) return `${Math.floor(seconds)}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.floor(seconds % 60);
  return `${minutes}m${remainingSeconds.toString().padStart(2, "0")}s`;
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
