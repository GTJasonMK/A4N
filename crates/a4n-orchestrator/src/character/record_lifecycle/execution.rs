use super::{
    CharacterProfileRecordDeletionResult, CharacterRecordLifecycleError,
    CharacterTimelineRollbackLifecycleResult,
};
use crate::{
    OrchestratorContext,
    character::{character_db_error, character_profile_delete_error},
};

impl OrchestratorContext {
    pub async fn delete_character_profile_records(
        &self,
        profile_id: i64,
    ) -> Result<CharacterProfileRecordDeletionResult, CharacterRecordLifecycleError> {
        let deletion =
            a4n_db::character::delete_profile_cascade_transaction(self.db.connection(), profile_id)
                .await
                .map_err(character_profile_delete_error)
                .map_err(CharacterRecordLifecycleError::Persistence)?;
        let rag = self
            .delete_character_profile_rag_records(&deletion)
            .await
            .map_err(CharacterRecordLifecycleError::Rag)?;
        Ok(CharacterProfileRecordDeletionResult { deletion, rag })
    }

    pub async fn rollback_character_profile_timeline(
        &self,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<CharacterTimelineRollbackLifecycleResult, CharacterRecordLifecycleError> {
        let rollback = a4n_db::character::rollback_profile_to_chapter(
            self.db.connection(),
            profile_id,
            chapter_number,
        )
        .await
        .map_err(character_db_error)
        .map_err(CharacterRecordLifecycleError::Persistence)?;
        let rag = self
            .delete_character_timeline_rag_records(rollback.project_id, &rollback.deleted_timeline)
            .await
            .map_err(CharacterRecordLifecycleError::Rag)?;
        Ok(CharacterTimelineRollbackLifecycleResult { rollback, rag })
    }
}
