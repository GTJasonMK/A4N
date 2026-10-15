import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import type { GeneratedImage } from "../../src/api/image";
import { I18nProvider } from "../../src/i18n";
import { ImageLightboxProvider } from "../../src/components/ui/ImageLightbox";
import { ImageGalleryPanel } from "../../src/pages/novel-detail/images/ImageGalleryPanel";
import "../../src/index.css";

const count = Number(new URLSearchParams(location.search).get("images") ?? 1000);
const runId = new URLSearchParams(location.search).get("run") ?? "default";
const allImages = Array.from({ length: count }, (_, index) => ({
  ai_score: null,
  chapter_id: null,
  created_at: "2026-01-01T00:00:00.000Z",
  file_path: "/perf-media/" + (index + 1) + ".svg?run=" + runId,
  file_size: 2048,
  height: 64,
  id: index + 1,
  image_type: "scene",
  is_favorite: index % 20 === 0,
  is_system_managed: false,
  metadata_json: null,
  model: "fixture",
  panel_id: null,
  project_id: 1,
  prompt: "Image " + (index + 1),
  width: 64,
})) as GeneratedImage[];

performance.setResourceTimingBufferSize(Math.max(2500, count + 100));
const root = createRoot(document.getElementById("root")!);
const startedAt = performance.now();
flushSync(() => {
  root.render(
    <I18nProvider initialLocale="zh-CN">
      <ImageLightboxProvider>
        <div id="image-gallery-scroll" style={{ height: "100vh", overflowY: "auto" }}>
          <ImageGalleryPanel
            allImages={allImages}
            chapter={null}
            hasChapterContext={false}
            chapterScoped={false}
            scopeMode="project"
            favOnly={false}
            loading={false}
            loadError={null}
            importBusy={false}
            currentContentHash={null}
            contentHashError={null}
            busyId={null}
            scoringId={null}
            canEditImages={false}
            onScopeModeChange={() => undefined}
            onFavoriteOnlyChange={() => undefined}
            onRefresh={() => undefined}
            onOpenTools={() => undefined}
            onImportImageFile={() => undefined}
            onToggleFavorite={() => undefined}
            onOpenActions={() => undefined}
          />
        </div>
      </ImageLightboxProvider>
    </I18nProvider>,
  );
});
const mountMs = performance.now() - startedAt;

window.addEventListener("load", () => {
  const imageCount = () => document.querySelectorAll('img[src^="/perf-media/"]').length;
  const firstBatchImageCount = imageCount();
  const resources = performance.getEntriesByType("resource")
    .filter((entry) => entry.name.includes("/perf-media/"));
  (window as Window & { gc?: () => void }).gc?.();
  const memory = performance as Performance & { memory?: { usedJSHeapSize: number } };
  const heapAfterFirstBatchBytes = memory.memory?.usedJSHeapSize ?? null;
  const params = new URLSearchParams(location.search);
  const scroller = document.getElementById("image-gallery-scroll");
  if (params.get("scrollBottom") === "1" && scroller) {
    scroller.scrollTop = scroller.scrollHeight;
  }
  const expandAll = params.get("expand") === "all";
  let loadMoreClickCount = 0;
  let loadMoreButton = [...document.querySelectorAll<HTMLButtonElement>("button")]
    .find((button) => button.textContent?.includes("再显示"));
  while (expandAll && loadMoreButton) {
    flushSync(() => loadMoreButton?.click());
    loadMoreClickCount += 1;
    loadMoreButton = [...document.querySelectorAll<HTMLButtonElement>("button")]
      .find((button) => button.textContent?.includes("再显示"));
  }
  document.getElementById("image-gallery-report")!.textContent = JSON.stringify({
    count,
    mountMs: Number(mountMs.toFixed(2)),
    totalImageCount: document.querySelectorAll("li").length,
    firstBatchImageCount,
    afterLoadMoreImageCount: imageCount(),
    loadMoreClickCount,
    requestedImageCount: resources.length,
    requestedImageTransferBytes: resources.reduce((total, entry) => total + entry.transferSize, 0),
    heapAfterFirstBatchBytes,
    viewportWidth: document.documentElement.clientWidth,
    documentScrollWidth: document.documentElement.scrollWidth,
    overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    scrollerScrollTop: scroller?.scrollTop ?? null,
    loadMoreButtonPresent: loadMoreButton != null,
    loadMoreButtonTop: loadMoreButton?.getBoundingClientRect().top ?? null,
  });
}, { once: true });
