import { describe, expect, it } from "vitest";
import type { ImageProviderDescriptor } from "../../../api/generated/config";
import type { MessageTranslator } from "../../../i18n";
import { DEFAULT_IMAGE_FORM, validateImageForm } from "./imageConfigUtils";

const t = ((key: string) => key) as MessageTranslator;
const requiredKeyProvider: ImageProviderDescriptor = {
  id: "openai-compatible-image",
  display_name: "OpenAI 兼容图像",
  requires_api_key: true,
};

const readyForm = {
  ...DEFAULT_IMAGE_FORM,
  name: "测试图像",
  model: "image-model",
  api_base_url: "https://api.example/v1",
};

describe("image config validation", () => {
  it("requires a key when the selected provider contract requires one", () => {
    expect(validateImageForm(readyForm, t, requiredKeyProvider)).toBe(
      "settingsForm.apiKeyRequired",
    );
    expect(validateImageForm({ ...readyForm, has_api_key: true }, t, requiredKeyProvider)).toBeNull();
    expect(validateImageForm({ ...readyForm, has_api_key: true, clear_api_key: true }, t, requiredKeyProvider))
      .toBe("settingsForm.apiKeyRequired");
  });

  it("does not require a key for providers that do not use one", () => {
    expect(validateImageForm({ ...readyForm, provider: "comfyui" }, t, {
      id: "comfyui",
      display_name: "ComfyUI",
      requires_api_key: false,
    })).toBeNull();
  });
});
