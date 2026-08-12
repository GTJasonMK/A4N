use a4n_core::error::AppError;
use a4n_core::event::snapshot::{Snapshot, SnapshotDelta};
use a4n_core::models::ai::generated_image;
use a4n_core::models::character::{
    attribute_event, character_portrait, protagonist_behavior_record, protagonist_deletion_mark,
    protagonist_profile, snapshot_reference,
};
use a4n_db::DbHandle;
use a4n_domain::character::{CharacterAttributeCategory, DeletionMarkStatus};
use serde_json::Value;

use super::OrchestratorContext;
use crate::character::{
    AppliedCharacterImplicitDecisionRecord, AppliedCharacterSyncPersistenceRecord,
    CharacterAttributeDeletionExecutionInput, CharacterAttributeDeletionExecutionResult,
    CharacterAttributeDeletionRequestInput, CharacterAttributeEventHistoryFilter,
    CharacterAttributeLifecycleError, CharacterAttributeMutationResult,
    CharacterAttributeValueChangeInput, CharacterConflictContextRows,
    CharacterImplicitReviewDecisionRecordInput, CharacterPortraitCreateInput,
    CharacterPortraitRollbackResult, CharacterProfileDeletionSummary,
    CharacterProfileRecordDeletionResult, CharacterProfileRollbackResult,
    CharacterRecordLifecycleError, CharacterSyncPersistenceInput,
    CharacterTimelineRollbackLifecycleResult, PortraitNamedPromptProfileRows,
    PortraitPromptProfileRows, character_db_error, character_profile_delete_error, profiles,
};
use crate::character::{
    CHARACTER_SNAPSHOT_ENTITY_TYPE, CharacterSyncConflictReadModel,
    ensure_positive_snapshot_chapter, initial_snapshot_data, validate_snapshot_diff_chapters,
    validate_snapshot_range,
};
use crate::collaboration::ProjectAccessRole;
use crate::project::required_text;
use profiles::errors::{
    character_portrait_record_error, character_portrait_rollback_error,
    character_profile_create_error,
};

#[derive(Clone)]
pub(super) struct CharacterApplication {
    database: DbHandle,
}

impl CharacterApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    // —— profiles ——

    async fn profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        a4n_db::character::profiles_by_project(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
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

    async fn rollback_failed_portrait_creation_records(
        &self,
        portrait_id: i64,
        image_path: &str,
    ) -> Result<CharacterPortraitRollbackResult, AppError> {
        a4n_db::character::rollback_failed_portrait_creation_records(
            self.database.connection(),
            portrait_id,
            image_path,
        )
        .await
        .map_err(character_portrait_rollback_error)
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

    async fn create_character_portrait_record(
        &self,
        input: CharacterPortraitCreateInput,
    ) -> Result<character_portrait::Model, AppError> {
        a4n_db::character::create_character_portrait_record(self.database.connection(), input)
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

    async fn primary_character_portrait_record(
        &self,
        profile_id: i64,
    ) -> Result<Option<character_portrait::Model>, AppError> {
        a4n_db::character::primary_character_portrait_record(self.database.connection(), profile_id)
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

impl OrchestratorContext {
    // —— profiles ——

    pub async fn profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        self.character.profiles_by_project(project_id).await
    }

    pub async fn profile_by_id(
        &self,
        profile_id: i64,
    ) -> Result<Option<protagonist_profile::Model>, AppError> {
        self.character.profile_by_id(profile_id).await
    }

    pub async fn profile_by_project_name(
        &self,
        project_id: i64,
        name: &str,
    ) -> Result<Option<protagonist_profile::Model>, AppError> {
        self.character
            .profile_by_project_name(project_id, name)
            .await
    }

    pub async fn missing_portrait_profiles(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        self.character.missing_portrait_profiles(project_id).await
    }

    /// 创建角色档案：校验姓名后落库。
    pub async fn create_profile_validated(
        &self,
        project_id: i64,
        name: String,
        base_attributes: Value,
    ) -> Result<protagonist_profile::Model, AppError> {
        self.character
            .create_profile_validated(project_id, name, base_attributes)
            .await
    }

    pub async fn update_initial_profile_validated(
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

    pub async fn rollback_failed_portrait_creation_records(
        &self,
        portrait_id: i64,
        image_path: &str,
    ) -> Result<CharacterPortraitRollbackResult, AppError> {
        self.character
            .rollback_failed_portrait_creation_records(portrait_id, image_path)
            .await
    }

    // —— portraits ——

    pub async fn character_portrait_by_id(
        &self,
        portrait_id: i64,
    ) -> Result<Option<character_portrait::Model>, AppError> {
        self.character.character_portrait_by_id(portrait_id).await
    }

    pub async fn create_character_portrait_record(
        &self,
        input: CharacterPortraitCreateInput,
    ) -> Result<character_portrait::Model, AppError> {
        self.character.create_character_portrait_record(input).await
    }

    pub async fn character_portrait_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<character_portrait::Model>, AppError> {
        self.character
            .character_portrait_records_by_project(project_id)
            .await
    }

    pub async fn primary_character_portrait_record(
        &self,
        profile_id: i64,
    ) -> Result<Option<character_portrait::Model>, AppError> {
        self.character
            .primary_character_portrait_record(profile_id)
            .await
    }

    pub async fn create_character_portrait_for_profile(
        &self,
        input: CharacterPortraitCreateInput,
    ) -> Result<character_portrait::Model, AppError> {
        self.character
            .create_character_portrait_for_profile(input)
            .await
    }

    pub async fn set_primary_character_portrait_record(
        &self,
        portrait_id: i64,
    ) -> Result<(), AppError> {
        self.character
            .set_primary_character_portrait_record(portrait_id)
            .await
    }

    pub async fn portrait_prompt_profile_rows(
        &self,
        project_id: i64,
        profile_id: i64,
    ) -> Result<PortraitPromptProfileRows, AppError> {
        self.character
            .portrait_prompt_profile_rows(project_id, profile_id)
            .await
    }

    pub async fn portrait_named_prompt_profile_rows(
        &self,
        project_id: i64,
        character_name: &str,
    ) -> Result<PortraitNamedPromptProfileRows, AppError> {
        self.character
            .portrait_named_prompt_profile_rows(project_id, character_name)
            .await
    }

    pub async fn portrait_reference_images_by_ids(
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

    pub async fn attribute_history(
        &self,
        profile_id: i64,
        filter: CharacterAttributeEventHistoryFilter,
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        self.character.attribute_history(profile_id, filter).await
    }

    /// 规范化属性输入后新增属性并同步 RAG。
    pub async fn add_profile_attribute_validated(
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
    pub async fn modify_profile_attribute_validated(
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
    pub async fn execute_ready_attribute_deletion_validated(
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

    pub async fn behavior_records_by_profile(
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
    pub async fn deletion_marks_by_profile_validated(
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
    pub async fn request_attribute_deletion_validated(
        &self,
        input: CharacterAttributeDeletionRequestInput,
    ) -> Result<protagonist_deletion_mark::Model, AppError> {
        self.character.request_attribute_deletion(input).await
    }

    /// 规范化分类/key 后重置删除标记。
    pub async fn reset_deletion_marks_validated(
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

    pub async fn snapshot_references_for_user_validated(
        &self,
        user_id: i64,
        profile_id: i64,
        start_chapter: Option<i32>,
        end_chapter: Option<i32>,
    ) -> Result<Vec<snapshot_reference::Model>, AppError> {
        validate_snapshot_range(start_chapter, end_chapter)?;
        let profile = self
            .character_profile_for_user_validated(user_id, profile_id)
            .await?;
        self.snapshot_references_by_profile(profile.id, start_chapter, end_chapter)
            .await
    }

    pub async fn profile_snapshot_for_user_at_chapter_validated(
        &self,
        user_id: i64,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<snapshot_reference::Model, AppError> {
        ensure_positive_snapshot_chapter(chapter_number)?;
        self.character_profile_for_user_validated(user_id, profile_id)
            .await?;
        self.snapshot_reference_by_chapter(profile_id, chapter_number)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("第 {chapter_number} 章没有角色快照")))
    }

    pub async fn character_snapshot_for_user_by_hash_validated(
        &self,
        user_id: i64,
        content_hash: &str,
    ) -> Result<Snapshot, AppError> {
        let content_hash =
            required_text(content_hash, "快照哈希").map_err(AppError::InvalidArgument)?;
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
        Ok(snapshot)
    }

    pub async fn diff_profile_snapshots_for_user_validated(
        &self,
        user_id: i64,
        profile_id: i64,
        from_chapter: i32,
        to_chapter: i32,
    ) -> Result<SnapshotDelta, AppError> {
        validate_snapshot_diff_chapters(from_chapter, to_chapter)?;
        self.character_profile_for_user_validated(user_id, profile_id)
            .await?;

        let to_ref = self
            .snapshot_reference_by_chapter(profile_id, to_chapter)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("第 {to_chapter} 章没有角色快照")))?;
        let to_snapshot = self
            .character_snapshot_by_hash(&to_ref.snapshot_hash)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("快照不存在: {}", to_ref.snapshot_hash)))?;
        let (from_hash, from_data) = if from_chapter == 0 {
            ("initial-state".to_string(), initial_snapshot_data())
        } else {
            let from_ref = self
                .snapshot_reference_by_chapter(profile_id, from_chapter)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("第 {from_chapter} 章没有角色快照")))?;
            let from_snapshot = self
                .character_snapshot_by_hash(&from_ref.snapshot_hash)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!("快照不存在: {}", from_ref.snapshot_hash))
                })?;
            (from_ref.snapshot_hash, from_snapshot.data)
        };

        Ok(SnapshotDelta {
            from_hash,
            to_hash: to_ref.snapshot_hash,
            delta: a4n_db::character::snapshot_store::Sha256SnapshotStore::diff_snapshot_data(
                &from_data,
                &to_snapshot.data,
            )?,
        })
    }

    pub async fn character_sync_conflict_for_user_validated(
        &self,
        user_id: i64,
        profile_id: i64,
    ) -> Result<CharacterSyncConflictReadModel, AppError> {
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
        Ok(CharacterSyncConflictReadModel {
            profile_id,
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
    pub async fn delete_character_profile_records_validated(
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
    pub async fn rollback_character_profile_timeline_validated(
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
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
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
        self.upsert_workflow_run(crate::workflow::UpsertWorkflowRunInput {
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
