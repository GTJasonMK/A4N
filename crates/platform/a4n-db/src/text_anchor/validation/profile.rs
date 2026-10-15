use a4n_db_models::models::{
    asset::object_profile, character::protagonist_profile, scene::scene_profile,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

use super::project::ensure_text_root_project;
use crate::text_anchor::{TextAnchorDbError, TextAnchorDbResult};
use a4n_domain::text_anchor::TextAnchorProfileKind;

pub(in crate::text_anchor) async fn ensure_profiles_belong_to_root<C>(
    db: &C,
    root_project_id: i64,
    profile_kind: TextAnchorProfileKind,
    profile_ids: &[i64],
) -> TextAnchorDbResult<()>
where
    C: ConnectionTrait,
{
    ensure_text_root_project(db, root_project_id).await?;
    if profile_ids.is_empty() {
        return Ok(());
    }
    let project_ids = match profile_kind {
        TextAnchorProfileKind::Character => protagonist_profile::Entity::find()
            .select_only()
            .column(protagonist_profile::Column::Id)
            .column(protagonist_profile::Column::ProjectId)
            .filter(protagonist_profile::Column::Id.is_in(profile_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>(),
        TextAnchorProfileKind::Object => object_profile::Entity::find()
            .select_only()
            .column(object_profile::Column::Id)
            .column(object_profile::Column::ProjectId)
            .filter(object_profile::Column::Id.is_in(profile_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>(),
        TextAnchorProfileKind::Scene => scene_profile::Entity::find()
            .select_only()
            .column(scene_profile::Column::Id)
            .column(scene_profile::Column::ProjectId)
            .filter(scene_profile::Column::Id.is_in(profile_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>(),
    };
    let (profile_name, missing_message) = match profile_kind {
        TextAnchorProfileKind::Character => ("人物档案", "人物档案不存在"),
        TextAnchorProfileKind::Object => ("物体档案", "物体档案不存在"),
        TextAnchorProfileKind::Scene => ("场景档案", "场景档案不存在"),
    };
    for profile_id in profile_ids {
        let project_id = project_ids.get(profile_id).ok_or_else(|| {
            TextAnchorDbError::NotFound(format!("{missing_message}: {profile_id}"))
        })?;
        if *project_id != root_project_id {
            return Err(TextAnchorDbError::InvalidArgument(format!(
                "{profile_name}不属于文本根项目: profile_id={profile_id}"
            )));
        }
    }
    Ok(())
}
