mod asset;
mod generated_image;
mod portrait;
mod target;

use a4n_core::models::project::novel_project;
use sea_orm::{ConnectionTrait, EntityTrait, QuerySelect};

use crate::galgame::{GalgameMediaRecordError, PROJECT_KIND_GALGAME};

async fn galgame_media_project_context<C>(
    db: &C,
    project_id: i64,
    use_row_lock: bool,
) -> Result<(novel_project::Model, i64), GalgameMediaRecordError>
where
    C: ConnectionTrait,
{
    let mut query = novel_project::Entity::find_by_id(project_id);
    if use_row_lock {
        query = query.lock_exclusive();
    }
    let project = query
        .one(db)
        .await
        .map_err(GalgameMediaRecordError::Database)?
        .ok_or_else(|| GalgameMediaRecordError::NotFound(format!("项目不存在: {project_id}")))?;
    if project.project_kind != PROJECT_KIND_GALGAME {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "项目不是 Galgame 派生项目: {}",
            project.id
        )));
    }
    let root_project_id = project.root_project_id.ok_or_else(|| {
        GalgameMediaRecordError::InvalidArgument(format!(
            "Galgame 项目缺少根文本项目: {}",
            project.id
        ))
    })?;
    Ok((project, root_project_id))
}

pub(super) use asset::galgame_bindable_media_asset;
pub(in crate::galgame) use asset::validate_completed_visual_asset;
#[cfg(test)]
pub(in crate::galgame) use asset::validate_galgame_media_asset_source_context;
pub use generated_image::galgame_generated_image_by_id;
pub use portrait::validate_galgame_portrait_position_samples;
pub(in crate::galgame) use portrait::validate_galgame_portrait_spec_source_lines;
pub(in crate::galgame) use portrait::{
    validate_galgame_portrait_spec_family_context, validate_galgame_portrait_spec_source_chapter,
};
pub use target::galgame_media_target_context;
pub(super) use target::galgame_media_target_context_in;
