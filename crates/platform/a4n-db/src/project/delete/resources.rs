use std::collections::BTreeSet;

use a4n_db_models::models::project::source_snapshot_profile;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};

#[derive(Default)]
pub(crate) struct RetainedSourceResources {
    pub image_ids: BTreeSet<i64>,
    pub scene_ids: BTreeSet<i64>,
    pub object_ids: BTreeSet<i64>,
}

/// Reset current content without deleting identities or files still used by frozen sources.
/// Full project deletion supplies an empty set after removing its source dependants.
pub(crate) async fn retained_source_resources<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<RetainedSourceResources, DbErr> {
    let (image_ids, scene_ids) = crate::chapter_visual_plan::visual_plan_resource_ids(db).await?;
    let mut resources = RetainedSourceResources {
        image_ids,
        scene_ids,
        object_ids: BTreeSet::new(),
    };
    let profiles = source_snapshot_profile::Entity::find()
        .select_only()
        .column(source_snapshot_profile::Column::ProfileKind)
        .column(source_snapshot_profile::Column::SourceProfileId)
        .filter(source_snapshot_profile::Column::RootProjectId.eq(project_id))
        .filter(source_snapshot_profile::Column::ProfileKind.is_in(["scene", "object"]))
        .into_tuple::<(String, i64)>()
        .all(db)
        .await?;
    for (kind, id) in profiles {
        if kind == "scene" {
            resources.scene_ids.insert(id);
        } else {
            resources.object_ids.insert(id);
        }
    }
    Ok(resources)
}
