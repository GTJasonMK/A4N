import { apiInvoke } from "./index";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import type { DownstreamConflictStrategy } from "./generated/host";
import { getLocalImageConfig } from "./localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "./runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../utils/runtimeMode";
import { normalizeImageConfigId } from "./image/configs";
import type {
  CreateObjectProfileInput,
  DeleteObjectProfileInput,
  ObjectProfileImageInput,
  ObjectProfileImagePromptOutput,
  ObjectProfileListItem,
  ObjectProfileDeleteResult,
  ObjectProfileInput,
  ObjectProfileMutationResult,
  ObjectProfileSummary,
  UpdateObjectProfileInput,
} from "./generated/host";

export type ObjectProfile = ObjectProfileSummary;
export type ObjectProfileIndex = ObjectProfileListItem;
export type { ObjectProfileInput };

export async function listObjectProfiles(projectId: Id): Promise<ObjectProfile[]> {
  return apiInvoke<ObjectProfile[]>("list_object_profiles", {
    input: { project_id: idNum(projectId) },
  });
}

export async function listObjectProfileIndex(projectId: Id): Promise<ObjectProfileIndex[]> {
  return apiInvoke<ObjectProfileIndex[]>("list_object_profile_index", {
    input: { project_id: idNum(projectId) },
  });
}

export async function createObjectProfile(
  projectId: Id,
  input: ObjectProfileInput,
): Promise<ObjectProfileMutationResult> {
  const request: CreateObjectProfileInput = {
    project_id: idNum(projectId),
    profile: normalizeObjectProfileInput(input),
  };
  return apiInvoke<ObjectProfileMutationResult>("create_object_profile", {
    input: request,
  });
}

export async function updateObjectProfile(
  objectProfileId: Id,
  expectedUpdatedAt: string,
  input: ObjectProfileInput,
  downstreamStrategy: DownstreamConflictStrategy,
): Promise<ObjectProfileMutationResult> {
  const request: UpdateObjectProfileInput = {
    object_profile_id: idNum(objectProfileId),
    expected_updated_at: expectedUpdatedAt,
    profile: normalizeObjectProfileInput(input),
    downstream_strategy: downstreamStrategy,
  };
  return apiInvoke<ObjectProfileMutationResult>("update_object_profile", {
    input: request,
  });
}

export async function deleteObjectProfile(
  objectProfileId: Id,
  downstreamStrategy: DownstreamConflictStrategy,
): Promise<ObjectProfileDeleteResult> {
  const input: DeleteObjectProfileInput = {
    object_profile_id: idNum(objectProfileId),
    downstream_strategy: downstreamStrategy,
  };
  return apiInvoke<ObjectProfileDeleteResult>("delete_object_profile", {
    input,
  });
}

function normalizeObjectProfileInput(input: ObjectProfileInput): ObjectProfileInput {
  return {
    name: input.name,
    aliases: input.aliases ?? [],
    object_type: input.object_type,
    importance: input.importance,
    visual_description: input.visual_description,
    material_and_state: input.material_and_state,
    story_usage: input.story_usage,
    visual_usage: input.visual_usage,
    generation_usage: input.generation_usage,
    location_context: input.location_context ?? null,
    first_seen_chapter: input.first_seen_chapter,
    last_seen_chapter: input.last_seen_chapter,
    evidence: input.evidence ?? [],
    image_prompt_seed: input.image_prompt_seed,
  };
}

export async function generateObjectProfileImagePrompt(
  input: ObjectProfileImageInput,
): Promise<ObjectProfileImagePromptOutput> {
  return apiInvoke<ObjectProfileImagePromptOutput>("generate_object_profile_image_prompt", {
    input,
  });
}

export async function previewObjectProfileImageRequest(
  input: ObjectProfileImageInput,
): Promise<ModelRequestPreview> {
  const configId = normalizeImageConfigId(input.config_id);
  return apiInvoke<ModelRequestPreview>("preview_object_profile_image_request", {
    input: { ...input, config_id: configId },
    ...(usesLocalRuntimeSettings() && configId != null
      ? {
          runtime_config_snapshot: buildRuntimeConfigSnapshot({
            image: getLocalImageConfig(configId),
          }),
        }
      : {}),
  });
}
