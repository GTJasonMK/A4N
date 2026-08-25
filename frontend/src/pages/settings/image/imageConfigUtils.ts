import type { ImageConfigInput, ImageGenConfig, UpdateImageConfigInput } from "../../../api/image";
import type { ConfigItem } from "../ConfigListPanel";
import type { ImageForm } from "./types";
import type { MessageKey, MessageParams } from "../../../i18n";

type Translate = (key: MessageKey, params?: MessageParams) => string;

export const DEFAULT_IMAGE_FORM: ImageForm = {
  name: "",
  provider: "openai-compatible-image",
  model: "dall-e-3",
  api_base_url: "",
  api_key: "",
  has_api_key: false,
  clear_api_key: false,
  size: "1024x1024",
  max_concurrent: 1,
};

export const IMAGE_SIZE_OPTIONS = [
  "512x512",
  "768x768",
  "1024x1024",
  "1024x1536",
  "1536x1024",
  "1024x1792",
  "1792x1024",
  "1920x1080",
  "1080x1920",
  "2048x2048",
  "3840x2160",
  "2160x3840",
  "4096x2160",
  "2160x4096",
  "4096x4096",
];

export function validateImageForm(form: ImageForm, t: Translate): string | null {
  if (!form.name.trim()) return t("settingsForm.nameRequired");
  if (!form.model.trim()) return t("settingsForm.modelRequired");
  if (!form.api_base_url.trim()) return t("settingsForm.baseUrlRequired");
  try {
    parseImageSize(form.size, t);
  } catch (err) {
    return err instanceof Error ? err.message : t("settingsForm.imageSizeInvalid");
  }
  if (!Number.isInteger(form.max_concurrent) || form.max_concurrent <= 0) {
    return t("settingsForm.concurrencyPositive");
  }
  return null;
}

export function formatImageItem(config: ImageForm): {
  primary: string;
  secondary: string;
  tertiary: string;
} {
  return {
    primary: config.name,
    secondary: `${config.provider} · ${config.model}`,
    tertiary: config.size,
  };
}

export function toImageItem(raw: ImageGenConfig): ConfigItem<ImageForm> {
  return {
    id: raw.id,
    isActive: raw.is_default,
    config: {
      name: raw.name,
      provider: raw.backend,
      model: raw.model,
      api_base_url: raw.base_url,
      api_key: "",
      has_api_key: raw.has_api_key,
      clear_api_key: false,
      size: `${raw.default_width}x${raw.default_height}`,
      max_concurrent: raw.max_concurrent,
    },
  };
}

export function toCreateImageInput(form: ImageForm, t: Translate): ImageConfigInput {
  const input = buildImageConfigInput(form, t);
  const apiKey = form.api_key.trim();

  return {
    ...input,
    api_key_encrypted: apiKey || undefined,
  };
}

export function toUpdateImageInput(form: ImageForm, t: Translate): UpdateImageConfigInput {
  const apiKey = form.api_key.trim();
  return {
    ...buildImageConfigInput(form, t),
    api_key_encrypted: form.clear_api_key ? null : apiKey || undefined,
  };
}

function buildImageConfigInput(
  form: ImageForm,
  t: Translate,
): Omit<ImageConfigInput, "api_key_encrypted"> {
  const { width, height } = parseImageSize(form.size, t);
  return {
    name: form.name.trim(),
    backend: form.provider,
    base_url: form.api_base_url.trim(),
    model: form.model.trim(),
    max_concurrent: form.max_concurrent,
    default_width: width,
    default_height: height,
  };
}

function parseImageSize(value: string, t: Translate): { width: number; height: number } {
  const match = value.trim().match(/^(\d+)\s*[xX]\s*(\d+)$/);
  if (!match) {
    throw new Error(t("settingsForm.imageSizeFormat"));
  }

  const width = Number(match[1]);
  const height = Number(match[2]);
  if (
    !Number.isSafeInteger(width) ||
    !Number.isSafeInteger(height) ||
    width <= 0 ||
    height <= 0
  ) {
    throw new Error(t("settingsForm.imageSizeValueInvalid", { value }));
  }
  if (width > 2147483647 || height > 2147483647) {
    throw new Error(t("settingsForm.imageSizeTooLarge"));
  }
  return { width, height };
}
