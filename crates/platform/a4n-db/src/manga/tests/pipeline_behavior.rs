use a4n_db_models::models::chapter::{
    chapter,
    chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE},
};
use a4n_domain::manga::MangaPagePromptSyncDraft;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

use crate::manga::{
    MangaPipelineDbError, manga_page_prompt_statuses_by_chapter,
    manga_page_prompt_statuses_by_chapters, sync_manga_page_prompts_by_chapter,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn project_and_chapter(db: &DatabaseConnection, title: &str) -> (i64, i64) {
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
    db: &DatabaseConnection,
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

#[tokio::test]
async fn sync_page_prompts_preserves_rendered_image_when_prompt_unchanged() {
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
    let rows = sync_manga_page_prompts_by_chapter(
        &db,
        project_id,
        chapter_id,
        drafts.clone(),
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
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].id, kept.id);
    assert_eq!(stored[0].generated_image_id, Some(9901));
    assert_eq!(stored[0].status, "completed");
    assert!(!stored.iter().any(|row| row.id == stale.id));

    let mut statuses = manga_page_prompt_statuses_by_chapter(&db, project_id, chapter_id)
        .await
        .unwrap();
    statuses.sort_unstable();
    assert_eq!(
        statuses,
        vec![
            (stored[0].id, "completed".to_string()),
            (stored[1].id, "pending".to_string())
        ]
    );
    let batch_statuses = manga_page_prompt_statuses_by_chapters(&db, project_id, &[chapter_id])
        .await
        .unwrap();
    assert_eq!(
        batch_statuses,
        vec![
            (chapter_id, stored[0].id, "completed".to_string()),
            (chapter_id, stored[1].id, "pending".to_string()),
        ]
    );
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
