import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../../../i18n";
import { DEFAULT_LLM_FORM } from "../llm/llmConfigUtils";
import { LlmConfigEditor } from "../llm/LlmConfigEditor";
import { DEFAULT_IMAGE_FORM } from "../image/imageConfigUtils";
import { ImageConfigEditor } from "../image/ImageConfigEditor";
import { DEFAULT_VIDEO_FORM } from "../video/videoConfigUtils";
import { VideoConfigEditor } from "../video/VideoConfigEditor";

const lockedEditors = [
  ["LLM", <LlmConfigEditor
    form={DEFAULT_LLM_FORM}
    setForm={() => undefined}
    configId={1}
    scope="local"
    templateLocked
    providers={[{
      id: "deepseek",
      display_name: "DeepSeek",
      default_base_url: "https://api.deepseek.com/v1",
      protocol: "OpenAI Chat Completions",
    }]}
  />],
  ["image", <ImageConfigEditor
    form={DEFAULT_IMAGE_FORM}
    setForm={() => undefined}
    configId={1}
    scope="local"
    templateLocked
    providers={[{
      id: "openai-compatible-image",
      display_name: "OpenAI 兼容图像",
      requires_api_key: true,
    }]}
  />],
  ["video", <VideoConfigEditor
    form={DEFAULT_VIDEO_FORM}
    setForm={() => undefined}
    configId={1}
    scope="local"
    templateLocked
    providers={[{
      id: "openai-compatible-video",
      display_name: "OpenAI 兼容视频",
      min_duration_secs: 1,
      max_duration_secs: 60,
    }]}
  />],
] as const;

describe("model discovery on locked system templates", () => {
  for (const [kind, editor] of lockedEditors) it(`disables discovery for ${kind}`, () => {
    const html = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">{editor}</I18nProvider>,
    );
    const detectButton = html.match(/<button[^>]*title="检测模型列表"[^>]*>/)?.[0];
    expect(detectButton).toContain('disabled=""');
  });
});
