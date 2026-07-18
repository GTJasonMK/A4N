import React from "react";
import {
  Loader2,
  Maximize2,
} from "lucide-react";
import type { GeneratedImage } from "../../../api/image";
import { toMediaSrc } from "../../../utils/mediaSrc";
import type { MangaPagePrompt } from "./mangaPagePromptModel";

export type ImagePreviewTarget = {
  pageNumber: number;
  image: GeneratedImage;
  src: string;
};

export const MangaPageImagePreview: React.FC<{
  page: MangaPagePrompt;
  image: GeneratedImage | null;
  loading: boolean;
  onOpenImage: (target: ImagePreviewTarget) => void;
}> = ({ page, image, loading, onOpenImage }) => {
  const src = toMediaSrc(image?.file_path);
  if (src && image) {
    return (
      <div className="min-h-[360px] overflow-hidden bg-book-bg-tertiary/40">
        <button
          type="button"
          onClick={() => onOpenImage({ pageNumber: page.pageNumber, image, src })}
          className="group relative flex max-h-[60vh] min-h-[320px] w-full cursor-zoom-in items-center justify-center p-2 focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary"
          title="查看大图"
        >
          <img
            src={src}
            alt={`第 ${page.pageNumber} 页漫画`}
            className="max-h-[58vh] w-full object-contain"
          />
          <span className="absolute right-2 top-2 inline-flex h-8 w-8 items-center justify-center rounded-xl border border-book-border/60 bg-book-bg-paper/90 text-book-text-sub shadow-sm opacity-100 transition-opacity sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-visible:opacity-100">
            <Maximize2 size={14} />
          </span>
        </button>
        <div className="flex flex-wrap items-center gap-2 border-t border-book-border/45 bg-book-bg-paper/70 px-3 py-2 text-[11px] text-book-text-muted">
          <span>漫画成图</span>
          <span>{image.width}×{image.height}</span>
          {image.file_size != null ? (
            <span>{Math.round(image.file_size / 1024)} KB</span>
          ) : null}
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-[360px] items-center justify-center bg-book-bg-tertiary/40 p-4 text-xs text-book-text-muted">
      {loading ? (
        <span className="inline-flex items-center gap-2">
          <Loader2 size={13} className="animate-spin" />
          加载图片素材…
        </span>
      ) : page.prompt.generated_image_id != null ? (
        "图片暂未加载"
      ) : (
        "暂无成图"
      )}
    </div>
  );
};
