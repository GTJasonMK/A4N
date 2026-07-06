import React, { useMemo, useState } from "react";
import { Check, Images, Plus, Trash2, Upload } from "lucide-react";
import { extractErrorMessage } from "../../../api";
import type { GeneratedImage } from "../../../api/image";
import type {
  GeneratedVideo,
  UpdateVideoInput,
  VideoGenConfig,
  VideoWorkflowMode,
} from "../../../api/video";
import { Modal } from "../../../components/ui/Modal";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { toMediaSrc } from "../../../utils/mediaSrc";
import type { ChapterRow } from "../shared";
import { VideoResultsPane } from "./VideoResultsPane";
import {
  durationLimitsLabel,
  isChapterVideoMaterialImage,
  isDurationAllowed,
  isProjectReusableVideoMaterialImage,
  MAX_CHAT_REFERENCE_IMAGES,
  MAX_REFERENCE_IMAGES,
  MAX_VIDEO_LIBRARY_IMAGES,
  parseNumberInput,
  referenceImageScopeLabel,
  VIDEO_ASPECT_RATIOS,
  type VideoAspectRatio,
  type VideoDurationLimits,
} from "./videoTabUtils";

export const REFERENCE_MEDIA_PROMPT_MIN_CHARS = 100;
export const REFERENCE_MEDIA_LIMITS = {
  image: 9,
  video: 3,
  audio: 3,
} as const;

export type ReferenceMediaKind = keyof typeof REFERENCE_MEDIA_LIMITS;
export type ReferenceMediaInputPurpose = "generate" | "preview";
export type ReferenceMediaRowsByKind = Record<ReferenceMediaKind, ReferenceMediaBindingRow[]>;

export interface ReferenceMediaBindingRow {
  id: string;
  url: string;
  usage: string;
}

export interface ReferenceMediaDraft {
  prompt: string;
  imageRows: ReferenceMediaBindingRow[];
  videoRows: ReferenceMediaBindingRow[];
  audioRows: ReferenceMediaBindingRow[];
  storageError: string | null;
}

export const REFERENCE_MEDIA_KIND_META: Record<
  ReferenceMediaKind,
  { label: string; field: "image_files" | "video_files" | "audio_files"; tokenPrefix: string }
> = {
  image: { label: "图片", field: "image_files", tokenPrefix: "image_file" },
  video: { label: "视频", field: "video_files", tokenPrefix: "video_file" },
  audio: { label: "音频", field: "audio_files", tokenPrefix: "audio_file" },
};

const REFERENCE_MEDIA_KINDS: ReferenceMediaKind[] = ["image", "video", "audio"];

let referenceMediaRowSeq = 0;

export const createReferenceMediaBindingRow = (): ReferenceMediaBindingRow => {
  referenceMediaRowSeq += 1;
  return {
    id: `reference-media-${Date.now()}-${referenceMediaRowSeq}`,
    url: "",
    usage: "",
  };
};

export const createReferenceMediaDraft = (
  storageError: string | null = null,
): ReferenceMediaDraft => ({
  prompt: "",
  imageRows: [createReferenceMediaBindingRow()],
  videoRows: [createReferenceMediaBindingRow()],
  audioRows: [createReferenceMediaBindingRow()],
  storageError,
});

const parseReferenceMediaBindingRows = (
  value: unknown,
  field: "imageRows" | "videoRows" | "audioRows",
): ReferenceMediaBindingRow[] => {
  if (!Array.isArray(value)) throw new Error(`参考媒体草稿 ${field} 必须是数组`);
  if (value.length === 0) return [createReferenceMediaBindingRow()];
  return value.map((item, index) => {
    const row = asRecord(item);
    if (!row) throw new Error(`参考媒体草稿 ${field}[${index}] 必须是对象`);
    if (typeof row.url !== "string") {
      throw new Error(`参考媒体草稿 ${field}[${index}].url 必须是字符串`);
    }
    if (typeof row.usage !== "string") {
      throw new Error(`参考媒体草稿 ${field}[${index}].usage 必须是字符串`);
    }
    return { ...createReferenceMediaBindingRow(), url: row.url, usage: row.usage };
  });
};

const serializeReferenceMediaBindingRows = (
  rows: ReferenceMediaBindingRow[],
): Array<Pick<ReferenceMediaBindingRow, "url" | "usage">> =>
  rows.map((row) => ({
    url: row.url,
    usage: row.usage,
  }));

export const parseReferenceMediaDraft = (raw: string): ReferenceMediaDraft => {
  try {
    const parsed = asRecord(JSON.parse(raw));
    if (!parsed) throw new Error("参考媒体草稿必须是对象");
    if (typeof parsed.prompt !== "string") {
      throw new Error("参考媒体草稿 prompt 必须是字符串");
    }
    return {
      prompt: parsed.prompt,
      imageRows: parseReferenceMediaBindingRows(parsed.imageRows, "imageRows"),
      videoRows: parseReferenceMediaBindingRows(parsed.videoRows, "videoRows"),
      audioRows: parseReferenceMediaBindingRows(parsed.audioRows, "audioRows"),
      storageError: null,
    };
  } catch (err) {
    return createReferenceMediaDraft(`参考媒体本地草稿损坏：${extractErrorMessage(err, "")}`);
  }
};

export const serializeReferenceMediaDraft = (draft: ReferenceMediaDraft): string =>
  JSON.stringify({
    prompt: draft.prompt,
    imageRows: serializeReferenceMediaBindingRows(draft.imageRows),
    videoRows: serializeReferenceMediaBindingRows(draft.videoRows),
    audioRows: serializeReferenceMediaBindingRows(draft.audioRows),
  });

export const referenceMediaRowsByKindFromDraft = (
  draft: ReferenceMediaDraft,
): ReferenceMediaRowsByKind => ({
  image: draft.imageRows,
  video: draft.videoRows,
  audio: draft.audioRows,
});

export const clearReferenceMediaDraftStorageError = (
  draft: ReferenceMediaDraft,
): ReferenceMediaDraft =>
  draft.storageError ? { ...draft, storageError: null } : draft;

const referenceMediaRowActive = (row: ReferenceMediaBindingRow): boolean =>
  Boolean(row.url.trim() || row.usage.trim());

const referenceMediaTokenForRow = (
  rows: ReferenceMediaBindingRow[],
  kind: ReferenceMediaKind,
  rowIndex: number,
): string => {
  const activeBefore = rows.slice(0, rowIndex + 1).filter(referenceMediaRowActive).length;
  const emptyPlaceholder = rows.slice(0, rowIndex).filter(referenceMediaRowActive).length + 1;
  const tokenIndex = activeBefore || emptyPlaceholder;
  return `@${REFERENCE_MEDIA_KIND_META[kind].tokenPrefix}_${tokenIndex}`;
};

const activeReferenceMediaRows = (
  rows: ReferenceMediaBindingRow[],
): ReferenceMediaBindingRow[] =>
  rows.filter(referenceMediaRowActive);

export const normalizeReferenceMediaRows = (
  rows: ReferenceMediaBindingRow[],
): ReferenceMediaBindingRow[] =>
  activeReferenceMediaRows(rows).map((row) => ({
    ...row,
    url: row.url.trim(),
    usage: row.usage.trim(),
  }));

export const normalizeReferenceMediaRowsByKind = (
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

export const buildReferenceMediaPrompt = (
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
  storageError: string | null = null,
): string | null => {
  if (storageError) {
    return `${storageError}；请重新编辑当前参考媒体 prompt 或媒体行后再生成`;
  }
  const normalizedRowsByKind = normalizeReferenceMediaRowsByKind(rowsByKind);
  for (const kind of REFERENCE_MEDIA_KINDS) {
    const meta = REFERENCE_MEDIA_KIND_META[kind];
    const rows = normalizedRowsByKind[kind];
    if (rows.length > REFERENCE_MEDIA_LIMITS[kind]) {
      return `${meta.field} 最多允许 ${REFERENCE_MEDIA_LIMITS[kind]} 个 URL`;
    }
    for (const [index, row] of rows.entries()) {
      const token = `@${meta.tokenPrefix}_${index + 1}`;
      if (!row.url) return `${token} 缺少 URL`;
      if (!validReferenceMediaUrl(row.url)) return `${token} URL 必须以小写 http:// 或 https:// 开头`;
      if (!row.usage) return `${token} 缺少用途描述`;
    }
  }
  const totalMediaCount = REFERENCE_MEDIA_KINDS.reduce(
    (count, kind) => count + normalizedRowsByKind[kind].length,
    0,
  );
  if (totalMediaCount === 0) {
    return "参考媒体模式至少需要一个 image_files、video_files 或 audio_files URL";
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

const toggleImageIdSelection = (ids: number[], id: number, field: string): number[] => {
  if (!Number.isInteger(id) || id <= 0) {
    throw new Error(`${field} 只能切换正整数图片 ID`);
  }
  assertImageIdSelection(ids, field);
  return ids.includes(id) ? ids.filter((value) => value !== id) : [...ids, id];
};

type ReferenceImageFilter = "all" | "selected" | "library" | "generation" | "chapter" | "project" | "profile_material";

export const VideoPromptReferenceInputDialog: React.FC<{
  images: GeneratedImage[];
  chapter: ChapterRow;
  workflowMode: VideoWorkflowMode;
  currentContentHash: string | null;
  generationReferenceIds: number[];
  videoLibraryImageIds: number[];
  invalidGenerationReferenceIds: number[];
  invalidVideoLibraryImageIds: number[];
  onGenerationReferenceIdsChange: React.Dispatch<React.SetStateAction<number[]>>;
  onVideoLibraryImageIdsChange: React.Dispatch<React.SetStateAction<number[]>>;
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
      if (isProjectReusableVideoMaterialImage(image)) counts.profile_material += 1;
    }
    return counts;
  }, [chapter, explicitLibraryIdSet, generationIdSet, images]);
  const filterOptions = useMemo(
    () =>
      [
        { id: "all", label: "全部", count: filterCounts.all },
        { id: "selected", label: "已选", count: filterCounts.selected },
        { id: "library", label: "素材库", count: filterCounts.library },
        { id: "generation", label: generationReferenceLabel, count: filterCounts.generation },
        { id: "chapter", label: "本章", count: filterCounts.chapter },
        { id: "project", label: "项目", count: filterCounts.project },
        { id: "profile_material", label: "档案素材", count: filterCounts.profile_material },
      ] satisfies Array<{ id: ReferenceImageFilter; label: string; count: number }>,
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
        if (filter === "profile_material") return isProjectReusableVideoMaterialImage(image);
        return true;
      }),
    [chapter, explicitLibraryIdSet, filter, generationIdSet, images],
  );

  const toggleGenerationReference = (imageId: number) => {
    try {
      if (
        !generationIdSet.has(imageId) &&
        generationReferenceIds.length >= generationReferenceLimit
      ) {
        onReferenceLimit();
        return;
      }
      onGenerationReferenceIdsChange(
        toggleImageIdSelection(generationReferenceIds, imageId, "生成参考图片选择"),
      );
    } catch (err) {
      onSelectionError(extractErrorMessage(err, ""));
    }
  };

  const toggleVideoLibraryImage = (image: GeneratedImage) => {
    if (isChapterVideoMaterialImage(image, chapter)) return;
    if (
      !explicitLibraryIdSet.has(image.id) &&
      videoLibraryImageIds.length >= MAX_VIDEO_LIBRARY_IMAGES
    ) {
      onSelectionError(`视频素材库最多只能选择 ${MAX_VIDEO_LIBRARY_IMAGES} 张`);
      return;
    }
    try {
      onVideoLibraryImageIdsChange(
        toggleImageIdSelection(videoLibraryImageIds, image.id, "视频素材库图片选择"),
      );
    } catch (err) {
      onSelectionError(extractErrorMessage(err, ""));
    }
  };
  const invalidGenerationLabel = invalidGenerationReferenceIds.join(", ");
  const invalidLibraryLabel = invalidVideoLibraryImageIds.join(", ");
  const hasInvalidSelection =
    invalidGenerationReferenceIds.length > 0 || invalidVideoLibraryImageIds.length > 0;

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={[
          "pyqt-btn-secondary inline-flex min-h-[28px] items-center gap-1.5 !px-2 text-[11px]",
          hasInvalidSelection ? "!border-error/50 !text-error" : "",
        ].join(" ")}
        title="提示词输入素材"
      >
        <Images size={13} />
        <span>输入素材</span>
        <span className="text-[10px] text-book-text-muted">
          库 {libraryActiveCount} · {generationReferenceLabel} {generationReferenceCountLabel}
        </span>
        {hasInvalidSelection && (
          <span className="rounded-sm bg-error/10 px-1 text-[10px] text-error">
            异常
          </span>
        )}
      </button>

      <Modal
        isOpen={open}
        onClose={() => setOpen(false)}
        title="提示词输入素材"
        icon={<Images size={15} />}
        maxWidthClassName="max-w-5xl"
        bodyClassName="min-h-0 flex-1 overflow-y-auto custom-scrollbar px-4 py-4 sm:px-5 sm:py-5"
        footer={
          <button
            type="button"
            onClick={() => setOpen(false)}
            className="pyqt-btn-secondary"
          >
            关闭
          </button>
        }
      >
        <div className="mb-3 flex min-w-0 flex-wrap items-center justify-between gap-2">
          <div className="flex shrink-0 flex-wrap items-center gap-1 text-[10px] text-book-text-muted">
            <span className="pyqt-pill !text-[10px]">素材库 {libraryActiveCount}</span>
            <span className="pyqt-pill !text-[10px]">
              {generationReferenceLabel} {generationReferenceCountLabel}
            </span>
          </div>
          <div className="flex min-w-0 flex-wrap items-center gap-1">
            <span className="mr-1 text-[10px] text-book-text-muted">筛选</span>
            {filterOptions.map((option) => (
              <button
                key={option.id}
                type="button"
                onClick={() => setFilter(option.id)}
                data-active={filter === option.id ? "1" : undefined}
                className={[
                  "inline-flex min-h-9 items-center gap-1 rounded-sm border px-2 text-[10px] transition-colors focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary sm:min-h-[24px]",
                  filter === option.id
                    ? "border-book-primary bg-book-primary/10 text-book-primary"
                    : "border-book-border/60 bg-book-bg-paper/70 text-book-text-sub hover:border-book-primary/60 hover:text-book-text-main",
                ].join(" ")}
              >
                <span>{option.label}</span>
                <span className="font-mono text-[9px] opacity-75">{option.count}</span>
              </button>
            ))}
          </div>
        </div>

        {hasInvalidSelection && (
          <div className="mb-3 space-y-1 rounded border border-error/40 bg-error/10 px-2 py-1.5 text-[11px] text-error">
            {invalidGenerationReferenceIds.length > 0 && (
              <div className="flex flex-wrap items-center gap-2">
                <span>生成参考包含不可用图片 ID：{invalidGenerationLabel}</span>
                <button
                  type="button"
                  onClick={onClearInvalidGenerationReferences}
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[22px]"
                >
                  清除不可用
                </button>
              </div>
            )}
            {invalidVideoLibraryImageIds.length > 0 && (
              <div className="flex flex-wrap items-center gap-2">
                <span>素材库包含不可用图片 ID：{invalidLibraryLabel}</span>
                <button
                  type="button"
                  onClick={onClearInvalidVideoLibraryImages}
                  className="pyqt-btn-secondary !min-h-9 !px-2 text-[10px] sm:!min-h-[22px]"
                >
                  清除不可用
                </button>
              </div>
            )}
          </div>
        )}

        {images.length === 0 ? (
          <div className="rounded border border-book-border/40 bg-book-bg-paper/35 px-2 py-1.5 text-[11px] text-book-text-muted">
            当前章节没有可选素材图。
          </div>
        ) : filteredImages.length === 0 ? (
          <div className="rounded border border-book-border/40 bg-book-bg-paper/35 px-2 py-1.5 text-[11px] text-book-text-muted">
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
              const profileCandidate = isProjectReusableVideoMaterialImage(image);
              const scopeLabel = referenceImageScopeLabel(image, chapter, currentContentHash);
              return (
                <div
                  key={image.id}
                  className="min-w-0 overflow-hidden rounded border border-book-border/50 bg-book-bg-paper/45"
                >
                  <div className="relative flex aspect-[4/3] items-center justify-center overflow-hidden bg-book-bg-tertiary text-[10px] text-book-text-muted">
                    {src ? <img src={src} alt="" className="h-full w-full object-cover" /> : "无图"}
                    <span className="absolute left-1 top-1 rounded bg-book-bg-paper/90 px-1 font-mono text-[9px] text-book-text-main">
                      #{image.id}
                    </span>
                    {(autoLibrary || manualLibrary || generationActive || profileCandidate) && (
                      <span className="absolute right-1 top-1 rounded border border-book-primary/30 bg-book-bg-paper/90 px-1 text-[9px] text-book-primary">
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
                    <div className="flex min-w-0 items-center justify-between gap-1 text-[10px]">
                      <span className="truncate text-book-text-sub">{scopeLabel}</span>
                      <span className="shrink-0 truncate font-mono text-book-text-muted">
                        {image.image_type}
                      </span>
                    </div>
                    <div className="grid grid-cols-2 gap-1">
                      <button
                        type="button"
                        onClick={() => toggleVideoLibraryImage(image)}
                        disabled={autoLibrary}
                        data-active={libraryActive ? "1" : undefined}
                        className={[
                          "inline-flex min-h-9 items-center justify-center gap-1 rounded border px-1 text-[10px] font-medium transition-colors focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary sm:min-h-[24px]",
                          libraryActive
                            ? "border-book-primary/70 bg-book-primary/10 text-book-primary"
                            : "border-book-border/60 bg-book-bg-paper/70 text-book-text-sub hover:border-book-primary/60 hover:text-book-text-main",
                          autoLibrary ? "cursor-default opacity-80" : "",
                        ].join(" ")}
                        title={autoLibrary ? "自动素材库成员" : "切换素材库输入"}
                      >
                        {libraryActive && <Check size={10} />}
                        素材库
                      </button>
                      <button
                        type="button"
                        onClick={() => toggleGenerationReference(image.id)}
                        data-active={generationActive ? "1" : undefined}
                        className={[
                          "inline-flex min-h-9 items-center justify-center gap-1 rounded border px-1 text-[10px] font-medium transition-colors focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary sm:min-h-[24px]",
                          generationActive
                            ? "border-book-primary bg-book-primary/10 text-book-primary"
                            : "border-book-border/60 bg-book-bg-paper/70 text-book-text-sub hover:border-book-primary/60 hover:text-book-text-main",
                        ].join(" ")}
                        title={chatVideoMode ? "切换外部 Chat 上传候选" : "切换生成参考输入"}
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
  draftStorageError: string | null;
  durationSecs: number;
  aspectRatio: VideoAspectRatio;
  durationLimits: VideoDurationLimits;
  activeVideoConfig: VideoGenConfig | null;
  videos: GeneratedVideo[];
  totalVideoAssetCount: number;
  selectedVideo: GeneratedVideo | null;
  selectedVideoId: number | null;
  selectedVideoPath: string;
  trimStart: string;
  trimEnd: string;
  acting: string | null;
  previewing: boolean;
  onPromptChange: (value: string) => void;
  onImageRowsChange: (rows: ReferenceMediaBindingRow[]) => void;
  onVideoRowsChange: (rows: ReferenceMediaBindingRow[]) => void;
  onAudioRowsChange: (rows: ReferenceMediaBindingRow[]) => void;
  onDurationSecsChange: (value: number) => void;
  onAspectRatioChange: (value: VideoAspectRatio) => void;
  onPreview: () => void | Promise<void>;
  onGenerate: () => void | Promise<void>;
  onImport: () => void | Promise<void>;
  onSelectVideo: (id: number) => void;
  onPollVideo: (video: GeneratedVideo) => void | Promise<void>;
  onPreviewVideoRequest: (video: GeneratedVideo) => void | Promise<void>;
  onToggleVideoFavorite: (video: GeneratedVideo) => void | Promise<void>;
  onUpdateVideo: (video: GeneratedVideo, input: UpdateVideoInput) => void | Promise<void>;
  onDeleteVideo: (video: GeneratedVideo) => void | Promise<void>;
  onDeleteAllVideoAssets: () => void | Promise<void>;
  onTrimStartChange: (value: string) => void;
  onTrimEndChange: (value: string) => void;
  onTrim: () => void | Promise<void>;
}> = ({
  prompt,
  imageRows,
  videoRows,
  audioRows,
  draftStorageError,
  durationSecs,
  aspectRatio,
  durationLimits,
  activeVideoConfig,
  videos,
  totalVideoAssetCount,
  selectedVideo,
  selectedVideoId,
  selectedVideoPath,
  trimStart,
  trimEnd,
  acting,
  previewing,
  onPromptChange,
  onImageRowsChange,
  onVideoRowsChange,
  onAudioRowsChange,
  onDurationSecsChange,
  onAspectRatioChange,
  onPreview,
  onGenerate,
  onImport,
  onSelectVideo,
  onPollVideo,
  onPreviewVideoRequest,
  onToggleVideoFavorite,
  onUpdateVideo,
  onDeleteVideo,
  onDeleteAllVideoAssets,
  onTrimStartChange,
  onTrimEndChange,
  onTrim,
}) => {
  const seedanceReady = activeVideoConfig?.backend === "seedance2-video";
  const durationSupported = isDurationAllowed(durationSecs, durationLimits);
  const rowsByKind: ReferenceMediaRowsByKind = {
    image: imageRows,
    video: videoRows,
    audio: audioRows,
  };
  const generateInputError = referenceMediaGenerateDraftError(prompt, rowsByKind, draftStorageError);
  const previewBlockedReason = draftStorageError
    ? `${draftStorageError}；请重新编辑后再查看模型请求`
    : undefined;
  const generateBlockedReason = !seedanceReady
    ? "参考媒体模式必须先激活 seedance2-video 视频配置"
    : durationLimits.configError
      ? durationLimits.configError
    : !durationSupported
      ? "当前时长不在默认视频模型支持范围内"
      : generateInputError ?? undefined;
  const setRowsByKind: Record<ReferenceMediaKind, (rows: ReferenceMediaBindingRow[]) => void> = {
    image: onImageRowsChange,
    video: onVideoRowsChange,
    audio: onAudioRowsChange,
  };

  const renderMediaRows = (kind: ReferenceMediaKind) => {
    const rows = rowsByKind[kind];
    const setRows = setRowsByKind[kind];
    const meta = REFERENCE_MEDIA_KIND_META[kind];
    const maxRows = REFERENCE_MEDIA_LIMITS[kind];
    const activeCount = activeReferenceMediaRows(rows).length;

    const updateRow = (rowId: string, patch: Partial<ReferenceMediaBindingRow>) => {
      setRows(rows.map((row) => (row.id === rowId ? { ...row, ...patch } : row)));
    };
    const removeRow = (rowId: string) => {
      const next = rows.filter((row) => row.id !== rowId);
      setRows(next.length > 0 ? next : [createReferenceMediaBindingRow()]);
    };

    return (
      <div className="space-y-2 rounded border border-book-border/40 bg-book-bg-paper/35 p-2">
        <div className="flex min-w-0 items-center justify-between gap-2">
          <div className="min-w-0 text-xs font-medium text-book-text-main">
            {meta.field}
            <span className="ml-1 font-mono text-[10px] text-book-text-muted">
              {activeCount}/{maxRows}
            </span>
          </div>
          <button
            type="button"
            onClick={() => setRows([...rows, createReferenceMediaBindingRow()])}
            disabled={activeCount >= maxRows}
            className="pyqt-btn-secondary inline-flex min-h-9 items-center gap-1 px-2 text-[11px] sm:min-h-[24px]"
            title={`添加${meta.label}`}
          >
            <Plus size={12} />
            添加
          </button>
        </div>
        <div className="space-y-2">
          {rows.map((row, index) => {
            const token = referenceMediaTokenForRow(rows, kind, index);
            return (
              <div key={row.id} className="grid grid-cols-[72px_minmax(0,1fr)_36px] gap-2 sm:grid-cols-[72px_minmax(0,1fr)_32px]">
                <div className="flex min-w-0 flex-col gap-1">
                  <span className="rounded border border-book-border/40 bg-book-bg-tertiary px-1.5 py-1 font-mono text-[10px] text-book-text-main">
                    {token}
                  </span>
                </div>
                <div className="min-w-0 space-y-1">
                  <input
                    value={row.url}
                    onChange={(event) => updateRow(row.id, { url: event.target.value })}
                    className="book-control h-10 w-full rounded-sm px-2 font-mono text-[11px] sm:h-7"
                    placeholder="https://example.com/media"
                  />
                  <textarea
                    value={row.usage}
                    onChange={(event) => updateRow(row.id, { usage: event.target.value })}
                    className="book-control min-h-[50px] w-full resize-y rounded-sm p-2 text-[11px] leading-relaxed"
                    placeholder="描述用途、主体、必须保持的细节"
                  />
                </div>
                <button
                  type="button"
                  onClick={() => removeRow(row.id)}
                  className="pyqt-btn-ghost flex h-9 w-9 items-center justify-center self-start sm:h-7 sm:w-7"
                  title="删除"
                >
                  <Trash2 size={13} />
                </button>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <div className="grid h-full min-h-0 gap-3 xl:grid-cols-[minmax(320px,420px)_minmax(0,1fr)]">
      <div className="flex min-h-0 flex-col rounded border border-book-border/50 bg-book-bg-paper/45 p-3">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <div className="inline-flex min-w-0 items-center gap-1.5 text-sm font-medium text-book-text-main">
            <Images size={14} className="text-book-primary" />
            参考媒体生成
          </div>
          <span className="pyqt-pill !text-[10px]">
            {activeVideoConfig?.backend ?? "未配置"}
          </span>
        </div>
        <div className="min-h-0 flex-1 space-y-3 overflow-y-auto custom-scrollbar pr-1">
          <label className="flex min-w-0 flex-col gap-1">
            <span className="text-xs text-book-text-muted">基础 prompt</span>
            <textarea
              value={prompt}
              onChange={(event) => onPromptChange(event.target.value)}
              className="book-control min-h-[130px] resize-y rounded-sm p-2 text-xs leading-relaxed"
              placeholder="整体画面、动作、风格、镜头和声音要求"
            />
          </label>
          {renderMediaRows("image")}
          {renderMediaRows("video")}
          {renderMediaRows("audio")}
          <div className="grid grid-cols-1 gap-2 text-[11px] sm:grid-cols-2">
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-book-text-muted">时长</span>
              {durationLimits.durationMode === "fixed" &&
              durationLimits.supportedDurations.length > 0 ? (
                <select
                  value={durationSecs}
                  onChange={(event) => {
                    const value = parseNumberInput(event.target.value);
                    if (value !== null && durationLimits.supportedDurations.includes(value)) {
                      onDurationSecsChange(value);
                    }
                  }}
                  disabled={durationLimits.configError != null}
                  className="pyqt-input !px-2 !py-1.5"
                >
                  {durationLimits.supportedDurations.map((duration) => (
                    <option key={duration} value={duration}>
                      {duration}s
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  type="number"
                  min={durationLimits.minDurationSecs}
                  max={durationLimits.maxDurationSecs}
                  value={durationSecs}
                  onChange={(event) => {
                    const value = parseNumberInput(event.target.value);
                    if (value !== null) onDurationSecsChange(value);
                  }}
                  disabled={durationLimits.configError != null}
                  className="pyqt-input !px-2 !py-1.5"
                />
              )}
            </label>
            <label className="flex min-w-0 flex-col gap-1">
              <span className="text-book-text-muted">画幅</span>
              <select
                value={aspectRatio}
                onChange={(event) => {
                  const value = event.target.value as VideoAspectRatio;
                  if (VIDEO_ASPECT_RATIOS.includes(value)) onAspectRatioChange(value);
                }}
                className="pyqt-input !px-2 !py-1.5"
              >
                {VIDEO_ASPECT_RATIOS.map((ratio) => (
                  <option key={ratio} value={ratio}>
                    {ratio}
                  </option>
                ))}
              </select>
            </label>
            <div className="a4n-stat-tile px-2 py-1 text-book-text-sub sm:col-span-2">
              {durationLimitsLabel(durationLimits)}
            </div>
          </div>
          {!seedanceReady && (
            <div className="rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-[11px] text-warning">
              当前默认视频配置不是 seedance2-video。
            </div>
          )}
          {seedanceReady && !durationLimits.configError && !durationSupported && (
            <div className="rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-[11px] text-warning">
              当前时长不在默认视频模型支持范围内。
            </div>
          )}
          {durationLimits.configError && (
            <div className="rounded border border-error/40 bg-error/10 px-2 py-1.5 text-[11px] text-error">
              {durationLimits.configError}
            </div>
          )}
          {draftStorageError && (
            <div className="rounded border border-error/40 bg-error/10 px-2 py-1.5 text-[11px] text-error">
              {draftStorageError}；请重新编辑当前参考媒体 prompt 或媒体行后再预览/生成。
            </div>
          )}
          {!draftStorageError && generateInputError && (
            <div className="rounded border border-warning/40 bg-warning/10 px-2 py-1.5 text-[11px] text-warning">
              {generateInputError}
            </div>
          )}
        </div>
        <div className="mt-3 flex shrink-0 flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void onPreview()}
            disabled={previewBlockedReason != null || previewing}
            title={previewBlockedReason}
            className="pyqt-btn-secondary min-h-[30px] flex-1 text-xs"
          >
            查看模型请求
          </button>
          <button
            type="button"
            onClick={() => void onImport()}
            disabled={draftStorageError != null || !prompt.trim() || acting === "reference-media-import"}
            title={
              draftStorageError
                ? `${draftStorageError}；请重新编辑后再导入`
                : !prompt.trim()
                  ? "参考媒体 prompt 不能为空"
                  : undefined
            }
            className="pyqt-btn-secondary inline-flex min-h-[30px] flex-1 items-center justify-center gap-1.5 text-xs"
          >
            <Upload size={12} />
            导入视频
          </button>
          <button
            type="button"
            onClick={() => void onGenerate()}
            disabled={
              generateBlockedReason != null ||
              acting === "reference-media-video" ||
              previewing
            }
            title={generateBlockedReason}
            className="pyqt-btn-primary min-h-[30px] flex-1 text-xs"
          >
            生成视频
          </button>
        </div>
      </div>
      <VideoResultsPane
        videos={videos}
        selectedVideo={selectedVideo}
        selectedVideoId={selectedVideoId}
        totalVideoAssetCount={totalVideoAssetCount}
        selectedVideoPath={selectedVideoPath}
        trimStart={trimStart}
        trimEnd={trimEnd}
        acting={acting}
        onSelectVideo={onSelectVideo}
        onPollVideo={onPollVideo}
        onPreviewVideoRequest={onPreviewVideoRequest}
        onToggleVideoFavorite={onToggleVideoFavorite}
        onUpdateVideo={onUpdateVideo}
        onDeleteVideo={onDeleteVideo}
        onDeleteAllVideoAssets={onDeleteAllVideoAssets}
        onTrimStartChange={onTrimStartChange}
        onTrimEndChange={onTrimEndChange}
        onTrim={onTrim}
      />
    </div>
  );
};
