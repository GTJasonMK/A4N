import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { GeneratedImage } from "../../../api/image";
import { I18nProvider } from "../../../i18n";
import { ImageLightboxProvider } from "../../../components/ui/ImageLightbox";
import { ImageGalleryPanel } from "./ImageGalleryPanel";

describe("ImageGalleryPanel local image import", () => {
  it("shows one import method by default and keeps path entry as a fallback", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <ImageGalleryPanel
          allImages={[]}
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
          canEditImages
          onScopeModeChange={() => undefined}
          onFavoriteOnlyChange={() => undefined}
          onRefresh={() => undefined}
          onOpenTools={() => undefined}
          onImportImageFile={() => undefined}
          onPickLocalImageFile={() => undefined}
          onToggleFavorite={() => undefined}
          onOpenActions={() => undefined}
        />
      </I18nProvider>,
    );

    expect(html).toContain('aria-label="导入图片"');
    expect(html).not.toContain("选择器没打开？手动输入");
  });

  it("renders a bounded first batch and offers the remaining project images", () => {
    const allImages = Array.from({ length: 501 }, (_, index) => ({
      ai_score: null,
      chapter_id: null,
      created_at: "2026-01-01T00:00:00.000Z",
      file_path: "",
      file_size: 0,
      height: 64,
      id: index + 1,
      image_type: "scene",
      is_favorite: false,
      is_system_managed: false,
      metadata_json: null,
      model: "fixture",
      panel_id: null,
      project_id: 1,
      prompt: "Image " + (index + 1),
      width: 64,
    })) as GeneratedImage[];
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <ImageLightboxProvider>
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
        </ImageLightboxProvider>
      </I18nProvider>,
    );

    expect(html.match(/<li\b/g)).toHaveLength(500);
    expect(html).toContain("Image 1");
    expect(html).not.toContain("Image 501");
    expect(html).toContain("已显示 500/501 张图片");
    expect(html.indexOf("已显示 500/501 张图片")).toBeLessThan(html.indexOf("Image 1"));
    expect(html).toContain("再显示 1 张");
  });
});
