use super::*;

impl WorkerState {
    pub(super) async fn snapshots(
        &self,
        request: WorkerEnvelope<ListSnapshotsInput>,
    ) -> Result<Vec<SnapshotReferenceSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_SNAPSHOTS_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        validate_snapshot_range(input.start_chapter, input.end_chapter)?;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let snapshots = a4n_db::character::snapshot_references_by_profile_records(
            self.database.connection(),
            input.profile_id,
            input.start_chapter,
            input.end_chapter,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        snapshots.iter().map(snapshot_reference_summary).collect()
    }

    pub(super) async fn state_at_chapter(
        &self,
        request: WorkerEnvelope<GetProfileStateAtChapterInput>,
    ) -> Result<ProfileChapterStateSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_STATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if input.chapter_number <= 0 {
            return Err(WorkerFailure::invalid_request("章节号必须大于 0"));
        }
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let snapshot = a4n_db::character::snapshot_reference_record_by_chapter(
            self.database.connection(),
            input.profile_id,
            input.chapter_number,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("第 {} 章没有角色快照", input.chapter_number))
        })?;
        profile_state_summary(&snapshot)
    }

    pub(super) async fn snapshot_by_hash(
        &self,
        request: WorkerEnvelope<GetSnapshotInput>,
    ) -> Result<SnapshotSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_SNAPSHOT_BY_HASH_OPERATION)?;
        let user_id = user_id(&request)?;
        let content_hash = request.payload.content_hash.trim();
        if content_hash.is_empty() {
            return Err(WorkerFailure::invalid_request("快照哈希不能为空"));
        }
        let snapshot_store =
            a4n_db::character::Sha256SnapshotStore::new(self.database.shared_connection());
        let snapshot = snapshot_store
            .load_snapshot(content_hash)
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
            .ok_or_else(|| WorkerFailure::not_found(format!("快照不存在: {content_hash}")))?;
        profile_for_editor(&self.database, user_id, snapshot.entity_id).await?;
        if snapshot.entity_type != "character_profile" {
            return Err(WorkerFailure::invalid_request(format!(
                "快照类型不匹配: expected character_profile, got {}",
                snapshot.entity_type
            )));
        }
        Ok(SnapshotSummary {
            data: snapshot.data,
            created_at: Some(snapshot.created_at.and_utc().to_rfc3339()),
        })
    }
}

impl WorkerState {
    pub(super) async fn attribute_history(
        &self,
        request: WorkerEnvelope<AttributeHistoryInput>,
    ) -> Result<Vec<AttributeEventSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_ATTRIBUTE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let events = a4n_db::character::attribute_history(
            self.database.connection(),
            input.profile_id,
            a4n_db::character::CharacterAttributeEventHistoryFilter {
                chapter_from: input.chapter_from,
                chapter_to: input.chapter_to,
                category: input.category.map(|value| value.as_str().to_string()),
                key: input.key,
                operation: input.operation.map(|value| value.as_str().to_string()),
                limit: input.limit,
            },
        )
        .await
        .map_err(character_db_error)?;
        events.iter().map(attribute_event_summary).collect()
    }

    pub(super) async fn behavior_history(
        &self,
        request: WorkerEnvelope<ListBehaviorsInput>,
    ) -> Result<Vec<BehaviorRecordSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_BEHAVIOR_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if input.limit.is_some_and(|limit| !(1..=100).contains(&limit)) {
            return Err(WorkerFailure::invalid_request(
                "行为记录 limit 必须在 1..=100 之间",
            ));
        }
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let records = a4n_db::character::behavior_records_by_profile_records(
            self.database.connection(),
            input.profile_id,
            input.chapter_from,
            input.chapter_to,
            input.limit,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        records.iter().map(behavior_record_summary).collect()
    }

    pub(super) async fn deletion_marks(
        &self,
        request: WorkerEnvelope<ListDeletionMarksInput>,
    ) -> Result<Vec<DeletionMarkSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_DELETION_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let marks = a4n_db::character::deletion_mark_records_by_profile(
            self.database.connection(),
            input.profile_id,
            input.category.as_ref().map(|value| value.as_str()),
            input.status,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        marks.iter().map(deletion_mark_summary).collect()
    }

    pub(super) async fn sync_conflict(
        &self,
        request: WorkerEnvelope<CheckProfileSyncConflictInput>,
    ) -> Result<ProfileSyncConflictSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_SYNC_CONFLICT_OPERATION)?;
        let user_id = user_id(&request)?;
        let profile =
            profile_for_editor(&self.database, user_id, request.payload.profile_id).await?;
        let chapters = a4n_db::project::character_sync_chapter_records(
            self.database.connection(),
            profile.project_id,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        let max_available_chapter = chapters
            .iter()
            .map(|chapter| chapter.chapter_number)
            .max()
            .unwrap_or_default();
        let snapshots = a4n_db::character::snapshot_references_by_profile_records(
            self.database.connection(),
            profile.id,
            None,
            Some(max_available_chapter),
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?;
        Ok(ProfileSyncConflictSummary {
            has_conflict: profile.last_synced_chapter > max_available_chapter,
            last_synced_chapter: profile.last_synced_chapter,
            max_available_chapter,
            available_snapshot_chapters: snapshots
                .iter()
                .map(|snapshot| snapshot.chapter_number)
                .collect(),
        })
    }

    pub(super) async fn diff_timeline(
        &self,
        request: WorkerEnvelope<DiffProfileChaptersInput>,
    ) -> Result<SnapshotDeltaSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_TIMELINE_DIFF_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if input.from_chapter < 0 {
            return Err(WorkerFailure::invalid_request("起始章节号不能小于 0"));
        }
        if input.to_chapter <= 0 {
            return Err(WorkerFailure::invalid_request("目标章节号必须大于 0"));
        }
        if input.from_chapter > input.to_chapter {
            return Err(WorkerFailure::invalid_request(
                "起始章节号不能大于目标章节号",
            ));
        }
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let to_ref = a4n_db::character::snapshot_reference_record_by_chapter(
            self.database.connection(),
            profile.id,
            input.to_chapter,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("第 {} 章没有角色快照", input.to_chapter))
        })?;
        let from_ref = if input.from_chapter == 0 {
            None
        } else {
            a4n_db::character::snapshot_reference_record_by_chapter(
                self.database.connection(),
                profile.id,
                input.from_chapter,
            )
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        };
        let store = a4n_db::character::Sha256SnapshotStore::new(self.database.shared_connection());
        let to_snapshot = store
            .load_snapshot(&to_ref.snapshot_hash)
            .await
            .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!("快照不存在: {}", to_ref.snapshot_hash))
            })?;
        let from_data = match from_ref {
            Some(from_ref) => {
                store
                    .load_snapshot(&from_ref.snapshot_hash)
                    .await
                    .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
                    .ok_or_else(|| {
                        WorkerFailure::not_found(format!("快照不存在: {}", from_ref.snapshot_hash))
                    })?
                    .data
            }
            None => a4n_domain::character::initial_snapshot_data(),
        };
        let delta = a4n_db::character::Sha256SnapshotStore::diff_snapshot_data(
            &from_data,
            &to_snapshot.data,
        )
        .map_err(|error| WorkerFailure::conflict(error.to_string()))?;
        Ok(SnapshotDeltaSummary { delta })
    }
}
