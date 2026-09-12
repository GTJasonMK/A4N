use a4n_api_contract::{
    collaboration::ProjectAccessRole,
    profile::{
        CreateObjectProfileInput, DeleteObjectProfileInput, ObjectProfileDeleteOutcome,
        ObjectProfileDeleteResult, ObjectProfileInput, ObjectProfileMutationOutcome,
        ObjectProfileMutationResult, ObjectProfileSummary, UpdateObjectProfileInput,
    },
};
use a4n_db::{DbHandle, asset::ObjectProfileUpsertRecordResult};
use a4n_db_models::models::asset::object_profile;
use a4n_domain::{
    object_profile::ObjectProfileEvidenceRecordInput, profile_material::ProfileMaterialKind,
};
use a4n_error::AppError;

use super::{OrchestratorContext, profile_mutation::resolve_downstream_image_ids};

use crate::asset::{
    asset_error,
    object_profiles::{
        normalize_object_profile_record_input, normalize_object_profile_record_inputs,
    },
};

#[derive(Clone)]
pub(super) struct AssetApplication {
    database: DbHandle,
}

impl AssetApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn object_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        a4n_db::asset::object_profiles_by_project(self.database.connection(), project_id)
            .await
            .map_err(asset_error)
    }

    async fn object_profile_by_id(
        &self,
        object_profile_id: i64,
    ) -> Result<object_profile::Model, AppError> {
        a4n_db::asset::object_profile_by_id(self.database.connection(), object_profile_id)
            .await
            .map_err(asset_error)
    }

    async fn delete_object_profile_record(&self, object_profile_id: i64) -> Result<(), AppError> {
        a4n_db::asset::delete_object_profile_record(self.database.connection(), object_profile_id)
            .await
            .map_err(asset_error)
    }

    /// 规范化草稿后创建物体档案。
    async fn create_object_profile_validated(
        &self,
        project_id: i64,
        draft: ObjectProfileInput,
    ) -> Result<object_profile::Model, AppError> {
        let record = normalize_object_profile_record_input(draft)?;
        a4n_db::asset::create_object_profile_record(self.database.connection(), project_id, record)
            .await
            .map_err(asset_error)
    }

    /// 规范化草稿后更新物体档案。
    async fn update_object_profile_validated(
        &self,
        object_profile_id: i64,
        draft: ObjectProfileInput,
    ) -> Result<object_profile::Model, AppError> {
        let record = normalize_object_profile_record_input(draft)?;
        a4n_db::asset::update_object_profile_record(
            self.database.connection(),
            object_profile_id,
            record,
        )
        .await
        .map_err(asset_error)
    }

    /// 批量规范化草稿后与 generation_run 一并 upsert。
    async fn upsert_object_profiles_from_drafts_validated(
        &self,
        project_id: i64,
        drafts: Vec<ObjectProfileInput>,
        source_chapters: Vec<a4n_db::project::ChapterSourceSnapshot>,
        profile_evidence: Vec<ObjectProfileEvidenceRecordInput>,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
    ) -> Result<ObjectProfileUpsertRecordResult, AppError> {
        let inputs = normalize_object_profile_record_inputs(drafts)?;
        a4n_db::asset::upsert_object_profiles_with_generation_run_record(
            self.database.connection(),
            project_id,
            inputs,
            source_chapters,
            profile_evidence,
            generation_run,
        )
        .await
        .map_err(asset_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn list_object_profiles_for_user(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ObjectProfileSummary>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        let profiles = self.object_profiles_by_project(project_id).await?;
        let profile_ids = profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<Vec<_>>();
        let image_counts = self
            .count_profile_material_images_by_profile(
                project_id,
                ProfileMaterialKind::Object,
                &profile_ids,
            )
            .await?;
        Ok(profiles
            .iter()
            .map(|profile| {
                object_profile_summary(
                    profile,
                    image_counts.get(&profile.id).copied().unwrap_or_default(),
                )
            })
            .collect())
    }

    pub(crate) async fn object_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        self.asset.object_profiles_by_project(project_id).await
    }

    pub(crate) async fn object_profile_by_id(
        &self,
        object_profile_id: i64,
    ) -> Result<object_profile::Model, AppError> {
        self.asset.object_profile_by_id(object_profile_id).await
    }

    /// 创建物体档案：规范化草稿后落库。
    pub(crate) async fn create_object_profile_validated(
        &self,
        project_id: i64,
        draft: ObjectProfileInput,
    ) -> Result<object_profile::Model, AppError> {
        self.asset
            .create_object_profile_validated(project_id, draft)
            .await
    }

    /// 更新物体档案：规范化草稿后落库。
    pub(crate) async fn update_object_profile_validated(
        &self,
        object_profile_id: i64,
        draft: ObjectProfileInput,
    ) -> Result<object_profile::Model, AppError> {
        self.asset
            .update_object_profile_validated(object_profile_id, draft)
            .await
    }

    /// 发现路径：批量规范化草稿后与 generation_run 一并 upsert。
    pub(crate) async fn upsert_object_profiles_from_drafts_validated(
        &self,
        project_id: i64,
        drafts: Vec<ObjectProfileInput>,
        source_chapters: Vec<a4n_db::project::ChapterSourceSnapshot>,
        profile_evidence: Vec<ObjectProfileEvidenceRecordInput>,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
    ) -> Result<ObjectProfileUpsertRecordResult, AppError> {
        self.asset
            .upsert_object_profiles_from_drafts_validated(
                project_id,
                drafts,
                source_chapters,
                profile_evidence,
                generation_run,
            )
            .await
    }

    /// 删除物体档案记录（下游图片清理仍由 Host 执行）。
    pub(crate) async fn delete_object_profile_record_validated(
        &self,
        object_profile_id: i64,
    ) -> Result<(), AppError> {
        if object_profile_id <= 0 {
            return Err(AppError::InvalidArgument(
                "物体档案 ID 必须大于 0".to_string(),
            ));
        }
        self.asset
            .delete_object_profile_record(object_profile_id)
            .await
    }
}

fn object_profile_summary(
    profile: &object_profile::Model,
    generated_image_count: i64,
) -> ObjectProfileSummary {
    ObjectProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        ownership_kind: profile.ownership_kind.clone(),
        name: profile.name.clone(),
        normalized_name: profile.normalized_name.clone(),
        aliases_json: profile.aliases_json.clone(),
        object_type: profile.object_type.clone(),
        importance: profile.importance.clone(),
        visual_description: profile.visual_description.clone(),
        material_and_state: profile.material_and_state.clone(),
        story_usage: profile.story_usage.clone(),
        visual_usage: profile.visual_usage.clone(),
        generation_usage: profile.generation_usage.clone(),
        location_context: profile.location_context.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        evidence_json: profile.evidence_json.clone(),
        image_prompt_seed: profile.image_prompt_seed.clone(),
        generated_image_count,
        created_at: Some(profile.created_at.to_string()),
        updated_at: Some(profile.updated_at.to_string()),
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::profile::ObjectProfileReadApplicationPort for OrchestratorContext {
    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ObjectProfileSummary>, AppError> {
        self.list_object_profiles_for_user(user_id, project_id)
            .await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::profile::ObjectProfileMutationApplicationPort for OrchestratorContext {
    async fn create(
        &self,
        user_id: i64,
        input: CreateObjectProfileInput,
    ) -> Result<ObjectProfileMutationOutcome, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let profile = self
            .create_object_profile_validated(input.project_id, input.profile)
            .await?;
        Ok(ObjectProfileMutationOutcome {
            result: ObjectProfileMutationResult {
                profile: object_profile_summary(&profile, 0),
                warnings: Vec::new(),
            },
            downstream_image_ids: Vec::new(),
        })
    }

    async fn update(
        &self,
        user_id: i64,
        input: UpdateObjectProfileInput,
    ) -> Result<ObjectProfileMutationOutcome, AppError> {
        let existing = self.object_profile_by_id(input.object_profile_id).await?;
        self.verify_project_permission(existing.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let image_ids = self
            .profile_material_image_ids(
                existing.project_id,
                ProfileMaterialKind::Object,
                existing.id,
            )
            .await?;
        let image_count = i64::try_from(image_ids.len()).map_err(|_| AppError::InvalidState {
            current: existing.id.to_string(),
            operation: "物体档案素材数量超过 i64 上限".to_string(),
        })?;
        let downstream_image_ids = resolve_downstream_image_ids(
            image_ids,
            input.downstream_strategy,
            &format!("物体档案 #{} ", existing.id),
            "修改",
        )?;
        let profile = self
            .update_object_profile_validated(input.object_profile_id, input.profile)
            .await?;
        Ok(ObjectProfileMutationOutcome {
            result: ObjectProfileMutationResult {
                profile: object_profile_summary(&profile, image_count),
                warnings: Vec::new(),
            },
            downstream_image_ids,
        })
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteObjectProfileInput,
    ) -> Result<ObjectProfileDeleteOutcome, AppError> {
        let existing = self.object_profile_by_id(input.object_profile_id).await?;
        self.verify_project_permission(existing.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let downstream_image_ids = resolve_downstream_image_ids(
            self.profile_material_image_ids(
                existing.project_id,
                ProfileMaterialKind::Object,
                existing.id,
            )
            .await?,
            input.downstream_strategy,
            &format!("物体档案 #{} ", existing.id),
            "删除",
        )?;
        self.delete_object_profile_record_validated(input.object_profile_id)
            .await?;
        Ok(ObjectProfileDeleteOutcome {
            result: ObjectProfileDeleteResult {
                warnings: Vec::new(),
            },
            downstream_image_ids,
        })
    }
}
