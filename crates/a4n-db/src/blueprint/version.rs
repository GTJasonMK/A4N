use a4n_core::models::project::blueprint_version;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::{BlueprintDbError, InsertBlueprintVersionInput};

/// 插入蓝图版本快照记录。
pub async fn insert_blueprint_version_record<C>(
    db: &C,
    input: InsertBlueprintVersionInput,
) -> Result<blueprint_version::Model, BlueprintDbError>
where
    C: ConnectionTrait,
{
    let change_description =
        canonical_version_text(input.change_description, "change_description")?;
    let created_by = canonical_version_text(input.created_by, "created_by")?;
    Ok(blueprint_version::ActiveModel {
        project_id: Set(input.project_id),
        version: Set(input.version),
        change_description: Set(Some(change_description)),
        blueprint_data: Set(input.blueprint_data),
        created_at: Set(input.now),
        created_by: Set(Some(created_by)),
        ..Default::default()
    }
    .insert(db)
    .await?)
}

fn canonical_version_text(value: String, field: &str) -> Result<String, BlueprintDbError> {
    if value.is_empty() || value.trim() != value {
        Err(BlueprintDbError::InvalidArgument(format!(
            "{field} 必须是非空规范文本"
        )))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::canonical_version_text;

    #[test]
    fn blueprint_version_metadata_requires_canonical_text() {
        assert_eq!(
            canonical_version_text("system".to_string(), "created_by").unwrap(),
            "system"
        );
        for value in ["", " system "] {
            assert!(canonical_version_text(value.to_string(), "created_by").is_err());
        }
    }
}
