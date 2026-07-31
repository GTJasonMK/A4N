import { useMemo } from "react";
import {
  AlertTriangle,
  Camera,
  ImagePlus,
  RefreshCw,
  Star,
} from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { Dropdown } from "../../../components/ui/Dropdown";
import { SegmentedControl } from "../../../components/ui/SegmentedControl";
import type { GeneratedImage } from "../../../api/image";
import { EmptyHint, type ChapterRow } from "../shared";
import { toMediaSrc } from "../../../utils/mediaSrc";
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
      ? `显示 ${stats.visible} 张收藏图片`
      : `共 ${stats.total} 张图片`;
    return stats.stale > 0 ? `${countText}，其中 ${stats.stale} 张需要更新` : countText;
  }, [favOnly, stats.stale, stats.total, stats.visible]);

  const renderImageCard = (img: GeneratedImage) => {
    const src = toMediaSrc(img.file_path);
    const stale = isImageStale(img, chapter, currentContentHash);
    const typeLabel = imageTypeLabel(img.image_type);
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
          title={canEditImages ? "管理图片" : "查看图片"}
        >
          <div className="relative flex aspect-square w-full items-center justify-center overflow-hidden bg-book-bg-tertiary text-xs text-book-text-muted">
            {src ? (
              <img
                src={src}
                alt={img.prompt.trim() || typeLabel}
                className="h-full w-full object-cover transition-transform duration-300 ease-out group-hover:scale-[1.02]"
              />
            ) : (
              "无图"
            )}
            {stale ? (
              <span className="absolute left-1.5 top-1.5 inline-flex items-center gap-0.5 rounded-md bg-warning/90 px-1.5 py-0.5 text-caption text-white backdrop-blur-sm">
                <AlertTriangle size={9} />
                需更新
              </span>
            ) : null}
          </div>
          <div className="p-2.5">
            <div className="flex min-w-0 items-center gap-2 text-caption text-book-text-muted">
              <span className="min-w-0 flex-1 truncate">{typeLabel}</span>
              {img.ai_score != null ? <span>评分 {img.ai_score.toFixed(2)}</span> : null}
            </div>
            <div className="mt-1 line-clamp-2 min-h-[2.5rem] text-small leading-relaxed text-book-text-sub">
              {img.prompt.trim() || "没有提示词说明"}
            </div>
          </div>
        </button>
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
            title={img.is_favorite ? "取消收藏" : "收藏"}
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
        {hasChapterContext || stats.total > 0 ? (
          <div className="flex min-w-0 items-center gap-2 overflow-x-auto no-scrollbar">
            {hasChapterContext ? (
              <SegmentedControl
                ariaLabel="图像范围"
                value={scopeMode}
                onChange={onScopeModeChange}
                options={CHAPTER_SCOPE_OPTIONS.map((option) => ({
                  id: option.id,
                  label: option.label,
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
                仅看收藏
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
          <div className="flex w-full items-center gap-2 sm:w-auto sm:shrink-0">
            <Dropdown
              items={[
                {
                  label: loading ? "正在刷新…" : "刷新图库",
                  icon: <RefreshCw size={12} className={loading ? "animate-spin" : undefined} />,
                  onClick: onRefresh,
                  disabled: loading,
                },
                ...(canEditImages
                  ? [
                      {
                        label: importBusy ? "正在导入…" : "导入图片",
                        icon: <ImagePlus size={12} />,
                        onClick: onImportImage,
                        disabled: importBusy,
                      },
                    ]
                  : []),
              ]}
            />
            {canEditImages ? (
              <BookButton
                variant="primary"
                size="sm"
                onClick={onOpenTools}
                className="min-w-0 flex-1 !text-xs sm:flex-none"
              >
                <Camera size={12} /> 生成图片
              </BookButton>
            ) : null}
          </div>
        </div>
      </div>

      {images.length === 0 ? (
        <EmptyHint
          text={
            loading
              ? "加载中…"
              : chapter
                ? "暂无图像。可通过“生成”创建章节场景图。"
                : "暂无图像。可通过“生成”创建项目场景图。"
          }
        />
      ) : (
        chapterScoped ? (
          chapterGroupsAreSingleImages ? (
            <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
              {imageGroups.map(([key, groupImages]) => (
                <section key={key} className="min-w-0">
                  <h3 className="mb-2 truncate text-xs font-medium text-book-text-sub">
                    {getImageGroupTitle(key, 1)}
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
                    {getImageGroupTitle(key, groupImages.length)}
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
