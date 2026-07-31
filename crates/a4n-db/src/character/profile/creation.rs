use a4n_core::models::character::protagonist_profile;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
};
use serde_json::Value;

use super::{
    state::{normalize_base_attributes, profile_current_state_fields},
    types::{CharacterProfileCreateError, CharacterProfileCreateInput},
};

pub async fn create_profile_record<C>(
    db: &C,
    input: CharacterProfileCreateInput,
) -> Result<protagonist_profile::Model, CharacterProfileCreateError>
where
    C: ConnectionTrait,
{
    let existing = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(input.project_id))
        .filter(protagonist_profile::Column::Name.eq(&input.name))
        .one(db)
        .await?;
    if existing.is_some() {
        return Err(CharacterProfileCreateError::AlreadyExists { name: input.name });
    }

    let now = chrono::Utc::now().naive_utc();
    protagonist_profile::ActiveModel {
        project_id: Set(input.project_id),
        name: Set(input.name),
        gender: Set(input.gender),
        age: Set(input.age),
        base_attributes: Set(input.base_attributes),
        current_state: Set(input.current_state),
        character_arc: Set(None),
        voice_fingerprint: Set(None),
        appearance: Set(input.appearance),
        background: Set(input.background),
        personality: Set(input.personality),
        last_synced_chapter: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(CharacterProfileCreateError::Database)
}

pub async fn create_profile<C>(
    db: &C,
    project_id: i64,
    name: &str,
    base_attributes: Value,
) -> Result<protagonist_profile::Model, CharacterProfileCreateError>
where
    C: ConnectionTrait,
{
    let name = name.trim();
    if name.is_empty() {
        return Err(CharacterProfileCreateError::InvalidArgument(
            "角色姓名不能为空".to_string(),
        ));
    }
    let normalized_base = normalize_base_attributes(base_attributes)
        .map_err(|err| CharacterProfileCreateError::InvalidArgument(err.to_string()))?;
    let fields = profile_current_state_fields(normalized_base.clone());

    create_profile_record(
        db,
        CharacterProfileCreateInput {
            project_id,
            name: name.to_string(),
            base_attributes: normalized_base,
            current_state: fields.current_state,
            gender: fields.gender,
            age: fields.age,
            appearance: fields.appearance,
            background: fields.background,
            personality: fields.personality,
        },
    )
    .await
}

pub async fn create_profile_with_generation_run<C>(
    db: &C,
    project_id: i64,
    name: &str,
    base_attributes: Value,
    mut generation_run: crate::generation::RecordCompletedGenerationRunInput,
) -> Result<protagonist_profile::Model, CharacterProfileCreateError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if generation_run.project_id != project_id {
        return Err(CharacterProfileCreateError::InvalidArgument(
            "角色档案生成运行记录 project_id 与项目不一致".to_string(),
        ));
    }
    if !generation_run.targets.is_empty() {
        return Err(CharacterProfileCreateError::InvalidArgument(
            "角色档案生成运行记录目标由角色服务绑定，调用方不能预填 targets".to_string(),
        ));
    }

    let tx = db
        .begin()
        .await
        .map_err(CharacterProfileCreateError::Database)?;
    let profile = create_profile(&tx, project_id, name, base_attributes).await?;
    generation_run.targets = vec![crate::generation::GenerationRunTraceTarget {
        target_kind: "character_profile_record".to_string(),
        target_table: "protagonist_profile".to_string(),
        target_id: profile.id,
    }];
    crate::generation::record_completed_generation_run_with_checkpoint_in(
        &tx,
        generation_run,
        crate::generation::GenerationRunCheckpointContext::default(),
    )
    .await?;
    tx.commit()
        .await
        .map_err(CharacterProfileCreateError::Database)?;
    Ok(profile)
}
