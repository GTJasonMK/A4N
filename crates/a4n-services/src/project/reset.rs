use chrono::Utc;
use sea_orm::*;

use a4n_core::error::AppError;
use a4n_core::event::bridge::InternalEvent;
use a4n_core::models::project::{novel_project, project_status_history};
use a4n_core::state::{status::ProjectStatus, validator::StateValidator};

use super::ProjectService;
use super::cleanup::{
    GeneratedImageDeleteScope, TextAnchorDeleteScope, delete_chapter_generation_outputs,
    delete_generation_media_graph, delete_optimization_tree, delete_project_blueprints,
    delete_project_conversations, delete_project_profile_scene_data, delete_project_tasks,
    delete_project_text_anchors,
};
use super::{PROJECT_KIND_GALGAME, PROJECT_KIND_TEXT};

mod character_profiles;

use self::character_profiles::reset_character_profiles_for_project;

#[derive(Debug, Clone)]
pub struct ProjectGeneratedDataResetResult {
    pub project: novel_project::Model,
    pub deleted_galgame_scripts: u64,
    pub deleted_galgame_scenes: u64,
    pub deleted_galgame_lines: u64,
    pub deleted_galgame_choices: u64,
    pub deleted_chapters: u64,
    pub deleted_chapter_outlines: u64,
    pub deleted_part_outlines: u64,
    pub deleted_chapter_derived_rows: u64,
    pub deleted_current_blueprints: u64,
    pub deleted_blueprint_characters: u64,
    pub deleted_blueprint_relationships: u64,
    pub preserved_blueprint_versions: u64,
    pub reset_character_profiles: u64,
    pub deleted_character_profiles: u64,
    pub deleted_character_snapshots: u64,
    pub deleted_character_timeline_rows: u64,
    pub deleted_character_portraits: u64,
    pub deleted_object_profiles: u64,
    pub deleted_foreshadowing: u64,
    pub deleted_scene_profiles: u64,
    pub deleted_scene_occurrences: u64,
    pub deleted_scene_transitions: u64,
    pub deleted_scene_entity_states: u64,
    pub deleted_text_anchors: u64,
    pub deleted_profile_evidence: u64,
    pub deleted_import_reviews: u64,
    pub deleted_conversations: u64,
    pub deleted_conversation_messages: u64,
    pub deleted_optimization_sessions: u64,
    pub deleted_optimization_session_messages: u64,
    pub deleted_optimization_turns: u64,
    pub deleted_optimization_events: u64,
    pub deleted_optimization_patch_sets: u64,
    pub deleted_optimization_patches: u64,
    pub deleted_generated_images: u64,
    pub deleted_generated_videos: u64,
    pub deleted_generation_specs: u64,
    pub deleted_prompt_artifacts: u64,
    pub deleted_media_assets: u64,
    pub deleted_media_provenance: u64,
    pub deleted_media_bindings: u64,
    pub deleted_tasks: u64,
    pub deleted_task_dependencies: u64,
}

impl ProjectService {
    /// 清空项目当前生成态，保留项目主体、蓝图版本与非正文派生角色基础档案。
    pub async fn reset_generated_data(
        &self,
        project_id: i64,
    ) -> Result<ProjectGeneratedDataResetResult, AppError> {
        use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};
        use a4n_core::models::character::foreshadowing_index;
        use a4n_core::models::project::{blueprint_version, import_review_queue};

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let project = novel_project::Entity::find_by_id(project_id)
            .one(&tx)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind == PROJECT_KIND_GALGAME {
            return self.reset_galgame_generated_data_in_tx(tx, project).await;
        }
        if project.project_kind != PROJECT_KIND_TEXT {
            return Err(AppError::InvalidArgument(
                "重置生成态仅支持文本项目".to_string(),
            ));
        }
        let previous_status = StateValidator::parse_status(&project.status)?;
        let now = Utc::now().naive_utc();

        let optimization_cleanup = delete_optimization_tree(&tx, project_id).await?;
        let preserved_blueprint_versions = blueprint_version::Entity::find()
            .filter(blueprint_version::Column::ProjectId.eq(project_id))
            .count(&tx)
            .await
            .map_err(AppError::Database)?;

        let chapter_generation_cleanup =
            delete_chapter_generation_outputs(&tx, project_id, GeneratedImageDeleteScope::Project)
                .await?;
        let generation_media_cleanup = delete_generation_media_graph(&tx, project_id).await?;

        let deleted_chapters = chapter::Entity::delete_many()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?
            .rows_affected;
        let deleted_chapter_outlines = chapter_outline::Entity::delete_many()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?
            .rows_affected;
        let deleted_part_outlines = part_outline::Entity::delete_many()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?
            .rows_affected;

        let blueprint_cleanup = delete_project_blueprints(&tx, project_id, false).await?;

        let character_profile_reset =
            reset_character_profiles_for_project(&tx, project_id, now).await?;

        let deleted_foreshadowing = foreshadowing_index::Entity::delete_many()
            .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?
            .rows_affected;
        let profile_scene_cleanup = delete_project_profile_scene_data(&tx, project_id).await?;
        let text_anchor_cleanup = delete_project_text_anchors(
            &tx,
            project_id,
            TextAnchorDeleteScope::CurrentProjectContent,
        )
        .await?;
        let deleted_import_reviews = import_review_queue::Entity::delete_many()
            .filter(import_review_queue::Column::ProjectId.eq(project_id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?
            .rows_affected;

        let conversation_cleanup = delete_project_conversations(&tx, project_id).await?;
        let task_cleanup = delete_project_tasks(&tx, project_id).await?;

        let mut active: novel_project::ActiveModel = project.into();
        active.status = Set(ProjectStatus::Draft.as_str().to_string());
        active.current_chapter = Set(0);
        active.total_chapters = Set(None);
        active.raw_blueprint = Set(None);
        active.import_analysis_progress = Set(None);
        active.avatar_svg = Set(None);
        active.avatar_animal = Set(None);
        active.updated_at = Set(now);
        let updated = active.update(&tx).await.map_err(AppError::Database)?;
        super::mark_derived_projects_root_updated_in(&tx, project_id, now).await?;

        project_status_history::ActiveModel {
            project_id: Set(project_id),
            from_status: Set(previous_status.as_str().to_string()),
            to_status: Set(ProjectStatus::Draft.as_str().to_string()),
            reason: Set(Some("项目生成数据重置".to_string())),
            changed_by: Set(Some("system".to_string())),
            changed_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        if previous_status != ProjectStatus::Draft {
            self.event_bus
                .publish(InternalEvent::ProjectStatusChanged {
                    project_id,
                    from: previous_status.as_str().to_string(),
                    to: ProjectStatus::Draft.as_str().to_string(),
                    timestamp: now,
                })
                .await?;
        }

        Ok(ProjectGeneratedDataResetResult {
            project: updated,
            deleted_galgame_scripts: 0,
            deleted_galgame_scenes: 0,
            deleted_galgame_lines: 0,
            deleted_galgame_choices: 0,
            deleted_chapters,
            deleted_chapter_outlines,
            deleted_part_outlines,
            deleted_chapter_derived_rows: chapter_generation_cleanup.deleted_manga_prompts
                + chapter_generation_cleanup.deleted_video_prompts
                + chapter_generation_cleanup.deleted_chapter_scenes,
            deleted_current_blueprints: blueprint_cleanup.deleted_current_blueprints,
            deleted_blueprint_characters: blueprint_cleanup.deleted_blueprint_characters,
            deleted_blueprint_relationships: blueprint_cleanup.deleted_blueprint_relationships,
            preserved_blueprint_versions,
            reset_character_profiles: character_profile_reset.reset_character_profiles,
            deleted_character_profiles: character_profile_reset.deleted_character_profiles,
            deleted_character_snapshots: character_profile_reset.deleted_character_snapshots,
            deleted_character_timeline_rows: character_profile_reset
                .deleted_character_timeline_rows,
            deleted_character_portraits: character_profile_reset.deleted_character_portraits,
            deleted_object_profiles: profile_scene_cleanup.deleted_object_profiles,
            deleted_foreshadowing,
            deleted_scene_profiles: profile_scene_cleanup.deleted_scene_profiles,
            deleted_scene_occurrences: profile_scene_cleanup.deleted_scene_occurrences,
            deleted_scene_transitions: profile_scene_cleanup.deleted_scene_transitions,
            deleted_scene_entity_states: profile_scene_cleanup.deleted_scene_entity_states,
            deleted_text_anchors: text_anchor_cleanup.deleted_text_anchors,
            deleted_profile_evidence: text_anchor_cleanup.deleted_profile_evidence,
            deleted_import_reviews,
            deleted_conversations: conversation_cleanup.deleted_conversations,
            deleted_conversation_messages: conversation_cleanup.deleted_messages,
            deleted_optimization_sessions: optimization_cleanup.deleted_sessions,
            deleted_optimization_session_messages: optimization_cleanup.deleted_session_messages,
            deleted_optimization_turns: optimization_cleanup.deleted_turns,
            deleted_optimization_events: optimization_cleanup.deleted_events,
            deleted_optimization_patch_sets: optimization_cleanup.deleted_patch_sets,
            deleted_optimization_patches: optimization_cleanup.deleted_patches,
            deleted_generated_images: chapter_generation_cleanup.deleted_generated_images,
            deleted_generated_videos: chapter_generation_cleanup.deleted_generated_videos,
            deleted_generation_specs: generation_media_cleanup.deleted_generation_specs,
            deleted_prompt_artifacts: generation_media_cleanup.deleted_prompt_artifacts,
            deleted_media_assets: generation_media_cleanup.deleted_media_assets,
            deleted_media_provenance: generation_media_cleanup.deleted_media_provenance,
            deleted_media_bindings: generation_media_cleanup.deleted_media_bindings,
            deleted_tasks: task_cleanup.deleted_tasks,
            deleted_task_dependencies: task_cleanup.deleted_dependencies,
        })
    }

    async fn reset_galgame_generated_data_in_tx(
        &self,
        tx: DatabaseTransaction,
        project: novel_project::Model,
    ) -> Result<ProjectGeneratedDataResetResult, AppError> {
        let project_id = project.id;
        let previous_status = StateValidator::parse_status(&project.status)?;
        let now = Utc::now().naive_utc();

        let optimization_cleanup = delete_optimization_tree(&tx, project_id).await?;
        let chapter_generation_cleanup =
            delete_chapter_generation_outputs(&tx, project_id, GeneratedImageDeleteScope::Project)
                .await?;
        let generation_media_cleanup = delete_generation_media_graph(&tx, project_id).await?;
        let galgame_cleanup = delete_galgame_script_data(&tx, project_id).await?;
        let conversation_cleanup = delete_project_conversations(&tx, project_id).await?;
        let task_cleanup = delete_project_tasks(&tx, project_id).await?;

        let mut active: novel_project::ActiveModel = project.into();
        active.status = Set(ProjectStatus::Draft.as_str().to_string());
        active.current_chapter = Set(0);
        active.total_chapters = Set(None);
        active.import_analysis_progress = Set(None);
        active.updated_at = Set(now);
        let updated = active.update(&tx).await.map_err(AppError::Database)?;

        project_status_history::ActiveModel {
            project_id: Set(project_id),
            from_status: Set(previous_status.as_str().to_string()),
            to_status: Set(ProjectStatus::Draft.as_str().to_string()),
            reason: Set(Some("Galgame 项目生成数据重置".to_string())),
            changed_by: Set(Some("system".to_string())),
            changed_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        if previous_status != ProjectStatus::Draft {
            self.event_bus
                .publish(InternalEvent::ProjectStatusChanged {
                    project_id,
                    from: previous_status.as_str().to_string(),
                    to: ProjectStatus::Draft.as_str().to_string(),
                    timestamp: now,
                })
                .await?;
        }

        Ok(ProjectGeneratedDataResetResult {
            project: updated,
            deleted_galgame_scripts: galgame_cleanup.deleted_scripts,
            deleted_galgame_scenes: galgame_cleanup.deleted_scenes,
            deleted_galgame_lines: galgame_cleanup.deleted_lines,
            deleted_galgame_choices: galgame_cleanup.deleted_choices,
            deleted_chapters: 0,
            deleted_chapter_outlines: 0,
            deleted_part_outlines: 0,
            deleted_chapter_derived_rows: 0,
            deleted_current_blueprints: 0,
            deleted_blueprint_characters: 0,
            deleted_blueprint_relationships: 0,
            preserved_blueprint_versions: 0,
            reset_character_profiles: 0,
            deleted_character_profiles: 0,
            deleted_character_snapshots: 0,
            deleted_character_timeline_rows: 0,
            deleted_character_portraits: 0,
            deleted_object_profiles: 0,
            deleted_foreshadowing: 0,
            deleted_scene_profiles: 0,
            deleted_scene_occurrences: 0,
            deleted_scene_transitions: 0,
            deleted_scene_entity_states: 0,
            deleted_text_anchors: 0,
            deleted_profile_evidence: 0,
            deleted_import_reviews: 0,
            deleted_conversations: conversation_cleanup.deleted_conversations,
            deleted_conversation_messages: conversation_cleanup.deleted_messages,
            deleted_optimization_sessions: optimization_cleanup.deleted_sessions,
            deleted_optimization_session_messages: optimization_cleanup.deleted_session_messages,
            deleted_optimization_turns: optimization_cleanup.deleted_turns,
            deleted_optimization_events: optimization_cleanup.deleted_events,
            deleted_optimization_patch_sets: optimization_cleanup.deleted_patch_sets,
            deleted_optimization_patches: optimization_cleanup.deleted_patches,
            deleted_generated_images: chapter_generation_cleanup.deleted_generated_images,
            deleted_generated_videos: chapter_generation_cleanup.deleted_generated_videos,
            deleted_generation_specs: generation_media_cleanup.deleted_generation_specs,
            deleted_prompt_artifacts: generation_media_cleanup.deleted_prompt_artifacts,
            deleted_media_assets: generation_media_cleanup.deleted_media_assets,
            deleted_media_provenance: generation_media_cleanup.deleted_media_provenance,
            deleted_media_bindings: generation_media_cleanup.deleted_media_bindings,
            deleted_tasks: task_cleanup.deleted_tasks,
            deleted_task_dependencies: task_cleanup.deleted_dependencies,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct GalgameScriptCleanupResult {
    deleted_scripts: u64,
    deleted_scenes: u64,
    deleted_lines: u64,
    deleted_choices: u64,
}

async fn delete_galgame_script_data<C>(
    db: &C,
    project_id: i64,
) -> Result<GalgameScriptCleanupResult, AppError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script};

    let script_ids = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if script_ids.is_empty() {
        return Ok(GalgameScriptCleanupResult::default());
    }
    let scene_ids = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .filter(galgame_scene::Column::ScriptId.is_in(script_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let line_ids = if scene_ids.is_empty() {
        Vec::new()
    } else {
        galgame_line::Entity::find()
            .select_only()
            .column(galgame_line::Column::Id)
            .filter(galgame_line::Column::SceneId.is_in(scene_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?
    };
    let deleted_choices = if line_ids.is_empty() {
        0
    } else {
        galgame_choice::Entity::delete_many()
            .filter(galgame_choice::Column::LineId.is_in(line_ids.clone()))
            .exec(db)
            .await
            .map_err(AppError::Database)?
            .rows_affected
    };
    let deleted_lines = if line_ids.is_empty() {
        0
    } else {
        galgame_line::Entity::delete_many()
            .filter(galgame_line::Column::Id.is_in(line_ids))
            .exec(db)
            .await
            .map_err(AppError::Database)?
            .rows_affected
    };
    let deleted_scenes = if scene_ids.is_empty() {
        0
    } else {
        galgame_scene::Entity::delete_many()
            .filter(galgame_scene::Column::Id.is_in(scene_ids))
            .exec(db)
            .await
            .map_err(AppError::Database)?
            .rows_affected
    };
    let deleted_scripts = galgame_script::Entity::delete_many()
        .filter(galgame_script::Column::Id.is_in(script_ids))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    Ok(GalgameScriptCleanupResult {
        deleted_scripts,
        deleted_scenes,
        deleted_lines,
        deleted_choices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{
        CreateDerivedProjectParams, DerivedChapterScope, PROJECT_TYPE_NOVEL_LONG,
    };
    use a4n_core::event::bridge::BroadcastEventBus;
    use a4n_core::models::{
        ai::generated_image,
        chapter::chapter,
        galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
        generation::{generation_spec, prompt_artifact},
        media::{media_asset, media_binding, media_provenance},
        project::{project_source_snapshot, source_snapshot_chapter, text_anchor},
    };
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
        QueryFilter, Set, TransactionTrait,
    };
    use serde_json::json;
    use std::sync::Arc;

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    fn service(db: Arc<DatabaseConnection>) -> ProjectService {
        ProjectService::new(db, Arc::new(BroadcastEventBus::new(16)))
    }

    async fn insert_chapter(db: &DatabaseConnection, project_id: i64) -> chapter::Model {
        let now = Utc::now().naive_utc();
        chapter::ActiveModel {
            project_id: Set(project_id),
            chapter_number: Set(1),
            title: Set(Some("第一章".to_string())),
            summary: Set(Some("主角进入房间".to_string())),
            content: Set("他推开门，走进昏暗的房间。".to_string()),
            word_count: Set(13),
            status: Set("draft".to_string()),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn reset_text_project_removes_new_generation_chain_but_keeps_snapshot_anchors() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let project = service
            .create(1, "根文本", "", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        let chapter = insert_chapter(db.as_ref(), project.id).await;
        let snapshot = service
            .create_source_snapshot(
                project.id,
                DerivedChapterScope {
                    start_chapter: 1,
                    end_chapter: 1,
                },
            )
            .await
            .unwrap();
        let snapshot_chapter = source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot.id))
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        let now = Utc::now().naive_utc();
        let spec = generation_spec::ActiveModel {
            project_id: Set(project.id),
            root_project_id: Set(project.id),
            source_context_kind: Set(Some("snapshot_backed".to_string())),
            source_snapshot_id: Set(Some(snapshot.id)),
            source_chapter_id: Set(Some(chapter.id)),
            source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
            target_kind: Set("object_material_image".to_string()),
            target_entity_kind: Set("object".to_string()),
            target_entity_id: Set(None),
            status: Set("completed".to_string()),
            source_version_hash: Set(snapshot.source_version_hash.clone()),
            params_json: Set(json!({
                "image_prompt": "清晰写实的物体素材图。",
                "negative_prompt": "低质量、模糊、文字水印"
            })),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        let prompt = prompt_artifact::ActiveModel {
            generation_spec_id: Set(spec.id),
            prompt_kind: Set("image_prompt".to_string()),
            prompt_text: Set("清晰写实的物体素材图。".to_string()),
            renderer_version: Set("test".to_string()),
            edited_by_user: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        let asset = media_asset::ActiveModel {
            project_id: Set(project.id),
            root_project_id: Set(project.id),
            source_snapshot_id: Set(Some(snapshot.id)),
            source_chapter_id: Set(Some(chapter.id)),
            source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
            media_kind: Set("image".to_string()),
            origin: Set("generated".to_string()),
            file_path: Set(Some("projects/1/media/image/1.png".to_string())),
            thumbnail_path: Set(None),
            width: Set(Some(1024)),
            height: Set(Some(1024)),
            duration_secs: Set(None),
            fps: Set(None),
            file_size: Set(Some(4)),
            checksum: Set(Some("checksum".to_string())),
            status: Set("completed".to_string()),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        media_provenance::ActiveModel {
            media_asset_id: Set(asset.id),
            generation_spec_id: Set(Some(spec.id)),
            prompt_artifact_id: Set(Some(prompt.id)),
            provider_job_id: Set(None),
            source_media_asset_id: Set(None),
            source_external_path: Set(None),
            imported_from_user: Set(false),
            request_snapshot_json: Set(None),
            created_at: Set(now),
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        media_binding::ActiveModel {
            media_asset_id: Set(asset.id),
            target_kind: Set("chapter".to_string()),
            target_id: Set(chapter.id),
            role: Set("reference".to_string()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();

        let snapshot_anchor_count = text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(project.id))
            .filter(text_anchor::Column::SourceSnapshotId.eq(snapshot.id))
            .count(db.as_ref())
            .await
            .unwrap();
        assert!(snapshot_anchor_count > 0);

        let result = service.reset_generated_data(project.id).await.unwrap();

        assert_eq!(result.deleted_generation_specs, 1);
        assert_eq!(result.deleted_prompt_artifacts, 1);
        assert_eq!(result.deleted_media_assets, 1);
        assert_eq!(result.deleted_media_provenance, 1);
        assert_eq!(result.deleted_media_bindings, 1);
        assert_eq!(
            generation_spec::Entity::find()
                .filter(generation_spec::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            media_asset::Entity::find()
                .filter(media_asset::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            text_anchor::Entity::find()
                .filter(text_anchor::Column::ProjectId.eq(project.id))
                .filter(text_anchor::Column::SourceSnapshotId.eq(snapshot.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            snapshot_anchor_count
        );
    }

    #[tokio::test]
    async fn reset_galgame_project_removes_script_tables_and_generation_media_graph() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        let chapter = insert_chapter(db.as_ref(), root.id).await;
        let galgame = service
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_GALGAME.to_string(),
                    title: "Galgame".to_string(),
                    description: String::new(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap();
        let snapshot_id = galgame.source_snapshot_id.unwrap();
        let snapshot = project_source_snapshot::Entity::find_by_id(snapshot_id)
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        let snapshot_chapter = source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        let now = Utc::now().naive_utc();
        let script = galgame_script::ActiveModel {
            project_id: Set(galgame.id),
            root_project_id: Set(root.id),
            source_snapshot_id: Set(snapshot_id),
            title: Set("Galgame".to_string()),
            style_config_json: Set(json!({})),
            status: Set("ready".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        let scene = galgame_scene::ActiveModel {
            script_id: Set(script.id),
            source_snapshot_chapter_id: Set(snapshot_chapter.id),
            scene_index: Set(1),
            title: Set("第一场".to_string()),
            location: Set(Some("房间".to_string())),
            time_of_day: Set(None),
            participants_json: Set(json!([])),
            background_note: Set(None),
            entry_kind: Set("main".to_string()),
            next_scene_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        let tx = db.begin().await.unwrap();
        let line = galgame_line::ActiveModel {
            scene_id: Set(scene.id),
            line_index: Set(1),
            line_type: Set("choice".to_string()),
            speaker_profile_id: Set(None),
            text: Set("选择".to_string()),
            expression: Set(None),
            pose: Set(None),
            position: Set(None),
            presentation_json: Set(json!({})),
            text_anchor_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .unwrap();
        for (option_index, option_text) in [(1, "继续"), (2, "返回")] {
            galgame_choice::ActiveModel {
                line_id: Set(line.id),
                option_index: Set(option_index),
                option_text: Set(option_text.to_string()),
                target_scene_id: Set(Some(scene.id)),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await
            .unwrap();
        }
        tx.commit().await.unwrap();
        generated_image::ActiveModel {
            project_id: Set(galgame.id),
            image_type: Set("galgame_background".to_string()),
            file_path: Set("projects/2/images/bg.png".to_string()),
            prompt: Set("背景".to_string()),
            model: Set("test".to_string()),
            width: Set(1024),
            height: Set(576),
            file_size: Set(Some(4)),
            ai_score: Set(None),
            is_favorite: Set(false),
            chapter_id: Set(Some(chapter.id)),
            panel_id: Set(None),
            metadata_json: Set(None),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        let spec = generation_spec::ActiveModel {
            project_id: Set(galgame.id),
            root_project_id: Set(root.id),
            source_context_kind: Set(Some("snapshot_backed".to_string())),
            source_snapshot_id: Set(Some(snapshot_id)),
            source_chapter_id: Set(Some(chapter.id)),
            source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
            target_kind: Set("galgame_background".to_string()),
            target_entity_kind: Set("galgame_scene".to_string()),
            target_entity_id: Set(Some(scene.id)),
            status: Set("completed".to_string()),
            source_version_hash: Set(snapshot.source_version_hash.clone()),
            params_json: Set(json!({
                "canonical_key": format!("galgame:{}:background:scene:{}", galgame.id, scene.id),
                "asset_kind": "background",
                "scene": {
                    "id": scene.id,
                    "index": scene.scene_index,
                    "title": scene.title,
                    "location": scene.location,
                    "time_of_day": scene.time_of_day,
                    "background_note": scene.background_note,
                },
                "source": {
                    "source_snapshot_chapter_id": snapshot_chapter.id,
                    "source_chapter_id": chapter.id,
                    "chapter_number": snapshot_chapter.chapter_number,
                    "content_hash": snapshot_chapter.content_hash,
                },
            })),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        let prompt = prompt_artifact::ActiveModel {
            generation_spec_id: Set(spec.id),
            prompt_kind: Set("background_image_prompt".to_string()),
            prompt_text: Set("背景".to_string()),
            renderer_version: Set("test".to_string()),
            edited_by_user: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        let asset = media_asset::ActiveModel {
            project_id: Set(galgame.id),
            root_project_id: Set(root.id),
            source_snapshot_id: Set(Some(snapshot_id)),
            source_chapter_id: Set(Some(chapter.id)),
            source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
            media_kind: Set("image".to_string()),
            origin: Set("generated".to_string()),
            file_path: Set(Some("projects/2/images/bg.png".to_string())),
            thumbnail_path: Set(None),
            width: Set(Some(1024)),
            height: Set(Some(576)),
            duration_secs: Set(None),
            fps: Set(None),
            file_size: Set(Some(4)),
            checksum: Set(Some("checksum".to_string())),
            status: Set("completed".to_string()),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        media_provenance::ActiveModel {
            media_asset_id: Set(asset.id),
            generation_spec_id: Set(Some(spec.id)),
            prompt_artifact_id: Set(Some(prompt.id)),
            provider_job_id: Set(None),
            source_media_asset_id: Set(None),
            source_external_path: Set(None),
            imported_from_user: Set(false),
            request_snapshot_json: Set(Some(json!({
                "source": "galgame_asset_generation",
                "target_kind": "galgame_background",
                "target_entity_kind": "galgame_scene",
                "target_entity_id": scene.id,
            }))),
            created_at: Set(now),
        }
        .insert(db.as_ref())
        .await
        .unwrap();
        media_binding::ActiveModel {
            media_asset_id: Set(asset.id),
            target_kind: Set("galgame_scene".to_string()),
            target_id: Set(scene.id),
            role: Set("background".to_string()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();

        let result = service.reset_generated_data(galgame.id).await.unwrap();

        assert_eq!(result.deleted_galgame_scripts, 1);
        assert_eq!(result.deleted_galgame_scenes, 1);
        assert_eq!(result.deleted_galgame_lines, 1);
        assert_eq!(result.deleted_galgame_choices, 2);
        assert_eq!(result.deleted_generation_specs, 1);
        assert_eq!(result.deleted_prompt_artifacts, 1);
        assert_eq!(result.deleted_media_assets, 1);
        assert_eq!(result.deleted_media_provenance, 1);
        assert_eq!(result.deleted_media_bindings, 1);
        assert_eq!(result.deleted_generated_images, 1);
        assert_eq!(
            galgame_script::Entity::find()
                .filter(galgame_script::Column::ProjectId.eq(galgame.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            media_asset::Entity::find()
                .filter(media_asset::Column::ProjectId.eq(galgame.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            novel_project::Entity::find_by_id(galgame.id)
                .one(db.as_ref())
                .await
                .unwrap()
                .unwrap()
                .status,
            ProjectStatus::Draft.as_str()
        );
    }
}
