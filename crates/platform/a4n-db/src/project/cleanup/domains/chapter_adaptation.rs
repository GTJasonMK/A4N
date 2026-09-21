use a4n_db_models::models::{
    chapter::{chapter_shooting_script, chapter_visual_plan},
    project::source_snapshot_chapter,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

use super::super::{ProjectChapterAdaptationDeleteScope, ProjectCleanupDbError};
use crate::project::validate_positive_i64;

pub(crate) async fn delete_project_chapter_adaptation_records<C>(
    db: &C,
    root_project_id: i64,
    scope: ProjectChapterAdaptationDeleteScope,
) -> Result<(), ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(root_project_id, "root_project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;

    let mut plan_query = chapter_visual_plan::Entity::find()
        .filter(chapter_visual_plan::Column::RootProjectId.eq(root_project_id));
    if matches!(
        scope,
        ProjectChapterAdaptationDeleteScope::CurrentProjectContent
    ) {
        plan_query = plan_query
            .filter(chapter_visual_plan::Column::SourceSnapshotId.is_null())
            .filter(chapter_visual_plan::Column::SourceSnapshotChapterId.is_null());
    }
    let plan_ids = plan_query
        .select_only()
        .column(chapter_visual_plan::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await?;

    if !plan_ids.is_empty() {
        source_snapshot_chapter::Entity::update_many()
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanId,
                sea_orm::sea_query::Expr::value(Option::<i64>::None),
            )
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanVersion,
                sea_orm::sea_query::Expr::value(Option::<i32>::None),
            )
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanSha256,
                sea_orm::sea_query::Expr::value(Option::<String>::None),
            )
            .filter(source_snapshot_chapter::Column::VisualPlanId.is_in(plan_ids.clone()))
            .exec(db)
            .await?;
    }

    let mut script_delete = chapter_shooting_script::Entity::delete_many()
        .filter(chapter_shooting_script::Column::RootProjectId.eq(root_project_id));
    if matches!(
        scope,
        ProjectChapterAdaptationDeleteScope::CurrentProjectContent
    ) {
        script_delete = script_delete
            .filter(chapter_shooting_script::Column::SourceSnapshotId.is_null())
            .filter(chapter_shooting_script::Column::SourceSnapshotChapterId.is_null());
    }
    script_delete.exec(db).await?;

    if !plan_ids.is_empty() {
        chapter_visual_plan::Entity::delete_many()
            .filter(chapter_visual_plan::Column::Id.is_in(plan_ids))
            .exec(db)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use a4n_db_models::models::{
        chapter::{chapter, chapter_shooting_script, chapter_visual_plan},
        project::{project_source_snapshot, source_snapshot_chapter},
    };
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
    use serde_json::json;

    use super::*;

    struct AdaptationRows {
        root_project_id: i64,
        live_draft_id: i64,
        live_confirmed_id: i64,
        snapshot_plan_id: i64,
        live_script_id: i64,
        snapshot_script_id: i64,
        snapshot_chapter_id: i64,
    }

    async fn seed_adaptation_rows(db: &sea_orm::DatabaseConnection) -> AdaptationRows {
        let now = Utc::now().naive_utc();
        let root_project = crate::project::create_project_record(
            db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: "导演编排清理测试".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .unwrap();
        let root_project_id = root_project.id;
        let source_chapter = chapter::ActiveModel {
            project_id: Set(root_project_id),
            outline_id: Set(None),
            chapter_number: Set(1),
            title: Set(Some("第一章".to_string())),
            content: Set("正文".to_string()),
            word_count: Set(2),
            status: Set(chapter::STATUS_COMPLETED.to_string()),
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
        let source_chapter_id = source_chapter.id;
        let content_sha256 = "a".repeat(64);
        let snapshot = project_source_snapshot::ActiveModel {
            root_project_id: Set(root_project_id),
            source_version_hash: Set("b".repeat(64)),
            blueprint_version_id: Set(None),
            chapter_scope_json: Set(json!({"start": 1, "end": 1})),
            chapter_hashes_json: Set(json!([])),
            profile_hashes_json: Set(json!([])),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
        let snapshot_chapter = source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(snapshot.id),
            root_project_id: Set(root_project_id),
            source_chapter_id: Set(source_chapter_id),
            chapter_number: Set(1),
            title: Set(Some("第一章".to_string())),
            content: Set("正文".to_string()),
            summary: Set(None),
            analysis_data_json: Set(None),
            content_hash: Set(content_sha256.clone()),
            analysis_hash: Set(None),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
        let insert_plan = |source_snapshot_id,
                           source_snapshot_chapter_id,
                           source_revision_key: String,
                           plan_version,
                           status: &str| {
            chapter_visual_plan::ActiveModel {
                root_project_id: Set(root_project_id),
                source_chapter_id: Set(source_chapter_id),
                source_snapshot_id: Set(source_snapshot_id),
                source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
                source_revision_key: Set(source_revision_key),
                source_content_sha256: Set(content_sha256.clone()),
                contract_version: Set(
                    a4n_domain::chapter_visual_plan::CHAPTER_VISUAL_PLAN_V3.to_string()
                ),
                plan_version: Set(plan_version),
                revision: Set(1),
                status: Set(status.to_string()),
                authoring_mode: Set("manual".to_string()),
                plan_sha256: Set("c".repeat(64)),
                plan_json: Set(json!({})),
                author_user_id: Set(1),
                confirmed_at: Set((status == "confirmed").then_some(now)),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
        };
        let live_revision_key = format!("live:{source_chapter_id}:{content_sha256}");
        let snapshot_revision_key = format!(
            "snapshot:{}:chapter:{}:{}",
            snapshot.id, snapshot_chapter.id, content_sha256
        );
        let live_draft = insert_plan(None, None, live_revision_key.clone(), 1, "draft")
            .insert(db)
            .await
            .unwrap();
        let live_confirmed = insert_plan(None, None, live_revision_key.clone(), 2, "confirmed")
            .insert(db)
            .await
            .unwrap();
        let snapshot_plan = insert_plan(
            Some(snapshot.id),
            Some(snapshot_chapter.id),
            snapshot_revision_key.clone(),
            3,
            "confirmed",
        )
        .insert(db)
        .await
        .unwrap();
        let mut snapshot_chapter_update: source_snapshot_chapter::ActiveModel =
            snapshot_chapter.clone().into();
        snapshot_chapter_update.visual_plan_id = Set(Some(snapshot_plan.id));
        snapshot_chapter_update.visual_plan_version = Set(Some(snapshot_plan.plan_version));
        snapshot_chapter_update.visual_plan_sha256 = Set(Some(snapshot_plan.plan_sha256.clone()));
        snapshot_chapter_update.update(db).await.unwrap();
        let insert_script = |source_snapshot_id,
                             source_snapshot_chapter_id,
                             source_revision_key: String,
                             visual_plan_id,
                             visual_plan_version| {
            chapter_shooting_script::ActiveModel {
                root_project_id: Set(root_project_id),
                source_chapter_id: Set(source_chapter_id),
                source_snapshot_id: Set(source_snapshot_id),
                source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
                source_revision_key: Set(source_revision_key),
                source_content_sha256: Set(content_sha256.clone()),
                visual_plan_id: Set(Some(visual_plan_id)),
                visual_plan_version: Set(Some(visual_plan_version)),
                visual_plan_sha256: Set(Some("c".repeat(64))),
                input_fingerprint_sha256: Set("d".repeat(64)),
                contract_version: Set("chapter_shooting_script_v1".to_string()),
                revision: Set(1),
                script_sha256: Set("e".repeat(64)),
                script_json: Set(json!({})),
                workflow_run_id: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
        };
        let live_script = insert_script(
            None,
            None,
            live_revision_key,
            live_confirmed.id,
            live_confirmed.plan_version,
        )
        .insert(db)
        .await
        .unwrap();
        let snapshot_script = insert_script(
            Some(snapshot.id),
            Some(snapshot_chapter.id),
            snapshot_revision_key,
            snapshot_plan.id,
            snapshot_plan.plan_version,
        )
        .insert(db)
        .await
        .unwrap();
        AdaptationRows {
            root_project_id,
            live_draft_id: live_draft.id,
            live_confirmed_id: live_confirmed.id,
            snapshot_plan_id: snapshot_plan.id,
            live_script_id: live_script.id,
            snapshot_script_id: snapshot_script.id,
            snapshot_chapter_id: snapshot_chapter.id,
        }
    }

    #[tokio::test]
    async fn current_content_cleanup_preserves_snapshot_plan_script_and_identity() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let rows = seed_adaptation_rows(&db).await;

        delete_project_chapter_adaptation_records(
            &db,
            rows.root_project_id,
            ProjectChapterAdaptationDeleteScope::CurrentProjectContent,
        )
        .await
        .unwrap();

        assert!(
            chapter_visual_plan::Entity::find_by_id(rows.live_draft_id)
                .one(&db)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            chapter_visual_plan::Entity::find_by_id(rows.live_confirmed_id)
                .one(&db)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            chapter_shooting_script::Entity::find_by_id(rows.live_script_id)
                .one(&db)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            chapter_visual_plan::Entity::find_by_id(rows.snapshot_plan_id)
                .one(&db)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            chapter_shooting_script::Entity::find_by_id(rows.snapshot_script_id)
                .one(&db)
                .await
                .unwrap()
                .is_some()
        );
        let snapshot_chapter =
            source_snapshot_chapter::Entity::find_by_id(rows.snapshot_chapter_id)
                .one(&db)
                .await
                .unwrap()
                .unwrap();
        assert_eq!(snapshot_chapter.visual_plan_id, Some(rows.snapshot_plan_id));
    }

    #[tokio::test]
    async fn project_cleanup_clears_snapshot_identity_before_deleting_all_adaptation_rows() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let rows = seed_adaptation_rows(&db).await;

        delete_project_chapter_adaptation_records(
            &db,
            rows.root_project_id,
            ProjectChapterAdaptationDeleteScope::Project,
        )
        .await
        .unwrap();

        assert_eq!(
            chapter_visual_plan::Entity::find()
                .filter(chapter_visual_plan::Column::RootProjectId.eq(rows.root_project_id))
                .count(&db)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            chapter_shooting_script::Entity::find()
                .filter(chapter_shooting_script::Column::RootProjectId.eq(rows.root_project_id))
                .count(&db)
                .await
                .unwrap(),
            0
        );
        let snapshot_chapter =
            source_snapshot_chapter::Entity::find_by_id(rows.snapshot_chapter_id)
                .one(&db)
                .await
                .unwrap()
                .unwrap();
        assert_eq!(
            (
                snapshot_chapter.visual_plan_id,
                snapshot_chapter.visual_plan_version,
                snapshot_chapter.visual_plan_sha256,
            ),
            (None, None, None)
        );
    }

    #[tokio::test]
    async fn source_change_marks_only_live_heads_stale() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let rows = seed_adaptation_rows(&db).await;

        crate::chapter_visual_plan::invalidate_live_chapter_visual_plans_for_source_change(
            &db,
            rows.root_project_id,
            source_snapshot_chapter::Entity::find_by_id(rows.snapshot_chapter_id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .source_chapter_id,
        )
        .await
        .unwrap();

        for id in [rows.live_draft_id, rows.live_confirmed_id] {
            assert_eq!(
                chapter_visual_plan::Entity::find_by_id(id)
                    .one(&db)
                    .await
                    .unwrap()
                    .unwrap()
                    .status,
                "stale"
            );
        }
        assert_eq!(
            chapter_visual_plan::Entity::find_by_id(rows.snapshot_plan_id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .status,
            "confirmed"
        );
        assert_eq!(
            source_snapshot_chapter::Entity::find_by_id(rows.snapshot_chapter_id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .visual_plan_id,
            Some(rows.snapshot_plan_id)
        );
    }
}
