use a4n_db_models::models::{
    asset::object_profile, character::protagonist_profile, scene::scene_profile,
};
use sea_orm::{ConnectionTrait, EntityTrait, QuerySelect};

use super::project::ensure_text_root_project;
use crate::text_anchor::{TextAnchorDbError, TextAnchorDbResult};
use a4n_domain::text_anchor::TextAnchorProfileKind;

pub(in crate::text_anchor) async fn ensure_profile_belongs_to_root<C>(
    db: &C,
    root_project_id: i64,
    profile_kind: TextAnchorProfileKind,
    profile_id: i64,
) -> TextAnchorDbResult<()>
where
    C: ConnectionTrait,
{
    ensure_text_root_project(db, root_project_id).await?;
    match profile_kind {
        TextAnchorProfileKind::Character => {
            let profile = protagonist_profile::Entity::find_by_id(profile_id)
                .select_only()
                .column(protagonist_profile::Column::ProjectId)
                .into_tuple::<i64>()
                .one(db)
                .await?
                .ok_or_else(|| {
                    TextAnchorDbError::NotFound(format!("人物档案不存在: {profile_id}"))
                })?;
            if profile != root_project_id {
                return Err(TextAnchorDbError::InvalidArgument(format!(
                    "人物档案不属于文本根项目: profile_id={profile_id}"
                )));
            }
        }
        TextAnchorProfileKind::Object => {
            let profile = object_profile::Entity::find_by_id(profile_id)
                .select_only()
                .column(object_profile::Column::ProjectId)
                .into_tuple::<i64>()
                .one(db)
                .await?
                .ok_or_else(|| {
                    TextAnchorDbError::NotFound(format!("物体档案不存在: {profile_id}"))
                })?;
            if profile != root_project_id {
                return Err(TextAnchorDbError::InvalidArgument(format!(
                    "物体档案不属于文本根项目: profile_id={profile_id}"
                )));
            }
        }
        TextAnchorProfileKind::Scene => {
            let profile = scene_profile::Entity::find_by_id(profile_id)
                .select_only()
                .column(scene_profile::Column::ProjectId)
                .into_tuple::<i64>()
                .one(db)
                .await?
                .ok_or_else(|| {
                    TextAnchorDbError::NotFound(format!("场景档案不存在: {profile_id}"))
                })?;
            if profile != root_project_id {
                return Err(TextAnchorDbError::InvalidArgument(format!(
                    "场景档案不属于文本根项目: profile_id={profile_id}"
                )));
            }
        }
    }
    Ok(())
}
