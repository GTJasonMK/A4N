use crate::project::dependency_graph::AssetLineageRecord;
use crate::project::dependency_graph::details::records::{
    db_record_result_with_related, db_related_record, lineage_ref, optional_prompt_sources,
    prompt_source, video_prompt_sources,
};
use a4n_db_models::models::{ai::generated_video, chapter::chapter_video_prompt};
use a4n_domain::GeneratedMediaReference;
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;

pub(in crate::project::dependency_graph) async fn generated_video_detail_records<C>(
    db: &C,
    project_ids: &[i64],
    warnings: &mut Vec<String>,
) -> Result<Vec<AssetLineageRecord>, AppError>
where
    C: ConnectionTrait,
{
    let (prompts_result, videos_result) = futures::join!(
        chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
            .filter(
                chapter_video_prompt::Column::PromptType
                    .eq(chapter_video_prompt::PROMPT_TYPE_VIDEO),
            )
            .all(db),
        generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.is_in(project_ids.iter().copied()))
            .order_by_asc(generated_video::Column::Id)
            .all(db),
    );
    let prompts = prompts_result.map_err(AppError::from)?;
    let prompt_by_video = prompts
        .iter()
        .filter_map(|prompt| prompt.generated_video_id.map(|id| (id, prompt)))
        .collect::<HashMap<_, _>>();
    let prompt_by_id = prompts
        .iter()
        .map(|prompt| (prompt.id, prompt))
        .collect::<HashMap<_, _>>();
    let videos = videos_result.map_err(AppError::from)?;
    Ok(videos
        .into_iter()
        .map(|video| {
            let prompt = prompt_by_video.get(&video.id).copied().or_else(|| {
                a4n_domain::video::generated_video_prompt_id(video.metadata_json.as_ref())
                    .and_then(|id| prompt_by_id.get(&id).copied())
            });
            if prompt.is_none() {
                warnings.push(format!(
                    "生成视频 #{} 未找到 chapter_video_prompt 来源。",
                    video.id
                ));
            }
            let mut prompt_sources = Vec::new();
            if let Some(prompt) = prompt {
                prompt_sources = video_prompt_sources(prompt);
            }
            prompt_sources.extend(optional_prompt_sources(vec![
                prompt_source(
                    &format!("generated_video_prompt:{}", video.id),
                    "stored_generated_video_prompt",
                    Some(&video.prompt),
                    Some(video.id),
                    None,
                ),
                prompt_source(
                    &format!("generated_video_negative_prompt:{}", video.id),
                    "video_negative_prompt",
                    video.negative_prompt.as_deref(),
                    Some(video.id),
                    None,
                ),
            ]));
            let mut media_sources = Vec::new();
            if let Some(id) = video.source_image_id {
                media_sources.push(lineage_ref(
                    &GeneratedMediaReference::Image(id).to_string(),
                    "源图片",
                    "image",
                    "generated_image",
                    Some(id),
                ));
            }
            if let Some(id) = video.source_video_id {
                media_sources.push(lineage_ref(
                    &GeneratedMediaReference::Video(id).to_string(),
                    "源视频",
                    "video",
                    "generated_video",
                    Some(id),
                ));
            }
            let request_snapshot = video
                .metadata_json
                .as_ref()
                .and_then(|value| value.get("provider_request_snapshot").cloned());
            let input_complete =
                prompt.is_some() || !media_sources.is_empty() || request_snapshot.is_some();
            AssetLineageRecord {
                id: GeneratedMediaReference::Video(video.id).to_string(),
                label: format!("生成视频 #{}", video.id),
                asset_kind: "generated_video".to_string(),
                asset_id: Some(video.id),
                status: Some(video.status.clone()),
                origin: "video_generated".to_string(),
                source_refs: prompt
                    .map(|prompt| {
                        vec![lineage_ref(
                            &format!("chapter_video_prompt:{}", prompt.id),
                            "视频生成提示词",
                            "prompt",
                            "chapter_video_prompt",
                            Some(prompt.id),
                        )]
                    })
                    .unwrap_or_default(),
                prompt_sources,
                media_sources,
                request_snapshot,
                input_status: if input_complete {
                    "complete"
                } else {
                    "missing"
                }
                .to_string(),
                generation_inputs: Vec::new(),
                result: db_record_result_with_related(
                    "generated_video",
                    &video,
                    prompt
                        .map(|prompt| db_related_record("chapter_video_prompt", prompt))
                        .into_iter()
                        .collect(),
                ),
                warnings: if prompt.is_none() {
                    vec!["未找到视频提示词来源，只能展示视频记录内保存的提示词。".to_string()]
                } else {
                    Vec::new()
                },
            }
        })
        .collect())
}
