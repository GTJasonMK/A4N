import type { VideoPlaybackData } from "../api/video";
import { base64ToBytes } from "./base64";

type VideoBlobData = Pick<VideoPlaybackData, "data_base64" | "mime_type">;

export function videoPlaybackDataToBlob(data: VideoBlobData): Blob {
  const bytes = base64ToBytes(data.data_base64);
  const buffer = bytes.buffer.slice(
    bytes.byteOffset,
    bytes.byteOffset + bytes.byteLength,
  ) as ArrayBuffer;
  return new Blob([buffer], { type: data.mime_type });
}

export function videoPlaybackDataToObjectUrl(data: VideoBlobData): string {
  return URL.createObjectURL(videoPlaybackDataToBlob(data));
}
