import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Copy, Eye } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../api";
import { loadVideoScrubPlaybackData } from "../../api/video";
import { BookButton } from "../../components/ui/BookButton";
import { Modal } from "../../components/ui/Modal";
import { copyTextToClipboard } from "../../utils/clipboard";
import { toMediaSrc } from "../../utils/mediaSrc";
import { videoPlaybackDataToObjectUrl } from "../../utils/videoPlayback";
import { nodeDisplayTitle, nodeKindMeta } from "./StudioCanvasNode";
import { configNumber, configObject, configPositiveNumber, configText } from "./canvasConfig";
import { isOutputNodeKind, isProbeNodeKind, nodeHasProviderRequest } from "./canvasNodeKinds";
import {
  nodeGeneratedImageRefId,
  nodeGeneratedVideoRefId,
  nodeMediaAssetRefId,
} from "./canvasNodeRefs";
import type { StudioNode } from "./types";

interface CanvasNodeContentPreviewDialogProps {
  node: StudioNode | null;
  onClose: () => void;
  onCopyError: (message: string) => void;
}

type CanvasNodeContentType = "text" | "image" | "video" | "audio" | "unknown";

type CanvasNodeContentPreview = {
  title: string;
  kindLabel: string;
  dataType: CanvasNodeContentType;
  text: string;
  filePath: string;
  thumbnailPath: string;
  generatedVideoId: number | null;
  metadataRows: Array<[string, string]>;
  emptyMessage: string;
};

export const CanvasNodeContentPreviewDialog: React.FC<CanvasNodeContentPreviewDialogProps> = ({
  node,
  onClose,
  onCopyError,
}) => {
  const [copied, setCopied] = useState(false);
  const preview = useMemo(() => (node ? buildNodeContentPreview(node) : null), [node]);
  const copyValue = preview?.text || preview?.filePath || "";
  const copyLabel = preview?.text ? "复制文本" : "复制路径";

  useEffect(() => {
    if (!node) setCopied(false);
  }, [node]);

  const copyContent = async () => {
    if (!copyValue) return;
    try {
      await copyTextToClipboard(copyValue);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch (err) {
      setCopied(false);
      onCopyError(extractErrorMessage(err, "复制节点内容失败"));
    }
  };

  return (
    <Modal
      isOpen={node !== null}
      onClose={onClose}
      title={preview ? `${preview.title} 内容查看` : "节点内容查看"}
      subtitle={
        node && preview
          ? `${preview.kindLabel} · ${contentTypeLabel(preview.dataType)}`
          : "查看节点当前保存或观察到的内容"
      }
      icon={<Eye size={15} />}
      maxWidthClassName="max-w-[min(96vw,72rem)]"
      bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4"
      footer={
        <div className="flex w-full items-center justify-end gap-2">
          {copied ? <span className="text-xs text-success">已复制</span> : null}
          <BookButton
            variant="secondary"
            size="sm"
            onClick={() => void copyContent()}
            disabled={!copyValue}
          >
            <Copy size={13} />
            {copyLabel}
          </BookButton>
        </div>
      }
    >
      {preview ? <CanvasNodeContentPreviewBody preview={preview} /> : null}
    </Modal>
  );
};

const CanvasNodeContentPreviewBody: React.FC<{ preview: CanvasNodeContentPreview }> = ({ preview }) => {
  const mediaPath = preview.dataType === "image" ? preview.filePath || preview.thumbnailPath : preview.filePath;
  const mediaSrc = mediaPath ? toMediaSrc(mediaPath) : "";
  const posterSrc = preview.thumbnailPath ? toMediaSrc(preview.thumbnailPath) : undefined;
  const {
    videoSrc,
    videoLoading,
    videoLoadError,
    handleVideoLoadStart,
    handleVideoLoaded,
    handleVideoError,
  } = useCanvasVideoPreviewSource(preview);
  const hasContent = Boolean(preview.text || mediaSrc || videoSrc);

  return (
    <div className="space-y-3 text-xs">
      {preview.dataType === "text" && preview.text ? (
        <pre className="whitespace-pre-wrap break-words rounded-xl border border-book-border bg-book-bg-paper px-3 py-2 text-sm leading-6 text-book-text-main">
          {preview.text}
        </pre>
      ) : null}
      {preview.dataType === "image" && mediaSrc ? (
        <div className="flex min-h-[18rem] items-center justify-center rounded-xl border border-book-border bg-book-bg">
          <img src={mediaSrc} alt={preview.title} className="max-h-[64vh] max-w-full object-contain" />
        </div>
      ) : null}
      {preview.dataType === "video" && (videoSrc || videoLoading || videoLoadError) ? (
        <div className="overflow-hidden rounded-xl border border-book-border bg-black">
          {videoSrc ? (
            <video
              key={videoSrc}
              src={videoSrc}
              poster={posterSrc}
              controls
              preload="metadata"
              onLoadStart={handleVideoLoadStart}
              onLoadedMetadata={handleVideoLoaded}
              onCanPlay={handleVideoLoaded}
              onError={handleVideoError}
              className="max-h-[64vh] w-full bg-black"
            />
          ) : null}
          {videoLoading ? (
            <div className="flex min-h-[18rem] items-center justify-center gap-2 text-book-text-muted">
              <LoadingSpinner size="md" />
              正在加载本地视频文件
            </div>
          ) : null}
          {videoLoadError ? (
            <div className="border-t border-book-border bg-book-bg px-3 py-2 text-small text-warning">
              {videoLoadError}
            </div>
          ) : null}
        </div>
      ) : null}
      {preview.dataType === "audio" && mediaSrc ? (
        <div className="rounded-xl border border-book-border bg-book-bg-paper p-4">
          <audio src={mediaSrc} controls className="w-full" />
        </div>
      ) : null}
      {!hasContent ? (
        <div className="flex min-h-[18rem] items-center justify-center rounded-xl border border-dashed border-book-border bg-book-bg p-6 text-center text-sm text-book-text-muted">
          {preview.emptyMessage}
        </div>
      ) : null}
      {preview.metadataRows.length > 0 ? (
        <details className="rounded-xl border border-book-border/70 bg-book-bg-paper/60">
          <summary className="cursor-pointer px-3 py-2 text-small text-book-text-muted">
            诊断信息
          </summary>
          <div className="grid grid-cols-[96px_minmax(0,1fr)] gap-x-3 gap-y-1 border-t border-book-border/60 px-3 py-2 text-small">
            {preview.metadataRows.map(([label, value]) => (
              <React.Fragment key={label}>
                <span className="text-book-text-muted">{label}</span>
                <span className="break-all font-mono text-book-text-main">{value}</span>
              </React.Fragment>
            ))}
          </div>
        </details>
      ) : null}
    </div>
  );
};

function buildNodeContentPreview(node: StudioNode): CanvasNodeContentPreview {
  const config = node.data.config;
  const output = configObject(config, "output");
  const dataType = contentDataTypeForNode(node);
  const text = firstNonEmptyString([
    configText(config, "observed_text"),
    configText(config, "text"),
    configText(config, "content"),
    configText(config, "prompt_text"),
    configText(config, "prompt"),
    configText(config, "instruction"),
    configText(output, "text"),
  ]);
  const filePath = firstNonEmptyString([
    configText(config, "file_path"),
    configText(output, "file_path"),
  ]);
  const thumbnailPath = firstNonEmptyString([
    configText(config, "thumbnail_path"),
    configText(output, "thumbnail_path"),
  ]);
  const metadataRows = nodeContentMetadataRows(node, dataType, filePath, thumbnailPath);

  return {
    title: nodeDisplayTitle(node.data),
    kindLabel: nodeKindMeta(node.data.nodeKind).label,
    dataType,
    text,
    filePath,
    thumbnailPath,
    generatedVideoId: nodeContentGeneratedVideoId(node),
    metadataRows,
    emptyMessage: nodeContentEmptyMessage(node, dataType),
  };
}

export function nodeHasContentPreview(node: StudioNode): boolean {
  const preview = buildNodeContentPreview(node);
  return Boolean(preview.text || preview.filePath || preview.thumbnailPath || preview.generatedVideoId);
}

function useCanvasVideoPreviewSource(preview: CanvasNodeContentPreview): {
  videoSrc: string;
  videoLoading: boolean;
  videoLoadError: string | null;
  handleVideoLoadStart: () => void;
  handleVideoLoaded: () => void;
  handleVideoError: (event: React.SyntheticEvent<HTMLVideoElement>) => void;
} {
  const [videoSrc, setVideoSrc] = useState("");
  const [videoLoading, setVideoLoading] = useState(false);
  const [videoLoadError, setVideoLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let objectUrl: string | null = null;

    setVideoSrc("");
    setVideoLoadError(null);
    if (preview.dataType !== "video" || !preview.filePath) {
      setVideoLoading(false);
      return () => {};
    }
    if (!preview.generatedVideoId) {
      setVideoSrc(toMediaSrc(preview.filePath));
      setVideoLoading(false);
      return () => {};
    }

    setVideoLoading(true);
    void loadVideoScrubPlaybackData(preview.generatedVideoId, true)
      .then((data) => {
        if (cancelled) return;
        objectUrl = videoPlaybackDataToObjectUrl(data);
        setVideoSrc(objectUrl);
      })
      .catch((err) => {
        if (cancelled) return;
        setVideoLoading(false);
        setVideoLoadError(extractErrorMessage(err, "无法读取本地视频文件"));
      });

    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [preview.dataType, preview.filePath, preview.generatedVideoId]);

  const handleVideoLoadStart = useCallback(() => {
    setVideoLoading(true);
    setVideoLoadError(null);
  }, []);

  const handleVideoLoaded = useCallback(() => {
    setVideoLoading(false);
  }, []);

  const handleVideoError = useCallback((event: React.SyntheticEvent<HTMLVideoElement>) => {
    setVideoLoading(false);
    const detail = event.currentTarget.error?.message?.trim();
    setVideoLoadError(
      detail
        ? `内置播放器无法播放该视频：${detail}`
        : "内置播放器无法解码该视频。文件已保存，可以复制路径后用本地播放器检查。",
    );
  }, []);

  return { videoSrc, videoLoading, videoLoadError, handleVideoLoadStart, handleVideoLoaded, handleVideoError };
}

function nodeContentGeneratedVideoId(node: StudioNode): number | null {
  const config = node.data.config;
  const output = configObject(config, "output");
  return (
    configPositiveNumber(config, "generated_video_id") ??
    configPositiveNumber(output, "generated_video_id") ??
    nodeGeneratedVideoRefId(node.data)
  );
}

function nodeContentMetadataRows(
  node: StudioNode,
  dataType: CanvasNodeContentType,
  filePath: string,
  thumbnailPath: string,
): Array<[string, string]> {
  const config = node.data.config;
  const rows: Array<[string, string]> = [["类型", contentTypeLabel(dataType)]];
  const canvasTextAssetId = configPositiveNumber(config, "canvas_text_asset_id");
  const generatedImageId =
    configPositiveNumber(config, "generated_image_id") ??
    nodeGeneratedImageRefId(node.data);
  const generatedVideoId = nodeContentGeneratedVideoId(node);
  const mediaAssetId =
    configPositiveNumber(config, "media_asset_id") ??
    nodeMediaAssetRefId(node.data);
  const durationSecs = configNumber(config, "duration_secs");
  const sourceNodeIds = config["source_node_ids"];

  if (canvasTextAssetId) rows.push(["文本资产", `#${canvasTextAssetId}`]);
  if (generatedImageId) rows.push(["图片", `#${generatedImageId}`]);
  if (generatedVideoId) rows.push(["视频", `#${generatedVideoId}`]);
  if (mediaAssetId) rows.push(["媒体", `#${mediaAssetId}`]);
  if (filePath) rows.push(["文件", filePath]);
  if (thumbnailPath) rows.push(["缩略图", thumbnailPath]);
  if (durationSecs !== "") rows.push(["时长", `${durationSecs}s`]);
  if (Array.isArray(sourceNodeIds) && sourceNodeIds.length > 0) {
    rows.push(["来源节点", sourceNodeIds.map(String).join(", ")]);
  }
  if (node.data.refType && node.data.refId) {
    rows.push(["引用", `${node.data.refType} #${node.data.refId}`]);
  }
  return rows;
}

function contentDataTypeForNode(node: StudioNode): CanvasNodeContentType {
  const config = node.data.config;
  const explicit = normalizeContentDataType(
    configText(config, "observed_data_type") || configText(config, "data_type"),
  );
  if (explicit !== "unknown") return explicit;
  const byKind = contentDataTypeForKind(node.data.nodeKind);
  if (byKind !== "unknown") return byKind;
  return contentDataTypeFromPath(configText(config, "file_path"));
}

function contentDataTypeForKind(kind: string): CanvasNodeContentType {
  if (kind.includes("Text") || kind === "RenderTextNode" || kind === "LlmTextNode") return "text";
  if (kind.includes("Image") || kind === "ExtractFrameNode") return "image";
  if (kind.includes("Video") || kind === "TrimVideoNode" || kind === "AssembleVideoNode") return "video";
  if (kind.includes("Audio")) return "audio";
  return "unknown";
}

function contentDataTypeFromPath(filePath: string): CanvasNodeContentType {
  if (/\.(png|jpe?g|webp|gif|avif)$/i.test(filePath)) return "image";
  if (/\.(mp4|mov|webm|mkv)$/i.test(filePath)) return "video";
  if (/\.(mp3|wav|m4a|aac|flac|ogg)$/i.test(filePath)) return "audio";
  return "unknown";
}

function normalizeContentDataType(value: string): CanvasNodeContentType {
  return value === "text" || value === "image" || value === "video" || value === "audio"
    ? value
    : "unknown";
}

function contentTypeLabel(dataType: CanvasNodeContentType): string {
  switch (dataType) {
    case "text":
      return "文本";
    case "image":
      return "图片";
    case "video":
      return "视频";
    case "audio":
      return "音频";
    default:
      return "未知";
  }
}

function nodeContentEmptyMessage(node: StudioNode, dataType: CanvasNodeContentType): string {
  if (isProbeNodeKind(node.data.nodeKind)) {
    return `该探针尚未观察到${contentTypeLabel(dataType)}内容。请运行经过该探针的工作流后再查看。`;
  }
  if (isOutputNodeKind(node.data.nodeKind)) {
    return `该输出节点尚未保存${contentTypeLabel(dataType)}结果。请运行画布或运行到该输出节点后再查看。`;
  }
  if (nodeHasProviderRequest(node)) {
    return "该节点会调用外部模型，请使用“模型请求”查看将发送的内容。";
  }
  return "该节点当前没有可查看的保存内容。";
}

function firstNonEmptyString(values: string[]): string {
  for (const value of values) {
    const trimmed = value.trim();
    if (trimmed) return trimmed;
  }
  return "";
}
