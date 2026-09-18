use a4n_api_contract::character::{
    AddProfileAttributeInput, AttributeCountsSummary, AttributeEventSummary, AttributeHistoryInput,
    BehaviorRecordSummary, CharacterProfileAttributeDeletionApplicationPort,
    CharacterProfileAttributeMutationApplicationPort, CharacterProfileCreateResult,
    CharacterProfileDeleteResult, CharacterProfileDeletionApplicationPort,
    CharacterProfileMutationApplicationPort, CharacterProfileMutationResult,
    CharacterProfileReadApplicationPort, CharacterProfileSummary,
    CharacterProfileTimelineMutationApplicationPort, CheckImplicitUpdateInput,
    CheckProfileSyncConflictInput, CreateProfileInput, DeletionMarkSummary,
    DiffProfileChaptersInput, ExecuteAttributeDeletionInput, GetImplicitReviewInput,
    GetProfileStateAtChapterInput, GetSnapshotInput, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary, ListBehaviorsInput, ListDeletionMarksInput, ListSnapshotsInput,
    ModifyProfileAttributeInput, ProfileChapterStateSummary, ProfileSyncConflictSummary,
    RequestAttributeDeletionInput, ResetDeletionMarksInput, RollbackResultSummary,
    RollbackToChapterInput, SnapshotDeltaSummary, SnapshotReferenceSummary, SnapshotSummary,
    UpdateCharacterProfileInput,
};
use a4n_api_contract::portrait::PortraitRecordMutationApplicationPort;
use a4n_db::DbHandle;
use a4n_db_models::models::ai::generated_image;
use a4n_db_models::models::character::{
    attribute_event, character_portrait, protagonist_behavior_record, protagonist_deletion_mark,
    protagonist_profile, snapshot_reference,
};
use a4n_domain::character::{
    CharacterAttributeCategory, CharacterAttributeDeletionExecutionInput,
    CharacterAttributeDeletionRequestInput, CharacterAttributeOperation,
    CharacterAttributeValueChangeInput, CharacterImplicitReviewDecisionRecordInput,
    CharacterPortraitCreateInput, CharacterTimelineSourceKind, DeletionMarkStatus,
};
use a4n_domain::snapshot::SnapshotDelta;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use super::OrchestratorContext;
use crate::character::{
    AppliedCharacterImplicitDecisionRecord, AppliedCharacterSyncPersistenceRecord,
    CharacterAttributeDeletionExecutionResult, CharacterAttributeEventHistoryFilter,
    CharacterAttributeLifecycleError, CharacterAttributeMutationResult,
    CharacterConflictContextRows, CharacterProfileDeletionSummary,
    CharacterProfileRecordDeletionResult, CharacterProfileRollbackResult,
    CharacterRecordLifecycleError, CharacterSyncPersistenceInput,
    CharacterTimelineRollbackLifecycleResult, PortraitNamedPromptProfileRows,
    PortraitPromptProfileRows, character_db_error, character_portrait_record_error,
    character_profile_create_error, character_profile_delete_error,
};
use crate::character::{
    CHARACTER_SNAPSHOT_ENTITY_TYPE, ensure_positive_snapshot_chapter, initial_snapshot_data,
    validate_snapshot_diff_chapters, validate_snapshot_range,
};
use crate::collaboration::ProjectAccessRole;
use a4n_domain::project::required_text;

#[derive(Clone)]
pub(super) struct CharacterApplication {
    database: DbHandle,
}

impl CharacterApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    // —— profiles ——

    async fn profiles_with_timeline_state(
        &self,
        project_id: i64,
    ) -> Result<Vec<(protagonist_profile::Model, bool, bool)>, AppError> {
        a4n_db::character::profiles_with_timeline_state(self.database.connection(), project_id)
            .await
            .map_err(character_db_error)
    }

    async fn profile_by_id(
        &self,
        profile_id: i64,
    ) -> Result<Option<protagonist_profile::Model>, AppError> {
        a4n_db::character::profile_by_id(self.database.connection(), profile_id)
            .await
            .map_err(AppError::Database)
    }

    async fn profile_by_project_name(
        &self,
        project_id: i64,
        name: &str,
    ) -> Result<Option<protagonist_profile::Model>, AppError> {
        a4n_db::character::profile_by_project_name(self.database.connection(), project_id, name)
            .await
            .map_err(AppError::Database)
    }

    async fn missing_portrait_profiles(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        a4n_db::character::missing_portrait_profiles(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    /// 校验角色姓名后创建档案。
    async fn create_profile_validated(
        &self,
        project_id: i64,
        name: String,
        base_attributes: Value,
    ) -> Result<protagonist_profile::Model, AppError> {
        let name = required_text(&name, "角色姓名")
            .map_err(AppError::InvalidArgument)?
            .to_string();
        a4n_db::character::create_profile(
            self.database.connection(),
            project_id,
            &name,
            base_attributes,
        )
        .await
        .map_err(character_profile_create_error)
    }

    async fn update_initial_profile(
        &self,
        project_id: i64,
        profile_id: i64,
        expected_current_state: Value,
        base_attributes: Value,
    ) -> Result<protagonist_profile::Model, AppError> {
        a4n_db::character::update_initial_profile(
            self.database.connection(),
            project_id,
            profile_id,
            expected_current_state,
            base_attributes,
        )
        .await
        .map_err(character_db_error)
    }

    // —— portraits ——

    async fn character_portrait_by_id(
        &self,
        portrait_id: i64,
    ) -> Result<Option<character_portrait::Model>, AppError> {
        a4n_db::character::character_portrait_by_id(self.database.connection(), portrait_id)
            .await
            .map_err(AppError::Database)
    }

    async fn character_portrait_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<character_portrait::Model>, AppError> {
        a4n_db::character::character_portrait_records_by_project(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn create_character_portrait_for_profile(
        &self,
        input: CharacterPortraitCreateInput,
    ) -> Result<character_portrait::Model, AppError> {
        a4n_db::character::create_character_portrait_for_profile(self.database.connection(), input)
            .await
            .map_err(character_portrait_record_error)
    }

    async fn set_primary_character_portrait_record(
        &self,
        portrait_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::character::set_primary_character_portrait_record(
            self.database.connection(),
            portrait_id,
        )
        .await
        .map_err(character_portrait_record_error)
    }

    async fn portrait_prompt_profile_rows(
        &self,
        project_id: i64,
        profile_id: i64,
    ) -> Result<PortraitPromptProfileRows, AppError> {
        a4n_db::character::portrait_prompt_profile_rows(
            self.database.connection(),
            project_id,
            profile_id,
        )
        .await
        .map_err(character_portrait_record_error)
    }

    async fn portrait_named_prompt_profile_rows(
        &self,
        project_id: i64,
        character_name: &str,
    ) -> Result<PortraitNamedPromptProfileRows, AppError> {
        a4n_db::character::portrait_named_prompt_profile_rows(
            self.database.connection(),
            project_id,
            character_name,
        )
        .await
        .map_err(character_portrait_record_error)
    }

    async fn portrait_reference_images_by_ids(
        &self,
        project_id: i64,
        image_ids: &[i64],
    ) -> Result<Vec<generated_image::Model>, AppError> {
        a4n_db::character::portrait_reference_images_by_ids(
            self.database.connection(),
            project_id,
            image_ids,
        )
        .await
        .map_err(character_portrait_record_error)
    }

    // —— attributes ——

    async fn attribute_events_by_ids(
        &self,
        attribute_event_ids: &[i64],
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        a4n_db::character::attribute_events_by_ids(self.database.connection(), attribute_event_ids)
            .await
            .map_err(AppError::Database)
    }

    async fn attribute_history(
        &self,
        profile_id: i64,
        filter: CharacterAttributeEventHistoryFilter,
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        a4n_db::character::attribute_history(self.database.connection(), profile_id, filter)
            .await
            .map_err(character_db_error)
    }

    async fn add_profile_attribute(
        &self,
        input: CharacterAttributeValueChangeInput,
    ) -> Result<CharacterAttributeMutationResult, AppError> {
        a4n_db::character::add_profile_attribute(self.database.connection(), input)
            .await
            .map_err(character_db_error)
    }

    async fn modify_profile_attribute(
        &self,
        input: CharacterAttributeValueChangeInput,
    ) -> Result<CharacterAttributeMutationResult, AppError> {
        a4n_db::character::modify_profile_attribute(self.database.connection(), input)
            .await
            .map_err(character_db_error)
    }

    async fn execute_ready_attribute_deletion(
        &self,
        input: CharacterAttributeDeletionExecutionInput,
    ) -> Result<CharacterAttributeDeletionExecutionResult, AppError> {
        a4n_db::character::execute_ready_attribute_deletion(self.database.connection(), input)
            .await
            .map_err(character_db_error)
    }

    async fn apply_implicit_review_decision_record(
        &self,
        input: CharacterImplicitReviewDecisionRecordInput,
    ) -> Result<AppliedCharacterImplicitDecisionRecord, AppError> {
        a4n_db::character::apply_implicit_review_decision_record(self.database.connection(), input)
            .await
            .map_err(character_db_error)
    }

    // —— behavior / deletion marks ——

    async fn behavior_records_by_profile(
        &self,
        profile_id: i64,
        chapter_from: Option<i32>,
        chapter_to: Option<i32>,
        limit: Option<u64>,
    ) -> Result<Vec<protagonist_behavior_record::Model>, AppError> {
        a4n_db::character::behavior_records_by_profile(
            self.database.connection(),
            profile_id,
            chapter_from,
            chapter_to,
            limit,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn implicit_review_behavior_records(
        &self,
        profile_id: i64,
    ) -> Result<Vec<protagonist_behavior_record::Model>, AppError> {
        a4n_db::character::implicit_review_behavior_records(self.database.connection(), profile_id)
            .await
            .map_err(AppError::Database)
    }

    async fn deletion_marks_by_profile(
        &self,
        profile_id: i64,
        category: Option<&str>,
        status: DeletionMarkStatus,
    ) -> Result<Vec<protagonist_deletion_mark::Model>, AppError> {
        a4n_db::character::deletion_marks_by_profile(
            self.database.connection(),
            profile_id,
            category,
            status,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn request_attribute_deletion(
        &self,
        input: CharacterAttributeDeletionRequestInput,
    ) -> Result<protagonist_deletion_mark::Model, AppError> {
        a4n_db::character::request_attribute_deletion(self.database.connection(), input)
            .await
            .map_err(character_db_error)
    }

    async fn reset_deletion_marks(
        &self,
        profile_id: i64,
        category: &str,
        key: &str,
    ) -> Result<u64, AppError> {
        a4n_db::character::reset_deletion_marks_in(
            self.database.connection(),
            profile_id,
            category,
            key,
        )
        .await
        .map_err(character_db_error)
    }

    async fn behavior_records_before_chapter(
        &self,
        profile_id: i64,
        start_chapter: i32,
    ) -> Result<Vec<protagonist_behavior_record::Model>, AppError> {
        a4n_db::character::behavior_records_before_chapter(
            self.database.connection(),
            profile_id,
            start_chapter,
        )
        .await
        .map_err(AppError::Database)
    }

    // —— snapshots / timeline ——

    async fn snapshot_references_by_profile(
        &self,
        profile_id: i64,
        start_chapter: Option<i32>,
        end_chapter: Option<i32>,
    ) -> Result<Vec<snapshot_reference::Model>, AppError> {
        a4n_db::character::snapshot_references_by_profile(
            self.database.connection(),
            profile_id,
            start_chapter,
            end_chapter,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn snapshot_reference_by_chapter(
        &self,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<Option<snapshot_reference::Model>, AppError> {
        a4n_db::character::snapshot_reference_by_chapter(
            self.database.connection(),
            profile_id,
            chapter_number,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn character_profile_state_through_chapter(
        &self,
        profile: &protagonist_profile::Model,
        chapter_number: i32,
    ) -> Result<Value, AppError> {
        a4n_db::character::replay_profile_state_through_chapter(
            self.database.connection(),
            profile,
            chapter_number,
        )
        .await
        .map_err(character_db_error)
    }

    async fn apply_character_sync_persistence_record(
        &self,
        input: CharacterSyncPersistenceInput,
    ) -> Result<AppliedCharacterSyncPersistenceRecord, AppError> {
        a4n_db::character::apply_character_sync_persistence_record(
            self.database.connection(),
            input,
        )
        .await
        .map_err(character_db_error)
    }

    async fn character_conflict_context_rows(
        &self,
        project_id: i64,
        profile_id: i64,
        start_chapter: Option<i32>,
        end_chapter: Option<i32>,
    ) -> Result<CharacterConflictContextRows, AppError> {
        a4n_db::character::character_conflict_context_rows(
            self.database.connection(),
            project_id,
            profile_id,
            start_chapter,
            end_chapter,
        )
        .await
        .map_err(character_db_error)
    }

    async fn delete_profile_cascade(
        &self,
        profile_id: i64,
    ) -> Result<CharacterProfileDeletionSummary, AppError> {
        a4n_db::character::delete_profile_cascade_transaction(
            self.database.connection(),
            profile_id,
        )
        .await
        .map_err(character_profile_delete_error)
    }

    async fn rollback_profile_to_chapter(
        &self,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<CharacterProfileRollbackResult, AppError> {
        a4n_db::character::rollback_profile_to_chapter(
            self.database.connection(),
            profile_id,
            chapter_number,
        )
        .await
        .map_err(character_db_error)
    }
}

fn character_profile_summary(
    profile: &protagonist_profile::Model,
    timeline_started: bool,
    initial_profile_editable: bool,
) -> Result<CharacterProfileSummary, AppError> {
    Ok(CharacterProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        name: profile.name.clone(),
        gender: profile.gender.clone(),
        age: profile.age.clone(),
        base_attributes: profile.base_attributes.clone(),
        current_state: profile.current_state.clone(),
        character_arc: profile.character_arc.clone(),
        voice_fingerprint: profile.voice_fingerprint.clone(),
        appearance: profile.appearance.clone(),
        background: profile.background.clone(),
        personality: profile.personality.clone(),
        last_synced_chapter: profile.last_synced_chapter,
        timeline_started,
        initial_profile_editable,
        attribute_counts: crate::character::attribute_counts(&profile.current_state)
            .map_err(AppError::InvalidArgument)?,
        created_at: Some(profile.created_at.to_string()),
        updated_at: Some(profile.updated_at.to_string()),
    })
}

fn character_attribute_event_summary(
    event: &attribute_event::Model,
) -> Result<AttributeEventSummary, AppError> {
    let source_kind = CharacterTimelineSourceKind::parse(&event.source_kind).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "角色时间线 source_kind 无效: {}",
            event.source_kind
        ))
    })?;
    let category = CharacterAttributeCategory::parse(&event.category).ok_or_else(|| {
        AppError::InvalidArgument(format!("角色属性分类无效: {}", event.category))
    })?;
    let operation = CharacterAttributeOperation::parse(&event.operation).ok_or_else(|| {
        AppError::InvalidArgument(format!("角色属性操作无效: {}", event.operation))
    })?;
    Ok(AttributeEventSummary {
        id: event.id,
        chapter_number: event.chapter_number,
        source_kind,
        timestamp: Some(event.timestamp.to_string()),
        category,
        key: event.key.clone(),
        operation,
        change_description: event.change_description.clone(),
        old_value: event.old_value.clone(),
        new_value: event.new_value.clone(),
        cause: event.cause.clone(),
        evidence: event.evidence.clone(),
    })
}

fn character_behavior_record_summary(
    record: &protagonist_behavior_record::Model,
) -> Result<BehaviorRecordSummary, AppError> {
    let source_kind = CharacterTimelineSourceKind::parse(&record.source_kind).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "角色时间线 source_kind 无效: {}",
            record.source_kind
        ))
    })?;
    Ok(BehaviorRecordSummary {
        id: record.id,
        chapter_number: record.chapter_number,
        source_kind,
        behavior_description: record.behavior_description.clone(),
        behavior_type: record.behavior_type.clone(),
        original_text: record.original_text.clone(),
        behavior_tags: record.behavior_tags.clone(),
        classification_results: record.classification_results.clone(),
        related_characters: record.related_characters.clone(),
        impact_analysis: record.impact_analysis.clone(),
    })
}

fn character_deletion_mark_summary(
    mark: &protagonist_deletion_mark::Model,
) -> Result<DeletionMarkSummary, AppError> {
    let source_kind = CharacterTimelineSourceKind::parse(&mark.source_kind).ok_or_else(|| {
        AppError::InvalidArgument(format!("角色时间线 source_kind 无效: {}", mark.source_kind))
    })?;
    let category = CharacterAttributeCategory::parse(&mark.category)
        .ok_or_else(|| AppError::InvalidArgument(format!("角色属性分类无效: {}", mark.category)))?;
    Ok(DeletionMarkSummary {
        id: mark.id,
        source_kind,
        category,
        key: mark.key.clone(),
        reason: mark.reason.clone(),
        evidence: mark.evidence.clone(),
        consecutive_count: mark.consecutive_count,
        last_marked_chapter: mark.last_marked_chapter,
        is_executed: mark.is_executed,
        created_at: mark.created_at.to_string(),
    })
}

struct CharacterSnapshotData {
    attributes: Value,
    attribute_counts: AttributeCountsSummary,
    state_summary: Option<String>,
    current_location: Option<String>,
    changes_in_chapter: usize,
    behaviors_in_chapter: usize,
}

fn character_snapshot_data(
    snapshot: &snapshot_reference::Model,
) -> Result<CharacterSnapshotData, AppError> {
    let attributes =
        a4n_db::character::snapshot_attributes_from_data(&snapshot.data, snapshot.chapter_number)
            .map_err(character_db_error)?;
    let attribute_counts =
        crate::character::attribute_counts(&attributes).map_err(AppError::InvalidArgument)?;
    let label = format!("第 {} 章角色快照", snapshot.chapter_number);
    let field = |name: &str| {
        snapshot
            .data
            .get(name)
            .ok_or_else(|| AppError::InvalidArgument(format!("{label}缺少字段 {name}")))?
            .as_u64()
            .ok_or_else(|| AppError::InvalidArgument(format!("{label}字段 {name} 必须是非负整数")))
            .and_then(|value| {
                usize::try_from(value).map_err(|_| {
                    AppError::InvalidArgument(format!("{label}字段 {name} 超出 usize 范围"))
                })
            })
    };
    Ok(CharacterSnapshotData {
        attributes,
        attribute_counts,
        state_summary: snapshot
            .data
            .get("state_summary")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        current_location: snapshot
            .data
            .get("current_location")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        changes_in_chapter: field("changes_in_chapter")?,
        behaviors_in_chapter: field("behaviors_in_chapter")?,
    })
}

fn character_snapshot_reference_summary(
    snapshot: &snapshot_reference::Model,
) -> Result<SnapshotReferenceSummary, AppError> {
    let data = character_snapshot_data(snapshot)?;
    Ok(SnapshotReferenceSummary {
        id: snapshot.id,
        chapter_number: snapshot.chapter_number,
        snapshot_hash: snapshot.snapshot_hash.clone(),
        attribute_counts: data.attribute_counts,
        state_summary: data.state_summary,
        changes_in_chapter: data.changes_in_chapter,
        behaviors_in_chapter: data.behaviors_in_chapter,
    })
}

fn character_profile_state_summary(
    snapshot: &snapshot_reference::Model,
) -> Result<ProfileChapterStateSummary, AppError> {
    let data = character_snapshot_data(snapshot)?;
    let category = |name: &str| {
        data.attributes
            .get(name)
            .and_then(Value::as_object)
            .map(|object| Value::Object(object.clone()))
            .ok_or_else(|| AppError::InvalidArgument(format!("角色快照属性缺少分类 {name}")))
    };
    Ok(ProfileChapterStateSummary {
        chapter_number: snapshot.chapter_number,
        attribute_counts: data.attribute_counts,
        explicit_attributes: category("explicit_attributes")?,
        implicit_attributes: category("implicit_attributes")?,
        social_attributes: category("social_attributes")?,
        state_summary: data.state_summary,
        current_location: data.current_location,
        changes_in_chapter: data.changes_in_chapter,
        behaviors_in_chapter: data.behaviors_in_chapter,
        created_at: Some(snapshot.created_at.to_string()),
    })
}

impl OrchestratorContext {
    async fn character_profile_by_name_for_user(
        &self,
        user_id: i64,
        project_id: i64,
        name: String,
    ) -> Result<protagonist_profile::Model, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let name = required_text(&name, "角色名称").map_err(AppError::InvalidArgument)?;
        self.profile_by_project_name(project_id, name)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("角色不存在: project_id={project_id}, name={name}"))
            })
    }
}

#[async_trait::async_trait]
impl CharacterProfileReadApplicationPort for OrchestratorContext {
    async fn profile_project_id(&self, user_id: i64, profile_id: i64) -> Result<i64, AppError> {
        Ok(self
            .character_profile_for_user_validated(user_id, profile_id)
            .await?
            .project_id)
    }

    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileSummary>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.profiles_with_timeline_state(project_id)
            .await?
            .iter()
            .map(|(profile, timeline_started, editable)| {
                character_profile_summary(profile, *timeline_started, *editable)
            })
            .collect()
    }

    async fn character_arc_by_name(
        &self,
        user_id: i64,
        project_id: i64,
        name: String,
    ) -> Result<Option<Value>, AppError> {
        Ok(self
            .character_profile_by_name_for_user(user_id, project_id, name)
            .await?
            .character_arc)
    }

    async fn voice_fingerprint_by_name(
        &self,
        user_id: i64,
        project_id: i64,
        name: String,
    ) -> Result<Option<Value>, AppError> {
        Ok(self
            .character_profile_by_name_for_user(user_id, project_id, name)
            .await?
            .voice_fingerprint)
    }

    async fn attribute_history(
        &self,
        user_id: i64,
        input: AttributeHistoryInput,
    ) -> Result<Vec<AttributeEventSummary>, AppError> {
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        let events = OrchestratorContext::attribute_history(
            self,
            input.profile_id,
            CharacterAttributeEventHistoryFilter {
                chapter_from: input.chapter_from,
                chapter_to: input.chapter_to,
                category: input.category.map(|value| value.as_str().to_string()),
                key: input.key,
                operation: input.operation.map(|value| value.as_str().to_string()),
                limit: input.limit,
            },
        )
        .await?;
        events
            .iter()
            .map(character_attribute_event_summary)
            .collect()
    }

    async fn behavior_history(
        &self,
        user_id: i64,
        input: ListBehaviorsInput,
    ) -> Result<Vec<BehaviorRecordSummary>, AppError> {
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        let records = OrchestratorContext::behavior_records_by_profile(
            self,
            input.profile_id,
            input.chapter_from,
            input.chapter_to,
            input.limit,
        )
        .await?;
        records
            .iter()
            .map(character_behavior_record_summary)
            .collect()
    }

    async fn deletion_marks(
        &self,
        user_id: i64,
        input: ListDeletionMarksInput,
    ) -> Result<Vec<DeletionMarkSummary>, AppError> {
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        self.deletion_marks_by_profile_validated(input.profile_id, input.category, input.status)
            .await?
            .iter()
            .map(character_deletion_mark_summary)
            .collect()
    }

    async fn snapshots(
        &self,
        user_id: i64,
        input: ListSnapshotsInput,
    ) -> Result<Vec<SnapshotReferenceSummary>, AppError> {
        validate_snapshot_range(input.start_chapter, input.end_chapter)?;
        let profile = self
            .character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        let snapshots = self
            .snapshot_references_by_profile(profile.id, input.start_chapter, input.end_chapter)
            .await?;
        snapshots
            .iter()
            .map(character_snapshot_reference_summary)
            .collect()
    }

    async fn state_at_chapter(
        &self,
        user_id: i64,
        input: GetProfileStateAtChapterInput,
    ) -> Result<ProfileChapterStateSummary, AppError> {
        ensure_positive_snapshot_chapter(input.chapter_number)?;
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        let snapshot = self
            .snapshot_reference_by_chapter(input.profile_id, input.chapter_number)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("第 {} 章没有角色快照", input.chapter_number))
            })?;
        character_profile_state_summary(&snapshot)
    }

    async fn snapshot_by_hash(
        &self,
        user_id: i64,
        input: GetSnapshotInput,
    ) -> Result<SnapshotSummary, AppError> {
        let content_hash =
            required_text(&input.content_hash, "快照哈希").map_err(AppError::InvalidArgument)?;
        let snapshot = self
            .character_snapshot_by_hash(content_hash)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("快照不存在: {content_hash}")))?;
        self.character_profile_for_user_validated(user_id, snapshot.entity_id)
            .await?;
        if snapshot.entity_type != CHARACTER_SNAPSHOT_ENTITY_TYPE {
            return Err(AppError::InvalidArgument(format!(
                "快照类型不匹配: expected {CHARACTER_SNAPSHOT_ENTITY_TYPE}, got {}",
                snapshot.entity_type
            )));
        }
        Ok(SnapshotSummary {
            data: snapshot.data,
            created_at: Some(snapshot.created_at.to_string()),
        })
    }

    async fn diff_chapters(
        &self,
        user_id: i64,
        input: DiffProfileChaptersInput,
    ) -> Result<SnapshotDeltaSummary, AppError> {
        let delta = self
            .diff_profile_snapshots_for_user(
                user_id,
                input.profile_id,
                input.from_chapter,
                input.to_chapter,
            )
            .await?;
        Ok(SnapshotDeltaSummary { delta: delta.delta })
    }

    async fn sync_conflict(
        &self,
        user_id: i64,
        input: CheckProfileSyncConflictInput,
    ) -> Result<ProfileSyncConflictSummary, AppError> {
        self.character_sync_conflict_for_user(user_id, input.profile_id)
            .await
    }
}

#[async_trait::async_trait]
impl CharacterProfileMutationApplicationPort for OrchestratorContext {
    async fn create(
        &self,
        user_id: i64,
        input: CreateProfileInput,
    ) -> Result<CharacterProfileCreateResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let profile = self
            .create_profile_validated(input.project_id, input.name, input.base_attributes)
            .await?;
        Ok(CharacterProfileCreateResult {
            profile_id: profile.id,
            warnings: Vec::new(),
        })
    }

    async fn update(
        &self,
        user_id: i64,
        input: UpdateCharacterProfileInput,
    ) -> Result<CharacterProfileMutationResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let profile = self
            .update_initial_profile_validated(
                input.project_id,
                input.profile_id,
                input.expected_current_state,
                input.base_attributes,
            )
            .await?;
        Ok(CharacterProfileMutationResult {
            profile: character_profile_summary(&profile, false, true)?,
            warnings: Vec::new(),
        })
    }
}

#[async_trait::async_trait]
impl CharacterProfileDeletionApplicationPort for OrchestratorContext {
    async fn delete(
        &self,
        user_id: i64,
        profile_id: i64,
    ) -> Result<CharacterProfileDeleteResult, AppError> {
        self.character_profile_for_user_validated(user_id, profile_id)
            .await?;
        match self
            .delete_character_profile_records_validated(profile_id)
            .await
        {
            Ok(_) => Ok(CharacterProfileDeleteResult {
                warnings: Vec::new(),
            }),
            Err(CharacterRecordLifecycleError::Persistence(error)) => Err(error),
            Err(CharacterRecordLifecycleError::Rag(error)) => Ok(CharacterProfileDeleteResult {
                warnings: vec![character_profile_rag_warning(error)],
            }),
        }
    }
}

#[async_trait::async_trait]
impl CharacterProfileTimelineMutationApplicationPort for OrchestratorContext {
    async fn rollback(
        &self,
        user_id: i64,
        input: RollbackToChapterInput,
    ) -> Result<RollbackResultSummary, AppError> {
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        let result = self
            .rollback_character_profile_timeline_validated(input.profile_id, input.chapter_number)
            .await
            .map_err(character_record_lifecycle_error)?;
        Ok(RollbackResultSummary {
            deleted_events: result.rollback.deleted_timeline.deleted_attribute_events,
            deleted_snapshots: result.rollback.deleted_timeline.deleted_snapshots,
            deleted_behaviors: result.rollback.deleted_timeline.deleted_behaviors,
            deleted_state_indexes: result.rollback.deleted_timeline.deleted_state_indexes,
            deleted_state_rag: result.rag.deleted_state_rag,
            deleted_attribute_event_rag: result.rag.deleted_attribute_event_rag,
            deleted_deletion_marks: result.rollback.deletion_mark_rewind.deleted,
            rewound_deletion_marks: result.rollback.deletion_mark_rewind.rewound,
        })
    }
}

#[async_trait::async_trait]
impl CharacterProfileAttributeMutationApplicationPort for OrchestratorContext {
    async fn add(&self, user_id: i64, input: AddProfileAttributeInput) -> Result<(), AppError> {
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        self.add_profile_attribute_validated(CharacterAttributeValueChangeInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            category: input.category.as_str().to_string(),
            key: input.key,
            value: input.value,
            change_description: None,
            cause: input.cause,
            evidence: input.evidence,
        })
        .await
        .map_err(character_attribute_lifecycle_error)?;
        Ok(())
    }

    async fn modify(
        &self,
        user_id: i64,
        input: ModifyProfileAttributeInput,
    ) -> Result<(), AppError> {
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        self.modify_profile_attribute_validated(CharacterAttributeValueChangeInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            category: input.category.as_str().to_string(),
            key: input.key,
            value: input.new_value,
            change_description: None,
            cause: input.cause,
            evidence: input.evidence,
        })
        .await
        .map_err(character_attribute_lifecycle_error)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl CharacterProfileAttributeDeletionApplicationPort for OrchestratorContext {
    async fn request(
        &self,
        user_id: i64,
        input: RequestAttributeDeletionInput,
    ) -> Result<DeletionMarkSummary, AppError> {
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        let mark = self
            .request_attribute_deletion_validated(CharacterAttributeDeletionRequestInput {
                profile_id: input.profile_id,
                category: input.category.as_str().to_string(),
                key: input.key,
                chapter_number: input.chapter_number,
                reason: input.reason,
                evidence: input.evidence,
            })
            .await?;
        character_deletion_mark_summary(&mark)
    }

    async fn execute(
        &self,
        user_id: i64,
        input: ExecuteAttributeDeletionInput,
    ) -> Result<(), AppError> {
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        self.execute_ready_attribute_deletion_validated(CharacterAttributeDeletionExecutionInput {
            profile_id: input.profile_id,
            category: input.category.as_str().to_string(),
            key: input.key,
            event_cause: input.cause,
            evidence: input.evidence,
            chapter_number: input.chapter_number,
        })
        .await
        .map_err(character_attribute_lifecycle_error)?;
        Ok(())
    }

    async fn reset(&self, user_id: i64, input: ResetDeletionMarksInput) -> Result<u64, AppError> {
        self.character_profile_for_user_validated(user_id, input.profile_id)
            .await?;
        self.reset_deletion_marks_validated(input.profile_id, input.category, &input.key)
            .await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::character::CharacterImplicitReviewReadApplicationPort
    for OrchestratorContext
{
    async fn summary(
        &self,
        user_id: i64,
        input: GetImplicitReviewInput,
    ) -> Result<ImplicitReviewSummary, AppError> {
        self.implicit_review_summary_validated(user_id, input).await
    }
}

#[async_trait::async_trait]
impl crate::character::CharacterImplicitReviewCheckApplicationPort for OrchestratorContext {
    async fn check(
        &self,
        user_id: i64,
        input: CheckImplicitUpdateInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ImplicitUpdateDecisionSummary, AppError> {
        self.check_implicit_update_validated(user_id, input, prompt_renderer)
            .await
    }
}

#[async_trait::async_trait]
impl crate::character::CharacterImplicitReviewPreviewApplicationPort for OrchestratorContext {
    async fn preview(
        &self,
        user_id: i64,
        input: CheckImplicitUpdateInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<a4n_llm_provider_contract::provider::ChatRequest, AppError> {
        self.prepare_implicit_update_review_request_preview_validated(
            user_id,
            input,
            prompt_renderer,
        )
        .await
    }
}

#[async_trait::async_trait]
impl crate::character::CharacterImplicitReviewApplyApplicationPort for OrchestratorContext {
    async fn apply(
        &self,
        user_id: i64,
        input: a4n_api_contract::character::ApplyImplicitUpdateDecisionInput,
    ) -> Result<a4n_api_contract::character::ApplyImplicitUpdateDecisionResultSummary, AppError>
    {
        let applied = self
            .apply_implicit_update_decision_validated(user_id, input)
            .await?;
        let action = match applied.action {
            crate::character::AppliedCharacterImplicitDecisionAction::Keep => {
                a4n_domain::character::AppliedImplicitUpdateAction::Keep
            }
            crate::character::AppliedCharacterImplicitDecisionAction::Modify => {
                a4n_domain::character::AppliedImplicitUpdateAction::Modify
            }
            crate::character::AppliedCharacterImplicitDecisionAction::MarkDeletion => {
                a4n_domain::character::AppliedImplicitUpdateAction::MarkDeletion
            }
        };
        Ok(
            a4n_api_contract::character::ApplyImplicitUpdateDecisionResultSummary {
                action,
                deletion_mark: applied
                    .deletion_mark
                    .as_ref()
                    .map(character_deletion_mark_summary)
                    .transpose()?,
            },
        )
    }
}

fn character_record_lifecycle_error(error: CharacterRecordLifecycleError) -> AppError {
    match error {
        CharacterRecordLifecycleError::Persistence(error) => error,
        CharacterRecordLifecycleError::Rag(error) => super::rag_ingest::map_sync_error(error),
    }
}

fn character_attribute_lifecycle_error(error: CharacterAttributeLifecycleError) -> AppError {
    match error {
        CharacterAttributeLifecycleError::Persistence(error) => error,
        CharacterAttributeLifecycleError::Rag(message) => {
            AppError::Internal(anyhow::anyhow!(message))
        }
    }
}

fn character_profile_rag_warning(error: impl std::fmt::Display) -> String {
    format!("角色资料检索索引未完全更新：{error}。正式资料未受影响，可稍后在项目操作中重新同步")
}

#[async_trait::async_trait]
impl PortraitRecordMutationApplicationPort for OrchestratorContext {
    async fn set_primary(&self, user_id: i64, portrait_id: i64) -> Result<(), AppError> {
        let portrait = self
            .character_portrait_by_id(portrait_id)
            .await?
            .ok_or_else(|| AppError::NotFound("立绘不存在".to_string()))?;
        self.verify_project_permission(portrait.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.set_primary_character_portrait_record(portrait_id)
            .await
    }
}

impl OrchestratorContext {
    // —— profiles ——

    pub(crate) async fn profiles_with_timeline_state(
        &self,
        project_id: i64,
    ) -> Result<Vec<(protagonist_profile::Model, bool, bool)>, AppError> {
        self.character
            .profiles_with_timeline_state(project_id)
            .await
    }

    pub(crate) async fn profile_by_id(
        &self,
        profile_id: i64,
    ) -> Result<Option<protagonist_profile::Model>, AppError> {
        self.character.profile_by_id(profile_id).await
    }

    pub(crate) async fn profile_by_project_name(
        &self,
        project_id: i64,
        name: &str,
    ) -> Result<Option<protagonist_profile::Model>, AppError> {
        self.character
            .profile_by_project_name(project_id, name)
            .await
    }

    pub(crate) async fn missing_portrait_profiles(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        self.character.missing_portrait_profiles(project_id).await
    }

    /// 创建角色档案：校验姓名后落库。
    pub(crate) async fn create_profile_validated(
        &self,
        project_id: i64,
        name: String,
        base_attributes: Value,
    ) -> Result<protagonist_profile::Model, AppError> {
        self.character
            .create_profile_validated(project_id, name, base_attributes)
            .await
    }

    pub(crate) async fn update_initial_profile_validated(
        &self,
        project_id: i64,
        profile_id: i64,
        expected_current_state: Value,
        base_attributes: Value,
    ) -> Result<protagonist_profile::Model, AppError> {
        self.character
            .update_initial_profile(
                project_id,
                profile_id,
                expected_current_state,
                base_attributes,
            )
            .await
    }

    // —— portraits ——

    pub(crate) async fn character_portrait_by_id(
        &self,
        portrait_id: i64,
    ) -> Result<Option<character_portrait::Model>, AppError> {
        self.character.character_portrait_by_id(portrait_id).await
    }

    pub(crate) async fn character_portrait_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<character_portrait::Model>, AppError> {
        self.character
            .character_portrait_records_by_project(project_id)
            .await
    }

    pub(crate) async fn create_character_portrait_for_profile(
        &self,
        input: CharacterPortraitCreateInput,
    ) -> Result<character_portrait::Model, AppError> {
        self.character
            .create_character_portrait_for_profile(input)
            .await
    }

    pub(crate) async fn set_primary_character_portrait_record(
        &self,
        portrait_id: i64,
    ) -> Result<(), AppError> {
        self.character
            .set_primary_character_portrait_record(portrait_id)
            .await
    }

    pub(crate) async fn portrait_prompt_profile_rows(
        &self,
        project_id: i64,
        profile_id: i64,
    ) -> Result<PortraitPromptProfileRows, AppError> {
        self.character
            .portrait_prompt_profile_rows(project_id, profile_id)
            .await
    }

    pub(crate) async fn portrait_named_prompt_profile_rows(
        &self,
        project_id: i64,
        character_name: &str,
    ) -> Result<PortraitNamedPromptProfileRows, AppError> {
        self.character
            .portrait_named_prompt_profile_rows(project_id, character_name)
            .await
    }

    pub(crate) async fn portrait_reference_images_by_ids(
        &self,
        project_id: i64,
        image_ids: &[i64],
    ) -> Result<Vec<generated_image::Model>, AppError> {
        self.character
            .portrait_reference_images_by_ids(project_id, image_ids)
            .await
    }

    // —— attributes ——

    pub(crate) async fn attribute_events_by_ids(
        &self,
        attribute_event_ids: &[i64],
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        self.character
            .attribute_events_by_ids(attribute_event_ids)
            .await
    }

    pub(crate) async fn attribute_history(
        &self,
        profile_id: i64,
        filter: CharacterAttributeEventHistoryFilter,
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        self.character.attribute_history(profile_id, filter).await
    }

    /// 规范化属性输入后新增属性并同步 RAG。
    pub(crate) async fn add_profile_attribute_validated(
        &self,
        input: CharacterAttributeValueChangeInput,
    ) -> Result<CharacterAttributeMutationResult, CharacterAttributeLifecycleError> {
        let result = self
            .character
            .add_profile_attribute(input)
            .await
            .map_err(CharacterAttributeLifecycleError::Persistence)?;
        self.sync_attribute_event_after_write(&result.profile, &result.event)
            .await?;
        Ok(result)
    }

    /// 规范化属性输入后修改属性并同步 RAG。
    pub(crate) async fn modify_profile_attribute_validated(
        &self,
        input: CharacterAttributeValueChangeInput,
    ) -> Result<CharacterAttributeMutationResult, CharacterAttributeLifecycleError> {
        let result = self
            .character
            .modify_profile_attribute(input)
            .await
            .map_err(CharacterAttributeLifecycleError::Persistence)?;
        self.sync_attribute_event_after_write(&result.profile, &result.event)
            .await?;
        Ok(result)
    }

    /// 规范化后执行就绪的属性删除并同步 RAG。
    pub(crate) async fn execute_ready_attribute_deletion_validated(
        &self,
        input: CharacterAttributeDeletionExecutionInput,
    ) -> Result<CharacterAttributeDeletionExecutionResult, CharacterAttributeLifecycleError> {
        let result = self
            .character
            .execute_ready_attribute_deletion(input)
            .await
            .map_err(CharacterAttributeLifecycleError::Persistence)?;
        self.sync_attribute_event_after_write(&result.profile, &result.event)
            .await?;
        Ok(result)
    }

    pub(crate) async fn apply_implicit_review_decision_record(
        &self,
        input: CharacterImplicitReviewDecisionRecordInput,
    ) -> Result<AppliedCharacterImplicitDecisionRecord, CharacterAttributeLifecycleError> {
        let applied = self
            .character
            .apply_implicit_review_decision_record(input)
            .await
            .map_err(CharacterAttributeLifecycleError::Persistence)?;
        if let Some(event) = crate::character::implicit_review_attribute_event(&applied) {
            self.sync_attribute_event_after_write(&applied.profile, event)
                .await?;
        }
        Ok(applied)
    }

    async fn sync_attribute_event_after_write(
        &self,
        profile: &protagonist_profile::Model,
        event: &attribute_event::Model,
    ) -> Result<(), CharacterAttributeLifecycleError> {
        self.sync_character_attribute_event_rag(profile.project_id, event, &profile.name)
            .await
            .map_err(CharacterAttributeLifecycleError::rag)
    }

    // —— behavior ——

    pub(crate) async fn behavior_records_by_profile(
        &self,
        profile_id: i64,
        chapter_from: Option<i32>,
        chapter_to: Option<i32>,
        limit: Option<u64>,
    ) -> Result<Vec<protagonist_behavior_record::Model>, AppError> {
        if let Some(limit) = limit
            && !(1..=100).contains(&limit)
        {
            return Err(AppError::InvalidArgument(
                "行为记录 limit 必须在 1..=100 之间".to_string(),
            ));
        }
        self.character
            .behavior_records_by_profile(profile_id, chapter_from, chapter_to, limit)
            .await
    }

    pub(crate) async fn implicit_review_behavior_records(
        &self,
        profile_id: i64,
    ) -> Result<Vec<protagonist_behavior_record::Model>, AppError> {
        self.character
            .implicit_review_behavior_records(profile_id)
            .await
    }

    /// 解析列表过滤后读取删除标记。
    pub(crate) async fn deletion_marks_by_profile_validated(
        &self,
        profile_id: i64,
        category: Option<CharacterAttributeCategory>,
        status: DeletionMarkStatus,
    ) -> Result<Vec<protagonist_deletion_mark::Model>, AppError> {
        self.character
            .deletion_marks_by_profile(
                profile_id,
                category.as_ref().map(|value| value.as_str()),
                status,
            )
            .await
    }

    /// 规范化后请求属性删除标记。
    pub(crate) async fn request_attribute_deletion_validated(
        &self,
        input: CharacterAttributeDeletionRequestInput,
    ) -> Result<protagonist_deletion_mark::Model, AppError> {
        self.character.request_attribute_deletion(input).await
    }

    /// 规范化分类/key 后重置删除标记。
    pub(crate) async fn reset_deletion_marks_validated(
        &self,
        profile_id: i64,
        category: CharacterAttributeCategory,
        key: &str,
    ) -> Result<u64, AppError> {
        self.character
            .reset_deletion_marks(profile_id, category.as_str(), key)
            .await
    }

    pub(crate) async fn behavior_records_before_chapter(
        &self,
        profile_id: i64,
        start_chapter: i32,
    ) -> Result<Vec<protagonist_behavior_record::Model>, AppError> {
        self.character
            .behavior_records_before_chapter(profile_id, start_chapter)
            .await
    }

    // —— snapshots / timeline ——

    pub(crate) async fn snapshot_references_by_profile(
        &self,
        profile_id: i64,
        start_chapter: Option<i32>,
        end_chapter: Option<i32>,
    ) -> Result<Vec<snapshot_reference::Model>, AppError> {
        self.character
            .snapshot_references_by_profile(profile_id, start_chapter, end_chapter)
            .await
    }

    pub(crate) async fn snapshot_reference_by_chapter(
        &self,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<Option<snapshot_reference::Model>, AppError> {
        self.character
            .snapshot_reference_by_chapter(profile_id, chapter_number)
            .await
    }

    pub(crate) async fn character_profile_state_through_chapter(
        &self,
        profile: &protagonist_profile::Model,
        chapter_number: i32,
    ) -> Result<Value, AppError> {
        self.character
            .character_profile_state_through_chapter(profile, chapter_number)
            .await
    }

    pub(crate) async fn character_profile_for_user_validated(
        &self,
        user_id: i64,
        profile_id: i64,
    ) -> Result<protagonist_profile::Model, AppError> {
        let profile = self
            .profile_by_id(profile_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("角色档案不存在: {profile_id}")))?;
        self.verify_project_permission(profile.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(profile)
    }

    async fn diff_profile_snapshots_for_user(
        &self,
        user_id: i64,
        profile_id: i64,
        from_chapter: i32,
        to_chapter: i32,
    ) -> Result<SnapshotDelta, AppError> {
        validate_snapshot_diff_chapters(from_chapter, to_chapter)?;
        self.character_profile_for_user_validated(user_id, profile_id)
            .await?;

        let (to_ref_result, from_ref_result) = futures::join!(
            self.snapshot_reference_by_chapter(profile_id, to_chapter),
            async {
                if from_chapter == 0 {
                    Ok(None)
                } else {
                    self.snapshot_reference_by_chapter(profile_id, from_chapter)
                        .await
                }
            },
        );
        let to_ref = to_ref_result?
            .ok_or_else(|| AppError::NotFound(format!("第 {to_chapter} 章没有角色快照")))?;
        let from_ref = match from_ref_result {
            Ok(from_ref) => from_ref,
            Err(error) => {
                // Preserve the original error precedence: the target snapshot was
                // validated before the source reference was inspected.
                self.character_snapshot_by_hash(&to_ref.snapshot_hash)
                    .await?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("快照不存在: {}", to_ref.snapshot_hash))
                    })?;
                return Err(error);
            }
        };
        let (to_snapshot_result, from_snapshot_result) = match from_ref.as_ref() {
            Some(from_ref) => futures::join!(
                self.character_snapshot_by_hash(&to_ref.snapshot_hash),
                self.character_snapshot_by_hash(&from_ref.snapshot_hash),
            ),
            None => (
                self.character_snapshot_by_hash(&to_ref.snapshot_hash).await,
                Ok(None),
            ),
        };
        let to_snapshot = to_snapshot_result?
            .ok_or_else(|| AppError::NotFound(format!("快照不存在: {}", to_ref.snapshot_hash)))?;
        let (from_hash, from_data) = match from_ref {
            Some(from_ref) => {
                let from_snapshot = from_snapshot_result?.ok_or_else(|| {
                    AppError::NotFound(format!("快照不存在: {}", from_ref.snapshot_hash))
                })?;
                (from_ref.snapshot_hash, from_snapshot.data)
            }
            None => ("initial-state".to_string(), initial_snapshot_data()),
        };

        Ok(SnapshotDelta {
            from_hash,
            to_hash: to_ref.snapshot_hash,
            delta: a4n_db::character::Sha256SnapshotStore::diff_snapshot_data(
                &from_data,
                &to_snapshot.data,
            )?,
        })
    }

    async fn character_sync_conflict_for_user(
        &self,
        user_id: i64,
        profile_id: i64,
    ) -> Result<ProfileSyncConflictSummary, AppError> {
        let profile = self
            .character_profile_for_user_validated(user_id, profile_id)
            .await?;
        let available_chapters = self.character_sync_chapters(profile.project_id).await?;
        let max_available_chapter = available_chapters
            .iter()
            .map(|chapter| chapter.chapter_number)
            .max()
            .unwrap_or_default();
        let snapshots = self
            .snapshot_references_by_profile(profile_id, None, Some(max_available_chapter))
            .await?;
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

    pub(crate) async fn apply_character_sync_persistence_record(
        &self,
        input: CharacterSyncPersistenceInput,
    ) -> Result<AppliedCharacterSyncPersistenceRecord, AppError> {
        self.character
            .apply_character_sync_persistence_record(input)
            .await
    }

    pub(crate) async fn character_conflict_context_rows(
        &self,
        project_id: i64,
        profile_id: i64,
        start_chapter: Option<i32>,
        end_chapter: Option<i32>,
    ) -> Result<CharacterConflictContextRows, AppError> {
        self.character
            .character_conflict_context_rows(project_id, profile_id, start_chapter, end_chapter)
            .await
    }

    /// 删除角色档案记录并同步清理 RAG（文件副作用仍由 Host 执行）。
    pub(crate) async fn delete_character_profile_records_validated(
        &self,
        profile_id: i64,
    ) -> Result<CharacterProfileRecordDeletionResult, CharacterRecordLifecycleError> {
        if profile_id <= 0 {
            return Err(CharacterRecordLifecycleError::Persistence(
                AppError::InvalidArgument("角色档案 ID 必须大于 0".to_string()),
            ));
        }
        let deletion = self
            .character
            .delete_profile_cascade(profile_id)
            .await
            .map_err(CharacterRecordLifecycleError::Persistence)?;
        let rag = self
            .delete_character_profile_rag_records(&deletion)
            .await
            .map_err(CharacterRecordLifecycleError::Rag)?;
        Ok(CharacterProfileRecordDeletionResult { deletion, rag })
    }

    /// 校验章节号后回滚角色时间线并清理 RAG。
    pub(crate) async fn rollback_character_profile_timeline_validated(
        &self,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<CharacterTimelineRollbackLifecycleResult, CharacterRecordLifecycleError> {
        if chapter_number < 0 {
            return Err(CharacterRecordLifecycleError::Persistence(
                AppError::InvalidArgument("回滚章节号不能小于 0".to_string()),
            ));
        }
        let rollback = self
            .character
            .rollback_profile_to_chapter(profile_id, chapter_number)
            .await
            .map_err(CharacterRecordLifecycleError::Persistence)?;
        let rag = self
            .delete_character_timeline_rag_records(rollback.project_id, &rollback.deleted_timeline)
            .await
            .map_err(CharacterRecordLifecycleError::Rag)?;
        Ok(CharacterTimelineRollbackLifecycleResult { rollback, rag })
    }

    /// 角色档案同步 application 内部创建/恢复 workflow_run（RUNNING）。
    ///
    /// 走 begin_or_resume 而非裸 upsert：同一角色 + 同一目标章节已有 RUNNING 运行时必须
    /// 报冲突，否则两个并发同步会接管同一行 workflow_run，各自跑完整套 LLM 后由 DB 乐观锁
    /// 淘汰一个。失败/中断的运行则按 checkpoint 恢复。
    pub(crate) async fn begin_character_profile_sync_workflow_validated(
        &self,
        project_id: i64,
        profile_id: i64,
        profile_name: &str,
        last_synced_chapter: i32,
        start_chapter: i32,
        target_chapter: i32,
        clear_to_chapter: i32,
        selected_chapter_numbers: &[i32],
        baseline_state: &Value,
        base_attributes: &Value,
        chapter_data: &Value,
        active_task_id: Option<&str>,
    ) -> Result<a4n_db_models::models::generation::workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        if profile_id <= 0 {
            return Err(AppError::InvalidArgument(
                "角色档案 ID 必须大于 0".to_string(),
            ));
        }
        if target_chapter <= 0 {
            return Err(AppError::InvalidArgument(
                "同步目标章节必须大于 0".to_string(),
            ));
        }
        let fingerprint = crate::character::character_profile_sync_workflow_fingerprint(
            project_id,
            profile_id,
            profile_name,
            last_synced_chapter,
            start_chapter,
            target_chapter,
            clear_to_chapter,
            selected_chapter_numbers,
            baseline_state,
            base_attributes,
            chapter_data,
        );
        self.begin_or_resume_workflow_run_validated(crate::workflow::UpsertWorkflowRunInput {
            business_key: crate::character::character_profile_sync_business_key(
                profile_id,
                target_chapter,
            ),
            workflow_kind: "character_profile_sync".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(profile_id),
            chapter_id: None,
            mode: Some(crate::character::character_profile_sync_mode_label(
                start_chapter,
                target_chapter,
            )),
            input_fingerprint_json: Some(fingerprint),
            status: crate::workflow::WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("character_sync.start".to_string()),
            active_task_id: active_task_id.map(str::to_string),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }
}
