use crate::project_dependency_graph::{
    AssetLineageRecord, db_record_result_with_related, db_related_record, lineage_ref,
    optional_prompt_sources, prompt_source,
};
use a4n_core::{
    error::AppError,
    models::{ai::generated_image, character::character_portrait},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;

pub(in crate::project_dependency_graph) async fn character_portrait_detail_records<C>(
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
    let portraits = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .order_by_asc(character_portrait::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_image::Column::ImageType.eq("portrait"))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(portraits
        .into_iter()
        .map(|portrait| {
            let mirror = images.iter().find(|image| {
                image
                    .metadata_json
                    .as_ref()
                    .and_then(|value| value.get("portrait_id"))
                    .and_then(Value::as_i64)
                    == Some(portrait.id)
            });
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
                            &format!("generated_image:{}", image.id),
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
                generation_runs: Vec::new(),
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
