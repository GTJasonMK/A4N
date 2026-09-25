import { apiInvoke } from "../index";
import {
  detectConfigModels,
  type ModelConfigScope,
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
  requireSystemDefaultProviderConfig,
  rejectSystemDefaultMutation,
  requireSystemDefaultKeyUpdate,
  systemDefaultProviderConfig,
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
  return systemDefaultProviderConfig(
    configs,
    SYSTEM_VIDEO_CONFIG_ID,
    hasSystemDefaultVideoApiKey(),
    usesSystemDefaultVideoConfig(),
  );
}

function requiredSystemDefaultVideoConfig(
  configs: VideoGenConfig[],
): VideoGenConfig {
  return requireSystemDefaultProviderConfig(
    systemDefaultVideoConfig(configs),
    "服务器尚未配置系统默认视频模板",
  );
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
    rejectSystemDefaultMutation();
  }
  if (usesLocalRuntimeSettings())
    return updateLocalVideoConfig(videoConfigId(configId), input);
  return updateServerVideoConfig(configId, input);
}

export async function updateSystemDefaultVideoKey(input: Pick<UpdateVideoConfigInput, "api_key_encrypted">): Promise<VideoGenConfig> {
  if (!usesLocalRuntimeSettings()) rejectSystemDefaultMutation();
  requireSystemDefaultKeyUpdate(input);
  await updateSystemDefaultVideoApiKey(input.api_key_encrypted);
  return requiredSystemDefaultVideoConfig(await listServerVideoConfigs());
}

export async function removeServerVideoConfig(configId: Id): Promise<void> {
  return apiInvoke<void>("remove_video_config", {
    configId: videoConfigId(configId),
  });
}

export async function removeVideoConfig(configId: Id): Promise<void> {
  if (isSystemDefaultVideoConfigId(configId)) rejectSystemDefaultMutation();
  if (usesLocalRuntimeSettings()) {
    return deleteLocalVideoConfig(videoConfigId(configId));
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
    return setSystemDefaultVideoConfig();
  }
  if (usesLocalRuntimeSettings()) {
    return setDefaultLocalVideoConfig(videoConfigId(configId));
  }
  return setDefaultServerVideoConfig(configId);
}

export async function detectVideoConfigModels(
  input: ModelDiscoveryInput,
  scope: ModelConfigScope,
): Promise<ModelDiscoveryResult> {
  return detectConfigModels("detect_video_config_models", input, scope, {
    systemConfigId: SYSTEM_VIDEO_CONFIG_ID,
    getSystemDefaultApiKey: getSystemDefaultVideoApiKey,
    getLocalConfigApiKey: (configId) =>
      getLocalVideoConfig(configId).api_key_encrypted,
  });
}
