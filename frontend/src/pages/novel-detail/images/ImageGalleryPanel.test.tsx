import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../../../i18n";
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
});
