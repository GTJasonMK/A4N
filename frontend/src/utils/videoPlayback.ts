import type { VideoPlaybackData } from "../api/video";

export function videoPlaybackDataToBlob(data: VideoPlaybackData): Blob {
  const binary = window.atob(data.data_base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  const buffer = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
  return new Blob([buffer], { type: data.mime_type || "video/mp4" });
}

export function videoPlaybackDataToObjectUrl(data: VideoPlaybackData): string {
  return URL.createObjectURL(videoPlaybackDataToBlob(data));
}
