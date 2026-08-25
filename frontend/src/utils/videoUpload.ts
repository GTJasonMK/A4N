import { pickLocalFile } from "./fileUpload";
import type { MessageTranslator } from "../i18n";

const MAX_UPLOAD_VIDEO_BYTES = 512 * 1024 * 1024;

export interface VideoUploadFile {
  file: File;
  mimeType: string;
}

const VIDEO_MIME_BY_EXTENSION: Record<string, string[]> = {
  mp4: ["video/mp4", "application/mp4"],
  webm: ["video/webm"],
  mov: ["video/quicktime", "video/mov"],
  m4v: ["video/x-m4v", "video/mp4", "application/mp4"],
};

function supportedVideoExtensions(): string[] {
  return Object.keys(VIDEO_MIME_BY_EXTENSION);
}

function supportedVideoAccept(): string {
  return supportedVideoExtensions().map((ext) => `.${ext}`).join(",");
}

function videoMimeForUploadFile(file: File): string | null {
  const extension = uploadVideoExtension(file.name);
  if (!extension) return null;
  const candidates = VIDEO_MIME_BY_EXTENSION[extension];
  const actualMime = file.type.trim().toLowerCase();
  if (!actualMime) return candidates[0] ?? null;
  return candidates.includes(actualMime) ? actualMime : null;
}

export async function pickVideoUploadFile(
  t: MessageTranslator,
  sizeLabel?: string,
): Promise<VideoUploadFile | null> {
  const file = await pickLocalFile(supportedVideoAccept());
  if (!file) return null;
  const mimeType = videoMimeForUploadFile(file);
  if (!mimeType) {
    throw new Error(t("video.upload.invalidType"));
  }
  if (file.size > MAX_UPLOAD_VIDEO_BYTES) {
    throw new Error(t("video.upload.tooLarge", {
      name: sizeLabel ?? t("video.upload.video"),
      size: MAX_UPLOAD_VIDEO_BYTES / 1024 / 1024,
    }));
  }
  return { file, mimeType };
}

function uploadVideoExtension(fileName: string): string | null {
  const match = /\.([^.]+)$/.exec(fileName.trim());
  if (!match) return null;
  const extension = match[1].toLowerCase();
  return extension in VIDEO_MIME_BY_EXTENSION ? extension : null;
}
