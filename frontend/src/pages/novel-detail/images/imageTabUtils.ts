import {
  IMAGE_TYPE_CANVAS,
  IMAGE_TYPE_CANVAS_FRAME,
  IMAGE_TYPE_OBJECT_MATERIAL,
  IMAGE_TYPE_SCENE_MATERIAL,
  IMAGE_TYPE_VIDEO_KEY_FRAME,
  IMAGE_TYPE_VIDEO_START_FRAME,
  IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
  IMAGE_TYPE_VIDEO_TAIL_FRAME,
  type GeneratedImage,
} from "../../../api/image";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import type { ChapterRow } from "../shared";
import type { MessageKey, MessageTranslator } from "../../../i18n";

export type ImageScope = "chapter" | "project";

export const CHAPTER_SCOPE_OPTIONS: Array<{ id: ImageScope; labelKey: MessageKey }> = [
  { id: "chapter", labelKey: "novel.imageLibrary.chapterScope" },
  { id: "project", labelKey: "novel.imageLibrary.projectScope" },
];

const IMAGE_TYPE_LABEL_KEYS: Record<string, MessageKey> = {
  scene: "novel.imageLibrary.scene",
  manga_page: "novel.imageLibrary.mangaPage",
  portrait: "novel.imageLibrary.portrait",
  cover: "novel.imageLibrary.cover",
  illustration: "novel.imageLibrary.illustration",
  [IMAGE_TYPE_SCENE_MATERIAL]: "novel.imageLibrary.sceneMaterial",
  [IMAGE_TYPE_OBJECT_MATERIAL]: "novel.imageLibrary.objectMaterial",
  [IMAGE_TYPE_VIDEO_KEY_FRAME]: "novel.imageLibrary.videoKeyFrame",
  [IMAGE_TYPE_VIDEO_START_FRAME]: "novel.imageLibrary.videoStartFrame",
  [IMAGE_TYPE_VIDEO_STORYBOARD_FRAME]: "novel.imageLibrary.videoStoryboardFrame",
  [IMAGE_TYPE_VIDEO_TAIL_FRAME]: "novel.imageLibrary.videoTailFrame",
  [IMAGE_TYPE_CANVAS]: "novel.imageLibrary.canvasImage",
  [IMAGE_TYPE_CANVAS_FRAME]: "novel.imageLibrary.canvasFrame",
  portrait_reference: "novel.imageLibrary.portraitReference",
  chapter_reference: "novel.imageLibrary.chapterReference",
  project_reference: "novel.imageLibrary.projectReference",
  galgame_background: "novel.imageLibrary.galgameBackground",
  galgame_cg: "novel.imageLibrary.otherMaterial",
};

export function imageTypeLabel(
  value: string | null | undefined,
  t: MessageTranslator,
): string {
  const normalized = String(value ?? "").trim();
  return normalized && IMAGE_TYPE_LABEL_KEYS[normalized]
    ? t(IMAGE_TYPE_LABEL_KEYS[normalized])
    : t("novel.imageLibrary.otherMaterial");
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

function getImageSourceHash(metadata: unknown): string | null {
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

export function getImageGroupTitle(
  key: string,
  count: number,
  t: MessageTranslator,
): string {
  if (key.startsWith("page:")) {
    return t("novel.imageLibrary.pageGroup", {
      number: key.slice("page:".length),
      count,
    });
  }
  if (key.startsWith("panel:")) {
    return t("novel.imageLibrary.panelGroup", {
      number: key.slice("panel:".length),
      count,
    });
  }
  const type = key.slice("type:".length);
  return t("novel.imageLibrary.group", { type: imageTypeLabel(type, t), count });
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
