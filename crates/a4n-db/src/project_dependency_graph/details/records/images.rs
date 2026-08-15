use super::generic::{lineage_ref, optional_prompt_sources, prompt_source};
use super::json_records::db_record_result;
use crate::project_dependency_graph::{AssetLineageRecord, AssetLineageRef};
use a4n_core::models::ai::generated_image;
use serde_json::Value;

pub(in crate::project_dependency_graph) fn generated_image_result(
    image: &generated_image::Model,
) -> Value {
    db_record_result("generated_image", image)
}

pub(in crate::project_dependency_graph) fn generated_image_record(
    image: generated_image::Model,
    asset_kind: &str,
    origin: &str,
) -> AssetLineageRecord {
    AssetLineageRecord {
        id: format!("generated_image:{}", image.id),
        label: format!("{} #{}", image.image_type, image.id),
        asset_kind: asset_kind.to_string(),
        asset_id: Some(image.id),
        status: None,
        origin: origin.to_string(),
        source_refs: image
            .chapter_id
            .map(|id| {
                vec![lineage_ref(
                    &format!("chapter:{id}"),
                    "章节正文",
                    "text",
                    "chapter_content",
                    Some(id),
                )]
            })
            .unwrap_or_default(),
        prompt_sources: optional_prompt_sources(vec![prompt_source(
            &format!("generated_image_prompt:{}", image.id),
            "image_generation_prompt",
            Some(&image.prompt),
            Some(image.id),
            None,
        )]),
        media_sources: generated_image_media_sources(&image),
        request_snapshot: image
            .metadata_json
            .as_ref()
            .and_then(|value| {
                value
                    .get("provider_request_snapshot")
                    .or_else(|| value.get("request_snapshot"))
                    .or_else(|| value.get("generation"))
            })
            .cloned(),
        input_status: "missing".to_string(),
        generation_inputs: Vec::new(),
        result: generated_image_result(&image),
        warnings: Vec::new(),
    }
}

pub(in crate::project_dependency_graph) fn generated_image_media_sources(
    image: &generated_image::Model,
) -> Vec<AssetLineageRef> {
    let Some(metadata) = image.metadata_json.as_ref() else {
        return Vec::new();
    };
    let mut refs = Vec::new();
    if let Some(ids) = metadata
        .get("reference_image_ids")
        .and_then(Value::as_array)
    {
        refs.extend(ids.iter().filter_map(Value::as_i64).map(|id| {
            lineage_ref(
                &format!("generated_image:{id}"),
                "参考图片",
                "image",
                "generated_image",
                Some(id),
            )
        }));
    }
    if let Some(ids) = metadata
        .get("generation_context")
        .and_then(Value::as_object)
        .and_then(|context| context.get("reference_image_ids"))
        .and_then(Value::as_array)
    {
        refs.extend(ids.iter().filter_map(Value::as_i64).map(|id| {
            lineage_ref(
                &format!("generated_image:{id}"),
                "参考图片",
                "image",
                "generated_image",
                Some(id),
            )
        }));
    }
    refs
}
