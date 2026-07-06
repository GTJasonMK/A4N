import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type { GeneratedImage } from "./image";
import { idNum, integerNum, type ApiId as Id } from "./shared";
import type { DownstreamConflictStrategy } from "./dependencyConflict";
import { getLocalImageConfig } from "./localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "./runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../utils/runtimeMode";
import { SYSTEM_IMAGE_CONFIG_ID } from "./image";

export interface SceneProfile {
  id: number;
  project_id: number;
  name: string;
  normalized_name: string;
  aliases_json: unknown;
  scene_type: string;
  importance: string;
  parent_scene_id: number | null;
  base_description: string;
  visual_description: string;
  spatial_layout: string;
  atmosphere: string;
  entry_points_json: unknown;
  rules_json: unknown;
  current_state_json: unknown;
  first_seen_chapter: number;
  last_seen_chapter: number;
  evidence_json: unknown;
  image_prompt_seed: string;
  occurrence_count: number;
  generated_image_count: number;
  created_at: string | null;
  updated_at: string | null;
}

export interface SceneProfileInput {
  name: string;
  aliases?: string[];
  scene_type: string;
  importance: string;
  parent_scene_id?: Id | null;
  base_description: string;
  visual_description: string;
  spatial_layout: string;
  atmosphere: string;
  entry_points_json?: unknown | null;
  rules_json?: unknown | null;
  first_seen_chapter: number;
  last_seen_chapter: number;
  evidence?: string[];
  image_prompt_seed: string;
  downstream_strategy?: DownstreamConflictStrategy;
}

interface SceneProfileImageInput {
  scene_profile_id: number;
  chapter_id?: number | null;
  style?: string | null;
  config_id?: number | null;
  style_template_id?: string | null;
  style_template_name?: string | null;
  style_template_version?: string | null;
  custom_style?: string | null;
}

interface SceneProfileImagePromptOutput {
  scene_profile_id: number;
  scene_name: string;
  prompt: string;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问场景档案");
  return id;
};

function normalizeImageConfigId(configId?: Id | null): number | null {
  if (configId == null) return null;
  const id = integerNum(configId, "图片配置 ID");
  return id === SYSTEM_IMAGE_CONFIG_ID ? null : idNum(id, "图片配置 ID");
}

export async function listSceneProfiles(projectId: Id): Promise<SceneProfile[]> {
  return apiInvoke<SceneProfile[]>("list_scene_profiles", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}

export async function createSceneProfile(
  projectId: Id,
  input: SceneProfileInput,
): Promise<SceneProfile> {
  return apiInvoke<SceneProfile>("create_scene_profile", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input: normalizeSceneProfileInput(input),
  });
}

export async function updateSceneProfile(
  sceneProfileId: Id,
  input: SceneProfileInput,
): Promise<SceneProfile> {
  return apiInvoke<SceneProfile>("update_scene_profile", {
    userId: currentUserId(),
    sceneProfileId: idNum(sceneProfileId),
    input: normalizeSceneProfileInput(input),
  });
}

export async function deleteSceneProfile(
  sceneProfileId: Id,
  downstreamStrategy: DownstreamConflictStrategy = "reject",
): Promise<void> {
  return apiInvoke<void>("delete_scene_profile", {
    userId: currentUserId(),
    sceneProfileId: idNum(sceneProfileId),
    downstreamStrategy,
  });
}

function normalizeSceneProfileInput(input: SceneProfileInput): Record<string, unknown> {
  return {
    name: input.name,
    aliases: input.aliases ?? [],
    scene_type: input.scene_type,
    importance: input.importance,
    parent_scene_id: input.parent_scene_id == null ? null : idNum(input.parent_scene_id),
    base_description: input.base_description,
    visual_description: input.visual_description,
    spatial_layout: input.spatial_layout,
    atmosphere: input.atmosphere,
    entry_points_json: input.entry_points_json ?? null,
    rules_json: input.rules_json ?? null,
    first_seen_chapter: input.first_seen_chapter,
    last_seen_chapter: input.last_seen_chapter,
    evidence: input.evidence ?? [],
    image_prompt_seed: input.image_prompt_seed,
    downstream_strategy: input.downstream_strategy ?? "reject",
  };
}

export async function syncSceneProfiles(
  projectId: Id,
  chapterNumber?: number | null,
  taskId?: string | null,
): Promise<unknown> {
  return apiInvoke<unknown>("sync_scene_profiles", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    chapterNumber: chapterNumber ?? null,
    taskId: taskId ?? null,
  });
}

export async function generateSceneProfileImage(
  input: SceneProfileImageInput,
  taskId?: string | null,
): Promise<GeneratedImage> {
  const configId = normalizeImageConfigId(input.config_id);
  return apiInvoke<GeneratedImage>("generate_scene_profile_image", {
    userId: currentUserId(),
    input: { ...input, config_id: configId },
    taskId: taskId ?? null,
    ...(usesLocalRuntimeSettings() && configId != null
      ? {
          runtime_config_snapshot: buildRuntimeConfigSnapshot({
            image: getLocalImageConfig(configId),
          }),
        }
      : {}),
  });
}

export async function generateSceneProfileImagePrompt(
  input: SceneProfileImageInput,
): Promise<SceneProfileImagePromptOutput> {
  return apiInvoke<SceneProfileImagePromptOutput>("generate_scene_profile_image_prompt", {
    userId: currentUserId(),
    input,
  });
}
