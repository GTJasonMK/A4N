import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../../../i18n";
import { ConfigEditorPanel } from "./ConfigEditorPanel";

describe("ConfigEditorPanel submission semantics", () => {
  it("uses a native form and submit button for field constraint validation", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <ConfigEditorPanel<{ maxConcurrent: string }>
          title="图像配置"
          itemCount={0}
          creating
          selectedItem={null}
          form={{ maxConcurrent: "1" }}
          dirty
          validationError={null}
          busy={false}
          saving={false}
          activating={null}
          deleting={null}
          testing={null}
          previewing={false}
          testResult={null}
          canTest={false}
          canPreviewTest={false}
          activateText="设为默认"
          renderEditor={(form) => (
            <input
              aria-label="并发上限"
              type="number"
              min={1}
              max={8}
              value={form.maxConcurrent}
              readOnly
            />
          )}
          onFormChange={() => undefined}
          onCancel={() => undefined}
          onSave={() => undefined}
          onActivate={() => undefined}
          onDelete={() => undefined}
          onPreviewTest={() => undefined}
          onTest={() => undefined}
        />
      </I18nProvider>,
    );

    expect(html).toContain("<form");
    expect(html).toContain('type="submit"');
    expect(html).toContain('min="1" max="8"');
  });
});
