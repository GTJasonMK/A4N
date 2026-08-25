import type { CanvasMediaAssetSummary } from "../../api/canvas";
import type { GeneratedImage } from "../../api/image/types";
import type { GeneratedVideo } from "../../api/video/types";
import type { MessageTranslator } from "../../i18n";
import { canvasNodePosition } from "./canvasGraphMapping";
import {
  CANVAS_REF_TYPE_GENERATED_IMAGE,
  CANVAS_REF_TYPE_GENERATED_VIDEO,
  CANVAS_REF_TYPE_MEDIA_ASSET,
} from "./canvasNodeRefs";
import type { StudioNode } from "./types";

type ImportedInputNodeData = Pick<
  StudioNode["data"],
  "label" | "nodeKind" | "refType" | "refId" | "state" | "config"
>;

function importedInputNode(
  assetId: number,
  index: number,
  idPrefix: string,
  data: ImportedInputNodeData,
): StudioNode {
  return {
    id: `${idPrefix}-${assetId}-${Date.now()}`,
    type: "studioNode",
    selected: true,
    position: canvasNodePosition(index),
    data,
  };
}

export function importedImageInputNode(
  image: GeneratedImage,
  asset: CanvasMediaAssetSummary,
  index: number,
  t: MessageTranslator,
): StudioNode {
  const title = image.prompt || t("video.canvas.imported.image", { id: image.id });
  return importedInputNode(asset.id, index, "imported-image", {
    label: title,
    nodeKind: "ImageInputNode",
    refType: CANVAS_REF_TYPE_GENERATED_IMAGE,
    refId: image.id,
    state: "ready",
    config: {
      title,
      file_path: image.file_path,
      generated_image_id: image.id,
      media_asset_id: asset.id,
      width: image.width,
      height: image.height,
    },
  });
}

export function importedVideoInputNode(
  video: GeneratedVideo,
  asset: CanvasMediaAssetSummary,
  index: number,
  t: MessageTranslator,
): StudioNode {
  const title = video.prompt || t("video.canvas.imported.video", { id: video.id });
  return importedInputNode(asset.id, index, "imported-video", {
    label: title,
    nodeKind: "VideoInputNode",
    refType: CANVAS_REF_TYPE_GENERATED_VIDEO,
    refId: video.id,
    state: video.status || "ready",
    config: {
      title,
      file_path: video.file_path,
      thumbnail_path: video.thumbnail_path,
      generated_video_id: video.id,
      media_asset_id: asset.id,
      width: video.width,
      height: video.height,
      duration_secs: video.duration_secs,
    },
  });
}

export function importedAudioInputNode(
  asset: CanvasMediaAssetSummary,
  index: number,
  t: MessageTranslator,
): StudioNode {
  const title = asset.file_path?.split("/").pop() || t("video.canvas.imported.audio", { id: asset.id });
  return importedInputNode(asset.id, index, "imported-audio", {
    label: title,
    nodeKind: "AudioInputNode",
    refType: CANVAS_REF_TYPE_MEDIA_ASSET,
    refId: asset.id,
    state: asset.status || "ready",
    config: {
      title,
      file_path: asset.file_path,
    },
  });
}
