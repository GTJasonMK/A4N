import React from "react";
import { Star } from "lucide-react";
import type { GeneratedImage } from "../../../api/image";
import type { ChapterVideoPrompt } from "../../../api/video";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { EmptyHint } from "../shared";
import {
  formatVideoCreatedAt,
  videoFrameImageRoleLabel,
  videoFrameImageRoleOfImage,
  videoImageGenerationPromptId,
} from "./videoTabUtils";

const IMAGE_TYPE_LABELS: Record<string, string> = {
  portrait: "立绘",
  scene: "场景",
  manga_page: "整页漫画",
  cover: "封面",
  illustration: "插图",
  video_tail_frame: "真实尾帧",
};

const imageTypeLabel = (type: string | null | undefined): string => {
  const normalized = type?.trim();
  return normalized ? IMAGE_TYPE_LABELS[normalized] ?? "素材" : "素材";
};

interface VideoFrameImageGalleryProps {
  images: GeneratedImage[];
  promptById: Map<number, ChapterVideoPrompt>;
  frameNumberByImageId?: Map<number, number>;
  label?: string;
  onToggleFavorite: (image: GeneratedImage) => void | Promise<void>;
  onOpenImageDetails: (image: GeneratedImage) => void;
}

export const VideoFrameImageGallery: React.FC<VideoFrameImageGalleryProps> = ({
  images,
  promptById,
  frameNumberByImageId,
  label = "边界帧",
  onToggleFavorite,
  onOpenImageDetails,
}) => {
  const renderGeneratedFrameImage = (image: GeneratedImage) => {
    const src = toMediaSrc(image.file_path);
    const role = videoFrameImageRoleOfImage(image);
    const promptId = videoImageGenerationPromptId(image);
    const prompt = promptId == null ? null : promptById.get(promptId) ?? null;
    const frameNumber = frameNumberByImageId?.get(image.id) ?? prompt?.shot_number ?? null;
    const roleLabel = videoFrameImageRoleLabel(role, imageTypeLabel(image.image_type));
    const frameTitle = frameNumber
      ? roleLabel === label
        ? `${label} ${frameNumber}`
        : `${roleLabel} · ${label} ${frameNumber}`
      : roleLabel;
    return (
      <li
        key={image.id}
        className="group relative min-w-0 overflow-hidden rounded-2xl border border-book-border/45 bg-book-bg-secondary/35"
      >
        <button
          type="button"
          onClick={() => onOpenImageDetails(image)}
          className="block h-full w-full text-left transition-colors duration-200 hover:bg-book-bg-tertiary/35 focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary"
        >
          <div className="flex aspect-video items-center justify-center overflow-hidden bg-book-bg-tertiary text-xs text-book-text-muted">
            {src ? (
              <img
                src={src}
                alt={frameTitle}
                className="h-full w-full object-cover transition-transform duration-300 ease-out group-hover:scale-[1.015]"
              />
            ) : (
              "无图"
            )}
          </div>
          <div className="space-y-1.5 p-3 text-xs">
            <div className="truncate text-[11px] font-medium text-book-text-main">
              {frameTitle}
            </div>
            <div className="line-clamp-2 text-[11px] leading-relaxed text-book-text-sub">
              {prompt?.title || image.prompt}
            </div>
            <div className="text-[10px] text-book-text-muted">
              {formatVideoCreatedAt(image.created_at)}
            </div>
          </div>
        </button>
        <button
          type="button"
          onClick={() => void onToggleFavorite(image)}
          className="absolute right-2 top-2 inline-flex h-8 w-8 items-center justify-center rounded-full border border-white/35 bg-black/35 text-white backdrop-blur-md transition hover:bg-black/50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
          title={image.is_favorite ? "取消收藏" : "收藏"}
          aria-label={image.is_favorite ? "取消收藏" : "收藏"}
        >
          <Star
            size={14}
            className={image.is_favorite ? "fill-current" : undefined}
          />
        </button>
      </li>
    );
  };

  return (
    <section className="flex min-h-0 flex-col" aria-label={`${label}图片`}>
      <div className="mb-3 flex shrink-0 items-baseline justify-between gap-3 px-1">
        <h3 className="text-xs font-medium text-book-text-main">{label}图片</h3>
        <span className="text-[10px] text-book-text-muted">
          {images.length === 0 ? "尚未生成" : `${images.length} 张`}
        </span>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto pr-1 custom-scrollbar">
        {images.length === 0 ? (
          <EmptyHint text={`还没有${label}图片。`} />
        ) : (
          <ul className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-4">
            {images.map(renderGeneratedFrameImage)}
          </ul>
        )}
      </div>
    </section>
  );
};
