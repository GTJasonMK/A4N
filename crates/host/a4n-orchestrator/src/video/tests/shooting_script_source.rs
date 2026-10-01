use a4n_db_models::models::{chapter::chapter, project::source_snapshot_chapter};
use a4n_domain::shooting_script::ShootingVisualPlanSourceV1;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use super::*;

#[tokio::test]
async fn batched_shooting_script_reads_preserve_missing_and_duplicate_source_positions() {
    let db = setup_db().await;
    let root = insert_video_project(&db, "批量拍摄剧本读取").await;
    let complete = insert_video_chapter(&db, root.id, 1).await;
    let missing = insert_video_chapter_without_shooting_script(&db, root.id, 2).await;
    let complete_source = a4n_db::project::resolve_chapter_source(&db, root.id, complete.id)
        .await
        .unwrap();
    let missing_source = a4n_db::project::resolve_chapter_source(&db, root.id, missing.id)
        .await
        .unwrap();
    let scripts = a4n_db::shooting_script::chapter_shooting_scripts_for_sources(
        &db,
        &[missing_source, complete_source.clone(), complete_source],
    )
    .await
    .unwrap();
    assert_eq!(scripts.len(), 3);
    assert!(scripts[0].is_none());
    assert_eq!(
        scripts[1].as_ref().unwrap().record.source_chapter_id,
        complete.id
    );
    assert_eq!(scripts[1], scripts[2]);
}

#[tokio::test]
async fn source_snapshot_freezes_completed_scenes_without_following_live_progress() {
    use a4n_api_contract::shooting_script::{
        ChapterShootingScriptUnitProgressCheckpoint, ChapterShootingScriptUnitProgressView,
    };
    use a4n_db::workflow::{persist_workflow_progress_snapshot, upsert_workflow_run};

    let db = Arc::new(setup_db().await);
    let root = insert_video_project(db.as_ref(), "部分拍摄设计根项目").await;
    let chapter = insert_video_chapter_without_shooting_script(db.as_ref(), root.id, 1).await;
    let (plan, script) =
        insert_confirmed_visual_plan_and_shooting_script(db.as_ref(), root.id, &chapter).await;
    a4n_db_models::models::chapter::chapter_shooting_script::Entity::delete_by_id(script.record.id)
        .exec(db.as_ref())
        .await
        .unwrap();
    let video = insert_derived_video_project(db.clone(), root.id, "部分场景视频项目").await;
    let source = a4n_db::project::resolve_chapter_source(db.as_ref(), video.id, chapter.id)
        .await
        .unwrap();
    let snapshot_plan =
        a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(db.as_ref(), &source)
            .await
            .unwrap()
            .unwrap();
    assert_ne!(snapshot_plan.record.id, plan.record.id);
    assert_eq!(snapshot_plan.record.plan_sha256, plan.record.plan_sha256);
    let run = upsert_workflow_run(
        db.as_ref(),
        serde_json::from_value(json!({
            "business_key": format!("partial-scene:{}:{}", root.id, chapter.id),
            "workflow_kind": "chapter_shooting_script",
            "project_id": root.id,
            "chapter_id": chapter.id,
            "input_fingerprint_json": {"visual_plan_id": plan.record.id},
            "status": "running",
            "can_resume": true,
        }))
        .unwrap(),
    )
    .await
    .unwrap();
    let count = script.script.scenes.len();
    let checkpoint = ChapterShootingScriptUnitProgressCheckpoint {
        version: 1,
        source_content_sha256: source.content_sha256.clone(),
        progress: ChapterShootingScriptUnitProgressView {
            workflow_run_id: run.id,
            script: script.script.clone(),
            completed_unit_count: count,
            total_unit_count: count,
        },
    };
    persist_workflow_progress_snapshot(
        db.as_ref(),
        run.id,
        serde_json::to_value(&checkpoint).unwrap(),
    )
    .await
    .unwrap();
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let input = crate::shooting_script::ChapterShootingScriptSourceInput {
        project_id: video.id,
        source_chapter_id: chapter.id,
        visual_plan_id: None,
    };
    assert!(
        service
            .current_chapter_shooting_script_unit_progress(input.clone())
            .await
            .unwrap()
            .is_none(),
        "an old snapshot must not borrow a live workflow"
    );
    let live = service
        .current_chapter_shooting_script_unit_progress(
            crate::shooting_script::ChapterShootingScriptSourceInput {
                project_id: root.id,
                ..input.clone()
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        live.script.visual_plan_source,
        script.script.visual_plan_source
    );

    let frozen_project = insert_derived_video_project(db.clone(), root.id, "冻结已完成场景").await;
    let frozen_input = crate::shooting_script::ChapterShootingScriptSourceInput {
        project_id: frozen_project.id,
        ..input.clone()
    };
    let frozen = service
        .require_current_chapter_shooting_script(frozen_input.clone())
        .await
        .unwrap();
    assert_eq!(frozen.shooting_script.script.scenes, script.script.scenes);
    assert_ne!(
        frozen.shooting_script.record.visual_plan_id,
        Some(plan.record.id)
    );

    for changed_field in ["source_content_sha256", "plan_sha256"] {
        let mut stale = checkpoint.clone();
        if changed_field == "source_content_sha256" {
            stale.source_content_sha256 = "f".repeat(64);
        } else {
            stale
                .progress
                .script
                .visual_plan_source
                .as_mut()
                .unwrap()
                .plan_sha256 = "f".repeat(64);
        }
        persist_workflow_progress_snapshot(
            db.as_ref(),
            run.id,
            serde_json::to_value(stale).unwrap(),
        )
        .await
        .unwrap();
        assert!(
            service
                .current_chapter_shooting_script_unit_progress(
                    crate::shooting_script::ChapterShootingScriptSourceInput {
                        project_id: root.id,
                        ..input.clone()
                    }
                )
                .await
                .unwrap()
                .is_none(),
            "{changed_field} mismatch must remain stale"
        );
        assert_eq!(
            service
                .require_current_chapter_shooting_script(frozen_input.clone())
                .await
                .unwrap()
                .shooting_script,
            frozen.shooting_script,
            "frozen scenes must not change with root progress",
        );
    }

    let run = a4n_db::workflow::workflow_run_model_by_id(db.as_ref(), run.id)
        .await
        .unwrap();
    let mut finished: a4n_db_models::models::generation::workflow_run::ActiveModel = run.into();
    finished.status = Set("completed".into());
    finished.can_resume = Set(false);
    finished.update(db.as_ref()).await.unwrap();
    assert_eq!(
        service
            .require_current_chapter_shooting_script(frozen_input)
            .await
            .unwrap()
            .shooting_script,
        frozen.shooting_script,
        "completed root workflows must not make frozen scenes disappear",
    );
}

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
