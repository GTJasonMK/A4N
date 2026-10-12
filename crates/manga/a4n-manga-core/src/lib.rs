//! Pure Manga page layout validation and API projection.

use a4n_api_contract::manga::MangaPagePromptSummary;
use a4n_domain::manga::{
    MANGA_PAGE_PIPELINE_STATE_V1, MangaPageGenerationLayoutV3, MangaPageGenerationLayoutV4,
    MangaPageGenerationSchemaVersion, MangaPagePipelineStage, MangaPagePipelineStateV1,
    MangaPagePipelineStatus, MangaPagePromptArtifactMetadata, MangaPagePromptRecord,
    MangaPipelineArtifactRecord, MangaPipelineArtifactStage, MangaPortraitReference,
};
use a4n_domain::source_spans::{ChapterSourceContext, source_metadata};
use serde_json::{Value, json};

pub fn manga_reference_assets_json<P: MangaPortraitReference>(refs: &[P]) -> Value {
    json!({
        "character_portraits": refs.iter().map(|portrait| json!({
            "profile_id": portrait.profile_id(),
            "portrait_id": portrait.portrait_id(),
            "character_name": portrait.character_name(),
            "image_path": portrait.image_path(),
            "prompt": portrait.prompt(),
            "portrait_type": portrait.portrait_type(),
            "expression": portrait.expression(),
            "pose": portrait.pose(),
            "is_primary": true,
        })).collect::<Vec<_>>(),
    })
}

pub fn manga_pipeline_fingerprint<P: MangaPortraitReference>(
    content: &str,
    character_portraits: &[P],
) -> Value {
    json!({
        "authoring_contract": a4n_domain::media_authoring::MEDIA_AUTHORING_PIPELINE_VERSION,
        "source": source_metadata(ChapterSourceContext { content }),
        "reference_assets": manga_reference_assets_json(character_portraits),
    })
}

pub fn manga_page_generation_layout(
    prompt_id: i64,
    page_number: i32,
    layout: Option<&Value>,
) -> Result<MangaPageGenerationLayoutV3, String> {
    let page_number = manga_page_number(prompt_id, page_number)?;
    let (value, schema_version) = manga_page_layout_value(prompt_id, layout)?;
    match schema_version {
        "manga_page_generation_v4" => {
            let layout: MangaPageGenerationLayoutV4 = serde_json::from_value(value.clone())
                .map_err(|error| {
                    format!("漫画页面提示词 #{prompt_id} 的 v4 layout 无效: {error}")
                })?;
            layout.validate(page_number).map_err(|error| {
                format!("漫画页面提示词 #{prompt_id} 的 v4 layout 无效: {error}")
            })?;
            let page_plan = layout
                .pipeline
                .storyboard
                .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 尚未完成 storyboard"))?;
            let drawing_prompt = layout
                .pipeline
                .drawing_prompt
                .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 尚未完成 drawing_prompt"))?;
            Ok(MangaPageGenerationLayoutV3 {
                schema_version: MangaPageGenerationSchemaVersion::V3,
                page_plan,
                page_plan_ref: layout.page_plan_ref,
                drawing_prompt,
                reference_bindings: layout.reference_bindings,
                prompt_fingerprint: layout.prompt_fingerprint,
                source: layout.source,
            })
        }
        "manga_page_generation_v3" => {
            let layout: MangaPageGenerationLayoutV3 = serde_json::from_value(value.clone())
                .map_err(|error| {
                    format!("漫画页面提示词 #{prompt_id} 的 v3 layout 无效: {error}")
                })?;
            layout.validate(page_number).map_err(|error| {
                format!("漫画页面提示词 #{prompt_id} 的 v3 layout 无效: {error}")
            })?;
            Ok(layout)
        }
        other => Err(unsupported_manga_page_layout(prompt_id, other)),
    }
}

pub fn manga_page_pipeline_layout(
    prompt_id: i64,
    page_number: i32,
    layout: Option<&Value>,
) -> Result<MangaPageGenerationLayoutV4, String> {
    let page_number = manga_page_number(prompt_id, page_number)?;
    let (value, schema_version) = manga_page_layout_value(prompt_id, layout)?;
    manga_page_pipeline_layout_with_schema(
        &format!("漫画页面提示词 #{prompt_id}"),
        page_number,
        value,
        schema_version,
    )
}

/// Parses a stored layout value when the page prompt record is not available.
pub fn manga_page_pipeline_layout_from_value(
    page_number: u32,
    value: &Value,
) -> Result<MangaPageGenerationLayoutV4, String> {
    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("漫画第 {page_number} 页的 schema_version 无效"))?;
    manga_page_pipeline_layout_with_schema(
        &format!("漫画第 {page_number} 页"),
        page_number,
        value,
        schema_version,
    )
}

fn manga_page_pipeline_layout_with_schema(
    label: &str,
    page_number: u32,
    value: &Value,
    schema_version: &str,
) -> Result<MangaPageGenerationLayoutV4, String> {
    match schema_version {
        "manga_page_generation_v4" => {
            let layout: MangaPageGenerationLayoutV4 = serde_json::from_value(value.clone())
                .map_err(|error| format!("{label} 的 v4 layout 无效: {error}"))?;
            layout
                .validate(page_number)
                .map_err(|error| format!("{label} 的 v4 layout 无效: {error}"))?;
            Ok(layout)
        }
        "manga_page_generation_v3" => {
            let layout: MangaPageGenerationLayoutV3 = serde_json::from_value(value.clone())
                .map_err(|error| format!("{label} 的 v3 layout 无效: {error}"))?;
            layout
                .validate(page_number)
                .map_err(|error| format!("{label} 的 v3 layout 无效: {error}"))?;
            let page_id = format!("page-{page_number:04}");
            let pipeline = MangaPagePipelineStateV1 {
                schema_version: MANGA_PAGE_PIPELINE_STATE_V1.into(),
                page_id: page_id.clone(),
                status: MangaPagePipelineStatus::Pending,
                stage: MangaPagePipelineStage::Adaptation,
                attempt: 1,
                source_fingerprint: layout.source.content_sha256.clone(),
                stage_fingerprints: Default::default(),
                page_content: None,
                page_adaptation: None,
                storyboard: Some(layout.page_plan),
                reference_discovery: None,
                drawing_prompt: Some(layout.drawing_prompt),
                image: None,
                error: None,
            };
            let upgraded = MangaPageGenerationLayoutV4 {
                schema_version: MangaPageGenerationSchemaVersion::V4,
                page_id,
                page_number,
                page_plan_ref: layout.page_plan_ref,
                pipeline,
                reference_bindings: layout.reference_bindings,
                prompt_fingerprint: layout.prompt_fingerprint,
                source: layout.source,
            };
            upgraded
                .validate(page_number)
                .map_err(|error| format!("{label} 的 v3 layout 无法升级: {error}"))?;
            Ok(upgraded)
        }
        other => Err(format!("{label} 的 schema_version 不支持: {other}")),
    }
}

fn manga_page_number(prompt_id: i64, page_number: i32) -> Result<u32, String> {
    u32::try_from(page_number)
        .ok()
        .filter(|page_number| *page_number > 0)
        .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 的 page_number 必须大于 0"))
}

fn manga_page_layout_value<'a>(
    prompt_id: i64,
    layout: Option<&'a Value>,
) -> Result<(&'a Value, &'a str), String> {
    let layout = layout.ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 缺少 layout"))?;
    let schema_version = layout
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 的 schema_version 无效"))?;
    Ok((layout, schema_version))
}

fn unsupported_manga_page_layout(prompt_id: i64, schema_version: &str) -> String {
    format!("漫画页面提示词 #{prompt_id} 的 schema_version 不支持: {schema_version}")
}

pub fn manga_page_prompt_stale_stages(
    prompt_id: i64,
    page_number: i32,
    artifacts: &[MangaPipelineArtifactRecord],
) -> Result<Vec<MangaPipelineArtifactStage>, String> {
    manga_page_prompt_stale_stages_from_metadata(
        prompt_id,
        page_number,
        artifacts
            .iter()
            .map(|artifact| (artifact.stage, artifact.page_number, artifact.stale)),
    )
}

pub fn manga_page_prompt_stale_stages_from_list_metadata(
    prompt_id: i64,
    page_number: i32,
    artifacts: &[MangaPagePromptArtifactMetadata],
) -> Result<Vec<MangaPipelineArtifactStage>, String> {
    manga_page_prompt_stale_stages_from_metadata(
        prompt_id,
        page_number,
        artifacts
            .iter()
            .map(|artifact| (artifact.stage, artifact.page_number, artifact.stale)),
    )
}

fn manga_page_prompt_stale_stages_from_metadata(
    prompt_id: i64,
    page_number: i32,
    artifacts: impl Iterator<Item = (MangaPipelineArtifactStage, Option<u32>, bool)>,
) -> Result<Vec<MangaPipelineArtifactStage>, String> {
    let page_number = manga_page_number(prompt_id, page_number)?;
    Ok(artifacts
        .filter_map(|(stage, artifact_page, stale)| {
            (stale && artifact_page == Some(page_number) && stage.page_scoped()).then_some(stage)
        })
        .collect())
}

pub fn manga_page_image_generation_layout(
    prompt_id: i64,
    page_number: i32,
    layout: Option<&Value>,
) -> Result<MangaPageGenerationLayoutV4, String> {
    let layout = manga_page_pipeline_layout(prompt_id, page_number, layout).map_err(|error| {
        format!(
            "漫画页面提示词 #{prompt_id} 不能直接生成图片：请先按页面内容、单页改编计划、单页分镜、整页提示词的顺序运行漫画流水线（{error}）"
        )
    })?;
    if layout.pipeline.stage != MangaPagePipelineStage::Image {
        return Err(format!(
            "漫画页面提示词 #{prompt_id} 不能跳过阶段直接生成图片：当前阶段为 {}，请先按页面内容、单页改编计划、单页分镜、整页提示词的顺序完成前置阶段",
            layout.pipeline.stage.key()
        ));
    }
    Ok(layout)
}

pub fn manga_page_prompt_summary(
    prompt: &MangaPagePromptRecord,
) -> Result<MangaPagePromptSummary, String> {
    let (_, schema_version) = manga_page_layout_value(prompt.id, prompt.layout.as_ref())?;
    match schema_version {
        "manga_page_generation_v4" => {
            let layout =
                manga_page_pipeline_layout(prompt.id, prompt.page_number, prompt.layout.as_ref())?;
            Ok(MangaPagePromptSummary {
                id: prompt.id,
                chapter_id: prompt.chapter_id,
                project_id: prompt.project_id,
                page_number: prompt.page_number,
                page_id: Some(layout.page_id),
                image_prompt: prompt.image_prompt.clone(),
                dialogue_text: prompt.dialogue_text.clone(),
                page_content: layout.pipeline.page_content,
                page_adaptation: layout.pipeline.page_adaptation,
                page_plan: layout.pipeline.storyboard,
                reference_discovery: layout.pipeline.reference_discovery.clone(),
                drawing_prompt: layout.pipeline.drawing_prompt,
                stale_stages: Vec::new(),
                orphaned: false,
                system_reference_images: layout
                    .pipeline
                    .reference_discovery
                    .map_or_else(Vec::new, |discovery| discovery.references),
                reference_bindings: layout.reference_bindings,
                source: layout.source,
                generated_image_id: prompt.generated_image_id,
                status: prompt.status.clone(),
                pipeline_stage: Some(layout.pipeline.stage),
                pipeline_status: Some(layout.pipeline.status),
                pipeline_error: layout.pipeline.error,
                created_at: prompt.created_at.to_string(),
                updated_at: prompt.updated_at.to_string(),
            })
        }
        "manga_page_generation_v3" => {
            let layout = manga_page_generation_layout(
                prompt.id,
                prompt.page_number,
                prompt.layout.as_ref(),
            )?;
            Ok(MangaPagePromptSummary {
                id: prompt.id,
                chapter_id: prompt.chapter_id,
                project_id: prompt.project_id,
                page_number: prompt.page_number,
                page_id: None,
                image_prompt: prompt.image_prompt.clone(),
                dialogue_text: prompt.dialogue_text.clone(),
                page_content: None,
                page_adaptation: None,
                page_plan: Some(layout.page_plan),
                reference_discovery: None,
                drawing_prompt: Some(layout.drawing_prompt),
                stale_stages: Vec::new(),
                orphaned: false,
                system_reference_images: Vec::new(),
                reference_bindings: layout.reference_bindings,
                source: layout.source,
                generated_image_id: prompt.generated_image_id,
                status: prompt.status.clone(),
                pipeline_stage: None,
                pipeline_status: None,
                pipeline_error: None,
                created_at: prompt.created_at.to_string(),
                updated_at: prompt.updated_at.to_string(),
            })
        }
        other => Err(unsupported_manga_page_layout(prompt.id, other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::manga::{MangaPagePipelineStage, MangaPagePipelineStatus};
    use chrono::NaiveDateTime;
    use serde_json::json;

    #[test]
    fn rejects_missing_layout() {
        let now =
            NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let prompt = MangaPagePromptRecord {
            id: 11,
            chapter_id: 22,
            project_id: 33,
            page_number: 1,
            prompt_type: "page_generation".to_string(),
            image_prompt: "整页提示词".to_string(),
            dialogue_text: None,
            characters_in_frame: None,
            layout: None,
            generated_image_id: None,
            status: "pending".to_string(),
            created_at: now,
            updated_at: now,
        };
        assert_eq!(
            manga_page_prompt_summary(&prompt).unwrap_err(),
            "漫画页面提示词 #11 缺少 layout"
        );
    }

    #[test]
    fn summary_preserves_v4_cancelled_pipeline_and_source() {
        let now =
            NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let prompt = MangaPagePromptRecord {
            id: 11,
            chapter_id: 22,
            project_id: 33,
            prompt_type: "page_generation".to_string(),
            page_number: 1,
            image_prompt: "整页提示词".to_string(),
            dialogue_text: Some("旁白".to_string()),
            characters_in_frame: None,
            layout: Some(json!({
                "schema_version": "manga_page_generation_v4",
                "page_id": "page-0001",
                "page_number": 1,
                "page_plan_ref": null,
                "pipeline": {
                    "schema_version": "manga_page_pipeline_state_v1",
                    "page_id": "page-0001",
                    "status": "cancelled",
                    "stage": "adaptation",
                    "attempt": 1,
                    "source_fingerprint": "content-sha256",
                    "stage_fingerprints": {},
                    "page_content": {
                        "summary": "本页内容",
                        "source_span_ids": [],
                        "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
                        "action": "角色抬头",
                        "reaction": null,
                        "dialogue_intent": null,
                        "entry_state": "平静",
                        "exit_state": "紧张"
                    },
                    "page_adaptation": null,
                    "storyboard": null,
                    "drawing_prompt": null,
                    "image": null,
                    "error": {
                        "stage": "adaptation",
                        "code": "cancelled",
                        "message": "用户取消",
                        "retryable": false
                    }
                },
                "reference_bindings": [],
                "prompt_fingerprint": {},
                "source": {
                    "root_project_id": 33,
                    "source_chapter_id": 22,
                    "source_snapshot_id": null,
                    "source_snapshot_chapter_id": null,
                    "source_revision_key": "live:22:content-sha256",
                    "content_sha256": "content-sha256",
                    "adaptation_plan_id": 3,
                    "adaptation_plan_revision": 1,
                    "adaptation_plan_sha256": "plan-sha256"
                }
            })),
            generated_image_id: None,
            status: "cancelled".to_string(),
            created_at: now,
            updated_at: now,
        };

        let summary = manga_page_prompt_summary(&prompt).unwrap();
        assert_eq!(summary.page_id.as_deref(), Some("page-0001"));
        assert_eq!(
            summary.pipeline_stage,
            Some(MangaPagePipelineStage::Adaptation)
        );
        assert_eq!(
            summary.pipeline_status,
            Some(MangaPagePipelineStatus::Cancelled)
        );
        assert_eq!(
            summary
                .pipeline_error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some("cancelled")
        );
        assert_eq!(summary.source.root_project_id, 33);
        assert_eq!(summary.source.source_chapter_id, 22);
        assert_eq!(summary.page_content.as_ref().unwrap().summary, "本页内容");
    }
}
