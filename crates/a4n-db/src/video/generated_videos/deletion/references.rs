use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use std::collections::BTreeMap;

fn clear_source_video_metadata_references(metadata: &mut Value) {
    let Some(obj) = metadata.as_object_mut() else {
        return;
    };
    obj.remove("source_video_id");
    if let Some(trim) = obj.get_mut("trim")
        && let Some(trim_obj) = trim.as_object_mut()
    {
        trim_obj.remove("source_video_id");
    }
}

pub(crate) async fn clear_generated_video_references_record<C>(
    db: &C,
    videos: &[generated_video::Model],
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let video_projects = videos
        .iter()
        .map(|video| (video.id, video.project_id))
        .collect::<BTreeMap<_, _>>();
    if video_projects.is_empty() {
        return Ok(());
    }
    let video_ids = video_projects.keys().copied().collect::<Vec<_>>();
    let now = Utc::now().naive_utc();

    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::GeneratedVideoId.is_in(video_ids.iter().copied()))
        .all(db)
        .await?;
    for prompt in prompts {
        let replacement = generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.eq(prompt.project_id))
            .filter(generated_video::Column::ChapterId.eq(prompt.chapter_id))
            .filter(generated_video::Column::Status.eq("completed"))
            .all(db)
            .await?
            .into_iter()
            .filter(|video| !video_ids.contains(&video.id))
            .filter(|video| {
                video
                    .metadata_json
                    .as_ref()
                    .and_then(|metadata| metadata.get("prompt_id"))
                    .and_then(Value::as_i64)
                    == Some(prompt.id)
            })
            .max_by_key(|video| (video.updated_at, video.id));
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        active.generated_video_id = Set(replacement.as_ref().map(|video| video.id));
        active.status = Set(if replacement.is_some() {
            "completed".to_string()
        } else {
            "pending".to_string()
        });
        active.updated_at = Set(now);
        active.update(db).await?;
    }

    let project_ids = video_projects.values().copied().collect::<Vec<_>>();
    let continuity_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(project_ids))
        .all(db)
        .await?;
    for prompt in continuity_prompts {
        let source_video_id = prompt
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("continuity_source"))
            .and_then(|source| source.get("video_id"))
            .and_then(Value::as_i64);
        if source_video_id.is_none_or(|source_id| !video_ids.contains(&source_id)) {
            continue;
        }
        let mut metadata = prompt.metadata_json.clone().unwrap_or(Value::Null);
        let Some(metadata) = metadata.as_object_mut() else {
            continue;
        };
        metadata.insert("continuity_stale".to_string(), Value::Bool(true));
        metadata.insert(
            "continuity_stale_reason".to_string(),
            Value::String("continuity_source_deleted".to_string()),
        );
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        active.metadata_json = Set(Some(Value::Object(metadata.clone())));
        active.updated_at = Set(now);
        active.update(db).await?;
    }

    let child_videos = generated_video::Entity::find()
        .filter(generated_video::Column::SourceVideoId.is_in(video_ids.iter().copied()))
        .all(db)
        .await?;
    for video in child_videos {
        let mut metadata = video.metadata_json.clone();
        let mut active: generated_video::ActiveModel = video.into();
        active.source_video_id = Set(None);
        if let Some(mut metadata) = metadata.take() {
            clear_source_video_metadata_references(&mut metadata);
            active.metadata_json = Set(Some(metadata));
        }
        active.updated_at = Set(now);
        active.update(db).await?;
    }

    for (video_id, project_id) in video_projects {
        crate::canvas::mark_canvas_generated_video_refs_missing(db, project_id, video_id).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn source_video_metadata_cleanup_preserves_unrelated_fields() {
        let mut metadata = json!({
            "source_video_id": 41,
            "provider": "test",
            "trim": {
                "source_video_id": 41,
                "start_secs": 1.5
            }
        });

        clear_source_video_metadata_references(&mut metadata);

        assert_eq!(
            metadata,
            json!({
                "provider": "test",
                "trim": {
                    "start_secs": 1.5
                }
            })
        );
    }
}
