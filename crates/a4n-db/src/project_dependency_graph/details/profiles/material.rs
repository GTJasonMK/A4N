use crate::project_dependency_graph::AssetLineageRecord;
use crate::project_dependency_graph::details::records::generated_image_record;
use a4n_core::{error::AppError, models::ai::generated_image};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(in crate::project_dependency_graph) async fn profile_material_image_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    profile_id: Option<i64>,
    profile_kind: &str,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let Some(profile_id) = profile_id else {
        return Ok(Vec::new());
    };
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(images
        .into_iter()
        .filter(|image| {
            crate::profile_material::generated_image_profile_material_identity(image)
                == Some((profile_kind, profile_id))
        })
        .map(|image| generated_image_record(image, "profile_material_image", "image_generated"))
        .collect())
}
