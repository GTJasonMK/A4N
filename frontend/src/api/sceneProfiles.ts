import { apiInvoke } from "./index";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import { idNum, type ApiId as Id } from "./shared";
import type { DownstreamConflictStrategy } from "./generated/host";
import type {
  SceneProfileDeleteResult,
  SceneProfileInput as ApiSceneProfileInput,
  SceneProfileMutationResult,
  SceneProfileSummary,
} from "./generated/host";

export type SceneProfile = SceneProfileSummary;

export type SceneProfileInput = Omit<
  ApiSceneProfileInput,
  "parent_scene_id" | "downstream_strategy"
> & {
  parent_scene_id?: Id | null;
  downstream_strategy?: DownstreamConflictStrategy;
};

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

export async function listSceneProfiles(projectId: Id): Promise<SceneProfile[]> {
  return apiInvoke<SceneProfile[]>("list_scene_profiles", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}

export async function createSceneProfile(
  projectId: Id,
  input: SceneProfileInput,
): Promise<SceneProfileMutationResult> {
  return apiInvoke<SceneProfileMutationResult>("create_scene_profile", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input: normalizeSceneProfileInput(input),
  });
}

export async function updateSceneProfile(
  sceneProfileId: Id,
  input: SceneProfileInput,
): Promise<SceneProfileMutationResult> {
  return apiInvoke<SceneProfileMutationResult>("update_scene_profile", {
    userId: currentUserId(),
    sceneProfileId: idNum(sceneProfileId),
    input: normalizeSceneProfileInput(input),
  });
}

export async function deleteSceneProfile(
  sceneProfileId: Id,
  downstreamStrategy: DownstreamConflictStrategy = "reject",
): Promise<SceneProfileDeleteResult> {
  return apiInvoke<SceneProfileDeleteResult>("delete_scene_profile", {
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

export async function generateSceneProfileImagePrompt(
  input: SceneProfileImageInput,
): Promise<SceneProfileImagePromptOutput> {
  return apiInvoke<SceneProfileImagePromptOutput>("generate_scene_profile_image_prompt", {
    userId: currentUserId(),
    input,
  });
}
