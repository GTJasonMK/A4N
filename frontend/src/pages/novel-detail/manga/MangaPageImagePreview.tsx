import React from "react";
import { Maximize2, } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import type { GeneratedImage } from "../../../api/image";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { formatNumber, useI18n } from "../../../i18n";
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
  const { locale, t } = useI18n();
  const src = toMediaSrc(image?.file_path);
  if (src && image) {
    return (
      <div className="min-h-[360px] overflow-hidden bg-book-bg-tertiary/40">
        <button
          type="button"
          onClick={() => onOpenImage({ pageNumber: page.pageNumber, image, src })}
          className="group relative flex max-h-[60vh] min-h-[320px] w-full cursor-zoom-in items-center justify-center p-2 focus-visible:outline focus-visible:outline-1 focus-visible:outline-book-primary"
          title={t("comic.prompt.viewLarge")}
          aria-label={t("comic.prompt.viewLarge")}
        >
          <img
            src={src}
            alt={t("comic.prompt.alt", { page: page.pageNumber })}
            className="max-h-[58vh] w-full object-contain"
          />
          <span className="absolute right-2 top-2 inline-flex h-8 w-8 items-center justify-center rounded-xl border border-book-border/60 bg-book-bg-paper/90 text-book-text-sub shadow-sm opacity-100 transition-opacity sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-visible:opacity-100">
            <Maximize2 size={14} />
          </span>
        </button>
        <div className="flex flex-wrap items-center gap-2 border-t border-book-border/45 bg-book-bg-paper/70 px-3 py-2 text-small text-book-text-muted">
          <span>{t("comic.prompt.comicImage")}</span>
          <span>{formatNumber(image.width, locale)}×{formatNumber(image.height, locale)}</span>
          {image.file_size != null ? (
            <span>{formatNumber(Math.round(image.file_size / 1024), locale)} KB</span>
          ) : null}
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-[360px] items-center justify-center bg-book-bg-tertiary/40 p-4 text-xs text-book-text-muted">
      {loading ? (
        <span className="inline-flex items-center gap-2">
          <LoadingSpinner size="sm" />
          {t("comic.prompt.loadingImage")}
        </span>
      ) : page.prompt.generated_image_id != null ? (
        t("comic.prompt.imageNotLoaded")
      ) : (
        t("comic.prompt.noGeneratedImage")
      )}
    </div>
  );
};
