//! 资料素材 runtime 窄 facade。

use std::collections::BTreeMap;

use a4n_core::error::AppError;
use a4n_orchestrator::{
    OrchestratorContext,
    profile_material::{
        ListProfileMaterialsInput, ProfileMaterialGroup, SetProfilePrimaryMaterialInput,
        UploadProfileMaterialImageInput,
    },
};

pub(crate) struct ProfileMaterialRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProfileMaterialRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn list_profile_materials_validated(
        &self,
        user_id: i64,
        input: ListProfileMaterialsInput,
    ) -> Result<Vec<ProfileMaterialGroup>, AppError> {
        self.orchestrator
            .list_profile_materials_validated(user_id, input)
            .await
    }

    pub(crate) async fn set_profile_primary_material_validated(
        &self,
        user_id: i64,
        input: SetProfilePrimaryMaterialInput,
    ) -> Result<ProfileMaterialGroup, AppError> {
        self.orchestrator
            .set_profile_primary_material_validated(user_id, input)
            .await
    }

    pub(crate) async fn upload_profile_material_image_validated(
        &self,
        user_id: i64,
        input: UploadProfileMaterialImageInput,
    ) -> Result<ProfileMaterialGroup, AppError> {
        self.orchestrator
            .upload_profile_material_image_validated(user_id, input)
            .await
    }

    pub(crate) async fn count_profile_material_images(
        &self,
        project_id: i64,
        kind: &str,
        profile_id: i64,
    ) -> Result<i64, AppError> {
        self.orchestrator
            .count_profile_material_images(project_id, kind, profile_id)
            .await
    }

    pub(crate) async fn count_profile_material_images_by_profile(
        &self,
        project_id: i64,
        kind: &str,
        profile_ids: &[i64],
    ) -> Result<BTreeMap<i64, i64>, AppError> {
        self.orchestrator
            .count_profile_material_images_by_profile(project_id, kind, profile_ids)
            .await
    }

    pub(crate) async fn profile_material_image_ids(
        &self,
        project_id: i64,
        kind: &str,
        profile_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        self.orchestrator
            .profile_material_image_ids(project_id, kind, profile_id)
            .await
    }
}
