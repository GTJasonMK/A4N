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
import { refreshPortableStorage } from "../../utils/portableStorage";
import { listModelProviderCatalog } from "../providerCatalog";
import {
  mergeLocalRuntimeConfigs,
  normalizeOptionalProviderConfigId,
  providerConfigIdOrSystem,
  requireSystemDefaultProviderConfig,
  rejectSystemDefaultMutation,
  requireSystemDefaultKeyUpdate,
  resolveProviderConnectivityTarget,
  systemDefaultProviderConfig,
} from "../config/shared";

export const SYSTEM_IMAGE_CONFIG_ID = -1;
export const IMAGE_GENERATION_CONFIG_CHANGED_EVENT = "a4n:image-generation-config-changed";
const IMAGE_GENERATION_CONFIG_CHANNEL = "a4n-image-generation-config";
let imageGenerationConfigChannel: BroadcastChannel | null = null;
let imageGenerationStorageListenerInstalled = false;

const publishImageGenerationConfigChanged = (error: Error | null = null) => {
  window.dispatchEvent(new CustomEvent<Error | null>(IMAGE_GENERATION_CONFIG_CHANGED_EVENT, {
    detail: error,
  }));
};

const asError = (reason: unknown): Error =>
  reason instanceof Error ? reason : new Error(String(reason));

const reloadImageGenerationConfigState = () => {
  void refreshPortableStorage().then(
    () => publishImageGenerationConfigChanged(),
    (reason) => publishImageGenerationConfigChanged(asError(reason)),
  );
};

const ensureImageGenerationConfigChannel = () => {
  if (typeof window === "undefined") return;
  if (!imageGenerationConfigChannel && "BroadcastChannel" in window) {
    imageGenerationConfigChannel = new BroadcastChannel(IMAGE_GENERATION_CONFIG_CHANNEL);
    imageGenerationConfigChannel.onmessage = reloadImageGenerationConfigState;
  }
  if (!imageGenerationStorageListenerInstalled) {
    window.addEventListener("storage", (event) => {
      if (event.key !== `${IMAGE_GENERATION_CONFIG_CHANNEL}:changed` || !event.newValue) return;
      reloadImageGenerationConfigState();
    });
    imageGenerationStorageListenerInstalled = true;
  }
};

export function subscribeImageGenerationConfigChanges(listener: (event: CustomEvent<Error | null>) => void): () => void {
  if (typeof window === "undefined") return () => undefined;
  ensureImageGenerationConfigChannel();
  const handleChange: EventListener = (event) => listener(event as CustomEvent<Error | null>);
  window.addEventListener(IMAGE_GENERATION_CONFIG_CHANGED_EVENT, handleChange);
  return () => {
    window.removeEventListener(IMAGE_GENERATION_CONFIG_CHANGED_EVENT, handleChange);
  };
}
function imageConfigId(configId: Id): number {
  return idNum(configId, "图片配置 ID");
}

function notifyImageGenerationConfigChanged(): void {
  ensureImageGenerationConfigChannel();
  publishImageGenerationConfigChanged();
  if (imageGenerationConfigChannel) {
    imageGenerationConfigChannel.postMessage({ changedAt: Date.now() });
  } else {
    window.localStorage.setItem(`${IMAGE_GENERATION_CONFIG_CHANNEL}:changed`, String(Date.now()));
  }
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

export async function loadImageConfigList(): Promise<{
  configs: ImageGenConfig[];
  warning?: unknown;
}> {
  if (!usesLocalRuntimeSettings()) {
    return { configs: await listServerImageConfigs() };
  }
  const [serverResult, localResult] = await Promise.allSettled([
    listServerImageConfigs(),
    Promise.resolve(listLocalImageConfigs()),
  ]);
  if (localResult.status === "rejected") throw localResult.reason;
  const serverConfigs = serverResult.status === "fulfilled" ? serverResult.value : [];
  return {
    configs: mergeLocalRuntimeConfigs(
      localResult.value,
      systemDefaultImageConfig(serverConfigs),
      usesSystemDefaultImageConfig(),
    ),
    warning: serverResult.status === "rejected" ? serverResult.reason : undefined,
  };
}

export async function listImageConfigs(): Promise<ImageGenConfig[]> {
  const loaded = await loadImageConfigList();
  if (loaded.warning !== undefined) throw loaded.warning;
  return loaded.configs;
}

export async function addServerImageConfig(
  input: ImageConfigInput,
): Promise<ImageGenConfig> {
  const created = await apiInvoke<ImageGenConfig>("add_image_config", { input });
  notifyImageGenerationConfigChanged();
  return created;
}

/** 后端 add_image_config(input: { name, backend, base_url, api_key_encrypted?, model, max_concurrent, supported_sizes }) */
export async function addImageConfig(
  input: ImageConfigInput,
): Promise<ImageGenConfig> {
  if (usesLocalRuntimeSettings()) {
    const created = await createLocalImageConfig(input);
    notifyImageGenerationConfigChanged();
    return created;
  }
  return addServerImageConfig(input);
}

export async function updateServerImageConfig(
  configId: Id,
  input: UpdateImageConfigInput,
): Promise<ImageGenConfig> {
  const updated = await apiInvoke<ImageGenConfig>("update_image_config", {
    configId: imageConfigId(configId),
    input,
  });
  notifyImageGenerationConfigChanged();
  return updated;
}

/** 后端 update_image_config(config_id, input) —— config_id 平铺 + input 包装 */
export async function updateImageConfig(
  configId: Id,
  input: UpdateImageConfigInput,
): Promise<ImageGenConfig> {
  if (isSystemDefaultImageConfigId(configId)) {
    rejectSystemDefaultMutation();
  }
  if (usesLocalRuntimeSettings()) {
    const updated = await updateLocalImageConfig(imageConfigId(configId), input);
    notifyImageGenerationConfigChanged();
    return updated;
  }
  return updateServerImageConfig(configId, input);
}

export async function updateSystemDefaultImageKey(input: Pick<UpdateImageConfigInput, "api_key_encrypted">): Promise<ImageGenConfig> {
  if (!usesLocalRuntimeSettings()) rejectSystemDefaultMutation();
  requireSystemDefaultKeyUpdate(input);
  await updateSystemDefaultImageApiKey(input.api_key_encrypted);
  return requiredSystemDefaultImageConfig(await listServerImageConfigs());
}

export async function removeServerImageConfig(configId: Id): Promise<void> {
  await apiInvoke<void>("remove_image_config", {
    configId: imageConfigId(configId),
  });
  notifyImageGenerationConfigChanged();
}

export async function removeImageConfig(configId: Id): Promise<void> {
  if (isSystemDefaultImageConfigId(configId)) rejectSystemDefaultMutation();
  if (usesLocalRuntimeSettings()) {
    await deleteLocalImageConfig(imageConfigId(configId));
    notifyImageGenerationConfigChanged();
    return;
  }
  return removeServerImageConfig(configId);
}

export async function setDefaultServerImageConfig(configId: Id): Promise<void> {
  await apiInvoke<void>("set_default_image_config", {
    configId: imageConfigId(configId),
  });
  notifyImageGenerationConfigChanged();
}

export async function setDefaultImageConfig(configId: Id): Promise<void> {
  if (usesLocalRuntimeSettings() && isSystemDefaultImageConfigId(configId)) {
    requiredSystemDefaultImageConfig(await listServerImageConfigs());
    await setSystemDefaultImageConfig();
    notifyImageGenerationConfigChanged();
    return;
  }
  if (usesLocalRuntimeSettings()) {
    await setDefaultLocalImageConfig(imageConfigId(configId));
    notifyImageGenerationConfigChanged();
    return;
  }
  await setDefaultServerImageConfig(configId);
}

async function resolveImageConnectivityTarget(
  configId: Id,
  scope: ModelConfigScope,
  requireApiKey: boolean,
): Promise<{ configId: number; config: ImageGenConfig | null }> {
  return resolveProviderConnectivityTarget({
    configId,
    scope,
    requireApiKey,
    label: "图片配置 ID",
    systemConfigId: SYSTEM_IMAGE_CONFIG_ID,
    isSystemDefault: isSystemDefaultImageConfigId,
    loadSystemDefault: (includeApiKey) =>
      loadSystemDefaultRuntimeImageConfig({ requireApiKey: includeApiKey }),
    getLocalConfig: (id) =>
      usesLocalRuntimeSettings() ? getLocalImageConfig(id) : null,
  });
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
