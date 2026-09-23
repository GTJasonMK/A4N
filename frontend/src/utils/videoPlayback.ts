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

export function videoPlaybackDataToDataUrl(data: VideoBlobData): string {
  if (!/^video\/[a-z0-9.+-]+$/i.test(data.mime_type) || !data.data_base64) {
    throw new Error("Invalid video playback data");
  }
  // Preserve Base64 validation without creating a second byte array and Blob.
  globalThis.atob(data.data_base64);
  // WebKitGTK's Blob pipeline can lose MP4 headers when rebuilding after EOS.
  // The API already supplies inline Base64; a data URL uses its regular player.
  return `data:${data.mime_type};base64,${data.data_base64}`;
}
