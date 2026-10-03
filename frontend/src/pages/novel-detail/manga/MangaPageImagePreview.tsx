import React from "react";
import { Image as ImageIcon } from "lucide-react";
import { LoadingSpinner } from "../../../components/ui/LoadingSpinner";
import { ZoomableImage } from "../../../components/ui/ImageLightbox";
import type { GeneratedImage } from "../../../api/image";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { formatNumber, useI18n } from "../../../i18n";
import type { MangaPagePrompt } from "./mangaPagePromptModel";

export const MangaPageImagePreview: React.FC<{
  page: MangaPagePrompt;
  image: GeneratedImage | null;
  loading: boolean;
}> = ({ page, image, loading }) => {
  const { locale, t } = useI18n();
  const src = toMediaSrc(image?.file_path);
  if (src && image) {
    return (
      <div className="min-h-[360px] overflow-hidden bg-book-bg-tertiary/40">
        <div className="relative flex max-h-[60vh] min-h-[320px] w-full items-center justify-center p-2">
          <ZoomableImage
            src={src}
            alt={t("comic.prompt.alt", { page: page.pageNumber })}
            className="max-h-[58vh] w-full object-contain"
          />
        </div>
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
    <div className="flex min-h-[360px] flex-col items-center justify-center gap-2 bg-book-bg-tertiary/40 p-4 text-center text-xs text-book-text-muted">
      <ImageIcon size={20} aria-hidden="true" />
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
