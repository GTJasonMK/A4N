use a4n_api_contract::{
    collaboration::ProjectAccessRole,
    profile::{
        CreateSceneProfileInput, DeleteSceneProfileInput, SceneProfileDeleteOutcome,
        SceneProfileDeleteResult, SceneProfileInput, SceneProfileMutationOutcome,
        SceneProfileMutationResult, SceneProfileSummary, UpdateSceneProfileInput,
    },
};
use a4n_db::{DbHandle, scene_profile::SceneSyncRecordResult};
use a4n_db_models::models::scene::scene_profile;
use a4n_domain::{
    profile_material::ProfileMaterialKind, scene_profile::SceneProfileEvidenceRecordInput,
};
use a4n_error::AppError;

use super::{OrchestratorContext, profile_mutation::resolve_downstream_image_ids};
use crate::scene_profile::{
    SceneSyncInput, normalize_scene_profile_record_input, normalize_scene_sync_record_input,
    scene_profile_db_error,
};

#[derive(Clone)]
pub(super) struct SceneProfileApplication {
    database: DbHandle,
}

impl SceneProfileApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn scene_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db_models::models::scene::scene_profile::Model>, AppError> {
        a4n_db::scene_profile::scene_profiles_by_project(self.database.connection(), project_id)
            .await
            .map_err(scene_profile_db_error)
    }

    async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        a4n_db::scene_profile::scene_profile_by_id(self.database.connection(), scene_profile_id)
            .await
            .map_err(scene_profile_db_error)
    }

    async fn delete_scene_profile_records(&self, scene_profile_id: i64) -> Result<(), AppError> {
        a4n_db::scene_profile::delete_scene_profile_records(
            self.database.connection(),
            scene_profile_id,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    /// 规范化 API 输入后创建场景档案。
    async fn create_scene_profile_validated(
        &self,
        project_id: i64,
        input: SceneProfileInput,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        let record = normalize_scene_profile_record_input(input)?;
        a4n_db::scene_profile::create_scene_profile_record_checked(
            self.database.connection(),
            project_id,
            record,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    /// 规范化 API 输入后更新场景档案。
    async fn update_scene_profile_validated(
        &self,
        scene_profile_id: i64,
        input: SceneProfileInput,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        let record = normalize_scene_profile_record_input(input)?;
        a4n_db::scene_profile::update_scene_profile_record_checked(
            self.database.connection(),
            scene_profile_id,
            record,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    /// 规范化同步草稿后与 generation_run 一并替换章节场景同步结果。
    async fn replace_chapter_scene_sync_from_input_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        source: a4n_db::project::ChapterSourceSnapshot,
        input: SceneSyncInput,
        profile_evidence: Vec<SceneProfileEvidenceRecordInput>,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
    ) -> Result<SceneSyncRecordResult, AppError> {
        let sync_input = normalize_scene_sync_record_input(input)?;
        a4n_db::scene_profile::replace_chapter_scene_sync_with_generation_run_records(
            self.database.connection(),
            project_id,
            chapter_id,
            chapter_number,
            source,
            sync_input,
            profile_evidence,
            generation_run,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    async fn scene_occurrence_count(
        &self,
        project_id: i64,
        scene_profile_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::scene_profile::scene_occurrence_count(
            self.database.connection(),
            project_id,
            scene_profile_id,
        )
        .await
        .map_err(scene_profile_db_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn scene_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db_models::models::scene::scene_profile::Model>, AppError> {
        self.scene_profile
            .scene_profiles_by_project(project_id)
            .await
    }

    pub(crate) async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        self.scene_profile
            .scene_profile_by_id(scene_profile_id)
            .await
    }

    /// 创建场景档案：规范化 API 输入后落库。
    pub(crate) async fn create_scene_profile_validated(
        &self,
        project_id: i64,
        input: SceneProfileInput,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        self.scene_profile
            .create_scene_profile_validated(project_id, input)
            .await
    }

    /// 更新场景档案：规范化 API 输入后落库。
    pub(crate) async fn update_scene_profile_validated(
        &self,
        scene_profile_id: i64,
        input: SceneProfileInput,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        self.scene_profile
            .update_scene_profile_validated(scene_profile_id, input)
            .await
    }

    /// 删除场景档案记录（下游图片清理仍由 Host 执行）。
    pub(crate) async fn delete_scene_profile_records_validated(
        &self,
        scene_profile_id: i64,
    ) -> Result<(), AppError> {
        if scene_profile_id <= 0 {
            return Err(AppError::InvalidArgument(
                "场景档案 ID 必须大于 0".to_string(),
            ));
        }
        self.scene_profile
            .delete_scene_profile_records(scene_profile_id)
            .await
    }

    /// 场景同步落库：规范化业务输入后与 generation_run 一并写入。
    pub(crate) async fn replace_chapter_scene_sync_from_input_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        source: a4n_db::project::ChapterSourceSnapshot,
        input: SceneSyncInput,
        profile_evidence: Vec<SceneProfileEvidenceRecordInput>,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
    ) -> Result<SceneSyncRecordResult, AppError> {
        self.scene_profile
            .replace_chapter_scene_sync_from_input_validated(
                project_id,
                chapter_id,
                chapter_number,
                source,
                input,
                profile_evidence,
                generation_run,
            )
            .await
    }

    pub(crate) async fn scene_occurrence_count(
        &self,
        project_id: i64,
        scene_profile_id: i64,
    ) -> Result<u64, AppError> {
        self.scene_profile
            .scene_occurrence_count(project_id, scene_profile_id)
            .await
    }
}

fn scene_profile_summary(
    profile: &scene_profile::Model,
    occurrence_count: u64,
    generated_image_count: i64,
) -> SceneProfileSummary {
    SceneProfileSummary {
        id: profile.id,
        project_id: profile.project_id,
        ownership_kind: profile.ownership_kind.clone(),
        name: profile.name.clone(),
        normalized_name: profile.normalized_name.clone(),
        aliases_json: profile.aliases_json.clone(),
        scene_type: profile.scene_type.clone(),
        importance: profile.importance.clone(),
        parent_scene_id: profile.parent_scene_id,
        base_description: profile.base_description.clone(),
        visual_description: profile.visual_description.clone(),
        spatial_layout: profile.spatial_layout.clone(),
        atmosphere: profile.atmosphere.clone(),
        entry_points_json: profile.entry_points_json.clone(),
        rules_json: profile.rules_json.clone(),
        current_state_json: profile.current_state_json.clone(),
        first_seen_chapter: profile.first_seen_chapter,
        last_seen_chapter: profile.last_seen_chapter,
        evidence_json: profile.evidence_json.clone(),
        image_prompt_seed: profile.image_prompt_seed.clone(),
        occurrence_count,
        generated_image_count,
        created_at: Some(profile.created_at.to_string()),
        updated_at: Some(profile.updated_at.to_string()),
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::profile::SceneProfileMutationApplicationPort for OrchestratorContext {
    async fn create(
        &self,
        user_id: i64,
        input: CreateSceneProfileInput,
    ) -> Result<SceneProfileMutationOutcome, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let project_id = input.project_id;
        let profile = self
            .create_scene_profile_validated(project_id, input.profile)
            .await?;
        Ok(SceneProfileMutationOutcome {
            result: SceneProfileMutationResult {
                profile: scene_profile_summary(&profile, 0, 0),
                warnings: Vec::new(),
            },
            downstream_image_ids: Vec::new(),
        })
    }

    async fn update(
        &self,
        user_id: i64,
        input: UpdateSceneProfileInput,
    ) -> Result<SceneProfileMutationOutcome, AppError> {
        let existing = self.scene_profile_by_id(input.scene_profile_id).await?;
        self.verify_project_permission(existing.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let image_ids = self
            .profile_material_image_ids(
                existing.project_id,
                ProfileMaterialKind::Scene,
                existing.id,
            )
            .await?;
        let image_count = i64::try_from(image_ids.len()).map_err(|_| AppError::InvalidState {
            current: existing.id.to_string(),
            operation: "场景档案素材数量超过 i64 上限".to_string(),
        })?;
        let occurrence_count = self
            .scene_occurrence_count(existing.project_id, existing.id)
            .await?;
        let downstream_image_ids = resolve_downstream_image_ids(
            image_ids,
            input.downstream_strategy,
            &format!("场景档案 #{} ", existing.id),
            "修改",
        )?;
        let profile = self
            .update_scene_profile_validated(input.scene_profile_id, input.profile)
            .await?;
        Ok(SceneProfileMutationOutcome {
            result: SceneProfileMutationResult {
                profile: scene_profile_summary(&profile, occurrence_count, image_count),
                warnings: Vec::new(),
            },
            downstream_image_ids,
        })
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteSceneProfileInput,
    ) -> Result<SceneProfileDeleteOutcome, AppError> {
        let existing = self.scene_profile_by_id(input.scene_profile_id).await?;
        self.verify_project_permission(existing.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let downstream_image_ids = resolve_downstream_image_ids(
            self.profile_material_image_ids(
                existing.project_id,
                ProfileMaterialKind::Scene,
                existing.id,
            )
            .await?,
            input.downstream_strategy,
            &format!("场景档案 #{} ", existing.id),
            "删除",
        )?;
        self.delete_scene_profile_records_validated(input.scene_profile_id)
            .await?;
        Ok(SceneProfileDeleteOutcome {
            result: SceneProfileDeleteResult {
                warnings: Vec::new(),
            },
            project_id: existing.project_id,
            downstream_image_ids,
        })
    }
}
