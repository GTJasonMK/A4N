#[async_trait::async_trait]
impl CharacterProfileOverviewReadApplicationPort for CharacterMutationWorkerClient {
    async fn profile_project_id(&self, user_id: i64, profile_id: i64) -> Result<i64, AppError> {
        self.command(
            user_id,
            profile_id,
            CHARACTER_OVERVIEW_PROJECT_OPERATION,
            ProfileIdInput { profile_id },
        )
        .await
    }

    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            CHARACTER_OVERVIEW_LIST_OPERATION,
            ListProfilesInput { project_id },
        )
        .await
    }

    async fn character_arc_by_name(
        &self,
        user_id: i64,
        project_id: i64,
        name: String,
    ) -> Result<Option<serde_json::Value>, AppError> {
        self.command(
            user_id,
            project_id,
            CHARACTER_OVERVIEW_ARC_OPERATION,
            NamedProfileInput { project_id, name },
        )
        .await
    }

    async fn voice_fingerprint_by_name(
        &self,
        user_id: i64,
        project_id: i64,
        name: String,
    ) -> Result<Option<serde_json::Value>, AppError> {
        self.command(
            user_id,
            project_id,
            CHARACTER_OVERVIEW_VOICE_OPERATION,
            NamedProfileInput { project_id, name },
        )
        .await
    }
}

#[async_trait::async_trait]
impl CharacterTimelineReadApplicationPort for CharacterMutationWorkerClient {
    async fn attribute_history(
        &self,
        user_id: i64,
        input: AttributeHistoryInput,
    ) -> Result<Vec<AttributeEventSummary>, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_ATTRIBUTE_OPERATION,
            input,
        )
        .await
    }

    async fn behavior_history(
        &self,
        user_id: i64,
        input: ListBehaviorsInput,
    ) -> Result<Vec<BehaviorRecordSummary>, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_BEHAVIOR_OPERATION,
            input,
        )
        .await
    }

    async fn deletion_marks(
        &self,
        user_id: i64,
        input: ListDeletionMarksInput,
    ) -> Result<Vec<DeletionMarkSummary>, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_DELETION_OPERATION,
            input,
        )
        .await
    }

    async fn snapshots(
        &self,
        user_id: i64,
        input: ListSnapshotsInput,
    ) -> Result<Vec<SnapshotReferenceSummary>, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_SNAPSHOTS_OPERATION,
            input,
        )
        .await
    }

    async fn state_at_chapter(
        &self,
        user_id: i64,
        input: GetProfileStateAtChapterInput,
    ) -> Result<ProfileChapterStateSummary, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_STATE_OPERATION,
            input,
        )
        .await
    }

    async fn snapshot_by_hash(
        &self,
        user_id: i64,
        input: GetSnapshotInput,
    ) -> Result<SnapshotSummary, AppError> {
        self.command_global(
            user_id,
            CHARACTER_TIMELINE_SNAPSHOT_BY_HASH_OPERATION,
            input,
        )
        .await
    }

    async fn sync_conflict(
        &self,
        user_id: i64,
        input: CheckProfileSyncConflictInput,
    ) -> Result<ProfileSyncConflictSummary, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_SYNC_CONFLICT_OPERATION,
            input,
        )
        .await
    }

    async fn diff_chapters(
        &self,
        user_id: i64,
        input: DiffProfileChaptersInput,
    ) -> Result<SnapshotDeltaSummary, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_DIFF_OPERATION,
            input,
        )
        .await
    }
}
