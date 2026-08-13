import { apiInvoke } from "../index";
import {
  withLocalRuntimeDiscoveryKey,
  type ModelDiscoveryInput,
  type ModelDiscoveryResult,
} from "../modelDiscovery";
import { idNum, type ApiId as Id } from "../shared";
import type {
  CreateVideoConfigInput,
  UpdateVideoConfigInput,
  VideoGenConfig,
} from "./types";
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
  return (
    providerConfigIdOrSystem(
      configId,
      "视频配置 ID",
      SYSTEM_VIDEO_CONFIG_ID,
    ) === SYSTEM_VIDEO_CONFIG_ID
  );
}

export function normalizeVideoConfigId(configId?: Id | null): number | null {
  return normalizeOptionalProviderConfigId(
    configId,
    "视频配置 ID",
    SYSTEM_VIDEO_CONFIG_ID,
  );
}

function systemDefaultVideoConfig(
  configs: VideoGenConfig[],
): VideoGenConfig | null {
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

function requiredSystemDefaultVideoConfig(
  configs: VideoGenConfig[],
): VideoGenConfig {
  const config = systemDefaultVideoConfig(configs);
  if (!config) throw new Error("服务器尚未配置系统默认视频模板");
  return config;
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

export async function addVideoConfig(
  input: CreateVideoConfigInput,
): Promise<VideoGenConfig> {
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
  if (usesLocalRuntimeSettings())
    return updateLocalVideoConfig(videoConfigId(configId), input);
  return updateServerVideoConfig(configId, input);
}

export async function removeServerVideoConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("remove_video_config", {
    configId: videoConfigId(configId),
  });
}

export async function removeVideoConfig(configId: Id): Promise<void> {
  if (isSystemDefaultVideoConfigId(configId)) rejectSystemDefaultMutation();
  if (usesLocalRuntimeSettings()) {
    deleteLocalVideoConfig(videoConfigId(configId));
    return;
  }
  return removeServerVideoConfig(configId);
}

export async function setDefaultServerVideoConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("set_default_video_config", {
    configId: videoConfigId(configId),
  });
}

export async function setDefaultVideoConfig(configId: Id): Promise<void> {
  if (usesLocalRuntimeSettings() && isSystemDefaultVideoConfigId(configId)) {
    requiredSystemDefaultVideoConfig(await listServerVideoConfigs());
    setSystemDefaultVideoConfig();
    return;
  }
  if (usesLocalRuntimeSettings()) {
    setDefaultLocalVideoConfig(videoConfigId(configId));
    return;
  }
  return setDefaultServerVideoConfig(configId);
}

export async function detectServerVideoConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return apiInvoke<ModelDiscoveryResult>("detect_video_config_models", {
    input,
  });
}

export async function detectVideoConfigModels(
  input: ModelDiscoveryInput,
): Promise<ModelDiscoveryResult> {
  return detectServerVideoConfigModels(
    withLocalRuntimeDiscoveryKey(input, {
      systemConfigId: SYSTEM_VIDEO_CONFIG_ID,
      getSystemDefaultApiKey: getSystemDefaultVideoApiKey,
      getLocalConfigApiKey: (configId) =>
        getLocalVideoConfig(configId).api_key_encrypted,
    }),
  );
}
