use a4n_core::models::chapter::chapter;
use sea_orm::{ActiveModelTrait, Set};

use super::*;

#[tokio::test]
async fn derived_video_project_reuses_live_script_for_identical_snapshot_content() {
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
    let video_project =
        insert_derived_video_project(db.clone(), root_project.id, "视频派生项目").await;
    let snapshot_source =
        a4n_db::project::resolve_chapter_source(db.as_ref(), video_project.id, root_chapter.id)
            .await
            .expect("snapshot chapter source should resolve");

    assert_eq!(snapshot_source.content_sha256, live_source.content_sha256);
    assert_ne!(
        snapshot_source.source_revision_key,
        live_source.source_revision_key
    );

    let service = crate::video::test_video_orchestrator(db).await;
    let resolved = service
        .require_current_chapter_shooting_script(
            crate::shooting_script::ChapterShootingScriptSourceInput {
                project_id: video_project.id,
                source_chapter_id: root_chapter.id,
            },
        )
        .await
        .expect("identical snapshot content should reuse the live shooting script");

    assert_eq!(resolved.shooting_script.record.id, live_script.record.id);
    assert_eq!(resolved.source, snapshot_source);
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
        })
        .await
        .expect("current shooting script query should succeed");

    assert!(resolved.is_none());
}
