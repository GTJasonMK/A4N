import { useMemo } from "react";
import {
  AlertTriangle,
  Camera,
  Image as ImageIcon,
  ImagePlus,
  Layers,
  MoreHorizontal,
  RefreshCw,
  Star,
} from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { BookCard } from "../../../components/ui/BookCard";
import type { GeneratedImage } from "../../../api/image";
import { EmptyHint, type ChapterRow } from "../shared";
import { toMediaSrc } from "../../../utils/mediaSrc";
import {
  CHAPTER_SCOPE_OPTIONS,
  compareGroupKey,
  getImageGroupKey,
  getImageGroupTitle,
  getImageSourceHash,
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
  importBusy: boolean;
  currentContentHash: string | null;
  busyId: number | null;
  scoringId: number | null;
  canEditImages: boolean;
  onScopeModeChange: (scope: ImageScope) => void;
  onFavoriteOnlyChange: (enabled: boolean) => void;
  onRefresh: () => void;
  onOpenTools: () => void;
  onImportImage: () => void;
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
  importBusy,
  currentContentHash,
  busyId,
  scoringId,
  canEditImages,
  onScopeModeChange,
  onFavoriteOnlyChange,
  onRefresh,
  onOpenTools,
  onImportImage,
  onToggleFavorite,
  onOpenActions,
}: ImageGalleryPanelProps) {
  const { images, stats } = useMemo(() => {
    const visibleImages: GeneratedImage[] = [];
    let chapterCount = 0;
    let staleCount = 0;
    let favoriteCount = 0;
    for (const img of allImages) {
      if (!favOnly || img.is_favorite) visibleImages.push(img);
      if (img.is_favorite) favoriteCount += 1;
      if (!chapter) continue;
      if (img.chapter_id === chapter.id) chapterCount += 1;
      if (isImageStale(img, chapter, currentContentHash)) staleCount += 1;
    }
    return {
      images: visibleImages,
      stats: {
        total: allImages.length,
        visible: visibleImages.length,
        chapter: chapterCount,
        stale: staleCount,
        favorite: favoriteCount,
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

  const renderImageCard = (img: GeneratedImage) => {
    const src = toMediaSrc(img.file_path);
    const sourceHash = getImageSourceHash(img.metadata_json);
    const stale = isImageStale(img, chapter, currentContentHash);
    return (
      <li key={img.id} className="a4n-stat-tile flex min-w-0 flex-col p-2">
        <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden rounded bg-book-bg-tertiary text-xs text-book-text-muted">
          {src ? (
            <img src={src} alt={`图片 ${img.id}`} className="h-full w-full object-cover" />
          ) : (
            "（无图）"
          )}
          {img.is_favorite ? (
            <span className="absolute right-1 top-1 inline-flex items-center gap-0.5 rounded bg-warning px-1.5 py-0.5 text-[10px] text-white">
              <Star size={9} fill="currentColor" />
            </span>
          ) : null}
          {stale ? (
            <span className="absolute left-1 top-1 inline-flex items-center gap-0.5 rounded bg-error px-1.5 py-0.5 text-[10px] text-white">
              <AlertTriangle size={9} />
              旧
            </span>
          ) : null}
        </div>
        <div className="mt-1.5 flex flex-wrap items-center gap-1 text-xs text-book-text-muted">
          <span className="font-mono">#{img.id}</span>
          <span className="pyqt-pill !text-[10px]">{img.image_type}</span>
          {img.ai_score != null ? (
            <span className="pyqt-pill !text-[10px]">★{img.ai_score.toFixed(2)}</span>
          ) : null}
          {img.panel_id ? (
            <span className="pyqt-pill !text-[10px] font-mono">{img.panel_id}</span>
          ) : null}
          {sourceHash ? (
            <span className={`pyqt-pill !text-[10px] font-mono ${stale ? "!text-error" : ""}`}>
              {sourceHash.slice(0, 8)}
            </span>
          ) : null}
        </div>
        <div className="mt-1 line-clamp-2 text-[10px] text-book-text-sub">{img.prompt}</div>
        <div className="mt-1.5 flex items-center gap-1">
          {canEditImages ? (
            <button
              type="button"
              onClick={() => onToggleFavorite(img)}
              disabled={busyId === img.id}
              className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
              title={img.is_favorite ? "取消收藏" : "收藏"}
            >
              <Star size={9} fill={img.is_favorite ? "currentColor" : "none"} />
            </button>
          ) : null}
          <span className="flex-1" />
          <button
            type="button"
            onClick={() => onOpenActions(img)}
            disabled={busyId === img.id || scoringId === img.id}
            className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
            title={canEditImages ? `管理图片 #${img.id}` : `查看图片 #${img.id}`}
          >
            <MoreHorizontal size={13} />
          </button>
        </div>
      </li>
    );
  };

  return (
    <BookCard className="p-4">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <h3 className="inline-flex items-center gap-2 font-serif text-sm text-book-text-main">
          <Camera size={14} className="text-book-primary" />
          {chapter ? `第 ${chapter.chapter_number} 章图像库` : "项目图像库"}
          <span className="text-xs text-book-text-muted">
            （显示 {stats.visible}/{stats.total} 张）
          </span>
        </h3>
        <div className="flex flex-wrap items-center justify-end gap-2 text-xs">
          {hasChapterContext ? (
            <select
              value={scopeMode}
              onChange={(event) => onScopeModeChange(event.target.value as ImageScope)}
              className="pyqt-input !px-2 !py-1"
            >
              {CHAPTER_SCOPE_OPTIONS.map((option) => (
                <option key={option.id} value={option.id}>
                  {option.label}
                </option>
              ))}
            </select>
          ) : null}
          <label className="inline-flex cursor-pointer items-center gap-1">
            <input
              type="checkbox"
              checked={favOnly}
              onChange={(event) => onFavoriteOnlyChange(event.target.checked)}
              className="accent-book-primary"
            />
            <Star size={11} /> 仅收藏
          </label>
          <button
            type="button"
            onClick={onRefresh}
            disabled={loading}
            className="pyqt-btn-secondary !min-h-9 !px-2 text-xs sm:!min-h-[28px]"
            title="刷新图像列表"
          >
            <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />
          </button>
          {canEditImages ? (
            <>
              <BookButton variant="secondary" size="sm" onClick={onImportImage} disabled={importBusy}>
                <span className="inline-flex items-center gap-1.5">
                  <ImagePlus size={12} /> {importBusy ? "导入中" : "导入"}
                </span>
              </BookButton>
              <BookButton variant="secondary" size="sm" onClick={onOpenTools}>
                <span className="inline-flex items-center gap-1.5">
                  <Camera size={12} /> 生成
                </span>
              </BookButton>
            </>
          ) : null}
        </div>
      </div>

      {chapter ? (
        <div className="mb-3 grid grid-cols-2 gap-2 text-[11px] sm:grid-cols-3">
          <div className="a4n-stat-tile px-2 py-1">
            <div className="text-book-text-muted">本章图片</div>
            <div className="font-mono text-book-text-main">{stats.chapter}</div>
          </div>
          <div className="a4n-stat-tile px-2 py-1">
            <div className="text-book-text-muted">过期来源</div>
            <div className={`font-mono ${stats.stale ? "text-error" : "text-book-text-main"}`}>
              {stats.stale}
            </div>
          </div>
          <div className="a4n-stat-tile px-2 py-1">
            <div className="text-book-text-muted">收藏</div>
            <div className="font-mono text-book-text-main">{stats.favorite}</div>
          </div>
        </div>
      ) : null}

      {images.length === 0 ? (
        <EmptyHint
          text={
            loading
              ? "加载中…"
              : chapter
                ? "暂无图像。点击右上「生成」可创建章节场景图。"
                : "暂无图像。点击右上「生成」可创建项目场景图。"
          }
        />
      ) : (
        <div className="space-y-4">
          {imageGroups.map(([key, groupImages]) => (
            <section key={key}>
              <div className="mb-2 flex items-center gap-2 text-xs text-book-text-sub">
                {key.startsWith("panel:") ? (
                  <Layers size={12} className="text-book-primary" />
                ) : (
                  <ImageIcon size={12} className="text-book-primary" />
                )}
                <span>{getImageGroupTitle(key, groupImages.length)}</span>
              </div>
              <ul className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
                {groupImages.map(renderImageCard)}
              </ul>
            </section>
          ))}
        </div>
      )}
    </BookCard>
  );
}
