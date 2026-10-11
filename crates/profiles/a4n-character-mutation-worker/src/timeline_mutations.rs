use super::*;

impl WorkerState {
    pub(super) async fn rollback_timeline(
        &self,
        request: WorkerEnvelope<RollbackToChapterInput>,
    ) -> Result<CharacterTimelineRollbackMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_ROLLBACK_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let input = request.payload;
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let result = a4n_db::character::rollback_profile_to_chapter(
            self.database.connection(),
            input.profile_id,
            input.chapter_number,
        )
        .await
        .map_err(character_db_error)?;
        Ok(CharacterTimelineRollbackMutationResult {
            project_id: profile.project_id,
            deleted_events: result.deleted_timeline.deleted_attribute_events,
            deleted_snapshots: result.deleted_timeline.deleted_snapshots,
            deleted_behaviors: result.deleted_timeline.deleted_behaviors,
            deleted_state_indexes: result.deleted_timeline.deleted_state_indexes,
            deleted_deletion_marks: result.deletion_mark_rewind.deleted,
            rewound_deletion_marks: result.deletion_mark_rewind.rewound,
            state_index_ids: result.deleted_timeline.state_index_ids,
            attribute_event_ids: result.deleted_timeline.attribute_event_ids,
        })
    }
}
