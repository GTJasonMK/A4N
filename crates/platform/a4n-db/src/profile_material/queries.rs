use a4n_db_models::models::{ai::generated_image, asset::profile_primary_material};
use std::collections::{BTreeMap, HashSet};

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::{
    contracts::{ProfileMaterialDbError, ProfileMaterialImageIdentityRow, ProfileMaterialListRows},
    identity::{PROFILE_KIND_OBJECT, PROFILE_KIND_SCENE, profile_material_identity},
    records::primary_material_record,
    validation::{database, require_positive, require_text},
};

pub async fn profile_material_list_rows(
    db: &impl crate::DbExecutor,
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
    let images = images?
        .into_iter()
        .map(crate::video::generated_image_record_from_model)
        .collect();
    let primary_rows = primary_rows?
        .into_iter()
        .map(primary_material_record)
        .collect();

    Ok(ProfileMaterialListRows {
        images,
        primary_rows,
    })
}

pub async fn profile_material_image_identity_rows_by_type(
    db: &impl crate::DbExecutor,
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

pub async fn profile_material_image_counts_by_profile(
    db: &impl crate::DbExecutor,
    project_id: i64,
    image_type: &str,
    profile_ids: &[i64],
) -> Result<BTreeMap<i64, i64>, ProfileMaterialDbError> {
    let project_id = require_positive(project_id, "project_id")?;
    let image_type = require_text(image_type, "image_type")?;
    if profile_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let requested = profile_ids.iter().copied().collect::<HashSet<_>>();
    let mut counts = BTreeMap::new();
    for image in profile_material_image_identity_rows_by_type(db, project_id, image_type).await? {
        if let Some((_, profile_id)) =
            profile_material_identity(&image.image_type, image.metadata_json.as_ref())
            && requested.contains(&profile_id)
        {
            *counts.entry(profile_id).or_insert(0) += 1;
        }
    }
    Ok(counts)
}

pub async fn profile_material_image_ids_by_profile(
    db: &impl crate::DbExecutor,
    project_id: i64,
    image_type: &str,
    profile_id: i64,
) -> Result<Vec<i64>, ProfileMaterialDbError> {
    let project_id = require_positive(project_id, "project_id")?;
    let profile_id = require_positive(profile_id, "profile_id")?;
    let image_type = require_text(image_type, "image_type")?;
    let expected_kind = match image_type {
        a4n_domain::image::IMAGE_TYPE_SCENE_MATERIAL => PROFILE_KIND_SCENE,
        a4n_domain::image::IMAGE_TYPE_OBJECT_MATERIAL => PROFILE_KIND_OBJECT,
        _ => {
            return Err(ProfileMaterialDbError::InvalidArgument(
                "image_type 不是档案素材类型".to_string(),
            ));
        }
    };
    Ok(
        profile_material_image_identity_rows_by_type(db, project_id, image_type)
            .await?
            .into_iter()
            .filter(|image| {
                profile_material_identity(&image.image_type, image.metadata_json.as_ref())
                    == Some((expected_kind, profile_id))
            })
            .map(|image| image.id)
            .collect(),
    )
}
