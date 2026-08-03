import React, { useMemo, useState } from "react";
import { Check, Eye, Images, Plus, Trash2, Upload } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../../api";
import type { GeneratedImage } from "../../../api/image";
import type {
  SegmentReferenceMediaRecipe,
  VideoGenConfig,
} from "../../../api/video";
import type { ReferenceMediaUploadInput } from "../../../api/generated/generationContracts";
import { Dropdown } from "../../../components/ui/Dropdown";
import { Modal } from "../../../components/ui/Modal";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { readFileAsBase64 } from "../../../utils/fileUpload";
import { imageTypeLabel } from "../images/imageTabUtils";
import type { ChapterRow } from "../shared";
import {
  durationLimitsLabel,
  isChapterVideoMaterialImage,
  isDurationAllowed,
  isProjectReusableVideoMaterialImage,
  MAX_CHAT_REFERENCE_IMAGES,
  MAX_REFERENCE_IMAGES,
  MAX_VIDEO_LIBRARY_IMAGES,
  referenceImageScopeLabel,
  type ChapterVideoWorkflowMode,
  type VideoAspectRatio,
  type VideoDurationLimits,
} from "./videoTabUtils";

const REFERENCE_MEDIA_PROMPT_MIN_CHARS = 100;
export const REFERENCE_MEDIA_LIMITS = {
  image: 9,
  video: 3,
  audio: 3,
} as const;

export type ReferenceMediaKind = keyof typeof REFERENCE_MEDIA_LIMITS;
export type ReferenceMediaInputPurpose = "generate" | "preview";
type ReferenceMediaRowsByKind = Record<
  ReferenceMediaKind,
  ReferenceMediaBindingRow[]
>;

export interface ReferenceMediaBindingRow {
  id: string;
  url: string;
  usage: string;
  upload?: ReferenceMediaUploadInput;
  uploadSha256?: string;
  uploadFileName?: string;
  uploadMimeType?: string;
}

export interface ReferenceMediaDraft {
  prompt: string;
  imageRows: ReferenceMediaBindingRow[];
  videoRows: ReferenceMediaBindingRow[];
  audioRows: ReferenceMediaBindingRow[];
}

export const REFERENCE_MEDIA_KIND_META: Record<
  ReferenceMediaKind,
  {
    label: string;
    field: "image_files" | "video_files" | "audio_files";
    tokenPrefix: string;
  }
> = {
  image: { label: "图片", field: "image_files", tokenPrefix: "image_file" },
  video: { label: "视频", field: "video_files", tokenPrefix: "video_file" },
  audio: { label: "音频", field: "audio_files", tokenPrefix: "audio_file" },
};

const REFERENCE_MEDIA_KINDS: ReferenceMediaKind[] = ["image", "video", "audio"];
const REFERENCE_MEDIA_ACCEPT: Record<ReferenceMediaKind, string> = {
  image: "image/png,image/jpeg,image/webp",
  video: "video/mp4,video/quicktime,video/webm",
  audio: "audio/mpeg,audio/wav,audio/x-wav,audio/mp4,audio/x-m4a,audio/ogg",
};
const MAX_REFERENCE_MEDIA_UPLOAD_BYTES = 20 * 1024 * 1024;

let referenceMediaRowSeq = 0;

const createReferenceMediaBindingRow = (): ReferenceMediaBindingRow => {
  referenceMediaRowSeq += 1;
  return {
    id: `reference-media-${Date.now()}-${referenceMediaRowSeq}`,
    url: "",
    usage: "",
  };
};

export const createReferenceMediaDraft = (): ReferenceMediaDraft => ({
  prompt: "",
  imageRows: [createReferenceMediaBindingRow()],
  videoRows: [createReferenceMediaBindingRow()],
  audioRows: [createReferenceMediaBindingRow()],
});

export const referenceMediaDraftScopeKey = (
  planIdentity: string | null | undefined,
  segmentNumber: number | null | undefined,
  sourceShotId: unknown,
  segmentIndexInShot: unknown,
): string | null => {
  const normalizedPlanIdentity = planIdentity?.trim();
  const normalizedSourceShotId =
    typeof sourceShotId === "string" ? sourceShotId.trim() : "";
  if (
    !normalizedPlanIdentity ||
    typeof segmentNumber !== "number" ||
    !Number.isInteger(segmentNumber) ||
    segmentNumber <= 0 ||
    !normalizedSourceShotId ||
    typeof segmentIndexInShot !== "number" ||
    !Number.isInteger(segmentIndexInShot) ||
    segmentIndexInShot <= 0
  ) {
    return null;
  }
  return JSON.stringify([
    normalizedPlanIdentity,
    segmentNumber,
    normalizedSourceShotId,
    segmentIndexInShot,
  ]);
};

export const referenceMediaDraftFromRecipe = (
  recipe: SegmentReferenceMediaRecipe | null | undefined,
): ReferenceMediaDraft => {
  const rows = (
    items: SegmentReferenceMediaRecipe["image_rows"] | undefined,
  ) =>
    items && items.length > 0
      ? items.map((item) => ({
          ...createReferenceMediaBindingRow(),
          url: item.url,
          usage: item.usage,
          uploadSha256: item.upload_sha256 ?? undefined,
          uploadFileName: item.upload_file_name ?? undefined,
          uploadMimeType: item.upload_mime_type ?? undefined,
        }))
      : [createReferenceMediaBindingRow()];
  return {
    prompt: recipe?.prompt ?? "",
    imageRows: rows(recipe?.image_rows),
    videoRows: rows(recipe?.video_rows),
    audioRows: rows(recipe?.audio_rows),
  };
};

export const referenceMediaRecipeFromDraft = (
  draft: ReferenceMediaDraft,
): SegmentReferenceMediaRecipe => {
  const rows = (items: ReferenceMediaBindingRow[]) =>
    items.flatMap((item) => {
      const url = item.url.trim();
      const usage = item.usage.trim();
      return referenceMediaRowActive(item)
        ? [
            {
              url,
              usage,
              upload_sha256: url ? null : (item.uploadSha256 ?? null),
              upload_file_name: url ? null : (item.uploadFileName ?? null),
              upload_mime_type: url ? null : (item.uploadMimeType ?? null),
            },
          ]
        : [];
    });
  return {
    prompt: draft.prompt,
    image_rows: rows(draft.imageRows),
    video_rows: rows(draft.videoRows),
    audio_rows: rows(draft.audioRows),
  };
};

export const referenceMediaRowsByKindFromDraft = (
  draft: ReferenceMediaDraft,
): ReferenceMediaRowsByKind => ({
  image: draft.imageRows,
  video: draft.videoRows,
  audio: draft.audioRows,
});

const referenceMediaRowActive = (row: ReferenceMediaBindingRow): boolean =>
  Boolean(
    row.url.trim() ||
    row.upload ||
    row.uploadSha256?.trim() ||
    row.uploadFileName?.trim() ||
    row.uploadMimeType?.trim() ||
    row.usage.trim(),
  );

const activeReferenceMediaRows = (
  rows: ReferenceMediaBindingRow[],
): ReferenceMediaBindingRow[] => rows.filter(referenceMediaRowActive);

export const normalizeReferenceMediaRows = (
  rows: ReferenceMediaBindingRow[],
): ReferenceMediaBindingRow[] =>
  activeReferenceMediaRows(rows).map((row) => ({
    ...row,
    url: row.url.trim(),
    usage: row.usage.trim(),
  }));

const normalizeReferenceMediaRowsByKind = (
  rowsByKind: ReferenceMediaRowsByKind,
): ReferenceMediaRowsByKind => ({
  image: normalizeReferenceMediaRows(rowsByKind.image),
  video: normalizeReferenceMediaRows(rowsByKind.video),
  audio: normalizeReferenceMediaRows(rowsByKind.audio),
});

export const validReferenceMediaUrl = (url: string): boolean =>
  url.startsWith("https://") || url.startsWith("http://");

const buildReferenceMediaBindingLines = (
  kind: ReferenceMediaKind,
  rows: ReferenceMediaBindingRow[],
): string[] => {
  const meta = REFERENCE_MEDIA_KIND_META[kind];
  return rows.flatMap((row, index) => {
    if (!row.usage) return [];
    const token = `@${meta.tokenPrefix}_${index + 1}`;
    return [`- ${token}：${row.usage}`];
  });
};

const buildReferenceMediaPrompt = (
  basePrompt: string,
  rowsByKind: ReferenceMediaRowsByKind,
): string => {
  const bindingLines = REFERENCE_MEDIA_KINDS.flatMap((kind) =>
    buildReferenceMediaBindingLines(kind, rowsByKind[kind]),
  );
  return [
    basePrompt.trim(),
    bindingLines.length > 0 ? `参考媒体绑定：\n${bindingLines.join("\n")}` : "",
  ]
    .filter(Boolean)
    .join("\n\n")
    .trim();
};

export const referenceMediaGenerateDraftError = (
  prompt: string,
  rowsByKind: ReferenceMediaRowsByKind,
): string | null => {
  const normalizedRowsByKind = normalizeReferenceMediaRowsByKind(rowsByKind);
  for (const kind of REFERENCE_MEDIA_KINDS) {
    const meta = REFERENCE_MEDIA_KIND_META[kind];
    const rows = normalizedRowsByKind[kind];
    if (rows.length > REFERENCE_MEDIA_LIMITS[kind]) {
      return `${meta.label}最多允许 ${REFERENCE_MEDIA_LIMITS[kind]} 个 URL`;
    }
    for (const [index, row] of rows.entries()) {
      const token = `@${meta.tokenPrefix}_${index + 1}`;
      if (row.url && row.upload) return `${token} 不能同时使用 URL 和本地文件`;
      if (!row.url && !row.upload) return `${token} 缺少 URL 或本地文件`;
      if (row.url && !validReferenceMediaUrl(row.url))
        return `${token} URL 必须以小写 http:// 或 https:// 开头`;
      if (!row.usage) return `${token} 缺少用途描述`;
    }
  }
  const totalMediaCount = REFERENCE_MEDIA_KINDS.reduce(
    (count, kind) => count + normalizedRowsByKind[kind].length,
    0,
  );
  if (totalMediaCount === 0) {
    return "至少添加一个图片、视频或音频 URL 或本地文件";
  }
  const builtPrompt = buildReferenceMediaPrompt(prompt, normalizedRowsByKind);
  if ([...builtPrompt].length < REFERENCE_MEDIA_PROMPT_MIN_CHARS) {
    return `参考媒体 prompt 至少需要 ${REFERENCE_MEDIA_PROMPT_MIN_CHARS} 个字符，请补全画面、动作、风格和媒体用途`;
  }
  return null;
};

const assertImageIdSelection = (ids: number[], field: string) => {
  const seen = new Set<number>();
  for (const [index, id] of ids.entries()) {
    if (!Number.isInteger(id) || id <= 0) {
      throw new Error(`${field}[${index}] 必须是正整数`);
    }
    if (seen.has(id)) {
      throw new Error(`${field} 包含重复图片 ID：${id}`);
    }
    seen.add(id);
  }
};

const toggleImageIdSelection = (
  ids: number[],
  id: number,
  field: string,
): number[] => {
  if (!Number.isInteger(id) || id <= 0) {
    throw new Error(`${field} 只能切换正整数图片 ID`);
  }
  assertImageIdSelection(ids, field);
  return ids.includes(id) ? ids.filter((value) => value !== id) : [...ids, id];
};

type ReferenceImageFilter =
  | "all"
  | "selected"
  | "library"
  | "generation"
  | "chapter"
  | "project"
  | "profile_material";

export const VideoReferenceInputSlots: React.FC<{
  images: GeneratedImage[];
  chapter: ChapterRow;
  workflowMode: ChapterVideoWorkflowMode;
  currentContentHash: string | null;
  generationReferenceIds: number[];
  videoLibraryImageIds: number[];
  invalidGenerationReferenceIds: number[];
  invalidVideoLibraryImageIds: number[];
  disabled: boolean;
  onGenerationReferenceIdsChange: (ids: number[]) => unknown | Promise<unknown>;
  onVideoLibraryImageIdsChange: (ids: number[]) => unknown | Promise<unknown>;
  onReferenceLimit: () => void;
  onSelectionError: (message: string) => void;
  onClearInvalidGenerationReferences: () => void;
  onClearInvalidVideoLibraryImages: () => void;
}> = ({
  images,
  chapter,
  workflowMode,
  currentContentHash,
  generationReferenceIds,
  videoLibraryImageIds,
  invalidGenerationReferenceIds,
  invalidVideoLibraryImageIds,
  disabled,
  onGenerationReferenceIdsChange,
  onVideoLibraryImageIdsChange,
  onReferenceLimit,
  onSelectionError,
  onClearInvalidGenerationReferences,
  onClearInvalidVideoLibraryImages,
}) => {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState<ReferenceImageFilter>("all");
  const chatVideoMode = workflowMode === "chat_video";
  const generationReferenceLabel = chatVideoMode ? "Chat上传" : "生成参考";
  const generationReferenceLimit = chatVideoMode
    ? MAX_CHAT_REFERENCE_IMAGES
    : MAX_REFERENCE_IMAGES;
  const generationReferenceCountLabel = chatVideoMode
    ? `${generationReferenceIds.length}/${MAX_CHAT_REFERENCE_IMAGES}`
    : `${generationReferenceIds.length}/${MAX_REFERENCE_IMAGES}`;
  const generationIdSet = useMemo(
    () => new Set(generationReferenceIds),
    [generationReferenceIds],
  );
  const explicitLibraryIdSet = useMemo(
    () => new Set(videoLibraryImageIds),
    [videoLibraryImageIds],
  );
  const libraryActiveCount = useMemo(
    () =>
      images.filter(
        (image) =>
          explicitLibraryIdSet.has(image.id) ||
          isChapterVideoMaterialImage(image, chapter),
      ).length,
    [chapter, explicitLibraryIdSet, images],
  );
  const filterCounts = useMemo(() => {
    const counts: Record<ReferenceImageFilter, number> = {
      all: images.length,
      selected: 0,
      library: 0,
      generation: 0,
      chapter: 0,
      project: 0,
      profile_material: 0,
    };
    for (const image of images) {
      const generationActive = generationIdSet.has(image.id);
      const libraryActive =
        explicitLibraryIdSet.has(image.id) ||
        isChapterVideoMaterialImage(image, chapter);
      if (generationActive || libraryActive) counts.selected += 1;
      if (libraryActive) counts.library += 1;
      if (generationActive) counts.generation += 1;
      if (image.chapter_id === chapter.id) counts.chapter += 1;
      if (image.chapter_id == null) counts.project += 1;
      if (isProjectReusableVideoMaterialImage(image))
        counts.profile_material += 1;
    }
    return counts;
  }, [chapter, explicitLibraryIdSet, generationIdSet, images]);
  const filterOptions = useMemo(
    () =>
      [
        { id: "all", label: "全部", count: filterCounts.all },
        { id: "selected", label: "已选", count: filterCounts.selected },
        { id: "library", label: "素材库", count: filterCounts.library },
        {
          id: "generation",
          label: generationReferenceLabel,
          count: filterCounts.generation,
        },
        { id: "chapter", label: "本章", count: filterCounts.chapter },
        { id: "project", label: "项目", count: filterCounts.project },
        {
          id: "profile_material",
          label: "档案素材",
          count: filterCounts.profile_material,
        },
      ] satisfies Array<{
        id: ReferenceImageFilter;
        label: string;
        count: number;
      }>,
    [filterCounts, generationReferenceLabel],
  );
  const filteredImages = useMemo(
    () =>
      images.filter((image) => {
        const generationActive = generationIdSet.has(image.id);
        const libraryActive =
          explicitLibraryIdSet.has(image.id) ||
          isChapterVideoMaterialImage(image, chapter);
        if (filter === "selected") return generationActive || libraryActive;
        if (filter === "library") return libraryActive;
        if (filter === "generation") return generationActive;
        if (filter === "chapter") return image.chapter_id === chapter.id;
        if (filter === "project") return image.chapter_id == null;
        if (filter === "profile_material")
          return isProjectReusableVideoMaterialImage(image);
        return true;
      }),
    [chapter, explicitLibraryIdSet, filter, generationIdSet, images],
  );

  const toggleGenerationReference = (imageId: number) => {
    if (disabled) return;
    try {
      if (
        !generationIdSet.has(imageId) &&
        generationReferenceIds.length >= generationReferenceLimit
      ) {
        onReferenceLimit();
        return;
      }
      void onGenerationReferenceIdsChange(
        toggleImageIdSelection(
          generationReferenceIds,
          imageId,
          "生成参考图片选择",
        ),
      );
    } catch (err) {
      onSelectionError(extractErrorMessage(err, ""));
    }
  };

  const toggleVideoLibraryImage = (image: GeneratedImage) => {
    if (disabled) return;
    if (isChapterVideoMaterialImage(image, chapter)) return;
    if (
      !explicitLibraryIdSet.has(image.id) &&
      videoLibraryImageIds.length >= MAX_VIDEO_LIBRARY_IMAGES
    ) {
      onSelectionError(`视频素材库最多只能选择 ${MAX_VIDEO_LIBRARY_IMAGES} 张`);
      return;
    }
    try {
      void onVideoLibraryImageIdsChange(
        toggleImageIdSelection(
          videoLibraryImageIds,
          image.id,
          "视频素材库图片选择",
        ),
      );
    } catch (err) {
      onSelectionError(extractErrorMessage(err, ""));
    }
  };
  const hasInvalidSelection =
    invalidGenerationReferenceIds.length > 0 ||
    invalidVideoLibraryImageIds.length > 0;
  const selectionSummary = [
    libraryActiveCount > 0 ? `素材库 ${libraryActiveCount}` : null,
    generationReferenceIds.length > 0
      ? `${chatVideoMode ? "上传" : "参考"} ${generationReferenceCountLabel}`
      : null,
  ].filter((item): item is string => item != null);
  const imageById = useMemo(
    () => new Map(images.map((image) => [image.id, image])),
    [images],
  );
  const selectedIds = useMemo(
    () => [...new Set([...generationReferenceIds, ...videoLibraryImageIds])],
    [generationReferenceIds, videoLibraryImageIds],
  );
  const canAdd =
    generationReferenceIds.length < generationReferenceLimit ||
    videoLibraryImageIds.length < MAX_VIDEO_LIBRARY_IMAGES;

  return (
    <>
      <section
        aria-label="片段参考图和素材"
        className="a4n-flat-surface p-3 sm:p-4"
      >
        <div className="mb-3 flex min-w-0 items-start gap-2">
          <Images size={14} className="mt-0.5 shrink-0 text-book-primary" />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-xs font-medium text-book-text-main">
                参考图 / 素材
              </h3>
              <span className="rounded-full bg-book-bg-tertiary px-2 py-0.5 text-caption text-book-text-muted">
                可选
              </span>
            </div>
            <div className="mt-0.5 text-caption text-book-text-muted">
              已选素材会参与提示词或视频请求；按需点击加号继续添加。
              {selectionSummary.length > 0
                ? ` 当前：${selectionSummary.join(" · ")}。`
                : ""}
            </div>
          </div>
          {hasInvalidSelection ? (
            <span className="shrink-0 text-caption text-error">需处理</span>
          ) : null}
        </div>

        <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
          {selectedIds.map((imageId) => {
            const image = imageById.get(imageId);
            const src = image ? toMediaSrc(image.file_path) : "";
            const roles = [
              generationIdSet.has(imageId)
                ? chatVideoMode
                  ? "Chat 参考"
                  : "生成参考"
                : null,
              explicitLibraryIdSet.has(imageId) ? "提示词素材" : null,
            ].filter((value): value is string => value != null);
            return (
              <button
                key={imageId}
                type="button"
                onClick={() => setOpen(true)}
                disabled={disabled}
                aria-label={`管理参考素材 ${imageId}`}
                className={`group min-w-0 overflow-hidden rounded-xl border text-left transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary disabled:cursor-not-allowed disabled:opacity-60 ${
                  image
                    ? "border-book-border/55 bg-book-bg-paper/45 hover:border-book-primary/60"
                    : "border-error/45 bg-error-bg"
                }`}
              >
                <span className="flex aspect-video items-center justify-center overflow-hidden bg-book-bg-tertiary text-caption text-book-text-muted">
                  {src ? (
                    <img
                      src={src}
                      alt=""
                      className="h-full w-full object-cover transition-transform group-hover:scale-[1.02]"
                    />
                  ) : image ? (
                    "无图"
                  ) : (
                    "素材已失效"
                  )}
                </span>
                <span className="block min-w-0 p-2">
                  <span className="block truncate text-small font-medium text-book-text-main">
                    {image?.prompt || `图片 #${imageId}`}
                  </span>
                  <span className="mt-0.5 block truncate text-caption text-book-text-muted">
                    {roles.join(" · ") || "已选择"}
                  </span>
                </span>
              </button>
            );
          })}
          {canAdd ? (
            <button
              type="button"
              onClick={() => setOpen(true)}
              disabled={disabled}
              aria-label="添加参考图或素材"
              className="flex aspect-video min-w-0 flex-col items-center justify-center gap-1.5 rounded-xl border border-dashed border-book-border/70 bg-book-bg-paper/30 text-small text-book-text-muted transition-colors hover:border-book-primary/70 hover:bg-book-primary/5 hover:text-book-primary focus-visible:outline focus-visible:outline-2 focus-visible:outline-book-primary disabled:cursor-not-allowed disabled:opacity-50"
            >
              <Plus size={18} />
              <span>
                {selectedIds.length === 0 ? "添加第一张" : "继续添加"}
              </span>
            </button>
          ) : null}
        </div>
      </section>

      <Modal
        isOpen={open}
        onClose={() => setOpen(false)}
        title="添加参考图 / 素材"
        icon={<Images size={15} />}
        maxWidthClassName="max-w-5xl"
        zIndexClassName="z-[70]"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4 sm:px-5 sm:py-5"
      >
        <div className="mb-3 space-y-2 border-b border-book-border/35 pb-3">
          <div className="text-small text-book-text-muted">
            素材库 {libraryActiveCount} 张 · {generationReferenceLabel}{" "}
            {generationReferenceCountLabel}
          </div>
          <nav
            role="tablist"
            aria-label="参考素材筛选"
            className="flex gap-1.5 overflow-x-auto pb-1 custom-scrollbar"
          >
            {filterOptions.map((option) => (
              <button
                key={option.id}
                type="button"
                role="tab"
                aria-selected={filter === option.id}
                onClick={() => setFilter(option.id)}
                data-active={filter === option.id ? "1" : undefined}
                className={[
                  "inline-flex min-h-9 shrink-0 items-center gap-1 rounded-xl border px-2.5 text-caption transition-colors focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary sm:min-h-8",
                  filter === option.id
                    ? "border-book-primary bg-book-primary/10 text-book-primary"
                    : "border-book-border/60 bg-book-bg-paper/60 text-book-text-sub hover:border-book-primary/60 hover:text-book-text-main",
                ].join(" ")}
              >
                <span>{option.label}</span>
                <span className="tabular-nums opacity-70">{option.count}</span>
              </button>
            ))}
          </nav>
        </div>

        {hasInvalidSelection && (
          <div
            role="alert"
            className="mb-3 space-y-1 rounded-xl border border-error/40 bg-error-bg px-2.5 py-2 text-small text-error"
          >
            {invalidGenerationReferenceIds.length > 0 && (
              <div className="flex flex-wrap items-center gap-2">
                <span>
                  生成参考中有 {invalidGenerationReferenceIds.length}{" "}
                  张图片已不可用
                </span>
                <button
                  type="button"
                  onClick={() => void onClearInvalidGenerationReferences()}
                  disabled={disabled}
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-caption sm:!min-h-[22px]"
                >
                  清除不可用
                </button>
              </div>
            )}
            {invalidVideoLibraryImageIds.length > 0 && (
              <div className="flex flex-wrap items-center gap-2">
                <span>
                  素材库中有 {invalidVideoLibraryImageIds.length} 张图片已不可用
                </span>
                <button
                  type="button"
                  onClick={() => void onClearInvalidVideoLibraryImages()}
                  disabled={disabled}
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-caption sm:!min-h-[22px]"
                >
                  清除不可用
                </button>
              </div>
            )}
          </div>
        )}

        {images.length === 0 ? (
          <div className="rounded-xl border border-book-border/40 bg-book-bg-paper/35 px-2.5 py-2 text-small text-book-text-muted">
            当前章节没有可选素材图。
          </div>
        ) : filteredImages.length === 0 ? (
          <div className="rounded-xl border border-book-border/40 bg-book-bg-paper/35 px-2.5 py-2 text-small text-book-text-muted">
            没有匹配当前筛选的素材图。
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
            {filteredImages.map((image) => {
              const src = toMediaSrc(image.file_path);
              const generationActive = generationIdSet.has(image.id);
              const autoLibrary = isChapterVideoMaterialImage(image, chapter);
              const manualLibrary = explicitLibraryIdSet.has(image.id);
              const libraryActive = autoLibrary || manualLibrary;
              const profileCandidate =
                isProjectReusableVideoMaterialImage(image);
              const scopeLabel = referenceImageScopeLabel(
                image,
                chapter,
                currentContentHash,
              );
              return (
                <div
                  key={image.id}
                  className="min-w-0 overflow-hidden rounded-2xl border border-book-border/50 bg-book-bg-paper/40"
                >
                  <div className="relative flex aspect-[4/3] items-center justify-center overflow-hidden bg-book-bg-tertiary text-caption text-book-text-muted">
                    {src ? (
                      <img
                        src={src}
                        alt=""
                        className="h-full w-full object-cover"
                      />
                    ) : (
                      "无图"
                    )}
                    {(autoLibrary ||
                      manualLibrary ||
                      generationActive ||
                      profileCandidate) && (
                      <span className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/65 to-transparent px-2 pb-1.5 pt-4 text-right text-caption text-white">
                        {manualLibrary
                          ? "手动"
                          : autoLibrary
                            ? "本章自动"
                            : generationActive
                              ? "参考"
                              : "档案候选"}
                      </span>
                    )}
                  </div>
                  <div className="space-y-1 p-1.5">
                    <div className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-1 text-caption">
                      <span className="min-w-0 truncate text-book-text-sub">
                        {scopeLabel}
                      </span>
                      <span className="min-w-0 shrink-0 truncate text-book-text-muted">
                        {imageTypeLabel(image.image_type)}
                      </span>
                    </div>
                    <div className="grid grid-cols-2 gap-1">
                      <button
                        type="button"
                        onClick={() => toggleVideoLibraryImage(image)}
                        disabled={disabled || autoLibrary}
                        data-active={libraryActive ? "1" : undefined}
                        className={[
                          "inline-flex min-h-9 items-center justify-center gap-1 rounded-xl border px-1 text-caption font-medium transition-colors focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary sm:min-h-8",
                          libraryActive
                            ? "border-book-primary/70 bg-book-primary/10 text-book-primary"
                            : "border-book-border/60 bg-book-bg-paper/70 text-book-text-sub hover:border-book-primary/60 hover:text-book-text-main",
                          autoLibrary ? "cursor-default opacity-80" : "",
                        ].join(" ")}
                        title={
                          autoLibrary ? "自动素材库成员" : "切换素材库输入"
                        }
                      >
                        {libraryActive && <Check size={10} />}
                        素材库
                      </button>
                      <button
                        type="button"
                        onClick={() => toggleGenerationReference(image.id)}
                        disabled={disabled}
                        data-active={generationActive ? "1" : undefined}
                        className={[
                          "inline-flex min-h-9 items-center justify-center gap-1 rounded-xl border px-1 text-caption font-medium transition-colors focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary sm:min-h-8",
                          generationActive
                            ? "border-book-primary bg-book-primary/10 text-book-primary"
                            : "border-book-border/60 bg-book-bg-paper/70 text-book-text-sub hover:border-book-primary/60 hover:text-book-text-main",
                        ].join(" ")}
                        title={
                          chatVideoMode
                            ? "切换外部 Chat 上传候选"
                            : "切换生成参考输入"
                        }
                      >
                        {generationActive && <Check size={10} />}
                        {generationReferenceLabel}
                      </button>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </Modal>
    </>
  );
};

export const ReferenceMediaWorkspace: React.FC<{
  prompt: string;
  imageRows: ReferenceMediaBindingRow[];
  videoRows: ReferenceMediaBindingRow[];
  audioRows: ReferenceMediaBindingRow[];
  durationSecs: number;
  aspectRatio: VideoAspectRatio;
  durationLimits: VideoDurationLimits;
  activeVideoConfig: VideoGenConfig | null;
  acting: string | null;
  previewing: boolean;
  draftDirty: boolean;
  savingDraft: boolean;
  onPromptChange: (value: string) => void;
  onImageRowsChange: (rows: ReferenceMediaBindingRow[]) => void;
  onVideoRowsChange: (rows: ReferenceMediaBindingRow[]) => void;
  onAudioRowsChange: (rows: ReferenceMediaBindingRow[]) => void;
  onSaveDraft: () => void | Promise<void>;
  onPreview: () => void | Promise<void>;
  onGenerate: () => void | Promise<void>;
  onImport: () => void | Promise<void>;
}> = ({
  prompt,
  imageRows,
  videoRows,
  audioRows,
  durationSecs,
  aspectRatio,
  durationLimits,
  activeVideoConfig,
  acting,
  previewing,
  draftDirty,
  savingDraft,
  onPromptChange,
  onImageRowsChange,
  onVideoRowsChange,
  onAudioRowsChange,
  onSaveDraft,
  onPreview,
  onGenerate,
  onImport,
}) => {
  const [uploadError, setUploadError] = useState<string | null>(null);
  const durationSupported = isDurationAllowed(durationSecs, durationLimits);
  const rowsByKind: ReferenceMediaRowsByKind = {
    image: imageRows,
    video: videoRows,
    audio: audioRows,
  };
  const generateInputError = referenceMediaGenerateDraftError(
    prompt,
    rowsByKind,
  );
  const generateBlockedReason = durationLimits.configError
    ? durationLimits.configError
    : !durationSupported
      ? "当前时长不在默认视频模型支持范围内"
      : (generateInputError ?? undefined);
  const setRowsByKind: Record<
    ReferenceMediaKind,
    (rows: ReferenceMediaBindingRow[]) => void
  > = {
    image: onImageRowsChange,
    video: onVideoRowsChange,
    audio: onAudioRowsChange,
  };
  const generationRunning = acting === "reference-media-video";
  const importRunning = acting === "reference-media-import";
  const operationRunning = generationRunning || importRunning;
  const importBlockedReason = !prompt.trim()
    ? "参考媒体提示词不能为空"
    : undefined;
  const secondaryActionItems: React.ComponentProps<typeof Dropdown>["items"] = [
    {
      label: "查看模型请求",
      icon: <Eye size={12} />,
      disabled: previewing,
      onClick: () => void onPreview(),
    },
    {
      label: "导入已有视频",
      icon: <Upload size={12} />,
      disabled: importBlockedReason != null || acting != null,
      onClick: () => void onImport(),
    },
  ];
  const showGeneratePrimary =
    generationRunning || generateBlockedReason == null;

  const renderMediaRows = (kind: ReferenceMediaKind) => {
    const rows = rowsByKind[kind];
    const setRows = setRowsByKind[kind];
    const meta = REFERENCE_MEDIA_KIND_META[kind];
    const maxRows = REFERENCE_MEDIA_LIMITS[kind];
    const activeCount = activeReferenceMediaRows(rows).length;

    const updateRow = (
      rowId: string,
      patch: Partial<ReferenceMediaBindingRow>,
    ) => {
      setRows(
        rows.map((row) => (row.id === rowId ? { ...row, ...patch } : row)),
      );
    };
    const removeRow = (rowId: string) => {
      const next = rows.filter((row) => row.id !== rowId);
      setRows(next.length > 0 ? next : [createReferenceMediaBindingRow()]);
    };
    const selectLocalFile = async (rowId: string, file: File | undefined) => {
      if (!file) return;
      setUploadError(null);
      const allowedMimes = REFERENCE_MEDIA_ACCEPT[kind].split(",");
      if (!allowedMimes.includes(file.type)) {
        setUploadError(
          `${meta.label}文件类型不支持：${file.type || file.name}`,
        );
        return;
      }
      if (file.size > MAX_REFERENCE_MEDIA_UPLOAD_BYTES) {
        setUploadError(`${meta.label}文件不能超过 20MB`);
        return;
      }
      try {
        if (!globalThis.crypto?.subtle) {
          throw new Error("当前环境不支持计算本地文件 SHA-256");
        }
        const [dataBase64, digest] = await Promise.all([
          readFileAsBase64(file),
          globalThis.crypto.subtle.digest("SHA-256", await file.arrayBuffer()),
        ]);
        const uploadSha256 = Array.from(new Uint8Array(digest))
          .map((byte) => byte.toString(16).padStart(2, "0"))
          .join("");
        updateRow(rowId, {
          url: "",
          upload: {
            file_name: file.name,
            mime_type: file.type,
            data_base64: dataBase64,
          },
          uploadSha256,
          uploadFileName: file.name,
          uploadMimeType: file.type,
        });
      } catch (error) {
        setUploadError(extractErrorMessage(error, `读取${meta.label}文件失败`));
      }
    };

    return (
      <section className="border-t border-book-border/35 pt-3">
        <div className="mb-2 flex min-w-0 items-center gap-2">
          <h3 className="text-xs font-medium text-book-text-main">
            {meta.label}
          </h3>
          <span className="text-caption text-book-text-muted">
            {activeCount} / {maxRows}
          </span>
          {activeCount < maxRows && (
            <button
              type="button"
              onClick={() =>
                setRows([...rows, createReferenceMediaBindingRow()])
              }
              className="ml-auto inline-flex min-h-8 items-center gap-1 text-small text-book-text-sub transition-colors hover:text-book-primary"
              title={`添加${meta.label}`}
            >
              <Plus size={12} />
              添加{meta.label}
            </button>
          )}
        </div>
        <div className="divide-y divide-book-border/30">
          {rows.map((row, index) => {
            return (
              <div key={row.id} className="py-2 first:pt-0 last:pb-0">
                <div className="mb-1.5 flex min-w-0 items-center gap-2">
                  <span className="text-small font-medium text-book-text-main">
                    {meta.label} {index + 1}
                  </span>
                  <button
                    type="button"
                    onClick={() => removeRow(row.id)}
                    className="ml-auto pyqt-btn-icon !h-8 !w-8"
                    title={`删除${meta.label}`}
                    aria-label={`删除${meta.label} ${index + 1}`}
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
                <div className="min-w-0 space-y-1.5">
                  <input
                    value={row.url}
                    onChange={(event) =>
                      updateRow(row.id, {
                        url: event.target.value,
                        upload: undefined,
                        uploadSha256: undefined,
                        uploadFileName: undefined,
                        uploadMimeType: undefined,
                      })
                    }
                    className="book-control h-10 w-full rounded-xl px-2.5 text-small sm:h-9"
                    placeholder={`${meta.label} URL（http:// 或 https://）`}
                    aria-label={`${meta.label} ${index + 1} URL`}
                  />
                  <div className="flex min-w-0 items-center gap-2">
                    <label className="pyqt-btn-secondary inline-flex min-h-8 shrink-0 cursor-pointer items-center gap-1 px-2 text-small">
                      <Upload size={12} />
                      本地文件
                      <input
                        type="file"
                        accept={REFERENCE_MEDIA_ACCEPT[kind]}
                        className="sr-only"
                        onChange={(event) => {
                          const file = event.currentTarget.files?.[0];
                          event.currentTarget.value = "";
                          void selectLocalFile(row.id, file);
                        }}
                      />
                    </label>
                    {(row.upload || row.uploadFileName) && (
                      <span
                        className="min-w-0 flex-1 truncate text-caption text-book-text-sub"
                        title={row.upload?.file_name ?? row.uploadFileName}
                      >
                        {row.upload?.file_name ?? row.uploadFileName}
                        {!row.upload ? "（刷新后需重新选择该文件）" : ""}
                      </span>
                    )}
                  </div>
                  <textarea
                    value={row.usage}
                    onChange={(event) =>
                      updateRow(row.id, { usage: event.target.value })
                    }
                    className="book-control min-h-[58px] w-full resize-y rounded-xl p-2.5 text-small leading-relaxed"
                    placeholder="描述用途、主体、必须保持的细节"
                    aria-label={`${meta.label} ${index + 1} 用途`}
                  />
                </div>
              </div>
            );
          })}
        </div>
        {uploadError && (
          <div className="mt-2 text-caption text-error">{uploadError}</div>
        )}
      </section>
    );
  };

  return (
    <section
      aria-label="参考媒体生成设置"
      className="a4n-flat-surface flex h-full min-h-0 flex-col overflow-hidden p-3"
    >
      <div className="mb-3 border-b border-book-border/35 pb-3 text-small text-book-text-muted">
        视频服务 ·{" "}
        {activeVideoConfig?.name?.trim() ||
          (activeVideoConfig ? "已配置" : "尚未配置")}
      </div>
      {operationRunning ? (
        <div
          role="status"
          className="flex min-h-0 flex-1 items-center justify-center gap-2 text-xs text-book-text-muted"
        >
          <LoadingSpinner size="sm" />
          {generationRunning ? "正在生成参考媒体视频" : "正在导入参考媒体视频"}
        </div>
      ) : (
        <>
          <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pb-2 pr-1 custom-scrollbar">
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-xs text-book-text-muted">基础提示词</span>
              <textarea
                value={prompt}
                onChange={(event) => onPromptChange(event.target.value)}
                className="book-control min-h-[130px] resize-y rounded-xl p-2.5 text-xs leading-relaxed"
                placeholder="整体画面、动作、风格、镜头和声音要求"
              />
            </label>
            {renderMediaRows("image")}
            {renderMediaRows("video")}
            {renderMediaRows("audio")}
            <div className="border-t border-book-border/35 pt-2 text-small text-book-text-sub">
              规划参数：{durationSecs} 秒 · {aspectRatio}
              。时长、画幅和风格属于共享片段规划；如需修改，请重新生成片段规划。
              <div className="mt-1 text-caption text-book-text-muted">
                {durationLimitsLabel(durationLimits)}
              </div>
            </div>
            {!durationLimits.configError && !durationSupported && (
              <div
                role="status"
                className="rounded-xl border border-warning/40 bg-warning-bg px-2.5 py-2 text-small text-warning"
              >
                当前时长不在视频服务支持范围内。
              </div>
            )}
            {durationLimits.configError && (
              <div
                role="alert"
                className="rounded-xl border border-error/40 bg-error-bg px-2.5 py-2 text-small text-error"
              >
                {durationLimits.configError}
              </div>
            )}
            {generateInputError && (
              <div
                role="status"
                className="rounded-xl border border-warning/40 bg-warning-bg px-2.5 py-2 text-small text-warning"
              >
                {generateInputError}
              </div>
            )}
          </div>
          <div className="mt-3 flex shrink-0 items-center gap-2">
            <button
              type="button"
              onClick={() => void onSaveDraft()}
              disabled={!draftDirty || savingDraft}
              className="pyqt-btn-secondary inline-flex min-h-9 items-center justify-center gap-1.5 px-3 text-xs"
              title={draftDirty ? "保存当前片段参考媒体草稿" : "草稿已保存"}
            >
              {savingDraft ? <LoadingSpinner size="sm" /> : <Check size={12} />}
              {savingDraft ? "保存中" : draftDirty ? "保存草稿" : "已保存"}
            </button>
            <Dropdown items={secondaryActionItems} />
            {showGeneratePrimary && (
              <button
                type="button"
                onClick={() => void onGenerate()}
                disabled={generationRunning || previewing}
                className="pyqt-btn-primary inline-flex min-h-9 min-w-0 flex-1 items-center justify-center gap-1.5 text-xs"
              >
                <Images size={12} />
                生成视频
              </button>
            )}
          </div>
        </>
      )}
    </section>
  );
};
