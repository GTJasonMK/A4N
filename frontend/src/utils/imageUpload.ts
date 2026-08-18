export const MAX_UPLOAD_IMAGE_BYTES = 50 * 1024 * 1024;

const SUPPORTED_IMAGE_MIME_TYPES = [
  "image/png",
  "image/jpeg",
  "image/webp",
] as const;

export function supportedImageAccept(): string {
  return SUPPORTED_IMAGE_MIME_TYPES.join(",");
}

export function isSupportedImageMime(mime: string): boolean {
  return (SUPPORTED_IMAGE_MIME_TYPES as readonly string[]).includes(mime.trim().toLowerCase());
}
