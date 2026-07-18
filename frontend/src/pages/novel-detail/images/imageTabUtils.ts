import type { GeneratedImage } from "../../../api/image";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import type { ChapterRow } from "../shared";

export type ImageScope = "chapter" | "project";

export const CHAPTER_SCOPE_OPTIONS: Array<{ id: ImageScope; label: string }> = [
  { id: "chapter", label: "本章图像" },
  { id: "project", label: "项目图库" },
];

const IMAGE_TYPE_LABELS: Record<string, string> = {
  scene: "场景",
  manga_page: "整页漫画",
  portrait: "立绘",
  cover: "封面",
  illustration: "插图",
  scene_material: "场景素材",
  object_material: "物品素材",
  video_key_frame: "视频关键帧",
  video_start_frame: "视频首帧",
  video_storyboard_frame: "视频分镜板",
  video_end_frame: "视频尾帧",
  video_tail_frame: "视频续接帧",
  canvas_image: "画布图片",
  canvas_frame: "画布帧",
  portrait_reference: "立绘参考",
  chapter_reference: "章节参考",
  project_reference: "项目参考",
  galgame_background: "Galgame 背景",
  galgame_cg: "Galgame CG",
};

export function imageTypeLabel(value: string | null | undefined): string {
  const normalized = String(value ?? "").trim();
  return normalized ? IMAGE_TYPE_LABELS[normalized] ?? "其他素材" : "其他素材";
}

function numberOrNull(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

function getImageSource(metadata: unknown): Record<string, unknown> | null {
  const source = asRecord(metadata)?.source;
  return asRecord(source);
}

export function getImageSourceHash(metadata: unknown): string | null {
  const hash = getImageSource(metadata)?.content_sha256;
  return typeof hash === "string" && hash.trim() ? hash.trim() : null;
}

function getImagePageNumber(metadata: unknown): number | null {
  const meta = asRecord(metadata);
  return numberOrNull(meta?.page_number);
}

function getImageType(img: GeneratedImage): string | null {
  if (typeof img.image_type !== "string") return null;
  const imageType = img.image_type.trim();
  return imageType ? imageType : null;
}

export function getImageGroupKey(img: GeneratedImage, chapterScoped: boolean): string {
  const imageType = getImageType(img);
  if (!chapterScoped) return `type:${imageType ?? "other"}`;

  const panelId = img.panel_id?.trim() ?? "";
  const pageMatch = /^page[-_ ]?(\d+)$/i.exec(panelId);
  if (pageMatch) return `page:${pageMatch[1]}`;
  const panelMatch = /^panel[-_ ]?(\d+)$/i.exec(panelId);
  if (panelMatch) return `panel:${panelMatch[1]}`;

  const pageNumber = getImagePageNumber(img.metadata_json);
  if (pageNumber != null) return `page:${pageNumber}`;
  return `type:${imageType ?? "other"}`;
}

export function getImageGroupTitle(key: string, count: number): string {
  if (key.startsWith("page:")) return `第 ${key.slice("page:".length)} 页 · ${count} 张`;
  if (key.startsWith("panel:")) return `第 ${key.slice("panel:".length)} 格 · ${count} 张`;
  const type = key.slice("type:".length);
  return `${imageTypeLabel(type)} · ${count} 张`;
}

function groupSortKey(key: string): [number, number, string] {
  if (key.startsWith("page:")) {
    const page = Number(key.slice("page:".length));
    return [0, Number.isFinite(page) ? page : 999999, key];
  }
  if (key.startsWith("panel:")) {
    const panel = Number(key.slice("panel:".length));
    return [1, Number.isFinite(panel) ? panel : 999999, key];
  }
  return [2, 0, key];
}

export function compareGroupKey(a: string, b: string): number {
  const ak = groupSortKey(a);
  const bk = groupSortKey(b);
  return ak[0] - bk[0] || ak[1] - bk[1] || ak[2].localeCompare(bk[2]);
}

export function isImageStale(
  img: GeneratedImage,
  chapter: ChapterRow | null,
  currentContentHash: string | null,
): boolean {
  if (!chapter || img.chapter_id !== chapter.id) return false;
  const sourceHash = getImageSourceHash(img.metadata_json);
  return Boolean(sourceHash && currentContentHash && sourceHash !== currentContentHash);
}
