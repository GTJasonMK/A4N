use super::generic::{lineage_ref, optional_prompt_sources, prompt_source};
use super::json_records::{db_record_result_with_related, db_related_record};
use crate::project_dependency_graph::AssetLineageRecord;
use a4n_db_models::models::{ai::generated_image, chapter::chapter_manga_prompt};
use a4n_domain::GeneratedMediaReference;

pub(in crate::project_dependency_graph) fn manga_prompt_record(
    prompt: chapter_manga_prompt::Model,
    image: Option<&generated_image::Model>,
) -> AssetLineageRecord {
    let mut warnings = Vec::new();
    if prompt.generated_image_id.is_some() && image.is_none() {
        warnings.push("提示词已记录生成图片 ID，但未找到对应图片记录。".to_string());
    }
    AssetLineageRecord {
        id: format!("chapter_manga_prompt:{}", prompt.id),
        label: format!("第 {} 章 第 {} 页", prompt.chapter_id, prompt.page_number),
        asset_kind: "chapter_manga_prompt".to_string(),
        asset_id: Some(prompt.id),
        status: Some(prompt.status.clone()),
        origin: "llm_generated".to_string(),
        source_refs: vec![lineage_ref(
            &format!("chapter:{}", prompt.chapter_id),
            "章节正文",
            "text",
            "chapter_content",
            Some(prompt.chapter_id),
        )],
        prompt_sources: optional_prompt_sources(vec![
            prompt_source(
                &format!("chapter_manga_prompt:{}", prompt.id),
                "manga_image_prompt",
                Some(&prompt.image_prompt),
                Some(prompt.id),
                None,
            ),
            prompt_source(
                &format!("chapter_manga_dialogue:{}", prompt.id),
                "dialogue_text",
                prompt.dialogue_text.as_deref(),
                Some(prompt.id),
                None,
            ),
        ]),
        media_sources: image
            .map(|image| {
                vec![lineage_ref(
                    &GeneratedMediaReference::Image(image.id).to_string(),
                    "生成图片",
                    "image",
                    "generated_image",
                    Some(image.id),
                )]
            })
            .unwrap_or_default(),
        request_snapshot: image
            .and_then(|image| image.metadata_json.as_ref())
            .and_then(|value| value.get("provider_request_snapshot").cloned()),
        input_status: "missing".to_string(),
        generation_inputs: Vec::new(),
        result: db_record_result_with_related(
            "chapter_manga_prompt",
            &prompt,
            image
                .map(|image| db_related_record("generated_image", image))
                .into_iter()
                .collect(),
        ),
        warnings,
    }
}
