use a4n_core::models::ai::system_settings;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::Value;

use super::identity::{
    normalize_project_override_identity, project_override_key, validate_project_override_content,
};
use super::types::database;
use super::{ProjectOverrideIdentity, PromptOverrideDbError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MissingProjectMessage {
    Command,
    Hydrate,
}

pub async fn load_project_override_persisted_content(
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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

pub(super) async fn ensure_project_exists(
    db: &DatabaseConnection,
    project_id: i64,
    missing_message: MissingProjectMessage,
) -> Result<(), PromptOverrideDbError> {
    let exists = crate::project::project_by_id_record(db, project_id)
        .await?
        .is_some();
    if exists {
        return Ok(());
    }

    match missing_message {
        MissingProjectMessage::Command => Err(PromptOverrideDbError::NotFound(format!(
            "项目不存在: {project_id}"
        ))),
        MissingProjectMessage::Hydrate => Err(PromptOverrideDbError::NotFound(format!(
            "提示词覆盖关联项目不存在: {project_id}"
        ))),
    }
}

pub(super) async fn all_system_settings(
    db: &DatabaseConnection,
) -> Result<Vec<system_settings::Model>, PromptOverrideDbError> {
    system_settings::Entity::find()
        .all(db)
        .await
        .map_err(|source| database("读取系统设置中的项目提示词覆盖失败", source))
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
