import { apiInvoke } from "./index";
import { idNum } from "./shared";
import type {
  ListProfileMaterialsInput,
  ProfileMaterialFilter,
  ProfileMaterialGroup,
  ProfileMaterialKind,
  ProfileMaterialKindFilter,
  ProfileMaterialSort,
  ProfileMaterialState,
  SetProfilePrimaryMaterialInput,
  UploadProfileMaterialImageInput,
} from "./generated/host";

export type {
  ProfileMaterialFilter,
  ProfileMaterialGroup,
  ProfileMaterialKind,
  ProfileMaterialKindFilter,
  ProfileMaterialSort,
  ProfileMaterialState,
};

export async function listProfileMaterials(
  input: ListProfileMaterialsInput,
): Promise<ProfileMaterialGroup[]> {
  const request: ListProfileMaterialsInput = {
    ...input,
    project_id: idNum(input.project_id),
  };
  return apiInvoke<ProfileMaterialGroup[]>("list_profile_materials", {
    input: request,
  });
}

export async function setProfilePrimaryMaterial(
  input: SetProfilePrimaryMaterialInput,
): Promise<ProfileMaterialGroup> {
  const request: SetProfilePrimaryMaterialInput = {
    project_id: idNum(input.project_id),
    profile_kind: input.profile_kind,
    profile_id: idNum(input.profile_id),
    image_id: input.image_id == null ? null : idNum(input.image_id),
  };
  return apiInvoke<ProfileMaterialGroup>("set_profile_primary_material", {
    input: request,
  });
}

export async function uploadProfileMaterialImage(
  input: UploadProfileMaterialImageInput,
): Promise<ProfileMaterialGroup> {
  const request: UploadProfileMaterialImageInput = {
    ...input,
    project_id: idNum(input.project_id),
    profile_id: idNum(input.profile_id),
  };
  return apiInvoke<ProfileMaterialGroup>("upload_profile_material_image", {
    input: request,
  });
}
