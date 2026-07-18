use super::{
    AppliedCharacterSyncPersistenceRecord, CharacterConflictContextRows,
    CharacterSyncPersistenceInput, character_db_error,
};
use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn apply_character_sync_persistence_record(
        &self,
        input: CharacterSyncPersistenceInput,
    ) -> Result<AppliedCharacterSyncPersistenceRecord, AppError> {
        a4n_db::character::apply_character_sync_persistence_record(self.db.connection(), input)
            .await
            .map_err(character_db_error)
    }
    pub async fn character_conflict_context_rows(
        &self,
        project_id: i64,
        profile_id: i64,
        start_chapter: Option<i32>,
        end_chapter: Option<i32>,
    ) -> Result<CharacterConflictContextRows, AppError> {
        a4n_db::character::character_conflict_context_rows(
            self.db.connection(),
            project_id,
            profile_id,
            start_chapter,
            end_chapter,
        )
        .await
        .map_err(character_db_error)
    }
}
