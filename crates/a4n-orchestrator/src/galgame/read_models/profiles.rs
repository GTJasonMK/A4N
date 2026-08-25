use a4n_core::error::AppError;

use crate::galgame::{
    GalgameSpeakerProfile, OrchestratorContext, galgame_speaker_profiles_from_prompt_records,
    validate_positive_id,
};

impl OrchestratorContext {
    pub(crate) async fn speaker_profiles_for_root_project(
        &self,
        root_project_id: i64,
    ) -> Result<Vec<GalgameSpeakerProfile>, AppError> {
        self.speaker_profiles(root_project_id).await
    }

    async fn speaker_profiles(
        &self,
        root_project_id: i64,
    ) -> Result<Vec<GalgameSpeakerProfile>, AppError> {
        validate_positive_id(root_project_id, "root_project_id")?;
        let profiles = a4n_db::galgame::galgame_speaker_profile_records(
            self.database.connection(),
            root_project_id,
        )
        .await
        .map_err(AppError::Database)?;
        Ok(galgame_speaker_profiles_from_prompt_records(&profiles))
    }
}
