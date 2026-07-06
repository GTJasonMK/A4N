use sea_orm::*;

use a4n_core::error::AppError;

use super::PROJECT_KIND_TEXT;
use super::ProjectService;
use super::cleanup::{
    GeneratedImageDeleteScope, TextAnchorDeleteScope, delete_chapter_generation_outputs,
    delete_generation_media_graph, delete_optimization_tree, delete_project_blueprints,
    delete_project_conversations, delete_project_profile_scene_data, delete_project_tasks,
    delete_project_text_anchors,
};
use crate::generation::delete_generation_run_targets_for;

impl ProjectService {
    /// 级联删除项目下的所有数据
    pub async fn delete(&self, project_id: i64) -> Result<(), AppError> {
        let project = self.get(project_id).await?;
        if project.project_kind == PROJECT_KIND_TEXT {
            let derived_count = a4n_core::models::project::novel_project::Entity::find()
                .filter(
                    a4n_core::models::project::novel_project::Column::RootProjectId.eq(project_id),
                )
                .count(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            if derived_count > 0 {
                return Err(AppError::InvalidArgument(format!(
                    "文本项目存在 {derived_count} 个派生项目，请先删除派生项目或执行项目族删除"
                )));
            }
        }
        self.cascade_delete(project_id).await
    }

    async fn cascade_delete(&self, project_id: i64) -> Result<(), AppError> {
        use a4n_core::models::chapter::*;
        use a4n_core::models::character::*;
        use a4n_core::models::galgame::*;
        use a4n_core::models::project::*;
        use a4n_core::models::source_sync::*;
        use a4n_core::models::video_project::*;

        // 通过 profile_id 子查询删除（profile 通过 project_id 过滤）
        let profile_ids = protagonist_profile::Entity::find()
            .select_only()
            .column(protagonist_profile::Column::Id)
            .filter(protagonist_profile::Column::ProjectId.eq(project_id))
            .into_query();
        let profile_id_values = protagonist_profile::Entity::find()
            .select_only()
            .column(protagonist_profile::Column::Id)
            .filter(protagonist_profile::Column::ProjectId.eq(project_id))
            .into_tuple::<i64>()
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        protagonist_behavior_record::Entity::delete_many()
            .filter(protagonist_behavior_record::Column::ProfileId.in_subquery(profile_ids.clone()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        protagonist_deletion_mark::Entity::delete_many()
            .filter(protagonist_deletion_mark::Column::ProfileId.in_subquery(profile_ids.clone()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        // 通过 profile_id 删除
        snapshot_reference::Entity::delete_many()
            .filter(snapshot_reference::Column::ProfileId.in_subquery(profile_ids.clone()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        character_state_index::Entity::delete_many()
            .filter(character_state_index::Column::ProfileId.in_subquery(profile_ids.clone()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let portrait_ids = character_portrait::Entity::find()
            .select_only()
            .column(character_portrait::Column::Id)
            .filter(character_portrait::Column::ProfileId.in_subquery(profile_ids.clone()))
            .into_tuple::<i64>()
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        delete_generation_run_targets_for(self.db.as_ref(), "character_portrait", &portrait_ids)
            .await?;
        character_portrait::Entity::delete_many()
            .filter(character_portrait::Column::ProfileId.in_subquery(profile_ids.clone()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        attribute_event::Entity::delete_many()
            .filter(attribute_event::Column::ProfileId.in_subquery(profile_ids.clone()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        // 有 project_id 的
        foreshadowing_index::Entity::delete_many()
            .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        delete_project_profile_scene_data(self.db.as_ref(), project_id).await?;
        delete_project_text_anchors(self.db.as_ref(), project_id, TextAnchorDeleteScope::Project)
            .await?;
        delete_generation_run_targets_for(
            self.db.as_ref(),
            "protagonist_profile",
            &profile_id_values,
        )
        .await?;
        protagonist_profile::Entity::delete_many()
            .filter(protagonist_profile::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        // 章节相关
        delete_chapter_generation_outputs(
            self.db.as_ref(),
            project_id,
            GeneratedImageDeleteScope::Project,
        )
        .await?;
        chapter::Entity::delete_many()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        // 大纲
        chapter_outline::Entity::delete_many()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        part_outline::Entity::delete_many()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        // 蓝图
        delete_project_blueprints(self.db.as_ref(), project_id, true).await?;

        // 对话/导入/状态历史/覆盖
        delete_project_conversations(self.db.as_ref(), project_id).await?;
        import_review_queue::Entity::delete_many()
            .filter(import_review_queue::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        project_status_history::Entity::delete_many()
            .filter(project_status_history::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        self.delete_project_prompt_override_settings(project_id)
            .await?;

        let timeline_ids = assembly_timeline::Entity::find()
            .select_only()
            .column(assembly_timeline::Column::Id)
            .filter(assembly_timeline::Column::ProjectId.eq(project_id))
            .into_query();
        let sync_conflict_ids = source_sync_conflict::Entity::find()
            .select_only()
            .column(source_sync_conflict::Column::Id)
            .filter(source_sync_conflict::Column::DerivedProjectId.eq(project_id))
            .into_query();

        source_sync_decision::Entity::delete_many()
            .filter(source_sync_decision::Column::ConflictId.in_subquery(sync_conflict_ids.clone()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        source_sync_conflict::Entity::delete_many()
            .filter(source_sync_conflict::Column::DerivedProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        source_sync_run::Entity::delete_many()
            .filter(source_sync_run::Column::DerivedProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        assembly_clip::Entity::delete_many()
            .filter(assembly_clip::Column::TimelineId.in_subquery(timeline_ids.clone()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        assembly_timeline::Entity::delete_many()
            .filter(assembly_timeline::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        video_segment::Entity::delete_many()
            .filter(video_segment::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        let galgame_script_ids = galgame_script::Entity::find()
            .select_only()
            .column(galgame_script::Column::Id)
            .filter(galgame_script::Column::ProjectId.eq(project_id))
            .into_tuple::<i64>()
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if !galgame_script_ids.is_empty() {
            let galgame_scene_ids = galgame_scene::Entity::find()
                .select_only()
                .column(galgame_scene::Column::Id)
                .filter(galgame_scene::Column::ScriptId.is_in(galgame_script_ids.clone()))
                .into_tuple::<i64>()
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            if !galgame_scene_ids.is_empty() {
                let galgame_line_ids = galgame_line::Entity::find()
                    .select_only()
                    .column(galgame_line::Column::Id)
                    .filter(galgame_line::Column::SceneId.is_in(galgame_scene_ids.clone()))
                    .into_tuple::<i64>()
                    .all(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?;
                if !galgame_line_ids.is_empty() {
                    galgame_choice::Entity::delete_many()
                        .filter(galgame_choice::Column::LineId.is_in(galgame_line_ids.clone()))
                        .exec(self.db.as_ref())
                        .await
                        .map_err(AppError::Database)?;
                    galgame_line::Entity::delete_many()
                        .filter(galgame_line::Column::Id.is_in(galgame_line_ids))
                        .exec(self.db.as_ref())
                        .await
                        .map_err(AppError::Database)?;
                }
                galgame_scene::Entity::delete_many()
                    .filter(galgame_scene::Column::Id.is_in(galgame_scene_ids))
                    .exec(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?;
            }
            galgame_script::Entity::delete_many()
                .filter(galgame_script::Column::Id.is_in(galgame_script_ids))
                .exec(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
        }

        delete_generation_media_graph(self.db.as_ref(), project_id).await?;

        let derivations = project_derivation::Entity::find()
            .filter(project_derivation::Column::DerivedProjectId.eq(project_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        project_derivation::Entity::delete_many()
            .filter(project_derivation::Column::DerivedProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        for derivation in derivations {
            source_snapshot_profile::Entity::delete_many()
                .filter(
                    source_snapshot_profile::Column::SnapshotId.eq(derivation.source_snapshot_id),
                )
                .exec(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            source_snapshot_outline::Entity::delete_many()
                .filter(
                    source_snapshot_outline::Column::SnapshotId.eq(derivation.source_snapshot_id),
                )
                .exec(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            source_snapshot_chapter::Entity::delete_many()
                .filter(
                    source_snapshot_chapter::Column::SnapshotId.eq(derivation.source_snapshot_id),
                )
                .exec(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            project_source_snapshot::Entity::delete_by_id(derivation.source_snapshot_id)
                .exec(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
        }

        delete_optimization_tree(self.db.as_ref(), project_id).await?;

        delete_project_tasks(self.db.as_ref(), project_id).await?;

        project_invite::Entity::delete_many()
            .filter(project_invite::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        project_member::Entity::delete_many()
            .filter(project_member::Column::ProjectId.eq(project_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        // 项目本身
        novel_project::Entity::delete_by_id(project_id)
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    async fn delete_project_prompt_override_settings(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        use a4n_core::models::ai::system_settings;

        let key_prefix = system_settings::prompt_override_key_prefix(project_id);
        let override_setting_ids = system_settings::Entity::find()
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .filter(|record| record.key.starts_with(&key_prefix))
            .map(|record| record.id)
            .collect::<Vec<_>>();

        if override_setting_ids.is_empty() {
            return Ok(());
        }

        system_settings::Entity::delete_many()
            .filter(system_settings::Column::Id.is_in(override_setting_ids))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::event::bridge::BroadcastEventBus;
    use a4n_core::models::ai::system_settings;
    use a4n_core::models::chapter::chapter;
    use a4n_core::models::project::novel_project;
    use chrono::Utc;
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
        QueryFilter, QueryOrder, Set,
    };
    use serde_json::json;
    use std::sync::Arc;

    use crate::project::{
        CreateDerivedProjectParams, DerivedChapterScope, PROJECT_KIND_VIDEO,
        PROJECT_TYPE_NOVEL_LONG,
    };

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    fn service(db: Arc<DatabaseConnection>) -> ProjectService {
        ProjectService::new(db, Arc::new(BroadcastEventBus::new(16)))
    }

    async fn insert_setting(
        db: &DatabaseConnection,
        key: String,
        category: &str,
    ) -> system_settings::Model {
        system_settings::ActiveModel {
            key: Set(key),
            value: Set(serde_json::Value::String("覆盖内容".to_string())),
            category: Set(category.to_string()),
            description: Set(None),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn setting_keys(db: &DatabaseConnection) -> Vec<String> {
        system_settings::Entity::find()
            .order_by_asc(system_settings::Column::Key)
            .all(db)
            .await
            .unwrap()
            .into_iter()
            .map(|record| record.key)
            .collect()
    }

    async fn insert_chapter(
        db: &DatabaseConnection,
        project_id: i64,
        chapter_number: i32,
    ) -> chapter::Model {
        let now = Utc::now().naive_utc();
        chapter::ActiveModel {
            project_id: Set(project_id),
            outline_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("第{chapter_number}章"))),
            content: Set("他推开门，走进雨夜。".to_string()),
            word_count: Set(10),
            status: Set(chapter::STATUS_IMPORTED.to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn create_root_with_video_derivative(
        db: Arc<DatabaseConnection>,
    ) -> (ProjectService, novel_project::Model, novel_project::Model) {
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "测试项目", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db.as_ref(), root.id, 1).await;
        let derived = service
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: "视频项目".to_string(),
                    description: "视频派生".to_string(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap();
        (service, root, derived)
    }

    #[tokio::test]
    async fn delete_removes_project_prompt_override_settings() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let project = service
            .create(1, "测试项目", "测试项目", "novel_long")
            .await
            .unwrap();
        let deleted_key = system_settings::prompt_override_key(project.id, "greeting");
        let deleted_wrong_category_key =
            system_settings::prompt_override_key(project.id, "chapter_prompt");
        let retained_key = system_settings::prompt_override_key(project.id + 1, "greeting");

        insert_setting(
            db.as_ref(),
            deleted_key.clone(),
            system_settings::PROMPT_OVERRIDE_CATEGORY,
        )
        .await;
        insert_setting(db.as_ref(), deleted_wrong_category_key.clone(), "general").await;
        insert_setting(
            db.as_ref(),
            retained_key.clone(),
            system_settings::PROMPT_OVERRIDE_CATEGORY,
        )
        .await;

        service.delete(project.id).await.unwrap();

        let keys = setting_keys(db.as_ref()).await;
        assert!(!keys.contains(&deleted_key));
        assert!(!keys.contains(&deleted_wrong_category_key));
        assert!(keys.contains(&retained_key));
    }

    #[tokio::test]
    async fn delete_text_root_with_derivatives_is_rejected() {
        let db = Arc::new(setup_db().await);
        let (service, root, derived) = create_root_with_video_derivative(db.clone()).await;

        let err = service
            .delete(root.id)
            .await
            .expect_err("root delete should require explicit family delete");

        assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("派生项目")));
        assert_eq!(
            novel_project::Entity::find_by_id(root.id)
                .count(db.as_ref())
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            novel_project::Entity::find_by_id(derived.id)
                .count(db.as_ref())
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn delete_derived_project_preserves_root_text_data() {
        let db = Arc::new(setup_db().await);
        let (service, root, derived) = create_root_with_video_derivative(db.clone()).await;

        service.delete(derived.id).await.unwrap();

        assert_eq!(
            novel_project::Entity::find_by_id(root.id)
                .count(db.as_ref())
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            chapter::Entity::find()
                .filter(chapter::Column::ProjectId.eq(root.id))
                .count(db.as_ref())
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            novel_project::Entity::find_by_id(derived.id)
                .count(db.as_ref())
                .await
                .unwrap(),
            0
        );
    }
}
