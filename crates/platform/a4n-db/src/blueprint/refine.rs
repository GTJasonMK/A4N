use a4n_db_models::models::project::novel_blueprint;
use a4n_domain::project::BlueprintRecord;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::{BlueprintDbError, BlueprintRefinePatchInput};

/// 应用蓝图优化补丁字段并更新版本号。
pub async fn apply_blueprint_refine_patch_record<C>(
    db: &C,
    existing: BlueprintRecord,
    patch: BlueprintRefinePatchInput,
) -> Result<BlueprintRecord, BlueprintDbError>
where
    C: ConnectionTrait,
{
    let mut active: novel_blueprint::ActiveModel =
        crate::project::blueprint_model_from_record(existing).into();
    if let Some(style) = patch.style {
        active.style = Set(Some(style));
    }
    if let Some(full_synopsis) = patch.full_synopsis {
        active.full_synopsis = Set(Some(full_synopsis));
    }
    if let Some(world_setting) = patch.world_setting {
        active.world_setting = Set(Some(world_setting));
    }
    active.current_version = Set(patch.current_version);
    active.updated_at = Set(patch.now);
    Ok(crate::project::blueprint_record(active.update(db).await?))
}
