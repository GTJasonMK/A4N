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
};

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
  if (!chapterScoped) return imageType ? `type:${imageType}` : `image:${img.id}`;
  if (img.panel_id?.trim()) return `panel:${img.panel_id.trim()}`;
  const pageNumber = getImagePageNumber(img.metadata_json);
  if (pageNumber != null) return `panel:page${pageNumber}`;
  return imageType ? `type:${imageType}` : `image:${img.id}`;
}

export function getImageGroupTitle(key: string, count: number): string {
  if (key.startsWith("panel:page")) {
    const page = key.slice("panel:page".length);
    return `第 ${page} 页 · ${count} 张`;
  }
  if (key.startsWith("panel:")) return `${key.slice("panel:".length)} · ${count} 张`;
  if (key.startsWith("image:")) return `图片 #${key.slice("image:".length)} · ${count} 张`;
  const type = key.slice("type:".length);
  return `${IMAGE_TYPE_LABELS[type] || type} · ${count} 张`;
}

function groupSortKey(key: string): [number, number, string] {
  if (key.startsWith("panel:page")) {
    const page = Number(key.slice("panel:page".length));
    return [0, Number.isFinite(page) ? page : 999999, key];
  }
  if (key.startsWith("panel:")) return [1, 0, key];
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
