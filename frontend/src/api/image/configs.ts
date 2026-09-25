import { apiInvoke } from "../index";
import {
  detectConfigModels,
  type ModelConfigScope,
  type ModelDiscoveryInput,
  type ModelDiscoveryResult,
} from "../modelDiscovery";
import type { ModelRequestPreview } from "../requestPreview";
import type {
  ImageConfigInput,
  ImageGenConfig,
  UpdateImageConfigInput,
} from "./types";
import { idNum, type ApiId as Id } from "../shared";
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
import { listModelProviderCatalog } from "../providerCatalog";
import {
  mergeLocalRuntimeConfigs,
  normalizeOptionalProviderConfigId,
  providerConfigIdOrSystem,
  requireSystemDefaultProviderConfig,
  rejectSystemDefaultMutation,
  requireSystemDefaultKeyUpdate,
  systemDefaultProviderConfig,
} from "../config/shared";

export const SYSTEM_IMAGE_CONFIG_ID = -1;
function imageConfigId(configId: Id): number {
  return idNum(configId, "图片配置 ID");
}

function isSystemDefaultImageConfigId(configId: Id): boolean {
  return (
    providerConfigIdOrSystem(
      configId,
      "图片配置 ID",
      SYSTEM_IMAGE_CONFIG_ID,
    ) === SYSTEM_IMAGE_CONFIG_ID
  );
}

export function normalizeImageConfigId(configId?: Id | null): number | null {
  return normalizeOptionalProviderConfigId(
    configId,
    "图片配置 ID",
    SYSTEM_IMAGE_CONFIG_ID,
  );
}

function systemDefaultImageConfig(
  configs: ImageGenConfig[],
): ImageGenConfig | null {
  return systemDefaultProviderConfig(
    configs,
    SYSTEM_IMAGE_CONFIG_ID,
    hasSystemDefaultImageApiKey(),
    usesSystemDefaultImageConfig(),
  );
}

function requiredSystemDefaultImageConfig(
  configs: ImageGenConfig[],
): ImageGenConfig {
  return requireSystemDefaultProviderConfig(
    systemDefaultImageConfig(configs),
    "服务器尚未配置系统默认图片模板",
  );
}

function systemDefaultRuntimeImageConfig(
  config: ImageGenConfig,
  apiKey?: string,
) {
  return {
    ...config,
    id: SYSTEM_IMAGE_CONFIG_ID,
    name: `系统默认：${config.name}`,
    api_key_encrypted: apiKey ?? null,
    has_api_key: apiKey !== undefined,
    is_default: true,
  };
}

async function loadSystemDefaultRuntimeImageConfig(options?: {
  requireApiKey?: boolean;
}) {
  const requireApiKey = options?.requireApiKey ?? true;
  const apiKey = requireApiKey ? getSystemDefaultImageApiKey() : undefined;
  const [serverConfigs, providerCatalog] = await Promise.all([
    listServerImageConfigs(),
    listModelProviderCatalog(),
  ]);
  const config = requiredSystemDefaultImageConfig(serverConfigs);
  const provider = providerCatalog.image.find(
    (item) => item.id === config.backend,
  );
  if (!provider) throw new Error(`image provider 未注册：${config.backend}`);
  if (apiKey === null && provider.requires_api_key) {
    throw new Error("使用系统默认图片配置需要先填写并保存你的个人 API Key");
  }
  return systemDefaultRuntimeImageConfig(config, apiKey ?? undefined);
}

export async function listServerImageConfigs(): Promise<ImageGenConfig[]> {
  return apiInvoke<ImageGenConfig[]>("list_image_configs");
}

export async function listImageConfigs(): Promise<ImageGenConfig[]> {
  if (!usesLocalRuntimeSettings()) return listServerImageConfigs();
  const [serverConfigs, localConfigs] = await Promise.all([
    listServerImageConfigs(),
    listLocalImageConfigs(),
  ]);
  return mergeLocalRuntimeConfigs(
    localConfigs,
    systemDefaultImageConfig(serverConfigs),
    usesSystemDefaultImageConfig(),
  );
}

export async function addServerImageConfig(
  input: ImageConfigInput,
): Promise<ImageGenConfig> {
  return apiInvoke<ImageGenConfig>("add_image_config", { input });
}

/** 后端 add_image_config(input: { name, backend, base_url, api_key_encrypted?, model, max_concurrent, supported_sizes }) */
export async function addImageConfig(
  input: ImageConfigInput,
): Promise<ImageGenConfig> {
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
    rejectSystemDefaultMutation();
  }
  if (usesLocalRuntimeSettings())
    return updateLocalImageConfig(imageConfigId(configId), input);
  return updateServerImageConfig(configId, input);
}

export async function updateSystemDefaultImageKey(input: Pick<UpdateImageConfigInput, "api_key_encrypted">): Promise<ImageGenConfig> {
  if (!usesLocalRuntimeSettings()) rejectSystemDefaultMutation();
  requireSystemDefaultKeyUpdate(input);
  await updateSystemDefaultImageApiKey(input.api_key_encrypted);
  return requiredSystemDefaultImageConfig(await listServerImageConfigs());
}

export async function removeServerImageConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("remove_image_config", {
    configId: imageConfigId(configId),
  });
}

export async function removeImageConfig(configId: Id): Promise<void> {
  if (isSystemDefaultImageConfigId(configId)) rejectSystemDefaultMutation();
  if (usesLocalRuntimeSettings()) {
    return deleteLocalImageConfig(imageConfigId(configId));
  }
  return removeServerImageConfig(configId);
}

export async function setDefaultServerImageConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("set_default_image_config", {
    configId: imageConfigId(configId),
  });
}

export async function setDefaultImageConfig(configId: Id): Promise<void> {
  if (usesLocalRuntimeSettings() && isSystemDefaultImageConfigId(configId)) {
    requiredSystemDefaultImageConfig(await listServerImageConfigs());
    return setSystemDefaultImageConfig();
  }
  if (usesLocalRuntimeSettings()) {
    return setDefaultLocalImageConfig(imageConfigId(configId));
  }
  return setDefaultServerImageConfig(configId);
}

async function resolveImageConnectivityTarget(
  configId: Id,
  scope: ModelConfigScope,
  requireApiKey: boolean,
): Promise<{ configId: number; config: ImageGenConfig | null }> {
  if (scope === "server") {
    return { configId: imageConfigId(configId), config: null };
  }
  if (isSystemDefaultImageConfigId(configId)) {
    return {
      configId: SYSTEM_IMAGE_CONFIG_ID,
      config: await loadSystemDefaultRuntimeImageConfig({ requireApiKey }),
    };
  }
  const id = imageConfigId(configId);
  return {
    configId: id,
    config: usesLocalRuntimeSettings() ? getLocalImageConfig(id) : null,
  };
}

export async function testImageConfig(
  configId: Id,
  scope: ModelConfigScope,
): Promise<void> {
  const target = await resolveImageConnectivityTarget(configId, scope, true);
  return apiInvoke<void>("test_image_config", {
    configId: target.configId,
    runtime_config_snapshot: target.config
      ? buildRuntimeConfigSnapshot({ image: target.config })
      : null,
  });
}

export async function previewTestImageConfigRequest(
  configId: Id,
  scope: ModelConfigScope,
): Promise<ModelRequestPreview> {
  const target = await resolveImageConnectivityTarget(configId, scope, false);
  return apiInvoke<ModelRequestPreview>("preview_test_image_config_request", {
    configId: target.configId,
    runtime_config_snapshot: target.config
      ? buildRuntimeConfigSnapshot(
          { image: target.config },
          { includeProviderApiKeys: false },
        )
      : null,
  });
}

export async function detectImageConfigModels(
  input: ModelDiscoveryInput,
  scope: ModelConfigScope,
): Promise<ModelDiscoveryResult> {
  return detectConfigModels("detect_image_config_models", input, scope, {
    systemConfigId: SYSTEM_IMAGE_CONFIG_ID,
    getSystemDefaultApiKey: getSystemDefaultImageApiKey,
    getLocalConfigApiKey: (configId) =>
      getLocalImageConfig(configId).api_key_encrypted,
  });
}
