import { useMemo, type ChangeEvent } from "react";
import {
  AlertTriangle,
  Camera,
  ImagePlus,
  RefreshCw,
  Star,
} from "lucide-react";
import { BookButton, BookIconButton } from "../../../components/ui/BookButton";
import { ImageZoomButton } from "../../../components/ui/ImageLightbox";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import type { GeneratedImage } from "../../../api/image";
import { EmptyHint, type ChapterRow } from "../shared";
import { toMediaSrc } from "../../../utils/mediaSrc";
import { formatNumber, useI18n } from "../../../i18n";
import { supportedImageAccept } from "../../../utils/imageUpload";
import {
  CHAPTER_SCOPE_OPTIONS,
  compareGroupKey,
  getImageGroupKey,
  getImageGroupTitle,
  imageTypeLabel,
  isImageStale,
  type ImageScope,
} from "./imageTabUtils";

interface ImageGalleryPanelProps {
  allImages: GeneratedImage[];
  chapter: ChapterRow | null;
  hasChapterContext: boolean;
  chapterScoped: boolean;
  scopeMode: ImageScope;
  favOnly: boolean;
  loading: boolean;
  loadError: string | null;
  importBusy: boolean;
  currentContentHash: string | null;
  contentHashError: string | null;
  busyId: number | null;
  scoringId: number | null;
  canEditImages: boolean;
  onScopeModeChange: (scope: ImageScope) => void;
  onFavoriteOnlyChange: (enabled: boolean) => void;
  onRefresh: () => void;
  onOpenTools: () => void;
  onImportImageFile: (file: File) => void;
  onToggleFavorite: (image: GeneratedImage) => void;
  onOpenActions: (image: GeneratedImage) => void;
}

export function ImageGalleryPanel({
  allImages,
  chapter,
  hasChapterContext,
  chapterScoped,
  scopeMode,
  favOnly,
  loading,
  loadError,
  importBusy,
  currentContentHash,
  contentHashError,
  busyId,
  scoringId,
  canEditImages,
  onScopeModeChange,
  onFavoriteOnlyChange,
  onRefresh,
  onOpenTools,
  onImportImageFile,
  onToggleFavorite,
  onOpenActions,
}: ImageGalleryPanelProps) {
  const { locale, t } = useI18n();
  const { images, stats } = useMemo(() => {
    const visibleImages: GeneratedImage[] = [];
    let staleCount = 0;
    for (const img of allImages) {
      if (favOnly && !img.is_favorite) continue;
      visibleImages.push(img);
      if (!chapter) continue;
      if (isImageStale(img, chapter, currentContentHash)) staleCount += 1;
    }
    return {
      images: visibleImages,
      stats: {
        total: allImages.length,
        visible: visibleImages.length,
        stale: staleCount,
      },
    };
  }, [allImages, chapter, currentContentHash, favOnly]);

  const imageGroups = useMemo(() => {
    const map = new Map<string, GeneratedImage[]>();
    for (const img of images) {
      const key = getImageGroupKey(img, chapterScoped);
      const list = map.get(key);
      if (list) {
        list.push(img);
      } else {
        map.set(key, [img]);
      }
    }
    return Array.from(map.entries()).sort(([a], [b]) => compareGroupKey(a, b));
  }, [chapterScoped, images]);
  const chapterGroupsAreSingleImages =
    chapterScoped && imageGroups.every(([, groupImages]) => groupImages.length === 1);

  const gallerySummary = useMemo(() => {
    if (stats.total === 0) return null;
    const countText = favOnly
      ? t("novel.imageLibrary.shownFavorites", { count: formatNumber(stats.visible, locale) })
      : t("novel.imageLibrary.totalImages", { count: formatNumber(stats.total, locale) });
    return stats.stale > 0
      ? t("novel.imageLibrary.staleSummary", {
          summary: countText,
          count: formatNumber(stats.stale, locale),
        })
      : countText;
  }, [favOnly, locale, stats.stale, stats.total, stats.visible, t]);

  const renderImageCard = (img: GeneratedImage) => {
    const src = toMediaSrc(img.file_path);
    const stale = isImageStale(img, chapter, currentContentHash);
    const typeLabel = imageTypeLabel(img.image_type, t);
    return (
      <li
        key={img.id}
        className="group relative min-w-0 overflow-hidden rounded-[14px] border border-book-border/55 bg-book-bg-paper/45 transition-colors hover:border-book-primary/45"
      >
        <button
          type="button"
          onClick={() => onOpenActions(img)}
          disabled={busyId === img.id || scoringId === img.id}
          className="block h-full w-full text-left disabled:cursor-wait disabled:opacity-60"
          title={canEditImages ? t("novel.imageLibrary.manage") : t("novel.imageLibrary.view")}
        >
          <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden bg-book-bg-tertiary text-xs text-book-text-muted">
            {src ? (
              <img
                src={src}
                alt={img.prompt.trim() || typeLabel}
                className="h-full w-full object-cover"
              />
            ) : (
              <span>{t("novel.imageLibrary.noImage")}</span>
            )}
            {stale ? (
              <span className="absolute bottom-1.5 left-1.5 inline-flex items-center gap-0.5 rounded-md bg-warning/90 px-1.5 py-0.5 text-caption text-white backdrop-blur-sm">
                <AlertTriangle size={9} />
                {t("novel.imageLibrary.stale")}
              </span>
            ) : null}
          </div>
          <div className="p-2.5">
            <div className="flex min-w-0 items-center gap-2 text-caption text-book-text-muted">
              <span className="min-w-0 flex-1 truncate">{typeLabel}</span>
              {img.ai_score != null ? (
                <span>{t("novel.imageLibrary.score", { value: formatNumber(img.ai_score, locale, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) })}</span>
              ) : null}
            </div>
            <div className="mt-1 line-clamp-2 min-h-[2.5rem] text-small leading-relaxed text-book-text-sub">
              {img.prompt.trim() || t("novel.imageLibrary.noPrompt")}
            </div>
          </div>
        </button>
        {src ? <ImageZoomButton src={src} alt={img.prompt.trim() || typeLabel} className="absolute left-1.5 top-1.5 z-10" /> : null}
        {canEditImages ? (
          <button
            type="button"
            onClick={() => onToggleFavorite(img)}
            disabled={busyId === img.id}
            className={`absolute right-1.5 top-1.5 z-10 inline-flex h-8 w-8 items-center justify-center rounded-[10px] border backdrop-blur-md transition-colors ${
              img.is_favorite
                ? "border-warning/60 bg-warning/90 text-white"
                : "border-white/20 bg-black/25 text-white/85 hover:bg-black/45"
            }`}
            title={img.is_favorite ? t("novel.imageLibrary.unfavorite") : t("novel.imageLibrary.favorite")}
          >
            <Star size={12} fill={img.is_favorite ? "currentColor" : "none"} />
          </button>
        ) : img.is_favorite ? (
          <span className="absolute right-1.5 top-1.5 z-10 inline-flex h-8 w-8 items-center justify-center rounded-[10px] bg-warning/90 text-white backdrop-blur-md">
            <Star size={12} fill="currentColor" />
          </span>
        ) : null}
      </li>
    );
  };

  return (
    <section>
      <div className="mb-3 space-y-2.5">
        {contentHashError ? (
          <p role="alert" className="text-small text-error">{contentHashError}</p>
        ) : null}
        {hasChapterContext || stats.total > 0 ? (
          <div className="flex min-w-0 items-center gap-2 overflow-x-auto no-scrollbar">
            {hasChapterContext ? (
              <SegmentedControl
                ariaLabel={t("novel.imageLibrary.scopeAria")}
                value={scopeMode}
                onChange={onScopeModeChange}
                options={CHAPTER_SCOPE_OPTIONS.map((option) => ({
                  id: option.id,
                  label: t(option.labelKey),
                }))}
              />
            ) : null}
            {stats.total > 0 ? (
              <button
                type="button"
                className="a4n-filter-chip shrink-0"
                data-active={favOnly ? "1" : undefined}
                aria-pressed={favOnly}
                onClick={() => onFavoriteOnlyChange(!favOnly)}
              >
                <Star size={11} fill={favOnly ? "currentColor" : "none"} />
                {t("novel.imageLibrary.favoritesOnly")}
              </button>
            ) : null}
          </div>
        ) : null}

        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          {gallerySummary ? (
            <p className="min-w-0 text-small leading-relaxed text-book-text-muted">
              {gallerySummary}
            </p>
          ) : (
            <span aria-hidden="true" />
          )}
          <div className="flex w-full flex-wrap items-center gap-2 sm:w-auto sm:shrink-0">
            <BookIconButton
              label={loading ? t("novel.imageLibrary.refreshing") : t("novel.imageLibrary.refresh")}
              size="sm"
              onClick={onRefresh}
              disabled={loading}
              className="shrink-0"
            >
              <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
            </BookIconButton>
            {canEditImages ? (
              <span className="relative min-w-0 flex-1 rounded-lg focus-within:ring-2 focus-within:ring-book-border sm:flex-none">
                <BookButton
                  variant="secondary"
                  size="sm"
                  disabled={importBusy}
                  aria-hidden="true"
                  tabIndex={-1}
                  className="pointer-events-none w-full !text-xs"
                >
                  <ImagePlus size={12} />
                  {importBusy ? t("novel.imageLibrary.importing") : t("novel.imageLibrary.import")}
                </BookButton>
                <input
                  type="file"
                  accept={supportedImageAccept()}
                  aria-label={t("novel.imageLibrary.import")}
                  disabled={importBusy}
                  className="absolute inset-0 h-full w-full cursor-pointer opacity-0 disabled:cursor-not-allowed"
                  onChange={(event: ChangeEvent<HTMLInputElement>) => {
                    const input = event.currentTarget;
                    const file = input.files?.[0];
                    input.value = "";
                    if (file) onImportImageFile(file);
                  }}
                />
              </span>
            ) : null}
            {canEditImages ? (
              <BookButton
                variant="primary"
                size="sm"
                onClick={onOpenTools}
                className="min-w-0 flex-1 !text-xs sm:flex-none"
              >
                <Camera size={12} /> {t("novel.imageLibrary.generate")}
              </BookButton>
            ) : null}
          </div>
        </div>
      </div>

      {loadError ? <p role="alert" className="mb-3 text-sm text-error">{loadError}</p> : null}
      {images.length === 0 ? (loadError ? null : (
        <EmptyHint
          text={
            loading
              ? t("novel.imageLibrary.loading")
              : favOnly && stats.total > 0
                ? t("novel.imageLibrary.noFavorites")
              : chapter
                ? t("novel.imageLibrary.emptyChapter")
                : t("novel.imageLibrary.emptyProject")
          }
        />
      )) : (
        chapterScoped ? (
          chapterGroupsAreSingleImages ? (
            <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
              {imageGroups.map(([key, groupImages]) => (
                <section key={key} className="min-w-0">
                  <h3 className="mb-2 truncate text-xs font-medium text-book-text-sub">
                    {getImageGroupTitle(key, 1, t)}
                  </h3>
                  <ul>{groupImages.map(renderImageCard)}</ul>
                </section>
              ))}
            </div>
          ) : (
            <div className="space-y-4">
              {imageGroups.map(([key, groupImages]) => (
                <section key={key}>
                  <h3 className="mb-2 text-xs font-medium text-book-text-sub">
                    {getImageGroupTitle(key, groupImages.length, t)}
                  </h3>
                  <ul className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
                    {groupImages.map(renderImageCard)}
                  </ul>
                </section>
              ))}
            </div>
          )
        ) : (
          <ul className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
            {images.map(renderImageCard)}
          </ul>
        )
      )}
    </section>
  );
}
