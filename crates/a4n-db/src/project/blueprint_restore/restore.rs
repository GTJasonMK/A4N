use super::fields::RestoredBlueprintFields;
use super::project_sync::sync_project_after_blueprint_restore_record;
use super::snapshots::{
    restore_blueprint_character_snapshot_record, restore_blueprint_relationship_snapshot_record,
};
use super::*;

pub async fn restore_blueprint_version_record(
    db: &DatabaseConnection,
    project_id: i64,
    version_name: &str,
) -> Result<BlueprintRestoreResult, BlueprintRestoreError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(BlueprintRestoreError::InvalidArgument)?;
    let version_name = version_name.trim();
    if version_name.is_empty() {
        return Err(BlueprintRestoreError::InvalidArgument(
            "版本号不能为空".to_string(),
        ));
    }

    let txn = db
        .begin()
        .await
        .map_err(|error| BlueprintRestoreError::database("启动蓝图版本恢复事务失败", error))?;
    let version = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .filter(blueprint_version::Column::Version.eq(version_name))
        .one(&txn)
        .await
        .map_err(|error| BlueprintRestoreError::database("读取蓝图版本失败", error))?
        .ok_or_else(|| BlueprintRestoreError::VersionNotFound {
            project_id,
            version: version_name.to_string(),
        })?;

    let now = Utc::now().naive_utc();
    let data = &version.blueprint_data;
    let fields = RestoredBlueprintFields::from_snapshot(data, &version.version)?;
    let existing = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(&txn)
        .await
        .map_err(|error| BlueprintRestoreError::database("读取当前蓝图失败", error))?;

    let restored = if let Some(blueprint) = existing {
        let mut active: novel_blueprint::ActiveModel = blueprint.into();
        fields.apply_to_existing(&mut active, now);
        active
            .update(&txn)
            .await
            .map_err(|error| BlueprintRestoreError::database("更新当前蓝图失败", error))?
    } else {
        fields
            .into_active_model(project_id, now)
            .insert(&txn)
            .await
            .map_err(|error| BlueprintRestoreError::database("创建当前蓝图失败", error))?
    };

    restore_blueprint_character_snapshot_record(
        &txn,
        project_id,
        data.get("characters"),
        now,
    )
    .await?;
    restore_blueprint_relationship_snapshot_record(
        &txn,
        project_id,
        data.get("relationships"),
        now,
    )
    .await?;
    let status_change =
        sync_project_after_blueprint_restore_record(&txn, project_id, &restored, data, now).await?;
    txn.commit()
        .await
        .map_err(|error| BlueprintRestoreError::database("提交蓝图版本恢复事务失败", error))?;

    Ok(BlueprintRestoreResult {
        blueprint: restored,
        version: version.version,
        status_change,
        restored_at: now,
    })
}
