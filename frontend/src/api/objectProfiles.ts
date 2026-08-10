import { apiInvoke } from "./index";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import type { DownstreamConflictStrategy } from "./generated/host";
import { getLocalImageConfig } from "./localRuntimeSettings";
import { buildRuntimeConfigSnapshot } from "./runtimeConfigSnapshot";
import { usesLocalRuntimeSettings } from "../utils/runtimeMode";
import { normalizeImageConfigId } from "./image/configs";
import type {
  ObjectProfileDeleteResult,
  ObjectProfileInput as ApiObjectProfileInput,
  ObjectProfileMutationResult,
  ObjectProfileSummary,
} from "./generated/host";

export type ObjectProfile = ObjectProfileSummary;

export type ObjectProfileInput = Omit<ApiObjectProfileInput, "downstream_strategy"> & {
  downstream_strategy?: DownstreamConflictStrategy;
};

interface ObjectProfileImageInput {
  object_profile_id: number;
  chapter_id?: number | null;
  style?: string | null;
  config_id?: number | null;
  style_template_id?: string | null;
  style_template_name?: string | null;
  style_template_version?: string | null;
  custom_style?: string | null;
}

interface ObjectProfileImagePromptOutput {
  object_profile_id: number;
  object_name: string;
  prompt: string;
}

export async function listObjectProfiles(projectId: Id): Promise<ObjectProfile[]> {
  return apiInvoke<ObjectProfile[]>("list_object_profiles", {
    userId: currentUserId(),
    input: { project_id: idNum(projectId) },
  });
}

export async function createObjectProfile(
  projectId: Id,
  input: ObjectProfileInput,
): Promise<ObjectProfileMutationResult> {
  return apiInvoke<ObjectProfileMutationResult>("create_object_profile", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    input: normalizeObjectProfileInput(input),
  });
}

export async function updateObjectProfile(
  objectProfileId: Id,
  input: ObjectProfileInput,
): Promise<ObjectProfileMutationResult> {
  return apiInvoke<ObjectProfileMutationResult>("update_object_profile", {
    userId: currentUserId(),
    objectProfileId: idNum(objectProfileId),
    input: normalizeObjectProfileInput(input),
  });
}

export async function deleteObjectProfile(
  objectProfileId: Id,
  downstreamStrategy: DownstreamConflictStrategy = "reject",
): Promise<ObjectProfileDeleteResult> {
  return apiInvoke<ObjectProfileDeleteResult>("delete_object_profile", {
    userId: currentUserId(),
    objectProfileId: idNum(objectProfileId),
    downstreamStrategy,
  });
}

function normalizeObjectProfileInput(input: ObjectProfileInput): Required<ObjectProfileInput> {
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
    downstream_strategy: input.downstream_strategy ?? "reject",
  };
}

export async function generateObjectProfileImagePrompt(
  input: ObjectProfileImageInput,
): Promise<ObjectProfileImagePromptOutput> {
  return apiInvoke<ObjectProfileImagePromptOutput>("generate_object_profile_image_prompt", {
    userId: currentUserId(),
    input,
  });
}

export async function previewObjectProfileImageRequest(
  input: ObjectProfileImageInput,
): Promise<ModelRequestPreview> {
  const configId = normalizeImageConfigId(input.config_id);
  return apiInvoke<ModelRequestPreview>("preview_object_profile_image_request", {
    userId: currentUserId(),
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
