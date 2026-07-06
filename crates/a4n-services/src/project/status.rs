use chrono::Utc;
use sea_orm::*;

use a4n_core::error::AppError;
use a4n_core::event::bridge::InternalEvent;
use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::{novel_blueprint, novel_project, project_status_history},
};
use a4n_core::state::{
    rollback::{CleanupAction, rollback_actions},
    status::ProjectStatus,
    validator::StateValidator,
};

use super::PROJECT_KIND_TEXT;
use super::ProjectService;
use super::cleanup::{
    GeneratedImageDeleteScope, delete_chapter_generation_outputs, delete_project_blueprints,
    delete_project_profile_scene_data, delete_project_tasks,
};

impl ProjectService {
    /// 前进转换项目状态
    pub async fn forward_status(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<novel_project::Model, AppError> {
        let project = self.get(project_id).await?;
        ensure_text_project_status_machine(&project)?;
        let from_status = StateValidator::parse_status(&project.status)?;
        let to_status = StateValidator::parse_status(target)?;
        StateValidator::validate_forward(&project.status, target)?;
        self.validate_forward_artifacts(&project, from_status, to_status)
            .await?;
        let from = from_status.as_str().to_string();
        let to = to_status.as_str().to_string();

        let mut active: novel_project::ActiveModel = project.into();
        active.status = Set(to.clone());
        let now = Utc::now().naive_utc();
        active.updated_at = Set(now);
        let updated = active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        self.record_status_history(project_id, &from, &to, "项目状态推进", now)
            .await?;

        self.event_bus
            .publish(InternalEvent::ProjectStatusChanged {
                project_id,
                from,
                to,
                timestamp: now,
            })
            .await?;

        Ok(updated)
    }

    /// 回退项目状态（执行清理动作）
    pub async fn rollback_status(
        &self,
        project_id: i64,
        target: &str,
    ) -> Result<novel_project::Model, AppError> {
        let project = self.get(project_id).await?;
        ensure_text_project_status_machine(&project)?;
        let from = StateValidator::parse_status(&project.status)?;
        let to = StateValidator::parse_status(target)?;
        StateValidator::validate_rollback(&project.status, target)?;
        let from_str = from.as_str().to_string();
        let to_str = to.as_str().to_string();
        let actions = rollback_actions(from, to);

        self.execute_cleanups(project_id, &actions).await?;

        let mut active: novel_project::ActiveModel = project.into();
        active.status = Set(to_str.clone());
        if to == ProjectStatus::Draft {
            active.raw_blueprint = Set(None);
            active.total_chapters = Set(None);
        }
        let now = Utc::now().naive_utc();
        active.updated_at = Set(now);
        let updated = active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        self.record_status_history(project_id, &from_str, &to_str, "项目状态回退", now)
            .await?;
        self.mark_derived_projects_root_updated(project_id).await?;

        self.event_bus
            .publish(InternalEvent::ProjectStatusChanged {
                project_id,
                from: from_str,
                to: to_str,
                timestamp: now,
            })
            .await?;

        Ok(updated)
    }

    /// 执行清理动作列表
    async fn execute_cleanups(
        &self,
        project_id: i64,
        actions: &[CleanupAction],
    ) -> Result<(), AppError> {
        use a4n_core::models::chapter::*;

        for action in actions {
            match action {
                CleanupAction::DeleteChapters => {
                    delete_chapter_generation_outputs(
                        self.db.as_ref(),
                        project_id,
                        GeneratedImageDeleteScope::Chapters,
                    )
                    .await?;
                    delete_project_profile_scene_data(self.db.as_ref(), project_id).await?;
                    delete_project_tasks(self.db.as_ref(), project_id).await?;
                    chapter::Entity::delete_many()
                        .filter(chapter::Column::ProjectId.eq(project_id))
                        .exec(self.db.as_ref())
                        .await
                        .map_err(AppError::Database)?;
                }
                CleanupAction::DeleteChapterOutlines => {
                    chapter_outline::Entity::delete_many()
                        .filter(chapter_outline::Column::ProjectId.eq(project_id))
                        .exec(self.db.as_ref())
                        .await
                        .map_err(AppError::Database)?;
                }
                CleanupAction::DeletePartOutlines => {
                    part_outline::Entity::delete_many()
                        .filter(part_outline::Column::ProjectId.eq(project_id))
                        .exec(self.db.as_ref())
                        .await
                        .map_err(AppError::Database)?;
                }
                CleanupAction::DeleteBlueprintVersions => {
                    delete_project_blueprints(self.db.as_ref(), project_id, true).await?;
                }
            }
        }
        Ok(())
    }

    async fn record_status_history(
        &self,
        project_id: i64,
        from_status: &str,
        to_status: &str,
        reason: &str,
        changed_at: chrono::NaiveDateTime,
    ) -> Result<(), AppError> {
        project_status_history::ActiveModel {
            project_id: Set(project_id),
            from_status: Set(from_status.to_string()),
            to_status: Set(to_status.to_string()),
            reason: Set(Some(reason.to_string())),
            changed_by: Set(Some("system".to_string())),
            changed_at: Set(changed_at),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn validate_forward_artifacts(
        &self,
        project: &novel_project::Model,
        from: ProjectStatus,
        to: ProjectStatus,
    ) -> Result<(), AppError> {
        match to {
            ProjectStatus::Draft => Ok(()),
            ProjectStatus::BlueprintReady => self.require_blueprint(project).await.map(|_| ()),
            ProjectStatus::PartOutlinesReady => {
                self.require_blueprint(project).await?;
                self.require_any_part_outline(project).await
            }
            ProjectStatus::ChapterOutlinesReady => {
                let _ = from;
                self.require_blueprint(project).await?;
                self.require_any_chapter_outline(project).await
            }
            ProjectStatus::Writing => {
                self.require_blueprint(project).await?;
                self.require_any_chapter_outline(project).await?;
                self.require_any_chapter_content(
                    project,
                    "前进到 writing（至少需要一个章节已有正文）",
                )
                .await
            }
            ProjectStatus::Completed => {
                self.require_blueprint(project).await?;
                self.require_any_chapter_content(
                    project,
                    "前进到 completed（至少需要一个章节已有正文）",
                )
                .await
            }
        }
    }

    async fn require_blueprint(
        &self,
        project: &novel_project::Model,
    ) -> Result<novel_blueprint::Model, AppError> {
        novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(project.id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| invalid_forward(project, "前进到蓝图后续阶段（缺少故事蓝图）"))
    }

    async fn require_any_part_outline(
        &self,
        project: &novel_project::Model,
    ) -> Result<(), AppError> {
        let count = part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project.id))
            .count(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if count == 0 {
            return Err(invalid_forward(
                project,
                "前进到分部大纲后续阶段（缺少分部大纲）",
            ));
        }
        Ok(())
    }

    async fn require_any_chapter_outline(
        &self,
        project: &novel_project::Model,
    ) -> Result<(), AppError> {
        let count = chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project.id))
            .count(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if count == 0 {
            return Err(invalid_forward(
                project,
                "前进到章节大纲后续阶段（缺少章节大纲）",
            ));
        }
        Ok(())
    }

    async fn require_any_chapter_content(
        &self,
        project: &novel_project::Model,
        operation: &'static str,
    ) -> Result<(), AppError> {
        let has_content = chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project.id))
            .filter(chapter::Column::Status.ne("deleted"))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .any(|chapter| chapter.has_available_content());
        if !has_content {
            return Err(invalid_forward(project, operation));
        }
        Ok(())
    }
}

fn ensure_text_project_status_machine(project: &novel_project::Model) -> Result<(), AppError> {
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(AppError::InvalidArgument(
            "非文本项目不能使用文本写作状态机".to_string(),
        ));
    }
    Ok(())
}

fn invalid_forward(project: &novel_project::Model, operation: impl Into<String>) -> AppError {
    AppError::InvalidState {
        current: project.status.clone(),
        operation: operation.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::event::bridge::BroadcastEventBus;
    use a4n_core::models::ai::{generated_image, generated_video};
    use a4n_core::models::asset::object_profile;
    use a4n_core::models::chapter::{chapter, chapter_outline, chapter_video_prompt, part_outline};
    use a4n_core::models::project::novel_blueprint;
    use a4n_core::models::scene::{
        scene_entity_state, scene_occurrence, scene_profile, scene_transition,
    };
    use a4n_core::models::task::{task_dependency, task_queue};
    use chrono::Utc;
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
        QueryFilter, Set,
    };
    use serde_json::json;
    use std::sync::Arc;

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    fn service(db: Arc<DatabaseConnection>) -> ProjectService {
        ProjectService::new(db, Arc::new(BroadcastEventBus::new(16)))
    }

    async fn insert_blueprint(
        db: &DatabaseConnection,
        project_id: i64,
        total_chapters: i32,
        needs_part_outlines: bool,
    ) {
        let now = Utc::now().naive_utc();
        let project = novel_project::Entity::find_by_id(project_id)
            .one(db)
            .await
            .unwrap()
            .unwrap();
        let mut active: novel_project::ActiveModel = project.into();
        active.target_chapters = Set(total_chapters);
        active.total_chapters = Set(Some(total_chapters));
        active.chapters_per_part = Set(if needs_part_outlines { Some(25) } else { None });
        active.updated_at = Set(now);
        active.update(db).await.unwrap();

        novel_blueprint::ActiveModel {
            project_id: Set(project_id),
            title: Set("测试蓝图".to_string()),
            genre: Set("测试".to_string()),
            style: Set(Some("测试风格".to_string())),
            tone: Set(Some("测试语气".to_string())),
            target_audience: Set(Some("测试读者".to_string())),
            one_sentence_summary: Set(Some("一句话".to_string())),
            full_synopsis: Set(Some("完整梗概".to_string())),
            world_setting: Set(Some(json!({"premise": "测试世界"}))),
            current_version: Set("1.0".to_string()),
            needs_part_outlines: Set(needs_part_outlines),
            total_chapters: Set(Some(total_chapters)),
            chapters_per_part: Set(if needs_part_outlines { Some(25) } else { None }),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    async fn insert_part_outline(db: &DatabaseConnection, project_id: i64, total_chapters: i32) {
        let now = Utc::now().naive_utc();
        part_outline::ActiveModel {
            project_id: Set(project_id),
            part_number: Set(1),
            title: Set("第一部".to_string()),
            start_chapter: Set(1),
            end_chapter: Set(total_chapters),
            summary: Set(Some("分部摘要".to_string())),
            theme: Set(Some("主题".to_string())),
            key_events: Set(None),
            character_focus: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    async fn insert_chapter_outlines(
        db: &DatabaseConnection,
        project_id: i64,
        total_chapters: i32,
    ) {
        let now = Utc::now().naive_utc();
        for chapter_number in 1..=total_chapters {
            chapter_outline::ActiveModel {
                project_id: Set(project_id),
                part_id: Set(None),
                chapter_number: Set(chapter_number),
                title: Set(Some(format!("第{chapter_number}章"))),
                summary: Set(Some("章纲摘要".to_string())),
                beats: Set(None),
                key_scenes: Set(None),
                pacing_score: Set(None),
                foreshadowing_setup: Set(None),
                foreshadowing_payoff: Set(None),
                characters_involved: Set(None),
                word_count_target: Set(Some(1000)),
                status: Set("completed".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .unwrap();
        }
    }

    async fn insert_chapter_content(db: &DatabaseConnection, project_id: i64, chapter_number: i32) {
        let now = Utc::now().naive_utc();
        chapter::ActiveModel {
            project_id: Set(project_id),
            outline_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("第{chapter_number}章"))),
            content: Set("正文".to_string()),
            word_count: Set(2),
            status: Set("completed".to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    async fn project_at_writing(service: &ProjectService) -> novel_project::Model {
        let project = service
            .create(1, "测试项目", "测试项目", "novel_long")
            .await
            .unwrap();
        insert_blueprint(service.db.as_ref(), project.id, 1, false).await;
        insert_chapter_outlines(service.db.as_ref(), project.id, 1).await;
        insert_chapter_content(service.db.as_ref(), project.id, 1).await;
        for target in [
            ProjectStatus::BlueprintReady,
            ProjectStatus::ChapterOutlinesReady,
            ProjectStatus::Writing,
        ] {
            service
                .forward_status(project.id, target.as_str())
                .await
                .unwrap();
        }
        service.get(project.id).await.unwrap()
    }

    async fn insert_chapter_video_generated_state(
        db: &DatabaseConnection,
        project_id: i64,
    ) -> chapter::Model {
        let now = Utc::now().naive_utc();
        let chapter = match chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .filter(chapter::Column::ChapterNumber.eq(1))
            .one(db)
            .await
            .unwrap()
        {
            Some(chapter) => chapter,
            None => chapter::ActiveModel {
                project_id: Set(project_id),
                outline_id: Set(None),
                chapter_number: Set(1),
                title: Set(Some("第一章".to_string())),
                content: Set("正文".to_string()),
                word_count: Set(2),
                status: Set("completed".to_string()),
                summary: Set(None),
                style_tags: Set(None),
                analysis_data: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .unwrap(),
        };

        let video = generated_video::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(Some(chapter.id)),
            video_type: Set("chapter_video".to_string()),
            workflow_mode: Set(Some("first_frame_chain".to_string())),
            file_path: Set(Some("projects/1/videos/chapter_1/video.mp4".to_string())),
            thumbnail_path: Set(Some("projects/1/videos/chapter_1/video.jpg".to_string())),
            prompt: Set("视频提示词".to_string()),
            negative_prompt: Set(None),
            model: Set("test-video".to_string()),
            width: Set(1280),
            height: Set(720),
            duration_secs: Set(5.0),
            fps: Set(24),
            file_size: Set(Some(10)),
            source_video_id: Set(None),
            source_image_id: Set(None),
            status: Set("completed".to_string()),
            provider_job_id: Set(Some("provider-job".to_string())),
            error_message: Set(None),
            metadata_json: Set(Some(json!({ "shot_number": 1 }))),
            is_favorite: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        chapter_video_prompt::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(chapter.id),
            style: Set(Some("电影感写实".to_string())),
            prompt_type: Set("video_prompt".to_string()),
            workflow_mode: Set("first_frame_chain".to_string()),
            shot_number: Set(1),
            title: Set(Some("镜头 1".to_string())),
            scene_summary: Set(Some("场景".to_string())),
            visual_prompt: Set("画面".to_string()),
            motion_prompt: Set(Some("运动".to_string())),
            negative_prompt: Set(None),
            duration_secs: Set(5),
            aspect_ratio: Set(Some("16:9".to_string())),
            reference_image_ids_json: Set(None),
            status: Set("completed".to_string()),
            generated_video_id: Set(Some(video.id)),
            metadata_json: Set(Some(json!({ "source": "test" }))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        generated_image::ActiveModel {
            project_id: Set(project_id),
            image_type: Set("video_first_frame".to_string()),
            file_path: Set("projects/1/images/frame.png".to_string()),
            prompt: Set("首帧".to_string()),
            model: Set("test-image".to_string()),
            width: Set(1280),
            height: Set(720),
            file_size: Set(Some(10)),
            ai_score: Set(None),
            is_favorite: Set(false),
            chapter_id: Set(Some(chapter.id)),
            panel_id: Set(None),
            metadata_json: Set(Some(json!({ "video_frame_role": "start_frame" }))),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        for task_id in ["video-task", "video-child-task"] {
            task_queue::Entity::insert(task_queue::ActiveModel {
                id: Set(task_id.to_string()),
                task_type: Set("video_generation".to_string()),
                priority: Set("medium".to_string()),
                status: Set("running".to_string()),
                project_id: Set(Some(project_id)),
                root_project_id: Set(None),
                source_snapshot_id: Set(None),
                task_scope: Set(None),
                workflow_run_id: Set(None),
                payload: Set(Some(json!({ "chapter_id": chapter.id }))),
                progress: Set(None),
                error_message: Set(None),
                dag_id: Set(None),
                created_at: Set(now),
                started_at: Set(Some(now)),
                completed_at: Set(None),
            })
            .exec(db)
            .await
            .unwrap();
        }
        task_dependency::ActiveModel {
            task_id: Set("video-child-task".to_string()),
            depends_on_task_id: Set("video-task".to_string()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        chapter
    }

    async fn insert_object_and_scene_generated_state(
        db: &DatabaseConnection,
        project_id: i64,
        chapter: &chapter::Model,
    ) {
        let now = Utc::now().naive_utc();
        object_profile::ActiveModel {
            project_id: Set(project_id),
            name: Set("青铜钥匙".to_string()),
            normalized_name: Set("青铜钥匙".to_string()),
            aliases_json: Set(json!([])),
            object_type: Set("token".to_string()),
            importance: Set("primary".to_string()),
            visual_description: Set("刻着星纹的青铜钥匙".to_string()),
            material_and_state: Set("青铜材质，边缘磨损".to_string()),
            story_usage: Set("开启密室的线索".to_string()),
            visual_usage: Set("保持青铜钥匙作为手中线索物的画面连续性".to_string()),
            generation_usage: Set("约束青铜钥匙星纹、磨损边缘和青铜材质".to_string()),
            location_context: Set(Some("旧书房抽屉".to_string())),
            first_seen_chapter: Set(chapter.chapter_number),
            last_seen_chapter: Set(chapter.chapter_number),
            evidence_json: Set(json!(["一枚青铜钥匙"])),
            image_prompt_seed: Set("一枚刻着星纹的青铜钥匙".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        let scene = scene_profile::ActiveModel {
            project_id: Set(project_id),
            name: Set("旧书房".to_string()),
            normalized_name: Set("旧书房".to_string()),
            aliases_json: Set(json!([])),
            scene_type: Set("room".to_string()),
            importance: Set("primary".to_string()),
            parent_scene_id: Set(None),
            base_description: Set("堆满旧书的小书房".to_string()),
            visual_description: Set("木书桌靠窗，旧书架贴着后墙".to_string()),
            spatial_layout: Set("门在左侧，书桌靠窗".to_string()),
            atmosphere: Set("昏黄、压抑".to_string()),
            entry_points_json: Set(json!(["门"])),
            rules_json: Set(json!([])),
            current_state_json: Set(json!({ "summary": "抽屉被打开" })),
            first_seen_chapter: Set(chapter.chapter_number),
            last_seen_chapter: Set(chapter.chapter_number),
            evidence_json: Set(json!(["他走进旧书房"])),
            image_prompt_seed: Set("昏黄灯光下的旧书房".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        let occurrence = scene_occurrence::ActiveModel {
            project_id: Set(project_id),
            scene_profile_id: Set(scene.id),
            chapter_id: Set(chapter.id),
            chapter_number: Set(chapter.chapter_number),
            occurrence_order: Set(1),
            timeline_position: Set(None),
            summary: Set("主角进入旧书房".to_string()),
            mood: Set(Some("谨慎".to_string())),
            weather_or_lighting: Set(None),
            source_span_ids_json: Set(json!(["ch1:s1"])),
            evidence_json: Set(json!(["他走进旧书房"])),
            characters_json: Set(json!([])),
            objects_json: Set(json!([])),
            key_events_json: Set(json!(["进入旧书房"])),
            entry_state_json: Set(json!({})),
            exit_state_json: Set(json!({ "left": true })),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        scene_transition::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(chapter.id),
            chapter_number: Set(chapter.chapter_number),
            from_scene_profile_id: Set(None),
            to_scene_profile_id: Set(scene.id),
            occurrence_id: Set(Some(occurrence.id)),
            transition_order: Set(1),
            movement_type: Set("enter".to_string()),
            route_description: Set("推门进入旧书房".to_string()),
            time_cost: Set(None),
            distance_hint: Set(None),
            participants_json: Set(json!([])),
            object_refs_json: Set(json!([])),
            source_span_ids_json: Set(json!(["ch1:s1"])),
            evidence_json: Set(json!(["推门进入旧书房"])),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        scene_entity_state::ActiveModel {
            project_id: Set(project_id),
            scene_profile_id: Set(scene.id),
            occurrence_id: Set(occurrence.id),
            chapter_number: Set(chapter.chapter_number),
            entity_type: Set("event".to_string()),
            entity_id: Set(None),
            entity_name_snapshot: Set("进入旧书房".to_string()),
            state_key: Set("event_progress".to_string()),
            state_before_json: Set(json!({})),
            state_after_json: Set(json!({ "entered": true })),
            position_description: Set(Some("门口".to_string())),
            action_description: Set("推门进入".to_string()),
            source_span_ids_json: Set(json!(["ch1:s1"])),
            evidence_json: Set(json!(["推门进入旧书房"])),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn forward_status_allows_chapter_outlines_without_full_part_coverage() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let project = service
            .create(1, "开放长篇", "不预设总章数项目", "novel_long")
            .await
            .unwrap();
        insert_blueprint(db.as_ref(), project.id, 51, true).await;
        insert_chapter_outlines(db.as_ref(), project.id, 51).await;
        service
            .forward_status(project.id, ProjectStatus::BlueprintReady.as_str())
            .await
            .unwrap();

        let updated = service
            .forward_status(project.id, ProjectStatus::ChapterOutlinesReady.as_str())
            .await
            .unwrap();

        assert_eq!(updated.status, ProjectStatus::ChapterOutlinesReady.as_str());
    }

    #[tokio::test]
    async fn forward_status_allows_projects_without_part_outlines_to_skip_them() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let project = service
            .create(1, "三十章", "无需分部项目", "novel_long")
            .await
            .unwrap();
        insert_blueprint(db.as_ref(), project.id, 30, false).await;
        insert_chapter_outlines(db.as_ref(), project.id, 30).await;
        service
            .forward_status(project.id, ProjectStatus::BlueprintReady.as_str())
            .await
            .unwrap();

        let updated = service
            .forward_status(project.id, ProjectStatus::ChapterOutlinesReady.as_str())
            .await
            .unwrap();

        assert_eq!(updated.status, ProjectStatus::ChapterOutlinesReady.as_str());
    }

    #[tokio::test]
    async fn forward_status_allows_part_outlines_when_any_part_exists() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let project = service
            .create(1, "五十一章", "需要分部项目", "novel_long")
            .await
            .unwrap();
        insert_blueprint(db.as_ref(), project.id, 51, true).await;
        insert_part_outline(db.as_ref(), project.id, 51).await;
        insert_chapter_outlines(db.as_ref(), project.id, 51).await;
        service
            .forward_status(project.id, ProjectStatus::BlueprintReady.as_str())
            .await
            .unwrap();
        service
            .forward_status(project.id, ProjectStatus::PartOutlinesReady.as_str())
            .await
            .unwrap();

        let updated = service
            .forward_status(project.id, ProjectStatus::ChapterOutlinesReady.as_str())
            .await
            .unwrap();

        assert_eq!(updated.status, ProjectStatus::ChapterOutlinesReady.as_str());
    }

    #[tokio::test]
    async fn rollback_deleting_chapters_clears_video_outputs_and_project_tasks() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let project = project_at_writing(&service).await;

        let chapter = insert_chapter_video_generated_state(db.as_ref(), project.id).await;
        insert_object_and_scene_generated_state(db.as_ref(), project.id, &chapter).await;

        let updated = service
            .rollback_status(project.id, ProjectStatus::ChapterOutlinesReady.as_str())
            .await
            .unwrap();

        assert_eq!(updated.status, ProjectStatus::ChapterOutlinesReady.as_str());
        assert_eq!(
            chapter::Entity::find()
                .filter(chapter::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            chapter_video_prompt::Entity::find()
                .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            generated_video::Entity::find()
                .filter(generated_video::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            generated_image::Entity::find()
                .filter(generated_image::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            object_profile::Entity::find()
                .filter(object_profile::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            scene_profile::Entity::find()
                .filter(scene_profile::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            scene_occurrence::Entity::find()
                .filter(scene_occurrence::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            scene_transition::Entity::find()
                .filter(scene_transition::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            scene_entity_state::Entity::find()
                .filter(scene_entity_state::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            task_queue::Entity::find()
                .filter(task_queue::Column::ProjectId.eq(project.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            task_dependency::Entity::find()
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
    }
}
