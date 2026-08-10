import type { VideoPlaybackData } from "../api/video";

export function base64ToBytes(dataBase64: string): Uint8Array {
  const binary = window.atob(dataBase64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

type VideoBlobData = Pick<VideoPlaybackData, "data_base64" | "mime_type">;

export function videoPlaybackDataToBlob(data: VideoBlobData): Blob {
  const bytes = base64ToBytes(data.data_base64);
  const buffer = bytes.buffer.slice(
    bytes.byteOffset,
    bytes.byteOffset + bytes.byteLength,
  ) as ArrayBuffer;
  return new Blob([buffer], { type: data.mime_type || "video/mp4" });
}

export function videoPlaybackDataToObjectUrl(data: VideoBlobData): string {
  return URL.createObjectURL(videoPlaybackDataToBlob(data));
}
