import { apiInvoke } from "../index";
import {
  withLocalRuntimeDiscoveryKey,
  type ModelDiscoveryInput,
  type ModelDiscoveryResult,
} from "../modelDiscovery";
import { idNum, integerNum, type ApiId as Id } from "../shared";
import type { CreateVideoConfigInput, UpdateVideoConfigInput, VideoGenConfig } from "./types";
import {
  createLocalVideoConfig,
  deleteLocalVideoConfig,
  getSystemDefaultVideoApiKey,
  getLocalVideoConfig,
  hasSystemDefaultVideoApiKey,
  listLocalVideoConfigs,
  setDefaultLocalVideoConfig,
  setSystemDefaultVideoConfig,
  updateLocalVideoConfig,
  updateSystemDefaultVideoApiKey,
  usesSystemDefaultVideoConfig,
} from "../localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "../runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../../utils/runtimeMode";

export const SYSTEM_VIDEO_CONFIG_ID = -1;

function videoConfigId(configId: Id): number {
  return idNum(configId, "视频配置 ID");
}

function videoConfigIdOrSystem(configId: Id): number {
  const value = integerNum(configId, "视频配置 ID");
  return value === SYSTEM_VIDEO_CONFIG_ID ? value : idNum(value, "视频配置 ID");
}

function isSystemDefaultVideoConfigId(configId: Id): boolean {
  return videoConfigIdOrSystem(configId) === SYSTEM_VIDEO_CONFIG_ID;
}

function systemDefaultVideoConfig(configs: VideoGenConfig[]): VideoGenConfig | null {
  const config = configs.find((item) => item.is_default) ?? null;
  if (!config) return null;
  return {
    ...config,
    id: SYSTEM_VIDEO_CONFIG_ID,
    name: `系统默认：${config.name}`,
    api_key_encrypted: null,
    has_api_key: hasSystemDefaultVideoApiKey(),
    is_default: usesSystemDefaultVideoConfig(),
  };
}

function rejectSystemDefaultMutation(): never {
  throw new Error("系统默认配置由管理员维护，不能在用户配置页编辑");
}

function requiredSystemDefaultVideoConfig(configs: VideoGenConfig[]): VideoGenConfig {
  const config = systemDefaultVideoConfig(configs);
  if (!config) throw new Error("服务器尚未配置系统默认视频模板");
  return config;
}

function systemDefaultRuntimeVideoConfig(config: VideoGenConfig, apiKey?: string | null) {
  const normalizedApiKey = apiKey?.trim();
  return {
    ...config,
    id: SYSTEM_VIDEO_CONFIG_ID,
    name: `系统默认：${config.name.replace(/^系统默认：/, "")}`,
    api_key_encrypted: normalizedApiKey || null,
    has_api_key: Boolean(normalizedApiKey),
    is_default: true,
  };
}

async function loadSystemDefaultRuntimeVideoConfig(options?: { requireApiKey?: boolean }) {
  const requireApiKey = options?.requireApiKey ?? true;
  const apiKey = getSystemDefaultVideoApiKey()?.trim();
  if (requireApiKey && !apiKey) {
    throw new Error("使用系统默认视频配置需要先填写并保存你的个人 API Key");
  }
  const config = requiredSystemDefaultVideoConfig(await listServerVideoConfigs());
  return systemDefaultRuntimeVideoConfig(config, requireApiKey ? apiKey : undefined);
}

export async function listServerVideoConfigs(): Promise<VideoGenConfig[]> {
  return apiInvoke<VideoGenConfig[]>("list_video_configs");
}

export async function listVideoConfigs(): Promise<VideoGenConfig[]> {
  if (!usesLocalRuntimeSettings()) return listServerVideoConfigs();
  const [serverConfigs, localConfigs] = await Promise.all([
    listServerVideoConfigs(),
    Promise.resolve(listLocalVideoConfigs()),
  ]);
  const systemDefault = systemDefaultVideoConfig(serverConfigs);
  const localItems = usesSystemDefaultVideoConfig()
    ? localConfigs.map((item) => ({ ...item, is_default: false }))
    : localConfigs;
  return systemDefault ? [systemDefault, ...localItems] : localItems;
}

export async function listEffectiveVideoConfigs(): Promise<VideoGenConfig[]> {
  return listVideoConfigs();
}

export async function addServerVideoConfig(
  input: CreateVideoConfigInput,
): Promise<VideoGenConfig> {
  return apiInvoke<VideoGenConfig>("add_video_config", { input });
}

export async function addVideoConfig(input: CreateVideoConfigInput): Promise<VideoGenConfig> {
  if (usesLocalRuntimeSettings()) return createLocalVideoConfig(input);
  return addServerVideoConfig(input);
}

export async function updateServerVideoConfig(
  configId: Id,
  input: UpdateVideoConfigInput,
): Promise<VideoGenConfig> {
  return apiInvoke<VideoGenConfig>("update_video_config", {
    configId: videoConfigId(configId),
    input,
  });
}

export async function updateVideoConfig(
  configId: Id,
  input: UpdateVideoConfigInput,
): Promise<VideoGenConfig> {
  if (isSystemDefaultVideoConfigId(configId)) {
    if (!usesLocalRuntimeSettings()) rejectSystemDefaultMutation();
    updateSystemDefaultVideoApiKey(input.api_key_encrypted);
    return requiredSystemDefaultVideoConfig(await listServerVideoConfigs());
  }
  if (usesLocalRuntimeSettings()) return updateLocalVideoConfig(videoConfigId(configId), input);
  return updateServerVideoConfig(configId, input);
}

export async function removeServerVideoConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("remove_video_config", { configId: videoConfigId(configId) });
}

export async function removeVideoConfig(configId: Id): Promise<void> {
  if (isSystemDefaultVideoConfigId(configId)) rejectSystemDefaultMutation();
  if (usesLocalRuntimeSettings()) {
    deleteLocalVideoConfig(videoConfigId(configId));
    return;
  }
  return removeServerVideoConfig(configId);
}

export async function activateServerVideoConfig(configId: Id): Promise<VideoGenConfig> {
  return apiInvoke<VideoGenConfig>("activate_video_config", {
    configId: videoConfigId(configId),
  });
}

export async function activateVideoConfig(configId: Id): Promise<VideoGenConfig> {
  if (usesLocalRuntimeSettings() && isSystemDefaultVideoConfigId(configId)) {
    setSystemDefaultVideoConfig();
    const systemDefault = (await listVideoConfigs()).find(
      (item) => item.id === SYSTEM_VIDEO_CONFIG_ID,
    );
    if (!systemDefault) throw new Error("服务器未设置系统默认视频配置");
    return systemDefault;
  }
  if (usesLocalRuntimeSettings()) return setDefaultLocalVideoConfig(videoConfigId(configId));
  return activateServerVideoConfig(configId);
}

export async function testServerVideoConfig(configId: Id): Promise<boolean> {
  return apiInvoke<boolean>("test_video_config", {
    configId: videoConfigId(configId),
    runtime_config_snapshot: null,
  });
}

export async function testVideoConfig(configId: Id): Promise<boolean> {
  if (isSystemDefaultVideoConfigId(configId)) {
    const video = await loadSystemDefaultRuntimeVideoConfig();
    return apiInvoke<boolean>("test_video_config", {
      configId: SYSTEM_VIDEO_CONFIG_ID,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({ video }),
    });
  }
  if (usesLocalRuntimeSettings()) {
    const id = videoConfigId(configId);
    return apiInvoke<boolean>("test_video_config", {
      configId: id,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({ video: getLocalVideoConfig(id) }),
    });
  }
  return testServerVideoConfig(configId);
}

export async function detectServerVideoConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return apiInvoke<ModelDiscoveryResult>("detect_video_config_models", { input });
}

export async function detectVideoConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return detectServerVideoConfigModels(
    withLocalRuntimeDiscoveryKey(input, {
      systemConfigId: SYSTEM_VIDEO_CONFIG_ID,
      getSystemDefaultApiKey: getSystemDefaultVideoApiKey,
      getLocalConfigApiKey: (configId) => getLocalVideoConfig(configId).api_key_encrypted,
    }),
  );
}
