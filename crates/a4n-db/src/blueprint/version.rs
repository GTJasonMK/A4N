use a4n_core::models::project::blueprint_version;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value;

use super::{BlueprintDbError, InsertBlueprintVersionInput};

pub(super) struct BlueprintVersionSnapshots {
    pub(super) world_setting: Option<Value>,
    pub(super) characters: Option<Value>,
    pub(super) relationships: Option<Value>,
    pub(super) synopsis: Option<String>,
}

pub(super) fn blueprint_version_snapshots(blueprint_data: &Value) -> BlueprintVersionSnapshots {
    BlueprintVersionSnapshots {
        world_setting: blueprint_data.get("world_setting").cloned(),
        characters: blueprint_data.get("characters").cloned(),
        relationships: blueprint_data.get("relationships").cloned(),
        synopsis: blueprint_data["full_synopsis"].as_str().map(str::to_string),
    }
}

/// 插入蓝图版本快照记录。
pub async fn insert_blueprint_version_record<C>(
    db: &C,
    input: InsertBlueprintVersionInput,
) -> Result<blueprint_version::Model, BlueprintDbError>
where
    C: ConnectionTrait,
{
    let snapshots = blueprint_version_snapshots(&input.blueprint_data);
    Ok(blueprint_version::ActiveModel {
        project_id: Set(input.project_id),
        version: Set(input.version),
        change_description: Set(input.change_description),
        blueprint_data: Set(input.blueprint_data),
        world_setting_snapshot: Set(snapshots.world_setting),
        characters_snapshot: Set(snapshots.characters),
        relationships_snapshot: Set(snapshots.relationships),
        synopsis_snapshot: Set(snapshots.synopsis),
        created_at: Set(input.now),
        created_by: Set(input.created_by),
        ..Default::default()
    }
    .insert(db)
    .await?)
}
