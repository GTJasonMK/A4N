import { IMAGE_UPLOAD_MIME_TYPES } from "../api/generated/config";

export const MAX_UPLOAD_IMAGE_BYTES = 50 * 1024 * 1024;

export function supportedImageAccept(): string {
  return IMAGE_UPLOAD_MIME_TYPES.join(",");
}

export function isSupportedImageMime(mime: string): boolean {
  return IMAGE_UPLOAD_MIME_TYPES.includes(mime.trim().toLowerCase());
}
