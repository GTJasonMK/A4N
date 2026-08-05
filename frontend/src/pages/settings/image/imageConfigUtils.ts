import type { ImageConfigInput, ImageGenConfig, UpdateImageConfigInput } from "../../../api/image";
import type { ConfigItem } from "../ConfigListPanel";
import type { ImageForm } from "./types";

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

export function validateImageForm(form: ImageForm): string | null {
  if (!form.name.trim()) return "请填写配置名称";
  if (!form.model.trim()) return "请填写模型名";
  if (!form.api_base_url.trim()) return "请填写 API Base URL";
  try {
    parseImageSize(form.size);
  } catch (err) {
    return err instanceof Error ? err.message : "图片尺寸无效";
  }
  if (!Number.isInteger(form.max_concurrent) || form.max_concurrent <= 0) {
    return "并发上限必须为正整数";
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
    isActive: Boolean(raw.is_default),
    config: {
      name: raw.name,
      provider: raw.backend,
      model: raw.model,
      api_base_url: raw.base_url ?? "",
      api_key: "",
      has_api_key: Boolean(raw.has_api_key ?? raw.api_key_encrypted),
      clear_api_key: false,
      size: imageSizeFromConfig(raw),
      max_concurrent: imageConcurrencyFromConfig(raw),
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
  const { width, height } = parseImageSize(form.size);
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

function parseImageSize(value: string): { width: number; height: number } {
  const match = value.trim().match(/^(\d+)\s*[xX]\s*(\d+)$/);
  if (!match) {
    throw new Error("图片尺寸格式必须为 宽x高，例如 3840x2160");
  }

  const width = Number(match[1]);
  const height = Number(match[2]);
  if (
    !Number.isSafeInteger(width) ||
    !Number.isSafeInteger(height) ||
    width <= 0 ||
    height <= 0
  ) {
    throw new Error(`图片尺寸无效：${value}`);
  }
  if (width > 2147483647 || height > 2147483647) {
    throw new Error("图片尺寸不能超过 2147483647");
  }
  return { width, height };
}

function imageSizeFromConfig(config: ImageGenConfig): string {
  if (
    config.default_width == null ||
    config.default_height == null ||
    !Number.isFinite(config.default_width) ||
    !Number.isFinite(config.default_height) ||
    config.default_width <= 0 ||
    config.default_height <= 0
  ) {
    throw new Error(`图片配置 ${config.id} 缺少有效默认尺寸`);
  }
  return `${config.default_width}x${config.default_height}`;
}

function imageConcurrencyFromConfig(config: ImageGenConfig): number {
  if (
    config.max_concurrent == null ||
    !Number.isInteger(config.max_concurrent) ||
    config.max_concurrent <= 0
  ) {
    throw new Error(`图片配置 ${config.id} 缺少有效并发上限`);
  }
  return config.max_concurrent;
}
