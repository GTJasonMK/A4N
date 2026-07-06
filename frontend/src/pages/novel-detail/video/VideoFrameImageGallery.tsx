import React from "react";
import {
  PanelTop,
  Star,
} from "lucide-react";
import type { GeneratedImage } from "../../../api/image";
import type { ChapterVideoPrompt } from "../../../api/video";
import { BookCard } from "../../../components/ui/BookCard";
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
  return normalized ? IMAGE_TYPE_LABELS[normalized] ?? normalized : "素材";
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
    return (
      <li
        key={image.id}
        className="a4n-interactive-row group overflow-hidden"
      >
        <button
          type="button"
          onClick={() => onOpenImageDetails(image)}
          className="block w-full text-left focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary"
        >
          <div className="flex aspect-video items-center justify-center overflow-hidden bg-book-bg-tertiary text-xs text-book-text-muted">
            {src ? (
              <img
                src={src}
                alt={frameNumber ? `${label} ${frameNumber}` : `${label}图片 ${image.id}`}
                className="h-full w-full object-cover transition-transform duration-200 group-hover:scale-[1.02]"
              />
            ) : (
              "无图"
            )}
          </div>
          <div className="space-y-1 p-2 text-xs">
            <div className="flex min-w-0 items-center gap-1">
              <span className="font-mono text-book-text-main">#{image.id}</span>
              <span className="pyqt-pill !text-[10px]">
                {videoFrameImageRoleLabel(role, imageTypeLabel(image.image_type))}
              </span>
              {frameNumber && (
                <span className="pyqt-pill !text-[10px]">
                  {label} {frameNumber}
                </span>
              )}
              {image.is_favorite && (
                <Star size={11} className="ml-auto fill-book-primary text-book-primary" />
              )}
            </div>
            <div className="line-clamp-2 text-[11px] leading-relaxed text-book-text-sub">
              {prompt?.title || image.prompt}
            </div>
            <div className="truncate text-[10px] text-book-text-muted">
              {image.model} · {formatVideoCreatedAt(image.created_at)}
            </div>
          </div>
        </button>
        <div className="flex items-center justify-end gap-1 border-t border-book-border/40 px-2 py-1.5">
          <button
            type="button"
            onClick={() => void onToggleFavorite(image)}
            className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
            title={image.is_favorite ? "取消收藏" : "收藏"}
          >
            <Star
              size={12}
              className={image.is_favorite ? "fill-book-primary text-book-primary" : undefined}
            />
          </button>
        </div>
      </li>
    );
  };

  return (
    <BookCard className="flex min-h-0 flex-col p-3">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <h3 className="inline-flex items-center gap-2 text-sm font-serif text-book-text-main">
          <PanelTop size={14} className="text-book-primary" />
          {label}图片
          <span className="text-xs text-book-text-muted">{images.length} 张</span>
        </h3>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto custom-scrollbar pr-1">
        {images.length === 0 ? (
          <EmptyHint text={`还没有${label}图片。`} />
        ) : (
          <ul className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-4">
            {images.map(renderGeneratedFrameImage)}
          </ul>
        )}
      </div>
    </BookCard>
  );
};
