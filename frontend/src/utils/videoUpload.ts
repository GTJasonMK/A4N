import { pickLocalFile } from "./fileUpload";

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
  sizeLabel = "上传视频",
): Promise<VideoUploadFile | null> {
  const file = await pickLocalFile(supportedVideoAccept());
  if (!file) return null;
  const mimeType = videoMimeForUploadFile(file);
  if (!mimeType) {
    throw new Error("只支持 MP4/WebM/MOV/M4V 视频，且文件类型必须与扩展名一致");
  }
  if (file.size > MAX_UPLOAD_VIDEO_BYTES) {
    throw new Error(`${sizeLabel}不能超过 ${MAX_UPLOAD_VIDEO_BYTES / 1024 / 1024}MB`);
  }
  return { file, mimeType };
}

function uploadVideoExtension(fileName: string): string | null {
  const match = /\.([^.]+)$/.exec(fileName.trim());
  if (!match) return null;
  const extension = match[1].toLowerCase();
  return extension in VIDEO_MIME_BY_EXTENSION ? extension : null;
}
