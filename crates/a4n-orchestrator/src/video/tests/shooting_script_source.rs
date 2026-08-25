use a4n_core::models::{chapter::chapter, project::source_snapshot_chapter};
use a4n_domain::shooting_script::ShootingVisualPlanSourceV1;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use super::*;

#[tokio::test]
async fn derived_video_project_freezes_matching_plan_and_script_into_snapshot_identity() {
    let db = Arc::new(setup_db().await);
    let root_project = insert_video_project(db.as_ref(), "根文本项目").await;
    let root_chapter =
        insert_video_chapter_without_shooting_script(db.as_ref(), root_project.id, 1).await;
    let (live_plan, live_script) = insert_confirmed_visual_plan_and_shooting_script(
        db.as_ref(),
        root_project.id,
        &root_chapter,
    )
    .await;
    let video_project =
        insert_derived_video_project(db.clone(), root_project.id, "视频派生项目").await;
    let snapshot_source =
        a4n_db::project::resolve_chapter_source(db.as_ref(), video_project.id, root_chapter.id)
            .await
            .expect("snapshot chapter source should resolve");
    let snapshot_chapter = source_snapshot_chapter::Entity::find_by_id(
        snapshot_source
            .source_snapshot_chapter_id
            .expect("snapshot chapter id"),
    )
    .one(db.as_ref())
    .await
    .expect("snapshot chapter query")
    .expect("snapshot chapter");
    let snapshot_plan = a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(
        db.as_ref(),
        &snapshot_source,
    )
    .await
    .expect("snapshot visual plan query")
    .expect("snapshot visual plan");
    let snapshot_script =
        a4n_db::shooting_script::chapter_shooting_script_for_source(db.as_ref(), &snapshot_source)
            .await
            .expect("snapshot shooting script query")
            .expect("snapshot shooting script");

    assert_ne!(snapshot_plan.record.id, live_plan.record.id);
    assert_eq!(
        snapshot_plan.record.source_snapshot_id,
        snapshot_source.source_snapshot_id
    );
    assert_eq!(
        snapshot_plan.record.plan_sha256,
        live_plan.record.plan_sha256
    );
    assert_eq!(
        snapshot_chapter.visual_plan_id,
        Some(snapshot_plan.record.id)
    );
    assert_eq!(
        snapshot_script.record.source_revision_key,
        snapshot_source.source_revision_key
    );
    assert_ne!(snapshot_script.record.id, live_script.record.id);
    assert_eq!(
        snapshot_script.record.visual_plan_id,
        Some(snapshot_plan.record.id)
    );
    assert_eq!(
        snapshot_script.script.visual_plan_source,
        Some(ShootingVisualPlanSourceV1 {
            plan_id: snapshot_plan.record.id,
            plan_version: snapshot_plan.record.plan_version,
            plan_sha256: snapshot_plan.record.plan_sha256.clone(),
        })
    );

    let service = crate::video::test_video_orchestrator(db).await;
    let resolved = service
        .require_current_chapter_shooting_script(
            crate::shooting_script::ChapterShootingScriptSourceInput {
                project_id: video_project.id,
                source_chapter_id: root_chapter.id,
                visual_plan_id: Some(snapshot_plan.record.id),
            },
        )
        .await
        .expect("snapshot identity should select the frozen shooting script");
    assert_eq!(
        resolved.shooting_script.record.id,
        snapshot_script.record.id
    );
}

#[tokio::test]
async fn derived_video_project_does_not_reuse_script_after_source_content_changes() {
    let db = Arc::new(setup_db().await);
    let root_project = insert_video_project(db.as_ref(), "根文本项目").await;
    let root_chapter = insert_video_chapter(db.as_ref(), root_project.id, 1).await;
    let live_source =
        a4n_db::project::resolve_chapter_source(db.as_ref(), root_project.id, root_chapter.id)
            .await
            .expect("live chapter source should resolve");
    let live_script =
        a4n_db::shooting_script::chapter_shooting_script_for_source(db.as_ref(), &live_source)
            .await
            .expect("live shooting script query should succeed")
            .expect("live shooting script should exist");

    let mut chapter_update: chapter::ActiveModel = root_chapter.clone().into();
    chapter_update.content = Set("章节正文已经修改".to_string());
    chapter_update.word_count = Set(8);
    let updated_chapter = chapter_update
        .update(db.as_ref())
        .await
        .expect("chapter content should update");
    let video_project =
        insert_derived_video_project(db.clone(), root_project.id, "视频派生项目").await;
    let snapshot_source =
        a4n_db::project::resolve_chapter_source(db.as_ref(), video_project.id, updated_chapter.id)
            .await
            .expect("snapshot chapter source should resolve");

    assert_ne!(
        snapshot_source.content_sha256,
        live_script.record.source_content_sha256
    );

    let service = crate::video::test_video_orchestrator(db).await;
    let resolved = service
        .current_chapter_shooting_script(crate::shooting_script::ChapterShootingScriptSourceInput {
            project_id: video_project.id,
            source_chapter_id: updated_chapter.id,
            visual_plan_id: None,
        })
        .await
        .expect("current shooting script query should succeed");

    assert!(resolved.is_none());
}
