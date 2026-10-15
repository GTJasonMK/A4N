use a4n_db_models::models::character::protagonist_profile;
use a4n_domain::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use serde_json::Value;

use super::{
    state::{normalize_base_attributes, profile_current_state_fields},
    types::CharacterProfileCreateError,
};
use a4n_domain::character::CharacterProfileCreateInput;
use a4n_domain::character::CharacterProfileRecord;

async fn create_profile_record<C>(
    db: &C,
    input: CharacterProfileCreateInput,
) -> Result<protagonist_profile::Model, CharacterProfileCreateError>
where
    C: ConnectionTrait,
{
    let existing = protagonist_profile::Entity::find()
        .select_only()
        .column(protagonist_profile::Column::Id)
        .filter(protagonist_profile::Column::ProjectId.eq(input.project_id))
        .filter(protagonist_profile::Column::Name.eq(&input.name))
        .into_tuple::<i64>()
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

fn profile_create_input(
    project_id: i64,
    name: &str,
    base_attributes: Value,
) -> Result<CharacterProfileCreateInput, CharacterProfileCreateError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CharacterProfileCreateError::InvalidArgument(
            "角色姓名不能为空".to_string(),
        ));
    }
    let normalized_base = normalize_base_attributes(base_attributes)
        .map_err(|err| CharacterProfileCreateError::InvalidArgument(err.to_string()))?;
    let fields = profile_current_state_fields(normalized_base.clone());

    Ok(CharacterProfileCreateInput {
        project_id,
        name: name.to_string(),
        base_attributes: normalized_base,
        current_state: fields.current_state,
        gender: fields.gender,
        age: fields.age,
        appearance: fields.appearance,
        background: fields.background,
        personality: fields.personality,
    })
}

async fn require_profile_project_active<C>(
    db: &C,
    project_id: i64,
) -> Result<(), CharacterProfileCreateError>
where
    C: ConnectionTrait,
{
    let project = crate::project::project_write_identity_for_update(db, project_id)
        .await?
        .ok_or_else(|| {
            CharacterProfileCreateError::InvalidArgument(format!("项目不存在: {project_id}"))
        })?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(CharacterProfileCreateError::InvalidState {
            current: project.lifecycle_gate,
            operation: "创建角色档案".to_string(),
        });
    }
    Ok(())
}

pub async fn create_profile(
    db: &impl crate::DbExecutor,
    project_id: i64,
    name: &str,
    base_attributes: Value,
) -> Result<protagonist_profile::Model, CharacterProfileCreateError> {
    let input = profile_create_input(project_id, name, base_attributes)?;
    let tx = db.begin().await?;
    require_profile_project_active(&tx, project_id).await?;
    let profile = create_profile_record(&tx, input).await?;
    tx.commit().await?;
    Ok(profile)
}

pub async fn create_profile_domain_record(
    db: &impl crate::DbExecutor,
    project_id: i64,
    name: &str,
    base_attributes: Value,
) -> Result<CharacterProfileRecord, CharacterProfileCreateError> {
    create_profile(db, project_id, name, base_attributes)
        .await
        .map(crate::character::character_profile_record)
}

pub async fn create_profile_with_generation_run<C>(
    db: &C,
    project_id: i64,
    name: &str,
    base_attributes: Value,
    generation_run: RecordCompletedGenerationRunInput,
) -> Result<protagonist_profile::Model, CharacterProfileCreateError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db
        .begin()
        .await
        .map_err(CharacterProfileCreateError::Database)?;
    let profile = create_profile_with_generation_run_in(
        &tx,
        project_id,
        name,
        base_attributes,
        generation_run,
    )
    .await?;
    tx.commit()
        .await
        .map_err(CharacterProfileCreateError::Database)?;
    Ok(profile)
}

pub async fn create_profile_with_generation_run_in<C>(
    db: &C,
    project_id: i64,
    name: &str,
    base_attributes: Value,
    mut generation_run: RecordCompletedGenerationRunInput,
) -> Result<protagonist_profile::Model, CharacterProfileCreateError>
where
    C: ConnectionTrait,
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

    require_profile_project_active(db, project_id).await?;
    let profile =
        create_profile_record(db, profile_create_input(project_id, name, base_attributes)?).await?;
    generation_run.targets = vec![GenerationRunTraceTarget {
        target_kind: "character_profile_record".to_string(),
        target_table: "protagonist_profile".to_string(),
        target_id: profile.id,
    }];
    crate::generation::record_completed_generation_run_with_checkpoint_in(
        db,
        generation_run,
        crate::generation::GenerationRunCheckpointContext::default(),
    )
    .await?;
    Ok(profile)
}
