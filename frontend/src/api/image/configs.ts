import { apiInvoke } from "../index";
import {
  withLocalRuntimeDiscoveryKey,
  type ModelDiscoveryInput,
  type ModelDiscoveryResult,
} from "../modelDiscovery";
import type { ModelRequestPreview } from "../requestPreview";
import type { ImageConfigInput, ImageGenConfig, UpdateImageConfigInput } from "./types";
import { idNum, integerNum, type ApiId as Id } from "../shared";
import {
  createLocalImageConfig,
  deleteLocalImageConfig,
  getSystemDefaultImageApiKey,
  hasSystemDefaultImageApiKey,
  getLocalImageConfig,
  listLocalImageConfigs,
  setDefaultLocalImageConfig,
  setSystemDefaultImageConfig,
  updateLocalImageConfig,
  updateSystemDefaultImageApiKey,
  usesSystemDefaultImageConfig,
} from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";

export const SYSTEM_IMAGE_CONFIG_ID = -1;
const IMAGE_BACKENDS_REQUIRING_API_KEY = new Set([
  "openai-compatible-image",
  "openai-compatible-image-edit",
  "openai-chat-image",
  "cliproxy-image",
]);

function imageConfigId(configId: Id): number {
  return idNum(configId, "图片配置 ID");
}

function imageConfigIdOrSystem(configId: Id): number {
  const value = integerNum(configId, "图片配置 ID");
  return value === SYSTEM_IMAGE_CONFIG_ID ? value : idNum(value, "图片配置 ID");
}

function isSystemDefaultImageConfigId(configId: Id): boolean {
  return imageConfigIdOrSystem(configId) === SYSTEM_IMAGE_CONFIG_ID;
}

function systemDefaultImageConfig(configs: ImageGenConfig[]): ImageGenConfig | null {
  const config = configs.find((item) => item.is_default) ?? null;
  if (!config) return null;
  return {
    ...config,
    id: SYSTEM_IMAGE_CONFIG_ID,
    name: `系统默认：${config.name}`,
    api_key_encrypted: null,
    has_api_key: hasSystemDefaultImageApiKey(),
    is_default: usesSystemDefaultImageConfig(),
  };
}

function rejectSystemDefaultMutation(): never {
  throw new Error("系统默认配置由管理员维护，不能在用户配置页编辑");
}

function requiredSystemDefaultImageConfig(configs: ImageGenConfig[]): ImageGenConfig {
  const config = systemDefaultImageConfig(configs);
  if (!config) throw new Error("服务器尚未配置系统默认图片模板");
  return config;
}

function systemDefaultRuntimeImageConfig(config: ImageGenConfig, apiKey?: string | null) {
  const normalizedApiKey = apiKey?.trim();
  return {
    ...config,
    id: SYSTEM_IMAGE_CONFIG_ID,
    name: `系统默认：${config.name.replace(/^系统默认：/, "")}`,
    api_key_encrypted: normalizedApiKey || null,
    has_api_key: Boolean(normalizedApiKey),
    is_default: true,
  };
}

async function loadSystemDefaultRuntimeImageConfig(options?: { requireApiKey?: boolean }) {
  const requireApiKey = options?.requireApiKey ?? true;
  const apiKey = getSystemDefaultImageApiKey()?.trim();
  const config = requiredSystemDefaultImageConfig(await listServerImageConfigs());
  if (requireApiKey && !apiKey && IMAGE_BACKENDS_REQUIRING_API_KEY.has(config.backend)) {
    throw new Error("使用系统默认图片配置需要先填写并保存你的个人 API Key");
  }
  return systemDefaultRuntimeImageConfig(config, requireApiKey ? apiKey : undefined);
}

export async function listServerImageConfigs(): Promise<ImageGenConfig[]> {
  return apiInvoke<ImageGenConfig[]>("list_image_configs");
}

export async function listImageConfigs(): Promise<ImageGenConfig[]> {
  if (!usesLocalRuntimeSettings()) return listServerImageConfigs();
  const [serverConfigs, localConfigs] = await Promise.all([
    listServerImageConfigs(),
    Promise.resolve(listLocalImageConfigs()),
  ]);
  const systemDefault = systemDefaultImageConfig(serverConfigs);
  const localItems = usesSystemDefaultImageConfig()
    ? localConfigs.map((item) => ({ ...item, is_default: false }))
    : localConfigs;
  return systemDefault ? [systemDefault, ...localItems] : localItems;
}

export async function addServerImageConfig(input: ImageConfigInput): Promise<ImageGenConfig> {
  return apiInvoke<ImageGenConfig>("add_image_config", { input });
}

/** 后端 add_image_config(input: { name, backend, base_url, api_key_encrypted?, model, max_concurrent, default_width, default_height }) */
export async function addImageConfig(input: ImageConfigInput): Promise<ImageGenConfig> {
  if (usesLocalRuntimeSettings()) return createLocalImageConfig(input);
  return addServerImageConfig(input);
}

export async function updateServerImageConfig(
  configId: Id,
  input: UpdateImageConfigInput,
): Promise<ImageGenConfig> {
  return apiInvoke<ImageGenConfig>("update_image_config", {
    configId: imageConfigId(configId),
    input,
  });
}

/** 后端 update_image_config(config_id, input) —— config_id 平铺 + input 包装 */
export async function updateImageConfig(
  configId: Id,
  input: UpdateImageConfigInput,
): Promise<ImageGenConfig> {
  if (isSystemDefaultImageConfigId(configId)) {
    if (!usesLocalRuntimeSettings()) rejectSystemDefaultMutation();
    updateSystemDefaultImageApiKey(input.api_key_encrypted);
    return requiredSystemDefaultImageConfig(await listServerImageConfigs());
  }
  if (usesLocalRuntimeSettings()) return updateLocalImageConfig(imageConfigId(configId), input);
  return updateServerImageConfig(configId, input);
}

export async function removeServerImageConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("remove_image_config", { configId: imageConfigId(configId) });
}

export async function removeImageConfig(configId: Id): Promise<void> {
  if (isSystemDefaultImageConfigId(configId)) rejectSystemDefaultMutation();
  if (usesLocalRuntimeSettings()) {
    deleteLocalImageConfig(imageConfigId(configId));
    return;
  }
  return removeServerImageConfig(configId);
}

export async function activateServerImageConfig(configId: Id): Promise<ImageGenConfig> {
  return apiInvoke<ImageGenConfig>("activate_image_config", {
    configId: imageConfigId(configId),
  });
}

export async function activateImageConfig(configId: Id): Promise<ImageGenConfig> {
  if (usesLocalRuntimeSettings() && isSystemDefaultImageConfigId(configId)) {
    setSystemDefaultImageConfig();
    const systemDefault = (await listImageConfigs()).find(
      (item) => item.id === SYSTEM_IMAGE_CONFIG_ID,
    );
    if (!systemDefault) throw new Error("服务器未设置系统默认图片配置");
    return systemDefault;
  }
  if (usesLocalRuntimeSettings()) return setDefaultLocalImageConfig(imageConfigId(configId));
  return activateServerImageConfig(configId);
}

export async function testServerImageConfig(configId: Id): Promise<boolean> {
  return apiInvoke<boolean>("test_image_config", {
    configId: imageConfigId(configId),
    runtime_config_snapshot: null,
  });
}

/** 后端 test_image_config(_config_id) —— 返回 bool */
export async function testImageConfig(configId: Id): Promise<boolean> {
  if (isSystemDefaultImageConfigId(configId)) {
    const image = await loadSystemDefaultRuntimeImageConfig();
    return apiInvoke<boolean>("test_image_config", {
      configId: SYSTEM_IMAGE_CONFIG_ID,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({ image }),
    });
  }
  if (usesLocalRuntimeSettings()) {
    const id = imageConfigId(configId);
    return apiInvoke<boolean>("test_image_config", {
      configId: id,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({ image: getLocalImageConfig(id) }),
    });
  }
  return testServerImageConfig(configId);
}

export async function previewServerTestImageConfigRequest(configId: Id): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_test_image_config_request", {
    configId: imageConfigId(configId),
    runtime_config_snapshot: null,
  });
}

export async function previewTestImageConfigRequest(configId: Id): Promise<ModelRequestPreview> {
  if (isSystemDefaultImageConfigId(configId)) {
    const image = await loadSystemDefaultRuntimeImageConfig({ requireApiKey: false });
    return apiInvoke<ModelRequestPreview>("preview_test_image_config_request", {
      configId: SYSTEM_IMAGE_CONFIG_ID,
      runtime_config_snapshot: buildRuntimeConfigSnapshot(
        { image },
        { includeProviderApiKeys: false },
      ),
    });
  }
  if (usesLocalRuntimeSettings()) {
    const id = imageConfigId(configId);
    return apiInvoke<ModelRequestPreview>("preview_test_image_config_request", {
      configId: id,
      runtime_config_snapshot: buildRuntimeConfigSnapshot(
        { image: getLocalImageConfig(id) },
        { includeProviderApiKeys: false },
      ),
    });
  }
  return previewServerTestImageConfigRequest(configId);
}

export async function detectServerImageConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return apiInvoke<ModelDiscoveryResult>("detect_image_config_models", { input });
}

export async function detectImageConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return detectServerImageConfigModels(
    withLocalRuntimeDiscoveryKey(input, {
      systemConfigId: SYSTEM_IMAGE_CONFIG_ID,
      getSystemDefaultApiKey: getSystemDefaultImageApiKey,
      getLocalConfigApiKey: (configId) => getLocalImageConfig(configId).api_key_encrypted,
    }),
  );
}
