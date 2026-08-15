//! Restore adopted segment videos only when their current execution identity is exact.

use a4n_core::{
    models::{ai::generated_video, chapter::chapter_video_prompt},
    video_execution::{
        VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1,
        video_segment_recipes_from_metadata,
    },
};
#[cfg(test)]
use sea_orm::prelude::DateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Set,
};
use sea_orm_migration::prelude::*;
use serde_json::{Map, Value};

#[derive(DeriveMigrationName)]
pub struct Migration;

const EXECUTION_IDENTITY_FIELDS: &[&str] = &[
    "contract_epoch",
    "shooting_script_identity",
    "projection_sha256",
    "segment_plan_sha256",
    "unit_plan_sha256",
    "source_content_sha256",
    "style_sha256",
    "duration_secs",
    "aspect_ratio",
];

fn execution_identity(metadata: Option<&Value>) -> Option<&Map<String, Value>> {
    metadata?
        .as_object()?
        .get("video_execution_identity")?
        .as_object()
}

fn identity_matches_segment(
    metadata: Option<&Value>,
    plan_identity: &Map<String, Value>,
    plan_id: i64,
    recipe: &VideoSegmentExecutionRecipeV1,
    fingerprint: &str,
) -> bool {
    let Some(identity) = execution_identity(metadata) else {
        return false;
    };
    EXECUTION_IDENTITY_FIELDS
        .iter()
        .all(|field| identity.get(*field) == plan_identity.get(*field))
        && identity
            .get("segment_plan_prompt_id")
            .and_then(Value::as_i64)
            == Some(plan_id)
        && identity.get("segment_number").and_then(Value::as_i64)
            == Some(i64::from(recipe.segment_number))
        && identity
            .get("segment_recipe_fingerprint")
            .and_then(Value::as_str)
            == Some(fingerprint)
}

fn video_matches_recipe(
    video: &generated_video::Model,
    plan: &chapter_video_prompt::Model,
    plan_identity: &Map<String, Value>,
    recipe: &VideoSegmentExecutionRecipeV1,
    fingerprint: &str,
    final_prompts: &[chapter_video_prompt::Model],
) -> bool {
    if video.project_id != plan.project_id
        || video.chapter_id != Some(plan.chapter_id)
        || video.status != "completed"
        || video
            .file_path
            .as_deref()
            .is_none_or(|path| path.trim().is_empty())
        || video.workflow_mode.as_deref() != Some(recipe.workflow_mode.as_str())
    {
        return false;
    }
    if identity_matches_segment(
        video.metadata_json.as_ref(),
        plan_identity,
        plan.id,
        recipe,
        fingerprint,
    ) {
        return true;
    }
    final_prompts.iter().any(|prompt| {
        prompt.generated_video_id == Some(video.id)
            && prompt.prompt_type == "video_prompt"
            && prompt.workflow_mode == recipe.workflow_mode.as_str()
            && identity_matches_segment(
                prompt.metadata_json.as_ref(),
                plan_identity,
                plan.id,
                recipe,
                fingerprint,
            )
    })
}

pub(crate) async fn repair_segment_recipe_adoptions<C>(db: &C) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let plans = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("shared_segment_plan"))
        .filter(chapter_video_prompt::Column::Status.eq("completed"))
        .all(db)
        .await?;
    for plan in plans {
        let Some(plan_identity) = execution_identity(plan.metadata_json.as_ref()).cloned() else {
            continue;
        };
        let mut recipes = video_segment_recipes_from_metadata(plan.metadata_json.as_ref())
            .map_err(|error| {
                DbErr::Custom(format!("片段规划 #{} 执行方案无效：{error}", plan.id))
            })?;
        if recipes.is_empty() {
            continue;
        }
        let videos = generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.eq(plan.project_id))
            .filter(generated_video::Column::ChapterId.eq(plan.chapter_id))
            .filter(generated_video::Column::Status.eq("completed"))
            .all(db)
            .await?;
        let final_prompts = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(plan.project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(plan.chapter_id))
            .filter(chapter_video_prompt::Column::PromptType.eq("video_prompt"))
            .all(db)
            .await?;
        let mut changed = false;
        for recipe in &mut recipes {
            let fingerprint = recipe.generation_fingerprint().map_err(|error| {
                DbErr::Custom(format!(
                    "片段规划 #{} 片段 {} 指纹无效：{error}",
                    plan.id, recipe.segment_number
                ))
            })?;
            if let Some(video_id) = recipe.adopted_video_id {
                let current_is_valid = videos
                    .iter()
                    .find(|video| video.id == video_id)
                    .is_some_and(|video| {
                        video_matches_recipe(
                            video,
                            &plan,
                            &plan_identity,
                            recipe,
                            &fingerprint,
                            &final_prompts,
                        )
                    });
                if current_is_valid {
                    continue;
                }
                recipe.adopted_video_id = None;
                changed = true;
            }
            let candidate = videos
                .iter()
                .filter(|video| {
                    video_matches_recipe(
                        video,
                        &plan,
                        &plan_identity,
                        recipe,
                        &fingerprint,
                        &final_prompts,
                    )
                })
                .max_by_key(|video| (video.updated_at, video.id));
            if let Some(video) = candidate {
                recipe.adopted_video_id = Some(video.id);
                changed = true;
            }
        }
        if !changed {
            continue;
        }
        let mut metadata = plan
            .metadata_json
            .clone()
            .ok_or_else(|| DbErr::Custom(format!("片段规划 #{} metadata_json 缺失", plan.id)))?;
        metadata
            .as_object_mut()
            .ok_or_else(|| DbErr::Custom(format!("片段规划 #{} metadata_json 不是对象", plan.id)))?
            .insert(
                VIDEO_SEGMENT_RECIPES_METADATA_KEY.to_string(),
                serde_json::to_value(recipes).map_err(|error| {
                    DbErr::Custom(format!("序列化片段规划 #{} 执行方案失败：{error}", plan.id))
                })?,
            );
        let updated_at = plan.updated_at;
        let mut active: chapter_video_prompt::ActiveModel = plan.into();
        active.metadata_json = Set(Some(metadata));
        active.updated_at = Set(updated_at);
        active.update(db).await?;
    }
    Ok(())
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N segment_recipe_adoption_repair migration only supports PostgreSQL".to_string(),
            ));
        }
        repair_segment_recipe_adoptions(manager.get_connection()).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "segment_recipe_adoption_repair 不可逆：精确恢复的采用状态不会自动清空".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::{chapter::chapter, project::novel_project};
    use sea_orm::{ActiveModelTrait, EntityTrait};
    use serde_json::json;

    fn identity(
        plan_id: Option<i64>,
        segment_number: Option<i32>,
        fingerprint: Option<&str>,
    ) -> Value {
        json!({
            "contract_epoch": "video_execution_contract_v5",
            "shooting_script_identity": "script",
            "projection_sha256": "projection",
            "segment_plan_sha256": "segment-plan",
            "unit_plan_sha256": "unit-plan",
            "source_content_sha256": "source",
            "style_sha256": "style",
            "duration_secs": 6,
            "aspect_ratio": "16:9",
            "segment_plan_prompt_id": plan_id,
            "segment_number": segment_number,
            "segment_recipe_fingerprint": fingerprint,
        })
    }

    #[test]
    fn adoption_identity_rejects_other_plan() {
        let recipe: VideoSegmentExecutionRecipeV1 = serde_json::from_value(json!({
            "schema_version": "video_segment_execution_recipe_v1",
            "segment_number": 1,
            "workflow_mode": "chat_video",
            "style": "cinematic",
            "prompt_output_language": "zh_cn",
            "duration_secs": 6,
            "duration_secs_options": [6],
            "aspect_ratio": "16:9",
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "guidance_mode": "none",
            "guidance_reference_image_ids": [],
            "continuity_source_video_id": null,
            "reference_media": null,
            "adopted_video_id": null
        }))
        .unwrap();
        let fingerprint = recipe.generation_fingerprint().unwrap();
        let plan_identity = identity(None, None, None);
        let plan_identity = plan_identity.as_object().unwrap();
        let current = json!({
            "video_execution_identity": identity(Some(42), Some(1), Some(&fingerprint))
        });
        let old = json!({
            "video_execution_identity": identity(Some(41), Some(1), Some(&fingerprint))
        });

        assert!(identity_matches_segment(
            Some(&current),
            plan_identity,
            42,
            &recipe,
            &fingerprint,
        ));
        assert!(!identity_matches_segment(
            Some(&old),
            plan_identity,
            42,
            &recipe,
            &fingerprint,
        ));
    }

    #[tokio::test]
    async fn repair_replaces_legacy_adoption_with_complete_identity_match() {
        let db = crate::test_support::connect_test_database().await;
        let now = DateTime::default();
        let project = novel_project::ActiveModel {
            user_id: Set(1),
            title: Set("adoption repair".to_string()),
            description: Set(String::new()),
            project_type: Set("novel_long".to_string()),
            project_kind: Set("text".to_string()),
            source_mode: Set("self".to_string()),
            root_project_id: Set(None),
            source_snapshot_id: Set(None),
            source_sync_status: Set("current".to_string()),
            status: Set("draft".to_string()),
            lifecycle_gate: Set("active".to_string()),
            target_chapters: Set(1),
            current_chapter: Set(0),
            total_chapters: Set(Some(1)),
            chapters_per_part: Set(None),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            raw_blueprint: Set(None),
            import_analysis_progress: Set(None),
            export_config: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let chapter = chapter::ActiveModel {
            project_id: Set(project.id),
            outline_id: Set(None),
            chapter_number: Set(1),
            title: Set(Some("chapter".to_string())),
            content: Set("content".to_string()),
            word_count: Set(7),
            status: Set("draft".to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let recipe_json = json!({
            "schema_version": "video_segment_execution_recipe_v1",
            "segment_number": 1,
            "workflow_mode": "chat_video",
            "style": "cinematic",
            "prompt_output_language": "zh_cn",
            "duration_secs": 6,
            "duration_secs_options": [6],
            "aspect_ratio": "16:9",
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "guidance_mode": "none",
            "guidance_reference_image_ids": [],
            "continuity_source_video_id": null,
            "reference_media": null,
            "adopted_video_id": null
        });
        let recipe: VideoSegmentExecutionRecipeV1 =
            serde_json::from_value(recipe_json.clone()).unwrap();
        let fingerprint = recipe.generation_fingerprint().unwrap();
        let plan = chapter_video_prompt::ActiveModel {
            project_id: Set(project.id),
            chapter_id: Set(chapter.id),
            style: Set(Some("cinematic".to_string())),
            prompt_type: Set("frame_segment_plan".to_string()),
            workflow_mode: Set("shared_segment_plan".to_string()),
            shot_number: Set(1),
            title: Set(Some("plan".to_string())),
            scene_summary: Set(None),
            visual_prompt: Set("{}".to_string()),
            motion_prompt: Set(None),
            negative_prompt: Set(None),
            duration_secs: Set(6),
            aspect_ratio: Set(Some("16:9".to_string())),
            reference_image_ids_json: Set(None),
            status: Set("completed".to_string()),
            generated_video_id: Set(None),
            metadata_json: Set(Some(json!({
                "video_execution_identity": identity(None, None, None),
                "segment_recipes": [recipe_json]
            }))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let video = |video_identity: Value| generated_video::ActiveModel {
            project_id: Set(project.id),
            chapter_id: Set(Some(chapter.id)),
            video_type: Set("chapter_video".to_string()),
            workflow_mode: Set(Some("chat_video".to_string())),
            file_path: Set(Some("video.mp4".to_string())),
            thumbnail_path: Set(None),
            prompt: Set("prompt".to_string()),
            negative_prompt: Set(None),
            model: Set("test".to_string()),
            width: Set(1280),
            height: Set(720),
            duration_secs: Set(6.0),
            fps: Set(24),
            file_size: Set(Some(1)),
            source_video_id: Set(None),
            source_image_id: Set(None),
            status: Set("completed".to_string()),
            provider_job_id: Set(None),
            error_message: Set(None),
            metadata_json: Set(Some(json!({
                "video_execution_identity": video_identity
            }))),
            is_favorite: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let stale = video(identity(Some(plan.id + 1), Some(1), Some(&fingerprint)))
            .insert(&db)
            .await
            .unwrap();
        let candidate = video(identity(Some(plan.id), Some(1), Some(&fingerprint)))
            .insert(&db)
            .await
            .unwrap();

        let mut plan_metadata = plan.metadata_json.clone().unwrap();
        plan_metadata["segment_recipes"][0]["adopted_video_id"] = json!(stale.id);
        let mut active: chapter_video_prompt::ActiveModel = plan.clone().into();
        active.metadata_json = Set(Some(plan_metadata));
        active.update(&db).await.unwrap();

        repair_segment_recipe_adoptions(&db).await.unwrap();

        let stored = chapter_video_prompt::Entity::find_by_id(plan.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            stored.metadata_json.unwrap()["segment_recipes"][0]["adopted_video_id"],
            json!(candidate.id)
        );
    }
}
