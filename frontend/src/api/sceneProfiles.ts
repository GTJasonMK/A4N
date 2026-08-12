import { apiInvoke } from "./index";
import { idNum, type ApiId as Id } from "./shared";
import type { DownstreamConflictStrategy } from "./generated/host";
import type {
  CreateSceneProfileInput,
  DeleteSceneProfileInput,
  SceneProfileImageInput,
  SceneProfileImagePromptOutput,
  SceneProfileDeleteResult,
  SceneProfileInput as ApiSceneProfileInput,
  SceneProfileMutationResult,
  SceneProfileSummary,
  SyncSceneProfilesInput,
  UpdateSceneProfileInput,
} from "./generated/host";

export type SceneProfile = SceneProfileSummary;

export type SceneProfileInput = Omit<ApiSceneProfileInput, "parent_scene_id"> & {
  parent_scene_id?: Id | null;
};

export async function listSceneProfiles(projectId: Id): Promise<SceneProfile[]> {
  return apiInvoke<SceneProfile[]>("list_scene_profiles", {
    input: { project_id: idNum(projectId) },
  });
}

export async function createSceneProfile(
  projectId: Id,
  input: SceneProfileInput,
): Promise<SceneProfileMutationResult> {
  const request: CreateSceneProfileInput = {
    project_id: idNum(projectId),
    profile: normalizeSceneProfileInput(input),
  };
  return apiInvoke<SceneProfileMutationResult>("create_scene_profile", {
    input: request,
  });
}

export async function updateSceneProfile(
  sceneProfileId: Id,
  input: SceneProfileInput,
  downstreamStrategy: DownstreamConflictStrategy,
): Promise<SceneProfileMutationResult> {
  const request: UpdateSceneProfileInput = {
    scene_profile_id: idNum(sceneProfileId),
    profile: normalizeSceneProfileInput(input),
    downstream_strategy: downstreamStrategy,
  };
  return apiInvoke<SceneProfileMutationResult>("update_scene_profile", {
    input: request,
  });
}

export async function deleteSceneProfile(
  sceneProfileId: Id,
  downstreamStrategy: DownstreamConflictStrategy,
): Promise<SceneProfileDeleteResult> {
  const input: DeleteSceneProfileInput = {
    scene_profile_id: idNum(sceneProfileId),
    downstream_strategy: downstreamStrategy,
  };
  return apiInvoke<SceneProfileDeleteResult>("delete_scene_profile", {
    input,
  });
}

function normalizeSceneProfileInput(input: SceneProfileInput): ApiSceneProfileInput {
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
  };
}

export async function syncSceneProfiles(
  projectId: Id,
  chapterNumber?: number | null,
  taskId?: string | null,
): Promise<void> {
  const input: SyncSceneProfilesInput = {
    project_id: idNum(projectId),
    chapter_number: chapterNumber ?? null,
  };
  return apiInvoke<void>("sync_scene_profiles", {
    input,
    taskId: taskId ?? null,
  });
}

export async function generateSceneProfileImagePrompt(
  input: SceneProfileImageInput,
): Promise<SceneProfileImagePromptOutput> {
  return apiInvoke<SceneProfileImagePromptOutput>("generate_scene_profile_image_prompt", {
    input,
  });
}
