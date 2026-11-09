use a4n_db_models::models::{
    ai::generated_image,
    chapter::{
        chapter,
        chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE},
    },
    task::cleanup_outbox,
};
use a4n_domain::manga::{
    MangaPagePromptSyncDraft, MangaPipelineArtifactDraft, MangaPipelineArtifactStage,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

use crate::manga::{
    MangaPipelineArtifactProjectionReset, MangaPipelineArtifactProjectionUpdate,
    MangaPipelineDbError, MangaPromptRecordError, delete_manga_page_by_number,
    delete_manga_pipeline_artifact, delete_manga_prompts_by_chapter,
    manga_page_prompt_statuses_by_chapter, manga_page_prompt_statuses_by_chapters,
    manga_pipeline_artifact, manga_pipeline_artifact_deletion_plan,
    manga_pipeline_artifacts_by_chapter, save_manga_pipeline_artifact_with_projection,
    sync_manga_page_prompts_by_chapter, update_manga_page_prompt_layout,
    upsert_manga_pipeline_artifact,
};

async fn setup_db() -> DatabaseConnection {
    let db = a4n_migration::test_support::connect_test_database().await;
    let owner = crate::auth::create_user(
        &db,
        "manga-test-owner",
        "manga-test-owner@example.test",
        "test-password",
        a4n_domain::auth::ROLE_USER,
    )
    .await
    .expect("manga test owner must be created");
    assert_eq!(owner.id, 1);
    db
}

async fn project_and_chapter(db: &impl crate::DbExecutor, title: &str) -> (i64, i64) {
    let project = crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = fixed_time(1);
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let comic = crate::project::create_derived_project_record(
        db,
        1,
        crate::project::CreateDerivedProjectParams {
            root_project_id: project.id,
            derived_kind: "comic".into(),
            title: title.into(),
            description: String::new(),
            chapter_scope: crate::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    (comic.id, chapter.id)
}

async fn standalone_comic_project_and_chapter(db: &DatabaseConnection, title: &str) -> (i64, i64) {
    use a4n_db_models::models::canvas::{canvas_text_asset, canvas_text_asset_version};

    a4n_migration::migrations::canvas_project_split::ensure_complete(db)
        .await
        .expect("测试库应完成 Canvas 项目拆分迁移");
    let project = crate::project::create_standalone_comic_project_record(
        db,
        1,
        crate::project::CreateStandaloneComicProjectParams {
            title: title.to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let content = "她推开门，桌上放着一封信。";
    let content_hash = a4n_domain::source_text_hash(content);
    let now = fixed_time(1);
    let asset = canvas_text_asset::ActiveModel {
        project_id: Set(project.id),
        title: Set("文本素材".to_string()),
        text_kind: Set("script".to_string()),
        content: Set(content.to_string()),
        content_hash: Set(content_hash.clone()),
        current_version: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    canvas_text_asset_version::ActiveModel {
        canvas_text_asset_id: Set(asset.id),
        version: Set(1),
        content: Set(content.to_string()),
        content_hash: Set(content_hash),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let chapter = crate::project::prepare_standalone_comic_source_chapter(
        db,
        crate::project::PrepareStandaloneComicSourceDbInput {
            project_id: project.id,
            asset_id: asset.id,
            version: 1,
            start_section: 0,
            end_section: 0,
        },
    )
    .await
    .unwrap();
    (project.id, chapter.id)
}

fn fixed_time(day: u32) -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 5, day)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

fn prompt_draft(
    chapter_id: i64,
    project_id: i64,
    page_number: i32,
    image_prompt: &str,
) -> MangaPagePromptSyncDraft {
    MangaPagePromptSyncDraft {
        chapter_id,
        project_id,
        page_number,
        image_prompt: image_prompt.to_string(),
        dialogue_text: None,
        characters_in_frame: None,
        layout: Some(page_layout(page_number)),
        status: "pending".to_string(),
    }
}

fn page_layout(page_number: i32) -> serde_json::Value {
    json!({
        "schema_version": "manga_page_generation_v3",
        "page_plan": {
            "page_number": page_number,
            "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
            "role": "setup",
            "pacing": "medium",
            "importance": "normal",
            "reading_flow": "top_to_bottom_left_to_right",
            "page_goal": "推进场景",
            "visual_thesis": "角色对峙",
            "entry_state": "平静",
            "exit_state": "紧张",
            "dominant_panel_number": 1,
            "panels": [{
                "panel_number": 1,
                "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
                "function": "dialogue",
                "size": "dominant",
                "shot_scale": "medium",
                "camera_angle": "eye_level",
                "participants": ["hero"],
                "composition": "单人构图",
                "observable_action": "抬头",
                "reaction": null,
                "spatial_relationship": "中央",
                "continuity_from_previous": null,
                "text_layers": []
            }]
        },
        "drawing_prompt": {
            "schema_version": "manga_page_drawing_prompt_v2",
            "page_number": page_number,
            "image_prompt": "整页"
        },
        "reference_bindings": [],
        "prompt_fingerprint": {"hash": "sha256"},
        "source": {
            "root_project_id": 1,
            "source_chapter_id": 2,
            "source_snapshot_id": null,
            "source_snapshot_chapter_id": null,
            "source_revision_key": "live:2:content-sha256",
            "content_sha256": "content-sha256",
            "adaptation_plan_id": 71,
            "adaptation_plan_revision": 2,
            "adaptation_plan_sha256": "plan-sha256"
        }
    })
}

async fn planned_drafts(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
) -> (
    Vec<MangaPagePromptSyncDraft>,
    a4n_domain::media_authoring::AuthoringDocumentRecord,
) {
    use crate::media_authoring::{
        SaveAuthoringDocument, adopt_authoring_document, save_authoring_document,
    };
    use a4n_domain::media_authoring::{
        AdaptationPlanV1, AuthoringDocumentInputs, AuthoringDocumentKind, AuthoringSourceRef,
    };
    let source = crate::project::resolve_chapter_source(db, project_id, chapter_id)
        .await
        .unwrap();
    let inputs = AuthoringDocumentInputs {
        sources: vec![AuthoringSourceRef {
            root_project_id: source.root_project_id,
            source_chapter_id: chapter_id,
            source_snapshot_id: source.source_snapshot_id.unwrap(),
            source_snapshot_chapter_id: source.source_snapshot_chapter_id.unwrap(),
            content_sha256: source.content_sha256.clone(),
        }],
        video_reference: None,
        dependencies: Vec::new(),
        preprocessing_fingerprint: None,
    };
    let document = json!({"schema_version":"adaptation_plan_v1", "target":"manga", "intention":"保留已有页面", "decisions":[],
        "units":[{"id":"unit-1", "origin":"historical", "title":"对峙", "location":null, "participants":[],
            "dramatic_question":"如何回应", "objective":"确认", "obstacle":"沉默", "entry_state":"平静", "exit_state":"紧张", "source_span_ids":[],
            "beats":[{"id":"beat-1", "kind":"action", "is_climax":false, "origin":"historical", "importance":"required",
                "action":"抬头", "reaction":null, "dialogue_intent":null, "entry_state":"平静", "exit_state":"紧张", "source_span_ids":[]}]}]});
    serde_json::from_value::<AdaptationPlanV1>(document.clone())
        .unwrap()
        .validate(&Default::default())
        .unwrap();
    let draft = save_authoring_document(
        db,
        SaveAuthoringDocument {
            project_id,
            source_snapshot_id: source.source_snapshot_id.unwrap(),
            source_chapter_id: Some(chapter_id),
            kind: AuthoringDocumentKind::AdaptationPlan,
            record_id: None,
            expected_revision: None,
            inputs: inputs.clone(),
            document,
        },
    )
    .await
    .unwrap();
    let adaptation = adopt_authoring_document(db, project_id, draft.id, draft.revision)
        .await
        .unwrap();
    let mut drafts = vec![
        prompt_draft(chapter_id, project_id, 1, "镜头一"),
        prompt_draft(chapter_id, project_id, 2, "镜头二"),
    ];
    let page_plan = json!({"schema_version":"manga_page_plan_v4", "adaptation_plan_id":adaptation.id, "adaptation_plan_revision":adaptation.revision,
        "pages":drafts.iter().map(|draft| draft.layout.as_ref().unwrap()["page_plan"].clone()).collect::<Vec<_>>()});
    serde_json::from_value::<a4n_domain::manga::MangaPagePlanV4>(page_plan.clone())
        .unwrap()
        .validate()
        .unwrap();
    let page_plan = save_authoring_document(
        db,
        SaveAuthoringDocument {
            project_id,
            source_snapshot_id: source.source_snapshot_id.unwrap(),
            source_chapter_id: Some(chapter_id),
            kind: AuthoringDocumentKind::MangaPagePlan,
            record_id: None,
            expected_revision: None,
            inputs: AuthoringDocumentInputs {
                dependencies: vec![adaptation.reference()],
                ..inputs
            },
            document: page_plan,
        },
    )
    .await
    .unwrap();
    let page_plan = adopt_authoring_document(db, project_id, page_plan.id, page_plan.revision)
        .await
        .unwrap();
    for draft in &mut drafts {
        let layout = draft.layout.as_mut().unwrap();
        layout["source"] = json!({"root_project_id":source.root_project_id, "source_chapter_id":chapter_id,
            "source_snapshot_id":source.source_snapshot_id, "source_snapshot_chapter_id":source.source_snapshot_chapter_id,
            "source_revision_key":source.source_revision_key, "content_sha256":source.content_sha256,
            "adaptation_plan_id":adaptation.id, "adaptation_plan_revision":adaptation.revision, "adaptation_plan_sha256":adaptation.sha256});
        layout["page_plan_ref"] = serde_json::to_value(page_plan.reference()).unwrap();
    }
    (drafts, adaptation)
}

fn upgrade_to_pipeline_layouts(drafts: &mut [MangaPagePromptSyncDraft]) {
    for draft in drafts {
        let previous = draft.layout.take().expect("page layout");
        let page_number = previous["page_plan"]["page_number"]
            .as_u64()
            .expect("page number") as u32;
        let page_id = format!("page-{page_number:04}");
        let source = previous["source"].clone();
        let source_hash = source["content_sha256"].as_str().expect("source hash");
        draft.layout = Some(json!({
            "schema_version": "manga_page_generation_v4",
            "page_id": page_id,
            "page_number": page_number,
            "page_plan_ref": previous["page_plan_ref"],
            "pipeline": {
                "schema_version": "manga_page_pipeline_state_v1",
                "page_id": page_id,
                "status": "pending",
                "stage": "content",
                "attempt": 1,
                "source_fingerprint": source_hash,
                "stage_fingerprints": {"content": "content-fingerprint"},
                "page_content": {
                    "summary": "推进冲突",
                    "source_span_ids": [],
                    "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
                    "action": "抬头",
                    "reaction": null,
                    "dialogue_intent": null,
                    "entry_state": "平静",
                    "exit_state": "紧张"
                }
            },
            "reference_bindings": previous["reference_bindings"],
            "prompt_fingerprint": previous["prompt_fingerprint"],
            "source": source
        }));
    }
}

#[tokio::test]
async fn sync_page_prompts_preserves_images_and_unmentioned_pages() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画提示词同步").await;
    let old_time = fixed_time(1);
    let (drafts, adaptation) = planned_drafts(&db, project_id, chapter_id).await;
    let kept = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(1),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.to_string()),
        image_prompt: Set("镜头一".to_string()),
        layout: Set(drafts[0].layout.clone()),
        generated_image_id: Set(Some(9901)),
        status: Set("completed".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let stale = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(9),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.to_string()),
        image_prompt: Set("旧镜头".to_string()),
        status: Set("pending".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let mut wrong_source = drafts.clone();
    wrong_source[0].layout.as_mut().unwrap()["source"]["adaptation_plan_id"] = json!(999);
    assert!(
        sync_manga_page_prompts_by_chapter(
            &db,
            project_id,
            chapter_id,
            wrong_source,
            fixed_time(2)
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("页面来源")
    );
    let mut changed_drafts = drafts.clone();
    changed_drafts[0].image_prompt = "已修改绘图提示词".to_string();
    let rows = sync_manga_page_prompts_by_chapter(
        &db,
        project_id,
        chapter_id,
        changed_drafts,
        fixed_time(2),
    )
    .await
    .unwrap();

    assert_eq!(rows.len(), 2);
    let stored = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(stored.len(), 3);
    assert_eq!(stored[0].id, kept.id);
    assert_eq!(stored[0].generated_image_id, Some(9901));
    assert_eq!(stored[0].status, "completed");
    assert!(stored.iter().any(|row| row.id == stale.id));

    let mut statuses = manga_page_prompt_statuses_by_chapter(&db, project_id, chapter_id)
        .await
        .unwrap();
    statuses.sort_unstable();
    let mut expected_statuses = vec![
        (stored[0].id, "completed".to_string()),
        (stored[1].id, "pending".to_string()),
        (stale.id, "pending".to_string()),
    ];
    expected_statuses.sort_unstable();
    assert_eq!(statuses, expected_statuses);
    let mut batch_statuses = manga_page_prompt_statuses_by_chapters(&db, project_id, &[chapter_id])
        .await
        .unwrap();
    batch_statuses.sort_unstable();
    let mut expected_batch_statuses = vec![
        (chapter_id, stored[0].id, "completed".to_string()),
        (chapter_id, stored[1].id, "pending".to_string()),
        (chapter_id, stale.id, "pending".to_string()),
    ];
    expected_batch_statuses.sort_unstable();
    assert_eq!(batch_statuses, expected_batch_statuses);
    let newer = crate::media_authoring::save_authoring_document(
        &db,
        crate::media_authoring::SaveAuthoringDocument {
            project_id,
            source_snapshot_id: adaptation.source_snapshot_id,
            source_chapter_id: adaptation.source_chapter_id,
            kind: adaptation.kind,
            record_id: None,
            expected_revision: None,
            inputs: adaptation.inputs,
            document: adaptation.document,
        },
    )
    .await
    .unwrap();
    crate::media_authoring::adopt_authoring_document(&db, project_id, newer.id, newer.revision)
        .await
        .unwrap();
    assert!(
        sync_manga_page_prompts_by_chapter(&db, project_id, chapter_id, drafts, fixed_time(2))
            .await
            .unwrap_err()
            .to_string()
            .contains("对应版本的改编计划")
    );
}

#[tokio::test]
async fn sync_accepts_adopted_plan_for_standalone_comic_chapter_snapshot() {
    let db = setup_db().await;
    let (project_id, chapter_id) =
        standalone_comic_project_and_chapter(&db, "独立漫画来源快照").await;
    let (mut drafts, adaptation) = planned_drafts(&db, project_id, chapter_id).await;
    upgrade_to_pipeline_layouts(&mut drafts);

    assert_eq!(
        adaptation.status,
        a4n_domain::media_authoring::AuthoringDocumentStatus::Adopted
    );
    let synced =
        sync_manga_page_prompts_by_chapter(&db, project_id, chapter_id, drafts, fixed_time(2))
            .await
            .expect("独立漫画应按章节来源快照校验已采用改编计划");

    assert_eq!(synced.len(), 2);
}

#[tokio::test]
async fn layout_update_preserves_concurrent_image_status() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画布局更新").await;
    let (drafts, _) = planned_drafts(&db, project_id, chapter_id).await;
    let mut rows =
        sync_manga_page_prompts_by_chapter(&db, project_id, chapter_id, drafts, fixed_time(1))
            .await
            .unwrap();
    let stale = rows.remove(0);
    let layout = stale.layout.clone().unwrap();

    chapter_manga_prompt::ActiveModel {
        id: Set(stale.id),
        generated_image_id: Set(Some(9901)),
        status: Set("completed".to_string()),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap();

    let updated = update_manga_page_prompt_layout(&db, stale, layout)
        .await
        .unwrap();
    assert_eq!(updated.generated_image_id, Some(9901));
    assert_eq!(updated.status, "completed");
}

#[tokio::test]
async fn stage_creation_preserves_the_concurrent_winner_until_explicit_deletion() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画阶段单次写入").await;
    for (stage, page_number) in [
        (MangaPipelineArtifactStage::Preprocessing, None),
        (MangaPipelineArtifactStage::Storyboard, Some(1)),
    ] {
        let draft = |value| MangaPipelineArtifactDraft {
            project_id,
            chapter_id,
            stage,
            page_number,
            input_fingerprint: format!("input-{value}"),
            payload: json!({"value": value}),
            stale: false,
        };
        let (left, right) = tokio::join!(
            crate::manga::create_manga_pipeline_artifact(&db, draft("left"), fixed_time(1)),
            crate::manga::create_manga_pipeline_artifact(&db, draft("right"), fixed_time(1)),
        );
        let (winner, loser) = match (left, right) {
            (Ok(winner), Err(loser)) | (Err(loser), Ok(winner)) => (winner, loser),
            other => panic!("exactly one concurrent creation must succeed: {other:?}"),
        };
        assert!(matches!(loser, MangaPromptRecordError::Conflict(_)));
        let stored = manga_pipeline_artifact(&db, project_id, chapter_id, stage, page_number)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.payload, winner.payload);
        assert_eq!(stored.output_fingerprint, winner.output_fingerprint);
        assert!(matches!(
            crate::manga::create_manga_pipeline_artifact(&db, draft("replacement"), fixed_time(2))
                .await,
            Err(MangaPromptRecordError::Conflict(_))
        ));
        delete_manga_pipeline_artifact(
            &db,
            project_id,
            chapter_id,
            stage,
            page_number,
            &[winner.id],
            &[],
        )
        .await
        .unwrap();
        let regenerated =
            crate::manga::create_manga_pipeline_artifact(&db, draft("replacement"), fixed_time(2))
                .await
                .unwrap();
        assert_eq!(regenerated.payload, json!({"value": "replacement"}));
        assert_ne!(regenerated.id, winner.id);
    }
}

#[tokio::test]
async fn page_image_completion_preserves_the_winner_and_reclaims_only_the_loser() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画成图并发写入").await;
    let (drafts, _) = planned_drafts(&db, project_id, chapter_id).await;
    let rows =
        sync_manga_page_prompts_by_chapter(&db, project_id, chapter_id, drafts, fixed_time(1))
            .await
            .unwrap();
    let prompt_id = rows[0].id;
    let mut images = Vec::new();
    for label in ["left", "right"] {
        images.push(
            generated_image::ActiveModel {
                project_id: Set(project_id),
                chapter_id: Set(Some(chapter_id)),
                image_type: Set("manga_page".into()),
                prompt: Set(format!("page {} {label}", rows[0].page_number)),
                model: Set("test".into()),
                width: Set(768),
                height: Set(1024),
                is_favorite: Set(false),
                file_path: Set(format!(
                    "projects/{project_id}/images/manga-race-{label}.png"
                )),
                created_at: Set(fixed_time(1)),
                ..Default::default()
            }
            .insert(&db)
            .await
            .unwrap(),
        );
    }
    let (left, right) = tokio::join!(
        crate::manga::mark_manga_page_prompt_image_completed_with_image(
            &db,
            prompt_id,
            images[0].id,
            fixed_time(1)
        ),
        crate::manga::mark_manga_page_prompt_image_completed_with_image(
            &db,
            prompt_id,
            images[1].id,
            fixed_time(1)
        ),
    );
    let (winner, loser, loser_error) = match (left, right) {
        (Ok(()), Err(error)) => (&images[0], &images[1], error),
        (Err(error), Ok(())) => (&images[1], &images[0], error),
        other => panic!("exactly one image completion must succeed: {other:?}"),
    };
    assert!(matches!(loser_error, MangaPromptRecordError::Conflict(_)));
    crate::manga::mark_manga_page_prompt_image_completed_with_image(
        &db,
        prompt_id,
        winner.id,
        fixed_time(2),
    )
    .await
    .expect("replaying the adopted image is idempotent");
    crate::video::delete_generated_image_record_with_cleanup(&db, loser.id)
        .await
        .unwrap();
    let stored = chapter_manga_prompt::Entity::find_by_id(prompt_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.generated_image_id, Some(winner.id));
    assert!(
        generated_image::Entity::find_by_id(winner.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        generated_image::Entity::find_by_id(loser.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    let cleanup = cleanup_outbox::Entity::find().all(&db).await.unwrap();
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].payload_json["paths"], json!([loser.file_path]));
}

#[tokio::test]
async fn stale_page_projection_rolls_back_the_stage_artifact_save() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画阶段产物版本冲突").await;
    let (drafts, _) = planned_drafts(&db, project_id, chapter_id).await;
    let prompts =
        sync_manga_page_prompts_by_chapter(&db, project_id, chapter_id, drafts, fixed_time(1))
            .await
            .unwrap();
    let prompt = prompts
        .into_iter()
        .find(|prompt| prompt.page_number == 1)
        .expect("page 1 prompt");
    let layout = prompt.layout.clone().expect("page pipeline layout");
    chapter_manga_prompt::ActiveModel {
        id: Set(prompt.id),
        updated_at: Set(fixed_time(2)),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap();

    let result = save_manga_pipeline_artifact_with_projection(
        &db,
        MangaPipelineArtifactDraft {
            project_id,
            chapter_id,
            stage: MangaPipelineArtifactStage::PageContent,
            page_number: Some(1),
            input_fingerprint: "manual-page-content-v1".to_string(),
            payload: json!({"summary": "手工修改"}),
            stale: false,
        },
        Some(MangaPipelineArtifactProjectionUpdate {
            page_prompt: prompt.clone(),
            layout,
            expected_updated_at: prompt.updated_at.to_string(),
        }),
        fixed_time(3),
    )
    .await;

    assert!(
        matches!(result, Err(MangaPromptRecordError::InvalidArgument(message)) if message.contains("已被其他操作修改"))
    );
    assert!(
        manga_pipeline_artifact(
            &db,
            project_id,
            chapter_id,
            MangaPipelineArtifactStage::PageContent,
            Some(1),
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn manga_pipeline_run_blocks_manual_artifact_writes_in_the_db_owner() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画流水线写入门禁").await;
    crate::workflow::upsert_workflow_run(
        &db,
        crate::workflow::UpsertWorkflowRunInput {
            business_key: format!("manga_pipeline:{project_id}:{chapter_id}"),
            workflow_kind: "manga_pipeline".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(chapter_id),
            chapter_id: Some(chapter_id),
            mode: None,
            input_fingerprint_json: Some(json!({"test": true})),
            status: crate::workflow::WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: None,
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .unwrap();

    let result = save_manga_pipeline_artifact_with_projection(
        &db,
        MangaPipelineArtifactDraft {
            project_id,
            chapter_id,
            stage: MangaPipelineArtifactStage::Preprocessing,
            page_number: None,
            input_fingerprint: "manual-preprocessing-v1".to_string(),
            payload: json!({"summary": "手工修改"}),
            stale: false,
        },
        None,
        fixed_time(3),
    )
    .await;

    assert!(
        matches!(result, Err(MangaPromptRecordError::InvalidArgument(message)) if message.contains("漫画流水线正在运行"))
    );
    assert!(
        matches!(delete_manga_prompts_by_chapter(&db, project_id, chapter_id).await,
            Err(MangaPromptRecordError::InvalidArgument(message)) if message.contains("漫画流水线正在运行"))
    );
    assert!(
        manga_pipeline_artifact(
            &db,
            project_id,
            chapter_id,
            MangaPipelineArtifactStage::Preprocessing,
            None,
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn deleting_page_content_cascades_only_for_that_page() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画阶段失效范围").await;
    let now = fixed_time(1);

    for (stage, page_number) in [
        (MangaPipelineArtifactStage::PageContent, 1),
        (MangaPipelineArtifactStage::PageContent, 2),
        (MangaPipelineArtifactStage::PageAdaptation, 1),
        (MangaPipelineArtifactStage::PageAdaptation, 2),
        (MangaPipelineArtifactStage::Storyboard, 1),
        (MangaPipelineArtifactStage::Storyboard, 2),
        (MangaPipelineArtifactStage::ReferenceDiscovery, 1),
        (MangaPipelineArtifactStage::ReferenceDiscovery, 2),
        (MangaPipelineArtifactStage::DrawingPrompt, 1),
        (MangaPipelineArtifactStage::DrawingPrompt, 2),
        (MangaPipelineArtifactStage::Image, 1),
        (MangaPipelineArtifactStage::Image, 2),
    ] {
        upsert_manga_pipeline_artifact(
            &db,
            MangaPipelineArtifactDraft {
                project_id,
                chapter_id,
                stage,
                page_number: Some(page_number),
                input_fingerprint: format!("{}:{page_number}", stage.key()),
                payload: json!({"stage": stage.key(), "page_number": page_number}),
                stale: false,
            },
            now,
        )
        .await
        .unwrap();
    }

    let deletion_plan = manga_pipeline_artifact_deletion_plan(
        &db,
        project_id,
        chapter_id,
        MangaPipelineArtifactStage::PageContent,
        Some(1),
    )
    .await
    .unwrap();
    let expected_artifact_ids = deletion_plan
        .iter()
        .map(|artifact| artifact.id)
        .collect::<Vec<_>>();
    delete_manga_pipeline_artifact(
        &db,
        project_id,
        chapter_id,
        MangaPipelineArtifactStage::PageContent,
        Some(1),
        &expected_artifact_ids,
        &[],
    )
    .await
    .unwrap();

    let artifacts = manga_pipeline_artifacts_by_chapter(&db, project_id, chapter_id)
        .await
        .unwrap();
    assert!(
        !artifacts
            .iter()
            .any(|artifact| artifact.page_number == Some(1))
    );
    assert_eq!(
        artifacts
            .iter()
            .filter(|artifact| artifact.page_number == Some(2))
            .count(),
        6
    );
    assert!(
        artifacts
            .iter()
            .filter(|artifact| artifact.page_number == Some(2))
            .all(|artifact| !artifact.stale)
    );
}

#[tokio::test]
async fn deleting_manga_pages_removes_their_images_and_queues_file_cleanup() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画页成图级联删除").await;
    let now = fixed_time(1);
    let mut image_ids = Vec::new();
    for page_number in [1, 2] {
        let file_path = format!("projects/{project_id}/images/manga-delete-page-{page_number}.png");
        let image = generated_image::ActiveModel {
            project_id: Set(project_id),
            image_type: Set("manga_page".into()),
            file_path: Set(file_path),
            prompt: Set(format!("第 {page_number} 页")),
            model: Set("test".into()),
            width: Set(768),
            height: Set(1024),
            file_size: Set(None),
            ai_score: Set(None),
            is_favorite: Set(false),
            chapter_id: Set(Some(chapter_id)),
            panel_id: Set((page_number == 2).then(|| format!("page{page_number}"))),
            metadata_json: Set((page_number == 2).then(|| json!({"page_number": page_number}))),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        image_ids.push(image.id);

        chapter_manga_prompt::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(chapter_id),
            page_number: Set(page_number),
            prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.into()),
            image_prompt: Set(format!("第 {page_number} 页")),
            dialogue_text: Set(None),
            characters_in_frame: Set(Some(json!([]))),
            layout: Set(Some(if page_number == 2 {
                // Bulk recovery must not depend on parsing a damaged page projection.
                json!({"schema_version":"manga_page_generation_v4", "pipeline":{"stage":"slots","status":"pending"}})
            } else {
                page_layout(page_number)
            })),
            generated_image_id: Set(Some(image.id)),
            status: Set("completed".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        upsert_manga_pipeline_artifact(
            &db,
            MangaPipelineArtifactDraft {
                project_id,
                chapter_id,
                stage: MangaPipelineArtifactStage::Image,
                page_number: Some(page_number as u32),
                input_fingerprint: format!("page:{page_number}"),
                payload: json!({"image_id": image.id}),
                stale: false,
            },
            now,
        )
        .await
        .unwrap();
    }
    upsert_manga_pipeline_artifact(
        &db,
        MangaPipelineArtifactDraft {
            project_id,
            chapter_id,
            stage: MangaPipelineArtifactStage::Preprocessing,
            page_number: None,
            input_fingerprint: "chapter-preprocessing".into(),
            payload: json!({"fixture": "chapter-preprocessing"}),
            stale: false,
        },
        now,
    )
    .await
    .unwrap();

    delete_manga_page_by_number(&db, project_id, chapter_id, 1)
        .await
        .unwrap();
    assert!(
        chapter_manga_prompt::Entity::find()
            .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
            .filter(chapter_manga_prompt::Column::PageNumber.eq(1))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(image_ids[0])
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(image_ids[1])
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        manga_pipeline_artifacts_by_chapter(&db, project_id, chapter_id)
            .await
            .unwrap()
            .iter()
            .filter(|artifact| artifact.page_number == Some(1))
            .count(),
        0
    );
    let cleanup_after_page = cleanup_outbox::Entity::find().all(&db).await.unwrap();
    assert_eq!(cleanup_after_page.len(), 1);
    assert_eq!(
        cleanup_after_page[0].payload_json["paths"],
        json!([format!(
            "projects/{project_id}/images/manga-delete-page-1.png"
        )])
    );

    assert_eq!(
        delete_manga_prompts_by_chapter(&db, project_id, chapter_id)
            .await
            .unwrap(),
        1
    );
    assert!(
        generated_image::Entity::find_by_id(image_ids[1])
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        cleanup_outbox::Entity::find().all(&db).await.unwrap().len(),
        2
    );
    assert!(
        chapter_manga_prompt::Entity::find()
            .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
            .all(&db)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        manga_pipeline_artifacts_by_chapter(&db, project_id, chapter_id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        delete_manga_prompts_by_chapter(&db, project_id, chapter_id)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn deleting_drawing_prompt_cascade_removes_image_record_and_queues_file_cleanup() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画提示词阶段级联删除成图").await;
    let now = fixed_time(1);
    let layout = page_layout(1);
    let image = generated_image::ActiveModel {
        project_id: Set(project_id),
        image_type: Set("manga_page".into()),
        file_path: Set(format!(
            "projects/{project_id}/images/manga-drawing-prompt-cascade.png"
        )),
        prompt: Set("整页画面".into()),
        model: Set("test".into()),
        width: Set(768),
        height: Set(1024),
        file_size: Set(None),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(Some(chapter_id)),
        panel_id: Set(Some("page1".into())),
        metadata_json: Set(Some(json!({"page_number": 1}))),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let prompt = chapter_manga_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        page_number: Set(1),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.into()),
        image_prompt: Set("整页画面".into()),
        dialogue_text: Set(None),
        characters_in_frame: Set(Some(json!([]))),
        layout: Set(Some(layout.clone())),
        generated_image_id: Set(Some(image.id)),
        status: Set("completed".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    for stage in [
        MangaPipelineArtifactStage::DrawingPrompt,
        MangaPipelineArtifactStage::Image,
    ] {
        upsert_manga_pipeline_artifact(
            &db,
            MangaPipelineArtifactDraft {
                project_id,
                chapter_id,
                stage,
                page_number: Some(1),
                input_fingerprint: format!("{}:1", stage.key()),
                payload: json!({"stage": stage.key()}),
                stale: false,
            },
            now,
        )
        .await
        .unwrap();
    }
    let plan = manga_pipeline_artifact_deletion_plan(
        &db,
        project_id,
        chapter_id,
        MangaPipelineArtifactStage::DrawingPrompt,
        Some(1),
    )
    .await
    .unwrap();
    let expected_artifact_ids = plan.iter().map(|artifact| artifact.id).collect::<Vec<_>>();

    let invalid_reset = MangaPipelineArtifactProjectionReset {
        page_prompt_id: prompt.id,
        page_number: 1,
        expected_layout: layout.clone(),
        expected_generated_image_id: Some(image.id),
        layout: json!({"schema_version":"manga_page_generation_v4", "pipeline":{"stage":"slots"}}),
        image_prompt: String::new(),
        clear_generated_image: true,
    };
    assert!(
        delete_manga_pipeline_artifact(
            &db,
            project_id,
            chapter_id,
            MangaPipelineArtifactStage::DrawingPrompt,
            Some(1),
            &expected_artifact_ids,
            &[invalid_reset],
        )
        .await
        .is_err()
    );
    let preserved = chapter_manga_prompt::Entity::find_by_id(prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preserved.layout.as_ref(), Some(&layout));
    assert_eq!(preserved.generated_image_id, Some(image.id));
    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        manga_pipeline_artifacts_by_chapter(&db, project_id, chapter_id)
            .await
            .unwrap()
            .len(),
        2
    );

    delete_manga_pipeline_artifact(
        &db,
        project_id,
        chapter_id,
        MangaPipelineArtifactStage::DrawingPrompt,
        Some(1),
        &expected_artifact_ids,
        &[MangaPipelineArtifactProjectionReset {
            page_prompt_id: prompt.id,
            page_number: 1,
            expected_layout: layout,
            expected_generated_image_id: Some(image.id),
            layout: page_layout(1),
            image_prompt: String::new(),
            clear_generated_image: true,
        }],
    )
    .await
    .unwrap();

    let updated_prompt = chapter_manga_prompt::Entity::find_by_id(prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_prompt.generated_image_id, None);
    assert_eq!(updated_prompt.status, "pending");
    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        manga_pipeline_artifacts_by_chapter(&db, project_id, chapter_id)
            .await
            .unwrap()
            .is_empty()
    );
    let cleanup = cleanup_outbox::Entity::find().all(&db).await.unwrap();
    assert_eq!(cleanup.len(), 1);
    assert_eq!(
        cleanup[0].payload_json["paths"],
        json!([format!(
            "projects/{project_id}/images/manga-drawing-prompt-cascade.png"
        )])
    );
}

#[tokio::test]
async fn sync_rejects_invalid_page_number() {
    let db = setup_db().await;
    let invalid_page = prompt_draft(8401, 8402, 0, "非法位置");
    let error =
        sync_manga_page_prompts_by_chapter(&db, 8402, 8401, vec![invalid_page], fixed_time(1))
            .await
            .expect_err("page number must fail");
    assert!(matches!(
        error,
        MangaPipelineDbError::InvalidArgument(message)
            if message == "page_generation 必须使用 page_number>0"
    ));
}
