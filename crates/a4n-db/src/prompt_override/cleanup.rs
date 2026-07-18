use std::collections::HashSet;

use a4n_core::models::ai::system_settings;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::PromptOverrideDbError;
use super::identity::parse_project_override_key;
use super::storage::all_system_settings;
use super::types::database;

pub async fn delete_project_overrides_for_prompt_names(
    db: &DatabaseConnection,
    prompt_names: &HashSet<String>,
) -> Result<u64, PromptOverrideDbError> {
    if prompt_names.is_empty() {
        return Ok(0);
    }

    let records = all_system_settings(db).await?;
    let ids = retired_project_override_record_ids(&records, prompt_names)?;
    if ids.is_empty() {
        return Ok(0);
    }

    let result = system_settings::Entity::delete_many()
        .filter(system_settings::Column::Id.is_in(ids))
        .exec(db)
        .await
        .map_err(|source| database("批量删除退役提示词覆盖失败", source))?;
    Ok(result.rows_affected)
}

pub async fn preflight_project_override_keys_for_retired_cleanup(
    db: &DatabaseConnection,
) -> Result<(), PromptOverrideDbError> {
    let records = all_system_settings(db).await?;
    validate_project_override_record_keys(&records)
}

pub(super) fn validate_project_override_record_keys(
    records: &[system_settings::Model],
) -> Result<(), PromptOverrideDbError> {
    for record in records {
        parse_project_override_key(&record.key)?;
    }
    Ok(())
}

pub(super) fn retired_project_override_record_ids(
    records: &[system_settings::Model],
    prompt_names: &HashSet<String>,
) -> Result<Vec<i64>, PromptOverrideDbError> {
    let mut ids = Vec::new();
    for record in records {
        let Some(identity) = parse_project_override_key(&record.key)? else {
            continue;
        };
        if prompt_names.contains(&identity.name) {
            ids.push(record.id);
        }
    }
    Ok(ids)
}
