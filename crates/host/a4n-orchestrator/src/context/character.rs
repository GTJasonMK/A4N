use a4n_api_contract::character::{
    CheckImplicitUpdateInput, DeletionMarkSummary, GetImplicitReviewInput, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};
use a4n_db::DbHandle;
use a4n_domain::character::{
    CharacterAttributeCategory, CharacterAttributeEventRecord, CharacterBehaviorRecord,
    CharacterDeletionMarkRecord, CharacterImplicitReviewDecisionRecordInput,
    CharacterPortraitCreateInput, CharacterPortraitRecord, CharacterProfileRecord,
    CharacterTimelineSourceKind,
};
use a4n_domain::media::GeneratedImageRecord;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use super::OrchestratorContext;
use crate::character::{
    AppliedCharacterImplicitDecisionRecord, AppliedCharacterSyncPersistenceRecord,
    CharacterAttributeLifecycleError, CharacterConflictContextRows, CharacterSyncPersistenceInput,
    PortraitNamedPromptProfileRows, PortraitPromptProfileRows, character_db_error,
    character_portrait_record_error,
};
use a4n_domain::collaboration::ProjectAccessRole;

#[derive(Clone)]
pub(super) struct CharacterApplication {
    database: DbHandle,
}

impl CharacterApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    // —— profiles ——

    async fn profile_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileRecord>, AppError> {
        a4n_db::character::profile_records_by_project(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn profile_record_by_project_name(
        &self,
        project_id: i64,
        name: &str,
    ) -> Result<Option<CharacterProfileRecord>, AppError> {
        a4n_db::character::profile_record_by_project_name(
            self.database.connection(),
            project_id,
            name,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn profile_record_by_id(
        &self,
        profile_id: i64,
    ) -> Result<Option<CharacterProfileRecord>, AppError> {
        a4n_db::character::profile_record_by_id(self.database.connection(), profile_id)
            .await
            .map_err(AppError::Database)
    }

    async fn missing_portrait_profiles(
        &self,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileRecord>, AppError> {
        a4n_db::character::missing_portrait_profile_records(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    // —— portraits ——

    async fn character_portrait_by_id(
        &self,
        portrait_id: i64,
    ) -> Result<Option<CharacterPortraitRecord>, AppError> {
        a4n_db::character::character_portrait_record_by_id(self.database.connection(), portrait_id)
            .await
            .map_err(AppError::Database)
    }

    async fn character_portrait_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<CharacterPortraitRecord>, AppError> {
        a4n_db::character::character_portrait_records_by_project_records(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn create_character_portrait_for_profile(
        &self,
        input: CharacterPortraitCreateInput,
    ) -> Result<CharacterPortraitRecord, AppError> {
        a4n_db::character::create_character_portrait_record_for_profile(
            self.database.connection(),
            input,
        )
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
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        a4n_db::video::read_models::portrait_reference_images_by_ids(
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
    ) -> Result<Vec<CharacterAttributeEventRecord>, AppError> {
        a4n_db::character::attribute_events_by_ids(self.database.connection(), attribute_event_ids)
            .await
            .map_err(AppError::Database)
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

    async fn implicit_review_behavior_records(
        &self,
        profile_id: i64,
    ) -> Result<Vec<CharacterBehaviorRecord>, AppError> {
        a4n_db::character::implicit_review_behavior_records(self.database.connection(), profile_id)
            .await
            .map_err(AppError::Database)
    }

    async fn behavior_records_before_chapter(
        &self,
        profile_id: i64,
        start_chapter: i32,
    ) -> Result<Vec<CharacterBehaviorRecord>, AppError> {
        a4n_db::character::behavior_records_before_chapter(
            self.database.connection(),
            profile_id,
            start_chapter,
        )
        .await
        .map_err(AppError::Database)
    }

    // —— snapshots / timeline ——

    async fn character_profile_state_through_chapter(
        &self,
        profile: &CharacterProfileRecord,
        chapter_number: i32,
    ) -> Result<Value, AppError> {
        a4n_db::character::replay_profile_state_through_chapter_by_id(
            self.database.connection(),
            profile.id,
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
}

fn character_deletion_mark_summary(
    mark: &CharacterDeletionMarkRecord,
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

impl OrchestratorContext {
    // —— profiles ——

    pub(crate) async fn profile_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileRecord>, AppError> {
        self.character.profile_records_by_project(project_id).await
    }

    pub(crate) async fn profile_record_by_project_name(
        &self,
        project_id: i64,
        name: &str,
    ) -> Result<Option<CharacterProfileRecord>, AppError> {
        self.character
            .profile_record_by_project_name(project_id, name)
            .await
    }

    pub(crate) async fn profile_record_by_id(
        &self,
        profile_id: i64,
    ) -> Result<Option<CharacterProfileRecord>, AppError> {
        self.character.profile_record_by_id(profile_id).await
    }

    pub(crate) async fn missing_portrait_profiles(
        &self,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileRecord>, AppError> {
        self.character.missing_portrait_profiles(project_id).await
    }

    // —— portraits ——

    pub(crate) async fn character_portrait_by_id(
        &self,
        portrait_id: i64,
    ) -> Result<Option<CharacterPortraitRecord>, AppError> {
        self.character.character_portrait_by_id(portrait_id).await
    }

    pub(crate) async fn character_portrait_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<CharacterPortraitRecord>, AppError> {
        self.character
            .character_portrait_records_by_project(project_id)
            .await
    }

    pub(crate) async fn create_character_portrait_for_profile(
        &self,
        input: CharacterPortraitCreateInput,
    ) -> Result<CharacterPortraitRecord, AppError> {
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
    ) -> Result<Vec<GeneratedImageRecord>, AppError> {
        self.character
            .portrait_reference_images_by_ids(project_id, image_ids)
            .await
    }

    // —— attributes ——

    pub(crate) async fn attribute_events_by_ids(
        &self,
        attribute_event_ids: &[i64],
    ) -> Result<Vec<CharacterAttributeEventRecord>, AppError> {
        self.character
            .attribute_events_by_ids(attribute_event_ids)
            .await
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
        profile: &CharacterProfileRecord,
        event: &CharacterAttributeEventRecord,
    ) -> Result<(), CharacterAttributeLifecycleError> {
        self.sync_character_attribute_event_rag(profile.project_id, event, &profile.name)
            .await
            .map_err(CharacterAttributeLifecycleError::rag)
    }

    // —— behavior ——

    pub(crate) async fn implicit_review_behavior_records(
        &self,
        profile_id: i64,
    ) -> Result<Vec<CharacterBehaviorRecord>, AppError> {
        self.character
            .implicit_review_behavior_records(profile_id)
            .await
    }

    /// 解析列表过滤后读取删除标记。
    pub(crate) async fn behavior_records_before_chapter(
        &self,
        profile_id: i64,
        start_chapter: i32,
    ) -> Result<Vec<CharacterBehaviorRecord>, AppError> {
        self.character
            .behavior_records_before_chapter(profile_id, start_chapter)
            .await
    }

    // —— snapshots / timeline ——

    pub(crate) async fn character_profile_state_through_chapter(
        &self,
        profile: &CharacterProfileRecord,
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
    ) -> Result<CharacterProfileRecord, AppError> {
        let profile = self
            .profile_record_by_id(profile_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("角色档案不存在: {profile_id}")))?;
        self.verify_project_permission(profile.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(profile)
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
    ) -> Result<a4n_domain::workflow::WorkflowRunSummary, AppError> {
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
