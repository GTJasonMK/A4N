import type { ImageConfigInput, ImageGenConfig, UpdateImageConfigInput } from "../../../api/image";
import type { ConfigItem } from "../ConfigListPanel";
import type { GrokReferenceMode, ImageForm, ImageProvider } from "./types";

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
  grok_reference_mode: "reference",
};

export const IMAGE_PROVIDERS: Array<{ value: ImageProvider; label: string }> = [
  { value: "openai-compatible-image", label: "OpenAI 兼容图像 (images/generations)" },
  {
    value: "openai-compatible-image-edit",
    label: "OpenAI 兼容图像编辑 (generations/edits，支持参考图)",
  },
  { value: "openai-chat-image", label: "Chat Completions（自动嗅探，适配中转站）" },
  { value: "cliproxy-image", label: "CLIProxy / Grok 图片 (generations/edits)" },
  { value: "stable-diffusion", label: "Stable Diffusion (Auto1111/SD-WebUI)" },
  { value: "comfyui", label: "ComfyUI 工作流" },
];

const IMAGE_PROVIDER_LABELS: Record<ImageProvider, string> = Object.fromEntries(
  IMAGE_PROVIDERS.map((provider) => [provider.value, provider.label]),
) as Record<ImageProvider, string>;

export const IMAGE_BASE_URL_HINTS: Record<ImageProvider, string> = {
  "openai-compatible-image":
    "走 POST {base}/images/generations，从 data[0].b64_json 或 data[0].url 取图片，兼容无 format 响应",
  "openai-compatible-image-edit":
    "无参考图走 POST {base}/images/generations；有参考图走 multipart POST {base}/images/edits",
  "openai-chat-image":
    "走 POST {base}/chat/completions，解析 message.images[] / content[].image_url / content 字符串中的图片引用",
  "cliproxy-image":
    "走 CLIProxy POST {base}/images/generations；有参考图时走 multipart {base}/images/edits；默认 response_format=url",
  "stable-diffusion": "本地 SD-WebUI 通常为 http://localhost:7860",
  comfyui: "本地 ComfyUI 通常为 http://localhost:8188",
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

export const GROK_REFERENCE_MODE_OPTIONS: Array<{
  value: GrokReferenceMode;
  label: string;
  hint: string;
}> = [
  {
    value: "reference",
    label: "参考模式",
    hint: "上传全部参考图作为图片编辑参考，保持当前默认行为。",
  },
  {
    value: "first_frame",
    label: "首帧模式",
    hint: "只上传第一张参考图作为编辑输入，其余参考图不发送。",
  },
];

export function parseImageProvider(value: string): ImageProvider {
  if (Object.prototype.hasOwnProperty.call(IMAGE_PROVIDER_LABELS, value)) {
    return value as ImageProvider;
  }
  throw new Error(`图片生成后端未注册：${value}`);
}

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
    secondary: `${IMAGE_PROVIDER_LABELS[config.provider]} · ${config.model}`,
    tertiary: config.size,
  };
}

export function toImageItem(raw: ImageGenConfig): ConfigItem<ImageForm> {
  return {
    id: raw.id,
    isActive: Boolean(raw.is_default),
    config: {
      name: raw.name,
      provider: parseImageProvider(raw.backend),
      model: raw.model,
      api_base_url: raw.base_url ?? "",
      api_key: "",
      has_api_key: Boolean(raw.has_api_key ?? raw.api_key_encrypted),
      clear_api_key: false,
      size: imageSizeFromConfig(raw),
      max_concurrent: imageConcurrencyFromConfig(raw),
      grok_reference_mode: grokReferenceModeFromConfig(raw),
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
    provider_options_json: imageProviderOptions(form),
  };
}

function imageProviderOptions(form: ImageForm): ImageConfigInput["provider_options_json"] {
  if (form.provider !== "cliproxy-image") return null;
  return { grok_reference_mode: form.grok_reference_mode };
}

function grokReferenceModeFromConfig(config: ImageGenConfig): GrokReferenceMode {
  const value = config.provider_options_json;
  if (value == null) return "reference";
  if (config.backend !== "cliproxy-image") {
    throw new Error(
      `图片配置 ${config.id} provider_options_json 仅支持 cliproxy-image 后端`,
    );
  }
  if (typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`图片配置 ${config.id} provider_options_json 必须是 JSON 对象`);
  }
  const payload = value as Record<string, unknown>;
  const unknownKey = Object.keys(payload).find((key) => key !== "grok_reference_mode");
  if (unknownKey) {
    throw new Error(`图片配置 ${config.id} provider_options_json 包含未知字段：${unknownKey}`);
  }
  const mode = payload.grok_reference_mode;
  if (mode == null) return "reference";
  if (typeof mode !== "string") {
    throw new Error(
      `图片配置 ${config.id} provider_options_json.grok_reference_mode 必须是字符串`,
    );
  }
  const normalized = mode.trim();
  if (normalized === "reference" || normalized === "first_frame") return normalized;
  throw new Error(
    `图片配置 ${config.id} provider_options_json.grok_reference_mode 无效：${normalized}`,
  );
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
