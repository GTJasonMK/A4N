import { VIDEO_UPLOAD_MIME_TYPES } from "../api/generated/config";
import { pickLocalFile } from "./fileUpload";
import type { MessageTranslator } from "../i18n";

const MAX_UPLOAD_VIDEO_BYTES = 512 * 1024 * 1024;

export interface VideoUploadFile {
  file: File;
  mimeType: string;
}

function videoMimeForUploadFile(file: File): string | null {
  const extension = uploadVideoExtension(file.name);
  if (!extension) return null;
  const candidates = VIDEO_UPLOAD_MIME_TYPES[extension];
  const actualMime = file.type.trim().toLowerCase();
  if (!actualMime) return candidates[0];
  return candidates.includes(actualMime) ? actualMime : null;
}

export async function pickVideoUploadFile(
  t: MessageTranslator,
  sizeLabel?: string,
): Promise<VideoUploadFile | null> {
  const accept = Object.keys(VIDEO_UPLOAD_MIME_TYPES)
    .map((extension) => `.${extension}`)
    .join(",");
  const file = await pickLocalFile(accept);
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
  return Object.prototype.hasOwnProperty.call(VIDEO_UPLOAD_MIME_TYPES, extension)
    ? extension
    : null;
}
