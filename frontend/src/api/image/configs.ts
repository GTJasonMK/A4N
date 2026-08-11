import { apiInvoke } from "../index";
import { requireCurrentUserId } from "../authStorage";
import {
  withLocalRuntimeDiscoveryKey,
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
  rejectSystemDefaultMutation,
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
  const config = configs.find((item) => item.is_default) ?? null;
  if (!config) return null;
  return {
    ...config,
    id: SYSTEM_IMAGE_CONFIG_ID,
    name: `系统默认：${config.name}`,
    has_api_key: hasSystemDefaultImageApiKey(),
    is_default: usesSystemDefaultImageConfig(),
  };
}

function requiredSystemDefaultImageConfig(
  configs: ImageGenConfig[],
): ImageGenConfig {
  const config = systemDefaultImageConfig(configs);
  if (!config) throw new Error("服务器尚未配置系统默认图片模板");
  return config;
}

function systemDefaultRuntimeImageConfig(
  config: ImageGenConfig,
  apiKey?: string,
) {
  return {
    ...config,
    id: SYSTEM_IMAGE_CONFIG_ID,
    name: `系统默认：${config.name.replace(/^系统默认：/, "")}`,
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
  const config = requiredSystemDefaultImageConfig(
    await listServerImageConfigs(),
  );
  const provider = (await listModelProviderCatalog()).image.find(
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

/** 后端 add_image_config(input: { name, backend, base_url, api_key_encrypted?, model, max_concurrent, default_width, default_height }) */
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
    if (!usesLocalRuntimeSettings()) rejectSystemDefaultMutation();
    updateSystemDefaultImageApiKey(input.api_key_encrypted);
    return requiredSystemDefaultImageConfig(await listServerImageConfigs());
  }
  if (usesLocalRuntimeSettings())
    return updateLocalImageConfig(imageConfigId(configId), input);
  return updateServerImageConfig(configId, input);
}

export async function removeServerImageConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("remove_image_config", {
    configId: imageConfigId(configId),
  });
}

export async function removeImageConfig(configId: Id): Promise<void> {
  if (isSystemDefaultImageConfigId(configId)) rejectSystemDefaultMutation();
  if (usesLocalRuntimeSettings()) {
    deleteLocalImageConfig(imageConfigId(configId));
    return;
  }
  return removeServerImageConfig(configId);
}

export async function setDefaultServerImageConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("set_default_image_config", {
    callerUserId: requireCurrentUserId(),
    configId: imageConfigId(configId),
  });
}

export async function setDefaultImageConfig(configId: Id): Promise<void> {
  if (usesLocalRuntimeSettings() && isSystemDefaultImageConfigId(configId)) {
    requiredSystemDefaultImageConfig(await listServerImageConfigs());
    setSystemDefaultImageConfig();
    return;
  }
  if (usesLocalRuntimeSettings()) {
    setDefaultLocalImageConfig(imageConfigId(configId));
    return;
  }
  return setDefaultServerImageConfig(configId);
}

export async function testServerImageConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("test_image_config", {
    userId: requireCurrentUserId(),
    configId: imageConfigId(configId),
    runtime_config_snapshot: null,
  });
}

export async function testImageConfig(configId: Id): Promise<void> {
  if (isSystemDefaultImageConfigId(configId)) {
    const image = await loadSystemDefaultRuntimeImageConfig();
    return apiInvoke<void>("test_image_config", {
      userId: requireCurrentUserId(),
      configId: SYSTEM_IMAGE_CONFIG_ID,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({ image }),
    });
  }
  if (usesLocalRuntimeSettings()) {
    const id = imageConfigId(configId);
    return apiInvoke<void>("test_image_config", {
      userId: requireCurrentUserId(),
      configId: id,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({
        image: getLocalImageConfig(id),
      }),
    });
  }
  return testServerImageConfig(configId);
}

export async function previewServerTestImageConfigRequest(
  configId: Id,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_test_image_config_request", {
    userId: requireCurrentUserId(),
    configId: imageConfigId(configId),
    runtime_config_snapshot: null,
  });
}

export async function previewTestImageConfigRequest(
  configId: Id,
): Promise<ModelRequestPreview> {
  if (isSystemDefaultImageConfigId(configId)) {
    const image = await loadSystemDefaultRuntimeImageConfig({
      requireApiKey: false,
    });
    return apiInvoke<ModelRequestPreview>("preview_test_image_config_request", {
      userId: requireCurrentUserId(),
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
      userId: requireCurrentUserId(),
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
  return apiInvoke<ModelDiscoveryResult>("detect_image_config_models", {
    userId: requireCurrentUserId(),
    input,
  });
}

export async function detectImageConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return detectServerImageConfigModels(
    withLocalRuntimeDiscoveryKey(input, {
      systemConfigId: SYSTEM_IMAGE_CONFIG_ID,
      getSystemDefaultApiKey: getSystemDefaultImageApiKey,
      getLocalConfigApiKey: (configId) =>
        getLocalImageConfig(configId).api_key_encrypted,
    }),
  );
}
