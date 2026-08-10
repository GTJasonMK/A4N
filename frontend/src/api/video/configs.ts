import { apiInvoke } from "../index";
import {
  withLocalRuntimeDiscoveryKey,
  type ModelDiscoveryInput,
  type ModelDiscoveryResult,
} from "../modelDiscovery";
import { idNum, type ApiId as Id } from "../shared";
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
import {
  mergeLocalRuntimeConfigs,
  normalizeOptionalProviderConfigId,
  providerConfigIdOrSystem,
  rejectSystemDefaultMutation,
} from "../config/shared";

export const SYSTEM_VIDEO_CONFIG_ID = -1;

function videoConfigId(configId: Id): number {
  return idNum(configId, "视频配置 ID");
}

function isSystemDefaultVideoConfigId(configId: Id): boolean {
  return providerConfigIdOrSystem(
    configId,
    "视频配置 ID",
    SYSTEM_VIDEO_CONFIG_ID,
  ) === SYSTEM_VIDEO_CONFIG_ID;
}

export function normalizeVideoConfigId(configId?: Id | null): number | null {
  return normalizeOptionalProviderConfigId(
    configId,
    "视频配置 ID",
    SYSTEM_VIDEO_CONFIG_ID,
  );
}

function systemDefaultVideoConfig(configs: VideoGenConfig[]): VideoGenConfig | null {
  const config = configs.find((item) => item.is_default) ?? null;
  if (!config) return null;
  return {
    ...config,
    id: SYSTEM_VIDEO_CONFIG_ID,
    name: `系统默认：${config.name}`,
    has_api_key: hasSystemDefaultVideoApiKey(),
    is_default: usesSystemDefaultVideoConfig(),
  };
}

function requiredSystemDefaultVideoConfig(configs: VideoGenConfig[]): VideoGenConfig {
  const config = systemDefaultVideoConfig(configs);
  if (!config) throw new Error("服务器尚未配置系统默认视频模板");
  return config;
}

function systemDefaultRuntimeVideoConfig(config: VideoGenConfig, apiKey?: string) {
  return {
    ...config,
    id: SYSTEM_VIDEO_CONFIG_ID,
    name: `系统默认：${config.name.replace(/^系统默认：/, "")}`,
    api_key_encrypted: apiKey ?? null,
    has_api_key: apiKey !== undefined,
    is_default: true,
  };
}

async function loadSystemDefaultRuntimeVideoConfig(options?: { requireApiKey?: boolean }) {
  const requireApiKey = options?.requireApiKey ?? true;
  const apiKey = requireApiKey ? getSystemDefaultVideoApiKey() : undefined;
  if (apiKey === null) {
    throw new Error("使用系统默认视频配置需要先填写并保存你的个人 API Key");
  }
  const config = requiredSystemDefaultVideoConfig(await listServerVideoConfigs());
  return systemDefaultRuntimeVideoConfig(config, apiKey);
}

export async function listServerVideoConfigs(): Promise<VideoGenConfig[]> {
  return apiInvoke<VideoGenConfig[]>("list_video_configs");
}

export async function listVideoConfigs(): Promise<VideoGenConfig[]> {
  if (!usesLocalRuntimeSettings()) return listServerVideoConfigs();
  const [serverConfigs, localConfigs] = await Promise.all([
    listServerVideoConfigs(),
    listLocalVideoConfigs(),
  ]);
  return mergeLocalRuntimeConfigs(
    localConfigs,
    systemDefaultVideoConfig(serverConfigs),
    usesSystemDefaultVideoConfig(),
  );
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

export async function activateServerVideoConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("activate_video_config", {
    configId: videoConfigId(configId),
  });
}

export async function activateVideoConfig(configId: Id): Promise<void> {
  if (usesLocalRuntimeSettings() && isSystemDefaultVideoConfigId(configId)) {
    setSystemDefaultVideoConfig();
    requiredSystemDefaultVideoConfig(await listServerVideoConfigs());
    return;
  }
  if (usesLocalRuntimeSettings()) {
    setDefaultLocalVideoConfig(videoConfigId(configId));
    return;
  }
  return activateServerVideoConfig(configId);
}

export async function testServerVideoConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("test_video_config", {
    configId: videoConfigId(configId),
    runtime_config_snapshot: null,
  });
}

export async function testVideoConfig(configId: Id): Promise<void> {
  if (isSystemDefaultVideoConfigId(configId)) {
    const video = await loadSystemDefaultRuntimeVideoConfig();
    return apiInvoke<void>("test_video_config", {
      configId: SYSTEM_VIDEO_CONFIG_ID,
      runtime_config_snapshot: buildRuntimeConfigSnapshot({ video }),
    });
  }
  if (usesLocalRuntimeSettings()) {
    const id = videoConfigId(configId);
    return apiInvoke<void>("test_video_config", {
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
