import type { MediaAssetRecord } from "../../api/canvas";
import type { GeneratedImage } from "../../api/image/types";
import type { GeneratedVideo } from "../../api/video/types";
import { canvasNodePosition } from "./canvasGraphMapping";
import type { StudioNode } from "./types";

export function importedImageInputNode(image: GeneratedImage, asset: MediaAssetRecord, index: number): StudioNode {
  return {
    id: `imported-image-${asset.id}-${Date.now()}`,
    type: "studioNode",
    selected: true,
    position: canvasNodePosition(index),
    data: {
      label: image.prompt || `图片 #${image.id}`,
      nodeKind: "ImageInputNode",
      refType: "generated_image",
      refId: image.id,
      state: "ready",
      config: {
        title: image.prompt || `图片 #${image.id}`,
        file_path: image.file_path,
        image_type: image.image_type,
        generated_image_id: image.id,
        media_asset_id: asset.id,
        width: image.width,
        height: image.height,
      },
    },
  };
}

export function importedVideoInputNode(video: GeneratedVideo, asset: MediaAssetRecord, index: number): StudioNode {
  return {
    id: `imported-video-${asset.id}-${Date.now()}`,
    type: "studioNode",
    selected: true,
    position: canvasNodePosition(index),
    data: {
      label: video.prompt || `视频 #${video.id}`,
      nodeKind: "VideoInputNode",
      refType: "generated_video",
      refId: video.id,
      state: video.status || "ready",
      config: {
        title: video.prompt || `视频 #${video.id}`,
        file_path: video.file_path,
        thumbnail_path: video.thumbnail_path,
        video_type: video.video_type,
        workflow_mode: video.workflow_mode,
        generated_video_id: video.id,
        media_asset_id: asset.id,
        width: video.width,
        height: video.height,
        duration_secs: video.duration_secs,
      },
    },
  };
}

export function importedAudioInputNode(asset: MediaAssetRecord, index: number): StudioNode {
  const title = asset.file_path?.split("/").pop() || `音频 #${asset.id}`;
  return {
    id: `imported-audio-${asset.id}-${Date.now()}`,
    type: "studioNode",
    selected: true,
    position: canvasNodePosition(index),
    data: {
      label: title,
      nodeKind: "AudioInputNode",
      refType: "media_asset",
      refId: asset.id,
      state: asset.status || "ready",
      config: {
        title,
        file_path: asset.file_path,
        media_kind: asset.media_kind,
        origin: asset.origin,
        file_size: asset.file_size,
      },
    },
  };
}
