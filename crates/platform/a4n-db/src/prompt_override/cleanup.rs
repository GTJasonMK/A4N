use std::collections::HashSet;

use a4n_db_models::models::ai::system_settings;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::PromptOverrideDbError;
use super::identity::parse_project_override_key;
use super::storage::all_prompt_override_settings;
use super::types::database;

pub async fn delete_project_overrides_for_prompt_names(
    db: &impl crate::DbExecutor,
    prompt_names: &HashSet<String>,
) -> Result<u64, PromptOverrideDbError> {
    if prompt_names.is_empty() {
        return Ok(0);
    }

    let records = all_prompt_override_settings(db).await?;
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
    db: &impl crate::DbExecutor,
) -> Result<(), PromptOverrideDbError> {
    let records = all_prompt_override_settings(db).await?;
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
        let identity = parse_project_override_key(&record.key)?;
        if prompt_names.contains(&identity.name) {
            ids.push(record.id);
        }
    }
    Ok(ids)
}
