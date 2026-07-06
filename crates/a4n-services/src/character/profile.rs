use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::ai::generated_image;
use a4n_core::models::character::{
    attribute_event, character_portrait, character_state_index, protagonist_behavior_record,
    protagonist_deletion_mark, protagonist_profile, snapshot_reference,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};

use super::CharacterService;
use super::state::{apply_profile_current_state, normalize_base_attributes};
use crate::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunInput, delete_generation_run_targets_for,
    record_completed_generation_run,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterProfileDeletionSummary {
    pub project_id: i64,
    pub profile_id: i64,
    pub portrait_paths: Vec<String>,
    pub attribute_event_ids: Vec<i64>,
    pub state_index_ids: Vec<i64>,
}

pub fn profile_first_seen_chapter(profile: &protagonist_profile::Model) -> Option<i32> {
    profile.first_seen_chapter()
}

pub(crate) fn is_text_discovered_profile(profile: &protagonist_profile::Model) -> bool {
    profile.is_text_discovered()
}

pub async fn delete_profile_in<C>(
    db: &C,
    profile_id: i64,
) -> Result<CharacterProfileDeletionSummary, AppError>
where
    C: ConnectionTrait,
{
    let profile = protagonist_profile::Entity::find_by_id(profile_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("主角档案不存在".into()))?;

    let state_index_ids = character_state_index::Entity::find()
        .select_only()
        .column(character_state_index::Column::Id)
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let attribute_event_ids = attribute_event::Entity::find()
        .select_only()
        .column(attribute_event::Column::Id)
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let portraits = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let portrait_ids = portraits
        .iter()
        .map(|portrait| portrait.id)
        .collect::<Vec<_>>();
    let portrait_paths = portraits
        .iter()
        .map(|portrait| portrait.image_path.clone())
        .collect::<Vec<_>>();
    if !portrait_paths.is_empty() {
        let image_ids = generated_image::Entity::find()
            .select_only()
            .column(generated_image::Column::Id)
            .filter(generated_image::Column::ProjectId.eq(profile.project_id))
            .filter(generated_image::Column::FilePath.is_in(portrait_paths.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?;
        delete_generation_run_targets_for(db, "generated_image", &image_ids).await?;
        generated_image::Entity::delete_many()
            .filter(generated_image::Column::ProjectId.eq(profile.project_id))
            .filter(generated_image::Column::FilePath.is_in(portrait_paths.clone()))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    delete_generation_run_targets_for(db, "character_portrait", &portrait_ids).await?;

    protagonist_behavior_record::Entity::delete_many()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    protagonist_deletion_mark::Entity::delete_many()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    snapshot_reference::Entity::delete_many()
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    character_state_index::Entity::delete_many()
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    attribute_event::Entity::delete_many()
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    character_portrait::Entity::delete_many()
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;

    delete_generation_run_targets_for(db, "protagonist_profile", &[profile_id]).await?;
    let deleted = protagonist_profile::Entity::delete_by_id(profile_id)
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    if deleted.rows_affected == 0 {
        return Err(AppError::NotFound("主角档案不存在".into()));
    }

    Ok(CharacterProfileDeletionSummary {
        project_id: profile.project_id,
        profile_id,
        portrait_paths,
        attribute_event_ids,
        state_index_ids,
    })
}

impl CharacterService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn create_profile(
        &self,
        project_id: i64,
        name: &str,
        base_attributes: serde_json::Value,
    ) -> Result<protagonist_profile::Model, AppError> {
        create_profile_in(self.db.as_ref(), project_id, name, base_attributes).await
    }

    pub async fn create_profile_with_generation_run(
        &self,
        project_id: i64,
        name: &str,
        base_attributes: serde_json::Value,
        mut generation_run: RecordCompletedGenerationRunInput,
    ) -> Result<protagonist_profile::Model, AppError> {
        if generation_run.project_id != project_id {
            return Err(AppError::InvalidArgument(
                "角色档案生成运行记录 project_id 与项目不一致".to_string(),
            ));
        }
        if !generation_run.targets.is_empty() {
            return Err(AppError::InvalidArgument(
                "角色档案生成运行记录目标由角色服务绑定，调用方不能预填 targets".to_string(),
            ));
        }
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let profile = create_profile_in(&tx, project_id, name, base_attributes).await?;
        generation_run.targets = vec![GenerationRunTraceTarget {
            target_kind: "character_profile_record".to_string(),
            target_table: "protagonist_profile".to_string(),
            target_id: profile.id,
        }];
        record_completed_generation_run(&tx, generation_run).await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(profile)
    }

    pub async fn delete_profile(
        &self,
        profile_id: i64,
    ) -> Result<CharacterProfileDeletionSummary, AppError> {
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let summary = delete_profile_in(&tx, profile_id).await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(summary)
    }

    pub async fn delete_profile_in<C>(
        &self,
        db: &C,
        profile_id: i64,
    ) -> Result<CharacterProfileDeletionSummary, AppError>
    where
        C: ConnectionTrait,
    {
        delete_profile_in(db, profile_id).await
    }
}

async fn create_profile_in<C>(
    db: &C,
    project_id: i64,
    name: &str,
    base_attributes: serde_json::Value,
) -> Result<protagonist_profile::Model, AppError>
where
    C: ConnectionTrait,
{
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidArgument("角色姓名不能为空".to_string()));
    }
    let existing = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(name))
        .one(db)
        .await
        .map_err(AppError::Database)?;
    if existing.is_some() {
        return Err(AppError::InvalidArgument(format!("角色档案已存在: {name}")));
    }

    let now = Utc::now().naive_utc();
    let normalized_base = normalize_base_attributes(base_attributes)?;
    let mut active = protagonist_profile::ActiveModel {
        project_id: Set(project_id),
        name: Set(name.to_string()),
        gender: Set(None),
        age: Set(None),
        base_attributes: Set(normalized_base.clone()),
        current_state: Set(normalized_base.clone()),
        character_arc: Set(None),
        voice_fingerprint: Set(None),
        appearance: Set(None),
        background: Set(None),
        personality: Set(None),
        last_synced_chapter: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    apply_profile_current_state(&mut active, normalized_base);
    active.insert(db).await.map_err(AppError::Database)
}
