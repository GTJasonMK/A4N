use a4n_db::chapter_visual_plan::{
    SaveChapterVisualPlanDraftInput, chapter_visual_plan_by_id, save_chapter_visual_plan_draft,
};
use a4n_db_models::models::chapter::chapter;
use a4n_domain::chapter::ImportChapterContentInput;
use a4n_domain::chapter_visual_plan::{
    ChapterVisualPlanAuthoringMode, ChapterVisualPlanV3, ExpectedVisualPlanDraft,
};
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};
use std::time::Duration;

#[tokio::test]
async fn director_resources_are_family_scoped_and_survive_supersession() {
    use a4n_db::chapter_visual_plan::confirm_chapter_visual_plan;
    use a4n_db_models::models::{ai::generated_image, scene::scene_profile};
    use a4n_domain::{
        chapter_visual_plan::{ConfirmChapterVisualPlanInput, SceneBindingV3},
        scene_profile::SceneProfileRecordInput,
    };
    use sea_orm::EntityTrait;

    let db = a4n_migration::test_support::connect_test_database().await;
    let root = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: 1,
            title: "Director resources".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let media_project = a4n_db::project::create_freeform_video_project_record(
        &db,
        1,
        a4n_db::project::CreateFreeformVideoProjectParams {
            title: "Director media".into(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id: root.id,
            chapter_number: 1,
            content: "走到门口。".into(),
            title: None,
        },
        "director resources test",
    )
    .await
    .unwrap();
    let source = a4n_db::project::resolve_chapter_source(&db, root.id, chapter.id)
        .await
        .unwrap();
    let image = a4n_db::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: media_project.id,
            image_type: "director_storyboard_frame".into(),
            file_path: format!("projects/{}/images/director.png", media_project.id),
            prompt: "站在门前".into(),
            model: "test".into(),
            width: 512,
            height: 512,
            file_size: Some(1),
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap();
    let scene = a4n_db::scene_profile::create_scene_profile_record(
        &db,
        media_project.id,
        SceneProfileRecordInput {
            name: "门口".into(),
            normalized_name: "door".into(),
            aliases_json: serde_json::json!([]),
            scene_type: "room".into(),
            importance: "primary".into(),
            parent_scene_id: None,
            base_description: "安静的门口".into(),
            visual_description: "木门".into(),
            spatial_layout: "走廊通向门口".into(),
            atmosphere: "安静".into(),
            entry_points_json: serde_json::json!([]),
            rules_json: serde_json::json!([]),
            first_seen_chapter: 1,
            last_seen_chapter: 1,
            evidence_json: serde_json::json!([]),
            image_prompt_seed: "木门".into(),
        },
        "user",
    )
    .await
    .unwrap();
    let mut input = SaveChapterVisualPlanDraftInput {
        source,
        author_user_id: 1,
        authoring_mode: ChapterVisualPlanAuthoringMode::Manual,
        expected: None,
        expected_source_hash: None,
        plan: ChapterVisualPlanV3::starter(),
    };
    input.plan.points[0].image_id = Some(image.id);
    assert!(
        save_chapter_visual_plan_draft(&db, input.clone())
            .await
            .unwrap_err()
            .to_string()
            .contains("图片不存在或不属于")
    );
    input.plan.points[0].image_id = None;
    input.plan.segments[0].prompt = "缓缓走到门前。".into();
    input.plan.segments[0].scene_binding = Some(SceneBindingV3::Scene {
        scene_profile_id: scene.id,
    });
    assert!(
        save_chapter_visual_plan_draft(&db, input.clone())
            .await
            .unwrap_err()
            .to_string()
            .contains("场景不存在或不属于")
    );

    // Move the fixture resources into a real derived project with a source snapshot.
    let media_project = a4n_db::project::create_derived_project_record(
        &db,
        1,
        a4n_db::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "video".into(),
            title: "Family media".into(),
            description: String::new(),
            chapter_scope: a4n_db::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let mut image_row: generated_image::ActiveModel = image.clone().into();
    image_row.project_id = Set(media_project.id);
    image_row.file_path = Set(format!("projects/{}/images/director.png", media_project.id));
    image_row.update(&db).await.unwrap();
    let mut scene_row: scene_profile::ActiveModel = scene.clone().into();
    scene_row.project_id = Set(media_project.id);
    scene_row.update(&db).await.unwrap();
    input.plan.points[0].image_id = Some(image.id);
    input.plan.points[1].reference_image_ids = vec![image.id];
    let saved = save_chapter_visual_plan_draft(&db, input.clone())
        .await
        .unwrap();
    assert!(
        a4n_db::video::delete_generated_image_record_with_cleanup(&db, image.id)
            .await
            .unwrap_err()
            .to_string()
            .contains("导演编排引用")
    );
    let mut deleting: a4n_db_models::models::project::novel_project::ActiveModel =
        media_project.clone().into();
    deleting.lifecycle_gate = Set(a4n_domain::generation::ProjectLifecycleOperation::Delete
        .gate()
        .to_string());
    deleting.update(&db).await.unwrap();
    assert!(
        a4n_db::project::delete::delete_project_cascade_record(&db, media_project.id)
            .await
            .unwrap_err()
            .to_string()
            .contains("导演编排引用")
    );
    let mut restored: a4n_db_models::models::project::novel_project::ActiveModel =
        media_project.clone().into();
    restored.lifecycle_gate = Set(media_project.lifecycle_gate.clone());
    restored.update(&db).await.unwrap();
    assert!(
        a4n_db::scene_profile::delete_scene_profile_records(&db, scene.id)
            .await
            .unwrap_err()
            .to_string()
            .contains("导演编排引用")
    );
    let confirmed = confirm_chapter_visual_plan(
        &db,
        ConfirmChapterVisualPlanInput {
            record_id: saved.record.id,
            expected_revision: saved.record.revision,
        },
    )
    .await
    .unwrap()
    .visual_plan;
    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );

    input.plan.points.iter_mut().for_each(|point| {
        point.image_id = None;
        point.reference_image_ids.clear();
    });
    let next = save_chapter_visual_plan_draft(&db, input).await.unwrap();
    confirm_chapter_visual_plan(
        &db,
        ConfirmChapterVisualPlanInput {
            record_id: next.record.id,
            expected_revision: next.record.revision,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        chapter_visual_plan_by_id(&db, confirmed.record.id)
            .await
            .unwrap()
            .record
            .status,
        "superseded"
    );
    assert!(
        a4n_db::video::delete_generated_image_record_with_cleanup(&db, image.id)
            .await
            .unwrap_err()
            .to_string()
            .contains("导演编排引用")
    );
}

#[tokio::test]
async fn source_precondition_is_checked_inside_the_save_transaction() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: 1,
            title: "Agent source CAS".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id: project.id,
            chapter_number: 1,
            content: "原文版本一".into(),
            title: None,
        },
        "Agent source CAS test",
    )
    .await
    .unwrap();
    let source = a4n_db::project::resolve_chapter_source(&db, project.id, chapter.id)
        .await
        .unwrap();
    let mut input = SaveChapterVisualPlanDraftInput {
        expected_source_hash: Some(source.content_sha256.clone()),
        source,
        author_user_id: 1,
        authoring_mode: ChapterVisualPlanAuthoringMode::Collaborative,
        expected: None,
        plan: ChapterVisualPlanV3::starter(),
    };
    let original = save_chapter_visual_plan_draft(&db, input.clone())
        .await
        .unwrap();
    input.expected = Some(ExpectedVisualPlanDraft {
        record_id: original.record.id,
        revision: original.record.revision,
    });
    assert_eq!(
        save_chapter_visual_plan_draft(&db, input.clone())
            .await
            .unwrap()
            .record
            .revision,
        original.record.revision
    );
    input.plan.frame.width = 1280;

    // Hold a real concurrent source update open. The plan save must wait for
    // it, then reject the old hash rather than publish the stale rewrite.
    let tx = db.begin().await.unwrap();
    let mut changed: chapter::ActiveModel = chapter.into();
    changed.content = Set("原文版本二".into());
    changed.update(&tx).await.unwrap();
    let save_db = db.clone();
    let mut pending =
        tokio::spawn(async move { save_chapter_visual_plan_draft(&save_db, input).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut pending)
            .await
            .is_err()
    );
    tx.commit().await.unwrap();
    let error = pending.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("原文版本已变化"));
    let current = chapter_visual_plan_by_id(&db, original.record.id)
        .await
        .unwrap();
    assert_eq!(current.record.revision, original.record.revision);
    assert_eq!(current.plan, original.plan);
}
