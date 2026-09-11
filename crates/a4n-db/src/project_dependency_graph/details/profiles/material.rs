use crate::project_dependency_graph::AssetLineageRecord;
use crate::project_dependency_graph::details::records::generated_image_record;
use a4n_db_models::models::ai::generated_image;
use a4n_domain::image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};
use a4n_error::AppError;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, sea_query::Expr,
};

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
    let image_type = match profile_kind {
        crate::profile_material::PROFILE_KIND_OBJECT => IMAGE_TYPE_OBJECT_MATERIAL,
        crate::profile_material::PROFILE_KIND_SCENE => IMAGE_TYPE_SCENE_MATERIAL,
        _ => return Ok(Vec::new()),
    };
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_image::Column::ImageType.eq(image_type))
        .filter(Expr::cust("metadata_json ->> 'profile_kind'").eq(profile_kind))
        .filter(Expr::cust("metadata_json ->> 'profile_id'").eq(profile_id.to_string()))
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
