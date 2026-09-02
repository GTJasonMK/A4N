import type { ImageConfigInput, ImageGenConfig, UpdateImageConfigInput } from "../../../api/image";
import { DEFAULT_IMAGE_SIZES, imageSizeLabel } from "../../../api/image";
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
  supported_sizes: DEFAULT_IMAGE_SIZES.map((size) => ({ ...size })),
  max_concurrent: 1,
};

export function validateImageForm(form: ImageForm, t: Translate): string | null {
  if (!form.name.trim()) return t("settingsForm.nameRequired");
  if (!form.model.trim()) return t("settingsForm.modelRequired");
  if (!form.api_base_url.trim()) return t("settingsForm.baseUrlRequired");
  if (form.supported_sizes.length === 0) {
    return t("settingsForm.imageSizesRequired");
  }
  const seen = new Set<string>();
  for (const size of form.supported_sizes) {
    if (
      !Number.isSafeInteger(size.width) ||
      !Number.isSafeInteger(size.height) ||
      size.width <= 0 ||
      size.height <= 0 ||
      size.width > 2147483647 ||
      size.height > 2147483647
    ) {
      return t("settingsForm.imageSizeValueInvalid", {
        value: `${size.width}x${size.height}`,
      });
    }
    const key = `${size.width}x${size.height}`;
    if (seen.has(key)) return t("settingsForm.imageSizeDuplicate", { value: key });
    seen.add(key);
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
    tertiary: config.supported_sizes.map(imageSizeLabel).join(" · "),
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
      supported_sizes: raw.supported_sizes.map((size) => ({ ...size })),
      max_concurrent: raw.max_concurrent,
    },
  };
}

export function toCreateImageInput(form: ImageForm): ImageConfigInput {
  const input = buildImageConfigInput(form);
  const apiKey = form.api_key.trim();

  return {
    ...input,
    api_key_encrypted: apiKey || undefined,
  };
}

export function toUpdateImageInput(form: ImageForm): UpdateImageConfigInput {
  const apiKey = form.api_key.trim();
  return {
    ...buildImageConfigInput(form),
    api_key_encrypted: form.clear_api_key ? null : apiKey || undefined,
  };
}

function buildImageConfigInput(
  form: ImageForm,
): Omit<ImageConfigInput, "api_key_encrypted"> {
  return {
    name: form.name.trim(),
    backend: form.provider,
    base_url: form.api_base_url.trim(),
    model: form.model.trim(),
    max_concurrent: form.max_concurrent,
    supported_sizes: form.supported_sizes.map((size) => ({
      width: size.width,
      height: size.height,
    })),
  };
}
