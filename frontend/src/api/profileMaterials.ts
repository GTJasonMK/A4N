import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type { GeneratedImage } from "./image";
import { idNum } from "./shared";

export type ProfileMaterialKind = "scene" | "object";
export type ProfileMaterialState = "missing" | "ready" | "stale" | "unknown";
type ProfileMaterialImageStaleness = "fresh" | "stale" | "unknown";

interface ProfileMaterialProfileSummary {
  id: number;
  project_id: number;
  name: string;
  category: string;
  importance: string;
  first_seen_chapter: number;
  last_seen_chapter: number;
  primary_text: string;
  secondary_text: string;
  updated_at: string | null;
}

interface ProfileMaterialImageSummary {
  image: GeneratedImage;
  is_primary: boolean;
  staleness: ProfileMaterialImageStaleness;
  style_template_id: string | null;
  style_template_name: string | null;
}

export interface ProfileMaterialGroup {
  kind: ProfileMaterialKind;
  profile: ProfileMaterialProfileSummary;
  material_state: ProfileMaterialState;
  current_profile_content_sha256: string;
  image_count: number;
  primary_image_id: number | null;
  primary_image: ProfileMaterialImageSummary | null;
  images: ProfileMaterialImageSummary[];
}

interface ListProfileMaterialsInput {
  project_id: number;
  kind?: "all" | ProfileMaterialKind | null;
  query?: string | null;
  filter?: "all" | ProfileMaterialState | "favorite" | null;
  sort?: "missing_first" | "stale_first" | "newest" | "importance" | null;
  image_limit_per_profile?: number | null;
}

interface SetProfilePrimaryMaterialInput {
  project_id: number;
  profile_kind: ProfileMaterialKind;
  profile_id: number;
  image_id: number | null;
}

interface UploadProfileMaterialImageInput {
  project_id: number;
  profile_kind: ProfileMaterialKind;
  profile_id: number;
  file_name: string;
  mime_type: string;
  data_base64: string;
  set_primary: boolean;
  prompt_snapshot?: string | null;
  style_template_id?: string | null;
  style_template_name?: string | null;
  style_template_version?: string | null;
  custom_style?: string | null;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问素材管理");
  return id;
};

export async function listProfileMaterials(
  input: ListProfileMaterialsInput,
): Promise<ProfileMaterialGroup[]> {
  return apiInvoke<ProfileMaterialGroup[]>("list_profile_materials", {
    userId: currentUserId(),
    input: {
      ...input,
      project_id: idNum(input.project_id),
      image_limit_per_profile: input.image_limit_per_profile ?? 12,
    },
  });
}

export async function setProfilePrimaryMaterial(
  input: SetProfilePrimaryMaterialInput,
): Promise<ProfileMaterialGroup> {
  return apiInvoke<ProfileMaterialGroup>("set_profile_primary_material", {
    userId: currentUserId(),
    input: {
      project_id: idNum(input.project_id),
      profile_kind: input.profile_kind,
      profile_id: idNum(input.profile_id),
      image_id: input.image_id == null ? null : idNum(input.image_id),
    },
  });
}

export async function uploadProfileMaterialImage(
  input: UploadProfileMaterialImageInput,
): Promise<ProfileMaterialGroup> {
  return apiInvoke<ProfileMaterialGroup>("upload_profile_material_image", {
    userId: currentUserId(),
    input: {
      ...input,
      project_id: idNum(input.project_id),
      profile_id: idNum(input.profile_id),
      prompt_snapshot: input.prompt_snapshot ?? null,
      style_template_id: input.style_template_id ?? null,
      style_template_name: input.style_template_name ?? null,
      style_template_version: input.style_template_version ?? null,
      custom_style: input.custom_style ?? null,
    },
  });
}
