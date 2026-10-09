use std::collections::HashSet;

use a4n_db_models::models::{ai::system_settings, project::novel_project};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set,
    sea_query::OnConflict,
};
use serde_json::Value;

use super::identity::{
    normalize_project_override_identity, project_override_key, validate_project_override_content,
};
use super::types::database;
use super::{ProjectOverrideIdentity, PromptOverrideDbError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MissingProjectMessage {
    Command,
}

pub async fn load_project_override_persisted_content(
    db: &impl crate::DbExecutor,
    identity: &ProjectOverrideIdentity,
) -> Result<Option<String>, PromptOverrideDbError> {
    ensure_project_exists(db, identity.project_id, MissingProjectMessage::Command).await?;
    let key = project_override_key(identity);
    let record = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(&key))
        .one(db)
        .await
        .map_err(|source| database("读取项目提示词覆盖失败", source))?;
    let Some(record) = record else {
        return Ok(None);
    };
    let content = project_override_record_content(&record)?;
    Ok(Some(content))
}

pub async fn save_project_override(
    db: &impl crate::DbExecutor,
    project_id: i64,
    name: &str,
    content: &str,
) -> Result<ProjectOverrideIdentity, PromptOverrideDbError> {
    let identity = normalize_project_override_identity(project_id, name)?;
    validate_project_override_content(content)?;
    ensure_project_exists(db, identity.project_id, MissingProjectMessage::Command).await?;

    let key = project_override_key(&identity);
    let now = Utc::now().naive_utc();
    let value = Value::String(content.to_string());
    let existing = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(&key))
        .one(db)
        .await
        .map_err(|source| database("读取待保存项目提示词覆盖失败", source))?;

    if let Some(record) = existing {
        let mut active: system_settings::ActiveModel = record.into();
        active.value = Set(value);
        active.updated_at = Set(now);
        active
            .update(db)
            .await
            .map_err(|source| database("更新项目提示词覆盖失败", source))?;
    } else {
        system_settings::ActiveModel {
            key: Set(key),
            value: Set(value),
            category: Set(system_settings::PROMPT_OVERRIDE_CATEGORY.to_string()),
            description: Set(Some(format!(
                "项目 {} 提示词覆盖：{}",
                identity.project_id, identity.name
            ))),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|source| database("插入项目提示词覆盖失败", source))?;
    }

    Ok(identity)
}

pub async fn delete_project_override(
    db: &impl crate::DbExecutor,
    identity: &ProjectOverrideIdentity,
) -> Result<(), PromptOverrideDbError> {
    let key = project_override_key(identity);
    system_settings::Entity::delete_many()
        .filter(system_settings::Column::Key.eq(key))
        .exec(db)
        .await
        .map_err(|source| database("删除项目提示词覆盖失败", source))?;
    Ok(())
}

pub async fn load_prompt_assembly_override(
    db: &impl crate::DbExecutor,
    project_id: Option<i64>,
    call_key: &str,
    prompt_name: &str,
) -> Result<Option<Value>, PromptOverrideDbError> {
    let key = match project_id {
        Some(project_id) if project_id > 0 => {
            system_settings::prompt_assembly_project_key(project_id, call_key, prompt_name)
        }
        Some(_) => return Err(super::types::invalid_argument("项目 ID 必须大于 0")),
        None => system_settings::prompt_assembly_global_key(call_key, prompt_name),
    };
    system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(key))
        .one(db)
        .await
        .map(|record| record.map(|record| record.value))
        .map_err(|source| database("读取提示词装配配置失败", source))
}

pub async fn save_prompt_assembly_override(
    db: &impl crate::DbExecutor,
    project_id: Option<i64>,
    call_key: &str,
    prompt_name: &str,
    value: Value,
) -> Result<(), PromptOverrideDbError> {
    if let Some(project_id) = project_id {
        ensure_project_exists(db, project_id, MissingProjectMessage::Command).await?;
    }
    let key = match project_id {
        Some(project_id) => {
            system_settings::prompt_assembly_project_key(project_id, call_key, prompt_name)
        }
        None => system_settings::prompt_assembly_global_key(call_key, prompt_name),
    };
    save_system_setting(db, key, value, "提示词装配配置").await
}

pub async fn delete_prompt_assembly_override(
    db: &impl crate::DbExecutor,
    project_id: Option<i64>,
    call_key: &str,
    prompt_name: &str,
) -> Result<(), PromptOverrideDbError> {
    let key = match project_id {
        Some(project_id) if project_id > 0 => {
            system_settings::prompt_assembly_project_key(project_id, call_key, prompt_name)
        }
        Some(_) => return Err(super::types::invalid_argument("项目 ID 必须大于 0")),
        None => system_settings::prompt_assembly_global_key(call_key, prompt_name),
    };
    system_settings::Entity::delete_many()
        .filter(system_settings::Column::Key.eq(key))
        .exec(db)
        .await
        .map_err(|source| database("删除提示词装配配置失败", source))?;
    Ok(())
}

async fn save_system_setting(
    db: &impl crate::DbExecutor,
    key: String,
    value: Value,
    description: &str,
) -> Result<(), PromptOverrideDbError> {
    let now = Utc::now().naive_utc();
    system_settings::Entity::insert(system_settings::ActiveModel {
        key: Set(key),
        value: Set(value),
        category: Set("prompt_assembly".to_string()),
        description: Set(Some(description.to_string())),
        updated_at: Set(now),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(system_settings::Column::Key)
            .update_columns([
                system_settings::Column::Value,
                system_settings::Column::Category,
                system_settings::Column::Description,
                system_settings::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(db)
    .await
    .map_err(|source| database("保存提示词装配配置失败", source))?;
    Ok(())
}

pub(super) async fn ensure_project_exists(
    db: &impl crate::DbExecutor,
    project_id: i64,
    missing_message: MissingProjectMessage,
) -> Result<(), PromptOverrideDbError> {
    let exists = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(crate::project::ProjectMutationError::Database)?
        .is_some();
    if exists {
        return Ok(());
    }

    match missing_message {
        MissingProjectMessage::Command => Err(PromptOverrideDbError::NotFound(format!(
            "项目不存在: {project_id}"
        ))),
    }
}

pub(super) async fn ensure_hydration_projects_exist(
    db: &impl crate::DbExecutor,
    project_ids: &[i64],
) -> Result<(), PromptOverrideDbError> {
    let requested_ids = project_ids.iter().copied().collect::<HashSet<_>>();
    if requested_ids.is_empty() {
        return Ok(());
    }

    let existing_ids = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .filter(novel_project::Column::Id.is_in(requested_ids.iter().copied()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(crate::project::ProjectMutationError::Database)?
        .into_iter()
        .collect::<HashSet<_>>();

    if let Some(project_id) = project_ids
        .iter()
        .find(|project_id| !existing_ids.contains(project_id))
    {
        return Err(PromptOverrideDbError::NotFound(format!(
            "提示词覆盖关联项目不存在: {project_id}"
        )));
    }

    Ok(())
}

pub(super) async fn all_prompt_override_settings(
    db: &impl crate::DbExecutor,
) -> Result<Vec<system_settings::Model>, PromptOverrideDbError> {
    system_settings::Entity::find()
        .filter(
            system_settings::Column::Key.starts_with(system_settings::PROMPT_OVERRIDE_KEY_PREFIX),
        )
        .all(db)
        .await
        .map_err(|source| database("读取系统设置中的项目提示词覆盖失败", source))
}

pub(super) async fn project_prompt_override_settings(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<Vec<system_settings::Model>, PromptOverrideDbError> {
    system_settings::Entity::find()
        .filter(
            system_settings::Column::Key
                .starts_with(system_settings::prompt_override_key_prefix(project_id)),
        )
        .all(db)
        .await
        .map_err(|source| database("读取项目提示词覆盖失败", source))
}

pub(super) fn project_override_record_content(
    record: &system_settings::Model,
) -> Result<String, PromptOverrideDbError> {
    let content = record.value.as_str().ok_or_else(|| {
        super::types::invalid_argument(format!("提示词覆盖记录 value 必须是字符串: {}", record.key))
    })?;

    validate_project_override_content(content)?;

    Ok(content.to_string())
}
