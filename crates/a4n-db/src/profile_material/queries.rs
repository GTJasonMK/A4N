use a4n_core::models::{ai::generated_image, asset::profile_primary_material};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::{
    contracts::{ProfileMaterialDbError, ProfileMaterialImageIdentityRow, ProfileMaterialListRows},
    validation::{database, require_positive, require_text},
};

pub async fn profile_material_list_rows(
    db: &DatabaseConnection,
    project_id: i64,
    image_types: &[&str],
) -> Result<ProfileMaterialListRows, ProfileMaterialDbError> {
    let project_id = require_positive(project_id, "project_id")?;
    if image_types.is_empty() {
        return Err(ProfileMaterialDbError::InvalidArgument(
            "image_types 不能为空".to_string(),
        ));
    }
    let image_types = image_types
        .iter()
        .map(|image_type| require_text(image_type, "image_type"))
        .collect::<Result<Vec<_>, _>>()?;
    let (images, primary_rows) = futures::join!(
        async {
            generated_image::Entity::find()
                .filter(generated_image::Column::ProjectId.eq(project_id))
                .filter(generated_image::Column::ImageType.is_in(image_types))
                .order_by_desc(generated_image::Column::Id)
                .all(db)
                .await
                .map_err(|source| database("读取档案素材图片失败", source))
        },
        async {
            profile_primary_material::Entity::find()
                .filter(profile_primary_material::Column::ProjectId.eq(project_id))
                .all(db)
                .await
                .map_err(|source| database("读取档案 primary material 失败", source))
        },
    );
    let images = images?;
    let primary_rows = primary_rows?;

    Ok(ProfileMaterialListRows {
        images,
        primary_rows,
    })
}

pub async fn profile_material_images_by_type(
    db: &DatabaseConnection,
    project_id: i64,
    image_type: &str,
) -> Result<Vec<generated_image::Model>, ProfileMaterialDbError> {
    let project_id = require_positive(project_id, "project_id")?;
    let image_type = require_text(image_type, "image_type")?;
    generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ImageType.eq(image_type))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(|source| database("按类型读取档案素材图片失败", source))
}

pub async fn profile_material_image_identity_rows_by_type(
    db: &DatabaseConnection,
    project_id: i64,
    image_type: &str,
) -> Result<Vec<ProfileMaterialImageIdentityRow>, ProfileMaterialDbError> {
    let project_id = require_positive(project_id, "project_id")?;
    let image_type = require_text(image_type, "image_type")?;
    generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ImageType.eq(image_type))
        .order_by_asc(generated_image::Column::Id)
        .into_model::<ProfileMaterialImageIdentityRow>()
        .all(db)
        .await
        .map_err(|source| database("按类型读取档案素材身份失败", source))
}
