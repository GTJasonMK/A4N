import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../../i18n";
import { SceneGenerationPanel } from "./SceneGenerationPanel";

describe("SceneGenerationPanel", () => {
  it("does not preview or generate while the scene list is unknown", () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <SceneGenerationPanel
          nextSceneNumber={null}
          prompt="prompt"
          generating={false}
          canGenerate={false}
          hasUnsavedSceneChanges={false}
          previewing={false}
          aiActionBlockedReason={null}
          onPromptChange={() => undefined}
          onPreviewRequest={() => undefined}
          onGenerate={() => undefined}
        />
      </I18nProvider>,
    );

    expect(html).not.toContain("将创建场景 1");
    const generateButton = [...html.matchAll(/<button\b[\s\S]*?<\/button>/g)]
      .map(([button]) => button)
      .find((button) => button.includes("生成场景"));
    expect(generateButton).toContain('disabled=""');
  });
});
