use crate::project::dependency_graph::AssetLineageRecord;
use crate::project::dependency_graph::details::records::{
    db_record_result_with_related, db_related_record, lineage_ref, optional_prompt_sources,
    prompt_source,
};
use a4n_db_models::models::{ai::generated_image, character::character_portrait};
use a4n_domain::GeneratedMediaReference;
use a4n_error::AppError;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
    sea_query::{Expr, ExprTrait},
};
use serde_json::Value;
use std::collections::HashMap;

pub(in crate::project::dependency_graph) async fn character_portrait_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    profile_id: Option<i64>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let Some(profile_id) = profile_id else {
        return Ok(Vec::new());
    };
    let portrait_ids = character_portrait::Entity::find()
        .select_only()
        .column_as(
            Expr::col(character_portrait::Column::Id).cast_as("text"),
            "portrait_id",
        )
        .filter(character_portrait::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .into_query();
    let (portraits_result, images_result) = futures::join!(
        character_portrait::Entity::find()
            .filter(character_portrait::Column::ProjectId.is_in(project_ids.iter().copied()))
            .filter(character_portrait::Column::ProfileId.eq(profile_id))
            .order_by_asc(character_portrait::Column::Id)
            .all(db),
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
            .filter(generated_image::Column::ImageType.eq("portrait"))
            .filter(Expr::cust("metadata_json ->> 'portrait_id'").in_subquery(portrait_ids))
            .order_by_asc(generated_image::Column::Id)
            .all(db),
    );
    let portraits = portraits_result.map_err(AppError::from)?;
    let images = images_result.map_err(AppError::from)?;
    let images_by_portrait_id = images.iter().fold(
        HashMap::with_capacity(images.len()),
        |mut by_portrait_id, image| {
            let Some(portrait_id) = image
                .metadata_json
                .as_ref()
                .and_then(|value| value.get("portrait_id"))
                .and_then(Value::as_i64)
            else {
                return by_portrait_id;
            };
            by_portrait_id.entry(portrait_id).or_insert(image);
            by_portrait_id
        },
    );
    Ok(portraits
        .into_iter()
        .map(|portrait| {
            let mirror = images_by_portrait_id.get(&portrait.id).copied();
            let variant_kind = mirror
                .and_then(|image| image.metadata_json.as_ref())
                .and_then(|value| value.get("variant_kind"))
                .and_then(Value::as_str);
            AssetLineageRecord {
                id: format!("character_portrait:{}", portrait.id),
                label: format!("{} #{}", portrait.portrait_type, portrait.id),
                asset_kind: "character_portrait".to_string(),
                asset_id: Some(portrait.id),
                status: None,
                origin: if variant_kind == Some("manual_upload") {
                    "uploaded".to_string()
                } else {
                    "image_generated".to_string()
                },
                source_refs: vec![lineage_ref(
                    &format!("character_profile:{profile_id}"),
                    "人物档案",
                    "text",
                    "character_profile_record",
                    Some(profile_id),
                )],
                prompt_sources: optional_prompt_sources(vec![prompt_source(
                    &format!("character_portrait_prompt:{}", portrait.id),
                    if variant_kind == Some("manual_upload") {
                        "manual_description"
                    } else {
                        "character_portrait_prompt"
                    },
                    Some(&portrait.prompt),
                    Some(portrait.id),
                    None,
                )]),
                media_sources: mirror
                    .map(|image| {
                        vec![lineage_ref(
                            &GeneratedMediaReference::Image(image.id).to_string(),
                            "镜像图片记录",
                            "image",
                            "generated_image",
                            Some(image.id),
                        )]
                    })
                    .unwrap_or_default(),
                request_snapshot: mirror
                    .and_then(|image| image.metadata_json.as_ref())
                    .and_then(|value| value.get("generation").cloned()),
                input_status: if mirror.is_some() {
                    "complete"
                } else {
                    "missing"
                }
                .to_string(),
                generation_inputs: Vec::new(),
                result: db_record_result_with_related(
                    "character_portrait",
                    &portrait,
                    mirror
                        .map(|image| db_related_record("generated_image", image))
                        .into_iter()
                        .collect(),
                ),
                warnings: if mirror.is_none() {
                    vec!["未找到对应 generated_image 镜像记录。".to_string()]
                } else {
                    Vec::new()
                },
            }
        })
        .collect())
}
