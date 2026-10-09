use a4n_db_models::models::chapter::{
    chapter,
    chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE},
};
use a4n_domain::manga::{
    MangaPagePromptSyncDraft, MangaPipelineArtifactDraft, MangaPipelineArtifactStage,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

use crate::manga::{
    MangaPipelineDbError, delete_manga_pipeline_artifact, manga_page_prompt_statuses_by_chapter,
    manga_page_prompt_statuses_by_chapters, manga_pipeline_artifact_deletion_plan,
    manga_pipeline_artifacts_by_chapter, sync_manga_page_prompts_by_chapter,
    update_manga_page_prompt_layout, upsert_manga_pipeline_artifact,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
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
    assert_eq!(
        statuses,
        vec![
            (stored[0].id, "completed".to_string()),
            (stored[1].id, "pending".to_string()),
            (stale.id, "pending".to_string()),
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
            (chapter_id, stale.id, "pending".to_string()),
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
