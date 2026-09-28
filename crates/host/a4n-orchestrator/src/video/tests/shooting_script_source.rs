use super::*;
use a4n_db_models::models::{chapter::chapter, project::source_snapshot_chapter};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

async fn empty_video(db: &DatabaseConnection, root_id: i64) -> novel_project::Model {
    create_derived_project_record(
        db,
        1,
        CreateDerivedProjectParams {
            root_project_id: root_id,
            derived_kind: "video".into(),
            title: "独立视频".into(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn shared_source_snapshot_does_not_allow_mutating_another_video_director() {
    use a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordInput;
    let db = Arc::new(setup_db().await);
    let producer = insert_video_project(db.as_ref(), "导演项目隔离").await;
    let chapter = insert_video_chapter_without_shooting_script(db.as_ref(), producer.id, 1).await;
    let (confirmed, _) =
        insert_confirmed_visual_plan_and_shooting_script(db.as_ref(), producer.id, &chapter).await;
    let source = a4n_db::project::resolve_chapter_source(db.as_ref(), producer.id, chapter.id)
        .await
        .unwrap();
    let draft = a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
        db.as_ref(),
        a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
            source: source.clone(),
            author_user_id: 1,
            authoring_mode: a4n_domain::chapter_visual_plan::ChapterVisualPlanAuthoringMode::Manual,
            expected: None,
            expected_source_hash: None,
            plan: confirmed.plan,
        },
    )
    .await
    .unwrap();
    let target = empty_video(db.as_ref(), producer.root_project_id.unwrap()).await;
    novel_project::ActiveModel {
        id: Set(target.id),
        source_snapshot_id: Set(source.source_snapshot_id),
        ..Default::default()
    }
    .update(db.as_ref())
    .await
    .unwrap();
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let input = || ChapterVisualPlanRecordInput {
        project_id: target.id,
        source_chapter_id: chapter.id,
        record_id: draft.record.id,
        expected_revision: draft.record.revision,
    };
    assert!(
        service
            .confirm_chapter_visual_plan_for_user_validated(1, input())
            .await
            .unwrap_err()
            .to_string()
            .contains("不匹配")
    );
    assert!(
        service
            .discard_chapter_visual_plan_draft_for_user_validated(1, input())
            .await
            .unwrap_err()
            .to_string()
            .contains("不匹配")
    );
    assert!(
        service
            .visual_plan_generation_source(
                1,
                target.id,
                chapter.id,
                draft.record.id,
                draft.record.revision
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("不匹配")
    );
    let retained =
        a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(db.as_ref(), draft.record.id)
            .await
            .unwrap();
    assert_eq!(retained.record.status, "draft");
    assert_eq!(retained.record.revision, draft.record.revision);
}

#[tokio::test]
async fn batched_shooting_script_reads_preserve_missing_and_duplicate_source_positions() {
    let db = setup_db().await;
    let video = insert_video_project(&db, "批量拍摄剧本读取").await;
    let complete = insert_video_chapter(&db, video.id, 1).await;
    let missing = insert_video_chapter_without_shooting_script(&db, video.id, 2).await;
    let complete_source = a4n_db::project::resolve_chapter_source(&db, video.id, complete.id)
        .await
        .unwrap();
    let missing_source = a4n_db::project::resolve_chapter_source(&db, video.id, missing.id)
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
async fn partial_progress_and_published_scripts_stay_with_their_video_project() {
    use a4n_api_contract::shooting_script::{
        ChapterShootingScriptUnitProgressCheckpoint, ChapterShootingScriptUnitProgressView,
    };
    use a4n_db::workflow::{persist_workflow_progress_snapshot, upsert_workflow_run};
    let db = Arc::new(setup_db().await);
    let producer = insert_video_project(db.as_ref(), "部分拍摄设计").await;
    let chapter = insert_video_chapter_without_shooting_script(db.as_ref(), producer.id, 1).await;
    let (plan, script) =
        insert_confirmed_visual_plan_and_shooting_script(db.as_ref(), producer.id, &chapter).await;
    a4n_db_models::models::chapter::chapter_shooting_script::Entity::delete_by_id(script.record.id)
        .exec(db.as_ref())
        .await
        .unwrap();
    let target = empty_video(db.as_ref(), producer.root_project_id.unwrap()).await;
    let source = a4n_db::project::resolve_chapter_source(db.as_ref(), producer.id, chapter.id)
        .await
        .unwrap();
    let run = upsert_workflow_run(db.as_ref(), serde_json::from_value(json!({
        "business_key":format!("partial-scene:{}:{}", producer.id, chapter.id), "workflow_kind":"chapter_shooting_script",
        "project_id":producer.id, "chapter_id":chapter.id, "status":"running", "can_resume":true,
        "input_fingerprint_json":{"project_id":producer.id,"source_chapter_id":chapter.id,"visual_plan_id":plan.record.id,
            "visual_plan_version":plan.record.plan_version,"visual_plan_sha256":plan.record.plan_sha256},
    })).unwrap()).await.unwrap();
    let count = script.script.scenes.len();
    let checkpoint = ChapterShootingScriptUnitProgressCheckpoint {
        version: 1,
        source_content_sha256: source.content_sha256,
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
    let input = |project_id| crate::shooting_script::ChapterShootingScriptSourceInput {
        project_id,
        source_chapter_id: chapter.id,
        visual_plan_id: None,
    };
    assert!(
        service
            .current_chapter_shooting_script_unit_progress(input(target.id))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        service
            .current_chapter_shooting_script_unit_progress(input(producer.id))
            .await
            .unwrap()
            .is_some()
    );
    let (_, independent) =
        insert_confirmed_visual_plan_and_shooting_script(db.as_ref(), target.id, &chapter).await;
    for field in ["source_content_sha256", "plan_sha256"] {
        let mut stale = checkpoint.clone();
        if field == "source_content_sha256" {
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
                .current_chapter_shooting_script_unit_progress(input(producer.id))
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            service
                .require_current_chapter_shooting_script(input(target.id))
                .await
                .unwrap()
                .shooting_script,
            independent
        );
    }
}

#[tokio::test]
async fn creating_a_video_captures_text_without_copying_another_videos_authoring() {
    let db = Arc::new(setup_db().await);
    let producer = insert_video_project(db.as_ref(), "已编排视频").await;
    let chapter = insert_video_chapter(db.as_ref(), producer.id, 1).await;
    let target = empty_video(db.as_ref(), producer.root_project_id.unwrap()).await;
    let source = a4n_db::project::resolve_chapter_source(db.as_ref(), target.id, chapter.id)
        .await
        .unwrap();
    let frozen =
        source_snapshot_chapter::Entity::find_by_id(source.source_snapshot_chapter_id.unwrap())
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
    assert_eq!(frozen.content, chapter.content);
    assert_eq!(frozen.content_hash, source.content_sha256);
    let json = serde_json::to_value(frozen).unwrap();
    for field in [
        "visual_plan_id",
        "visual_plan_version",
        "visual_plan_sha256",
    ] {
        assert!(json.get(field).is_none());
    }
    assert!(
        a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(db.as_ref(), &source)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        a4n_db::shooting_script::chapter_shooting_script_for_source(db.as_ref(), &source)
            .await
            .unwrap()
            .is_none()
    );
    let snapshot = project_source_snapshot::Entity::find_by_id(source.source_snapshot_id.unwrap())
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let producer_source =
        a4n_db::project::resolve_chapter_source(db.as_ref(), producer.id, chapter.id)
            .await
            .unwrap();
    let mut plan = a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(
        db.as_ref(),
        &producer_source,
    )
    .await
    .unwrap()
    .unwrap()
    .plan;
    plan.points[0].label = "修改导演意图".into();
    let draft = a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
        db.as_ref(),
        a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
            source: producer_source,
            author_user_id: 1,
            authoring_mode: a4n_domain::chapter_visual_plan::ChapterVisualPlanAuthoringMode::Manual,
            expected: None,
            expected_source_hash: None,
            plan,
        },
    )
    .await
    .unwrap();
    a4n_db::chapter_visual_plan::confirm_chapter_visual_plan(
        db.as_ref(),
        a4n_domain::chapter_visual_plan::ConfirmChapterVisualPlanInput {
            record_id: draft.record.id,
            expected_revision: draft.record.revision,
        },
    )
    .await
    .unwrap();
    let next = empty_video(db.as_ref(), producer.root_project_id.unwrap()).await;
    let next_snapshot =
        project_source_snapshot::Entity::find_by_id(next.source_snapshot_id.unwrap())
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
    assert_eq!(
        snapshot.source_version_hash, next_snapshot.source_version_hash,
        "导演修改不能改变原文快照版本"
    );
}

#[tokio::test]
async fn source_edits_do_not_replace_existing_video_scripts_or_seed_new_ones() {
    let db = Arc::new(setup_db().await);
    let producer = insert_video_project(db.as_ref(), "已有视频").await;
    let chapter = insert_video_chapter(db.as_ref(), producer.id, 1).await;
    let producer_source =
        a4n_db::project::resolve_chapter_source(db.as_ref(), producer.id, chapter.id)
            .await
            .unwrap();
    let script =
        a4n_db::shooting_script::chapter_shooting_script_for_source(db.as_ref(), &producer_source)
            .await
            .unwrap()
            .unwrap();
    let mut changed: chapter::ActiveModel = chapter.clone().into();
    changed.content = Set("章节正文已经修改".into());
    changed.update(db.as_ref()).await.unwrap();
    let target = empty_video(db.as_ref(), producer.root_project_id.unwrap()).await;
    let source = a4n_db::project::resolve_chapter_source(db.as_ref(), target.id, chapter.id)
        .await
        .unwrap();
    assert_ne!(source.content_sha256, script.record.source_content_sha256);
    assert!(
        a4n_db::shooting_script::chapter_shooting_script_for_source(db.as_ref(), &source)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        a4n_db::shooting_script::chapter_shooting_script_for_source(db.as_ref(), &producer_source)
            .await
            .unwrap()
            .unwrap(),
        script
    );
}
