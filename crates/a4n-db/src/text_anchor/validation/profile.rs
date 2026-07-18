use a4n_core::models::{
    asset::object_profile, character::protagonist_profile, scene::scene_profile,
};
use sea_orm::{DatabaseConnection, EntityTrait};

use super::project::ensure_text_root_project;
use crate::text_anchor::{TextAnchorDbError, TextAnchorDbResult, TextAnchorProfileKind};

pub(in crate::text_anchor) async fn ensure_profile_belongs_to_root(
    db: &DatabaseConnection,
    root_project_id: i64,
    profile_kind: TextAnchorProfileKind,
    profile_id: i64,
) -> TextAnchorDbResult<()> {
    ensure_text_root_project(db, root_project_id).await?;
    match profile_kind {
        TextAnchorProfileKind::Character => {
            let profile = protagonist_profile::Entity::find_by_id(profile_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    TextAnchorDbError::NotFound(format!("人物档案不存在: {profile_id}"))
                })?;
            if profile.project_id != root_project_id {
                return Err(TextAnchorDbError::InvalidArgument(format!(
                    "人物档案不属于文本根项目: profile_id={profile_id}"
                )));
            }
        }
        TextAnchorProfileKind::Object => {
            let profile = object_profile::Entity::find_by_id(profile_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    TextAnchorDbError::NotFound(format!("物体档案不存在: {profile_id}"))
                })?;
            if profile.project_id != root_project_id {
                return Err(TextAnchorDbError::InvalidArgument(format!(
                    "物体档案不属于文本根项目: profile_id={profile_id}"
                )));
            }
        }
        TextAnchorProfileKind::Scene => {
            let profile = scene_profile::Entity::find_by_id(profile_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    TextAnchorDbError::NotFound(format!("场景档案不存在: {profile_id}"))
                })?;
            if profile.project_id != root_project_id {
                return Err(TextAnchorDbError::InvalidArgument(format!(
                    "场景档案不属于文本根项目: profile_id={profile_id}"
                )));
            }
        }
    }
    Ok(())
}
