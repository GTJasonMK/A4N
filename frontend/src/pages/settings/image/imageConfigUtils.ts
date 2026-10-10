import type { ImageConfigInput, ImageGenConfig, UpdateImageConfigInput } from "../../../api/image";
import { DEFAULT_IMAGE_SIZES, imageSizeLabel } from "../../../api/image";
import type { ConfigItem } from "../ConfigListPanel";
import type { ImageForm } from "./types";
import type { MessageKey, MessageParams } from "../../../i18n";
import type { ImageProviderDescriptor } from "../../../api/generated/config";
import { clonePricingPlan, DEFAULT_PRICING_PLAN } from "../config-list/pricing";
import { isPositiveSafeInteger } from "../../../utils/numberRange";

type Translate = (key: MessageKey, params?: MessageParams) => string;

export const DEFAULT_IMAGE_FORM: ImageForm = {
  name: "",
  provider: "openai-compatible-image",
  model: "dall-e-3",
  api_base_url: "",
  api_key: "",
  has_api_key: false,
  clear_api_key: false,
  supported_sizes: DEFAULT_IMAGE_SIZES.map((size) => ({ width: String(size.width), height: String(size.height) })),
  max_concurrent: "1",
  pricing: DEFAULT_PRICING_PLAN,
};

export function validateImageForm(
  form: ImageForm,
  t: Translate,
  provider?: ImageProviderDescriptor,
): string | null {
  if (!form.name.trim()) return t("settingsForm.nameRequired");
  if (!form.model.trim()) return t("settingsForm.modelRequired");
  if (!form.api_base_url.trim()) return t("settingsForm.baseUrlRequired");
  if (
    provider?.id === form.provider &&
    provider.requires_api_key &&
    !form.api_key.trim() &&
    (!form.has_api_key || form.clear_api_key)
  ) {
    return t("settingsForm.apiKeyRequired");
  }
  if (form.supported_sizes.length === 0) {
    return t("settingsForm.imageSizesRequired");
  }
  const seen = new Set<string>();
  for (const size of form.supported_sizes) {
    const width = Number(size.width);
    const height = Number(size.height);
    if (
      !isPositiveSafeInteger(width) ||
      !isPositiveSafeInteger(height) ||
      width > 2147483647 ||
      height > 2147483647
    ) {
      return t("settingsForm.imageSizeValueInvalid", {
        value: `${size.width}x${size.height}`,
      });
    }
    const key = `${width}x${height}`;
    if (seen.has(key)) return t("settingsForm.imageSizeDuplicate", { value: key });
    seen.add(key);
  }
  if (!isPositiveSafeInteger(Number(form.max_concurrent))) {
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
    tertiary: config.supported_sizes.map((size) => imageSizeLabel({ width: Number(size.width), height: Number(size.height) })).join(" · "),
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
      supported_sizes: raw.supported_sizes.map((size) => ({ width: String(size.width), height: String(size.height) })),
      max_concurrent: String(raw.max_concurrent),
      pricing: clonePricingPlan(raw.pricing),
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
    max_concurrent: Number(form.max_concurrent),
    supported_sizes: form.supported_sizes.map((size) => ({
      width: Number(size.width),
      height: Number(size.height),
    })),
    pricing: form.pricing,
  };
}
