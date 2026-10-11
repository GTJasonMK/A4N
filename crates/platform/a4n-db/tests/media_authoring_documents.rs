use a4n_db::{
    media_authoring::{
        AuthoringDocumentError, SaveAuthoringDocument, adopt_authoring_document,
        authoring_document, current_authoring_document, delete_authoring_document,
        preview_delete_authoring_document, save_authoring_document,
    },
    project::{
        CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope,
        create_derived_project_record, create_project_record, resolve_chapter_source,
    },
};
use a4n_domain::{
    chapter::{ImportChapterContentInput, ResolvedChapterSource},
    media_authoring::{
        AdaptationPlanV1, AuthoringDocumentInputs, AuthoringDocumentKind, AuthoringDocumentStatus,
        AuthoringSourceRef,
    },
    source_spans::source_spans_for_content,
};
use serde_json::json;

fn input(source: &ResolvedChapterSource) -> SaveAuthoringDocument {
    let spans = source_spans_for_content(
        source.source_chapter_id,
        source.chapter_number,
        &source.content,
    );
    let span = &spans[0].id;
    let plan: AdaptationPlanV1 = serde_json::from_value(json!({
        "schema_version": "adaptation_plan_v1", "target": "manga", "intention": "保留开门的动作与反应", "decisions": [],
        "units": [{
            "id": "door", "origin": "source", "title": "开门", "location": "门口", "participants": [],
            "dramatic_question": "门后有什么", "objective": "进入房间", "obstacle": "门关着", "entry_state": "门关闭", "exit_state": "门打开",
            "source_span_ids": [span], "beats": [{
                "id": "open", "kind": "action", "is_climax": false, "origin": "source", "importance": "required",
                "action": "推开门", "reaction": null, "dialogue_intent": null, "entry_state": "门关闭", "exit_state": "门打开", "source_span_ids": [span]
            }]
        }]
    })).unwrap();
    plan.validate(&spans.iter().map(|span| span.id.clone()).collect())
        .unwrap();
    SaveAuthoringDocument {
        project_id: source.requested_project_id,
        source_snapshot_id: source.source_snapshot_id.unwrap(),
        source_chapter_id: Some(source.source_chapter_id),
        kind: AuthoringDocumentKind::AdaptationPlan,
        record_id: None,
        expected_revision: None,
        inputs: AuthoringDocumentInputs {
            sources: vec![AuthoringSourceRef {
                root_project_id: source.root_project_id,
                source_chapter_id: source.source_chapter_id,
                source_snapshot_id: source.source_snapshot_id.unwrap(),
                source_snapshot_chapter_id: source.source_snapshot_chapter_id.unwrap(),
                content_sha256: source.content_sha256.clone(),
            }],
            video_reference: None,
            dependencies: Vec::new(),
            preprocessing_fingerprint: None,
        },
        document: serde_json::to_value(plan).unwrap(),
    }
}

#[tokio::test]
async fn documents_are_project_owned_revision_fenced_and_independent_per_source_snapshot() {
    let db = a4n_migration::test_support::connect_test_database().await;
    a4n_migration::migrations::media_authoring::create_document_schema(&db)
        .await
        .unwrap();
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "媒体改编边界".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id: root.id,
            chapter_number: 1,
            content: "她推开门。".into(),
            title: Some("门口".into()),
        },
        "media authoring boundary test",
    )
    .await
    .unwrap();
    let mut projects = Vec::new();
    for title in ["漫画甲", "漫画乙"] {
        projects.push(
            create_derived_project_record(
                &db,
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: "comic".into(),
                    title: title.into(),
                    description: String::new(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                },
            )
            .await
            .unwrap(),
        );
    }
    let first = resolve_chapter_source(&db, projects[0].id, chapter.id)
        .await
        .unwrap();
    let second = resolve_chapter_source(&db, projects[1].id, chapter.id)
        .await
        .unwrap();
    let draft = save_authoring_document(&db, input(&first)).await.unwrap();
    assert!(matches!(
        authoring_document(&db, projects[1].id, draft.id).await,
        Err(AuthoringDocumentError::NotFound(_))
    ));

    let mut left = input(&first);
    left.record_id = Some(draft.id);
    left.expected_revision = Some(draft.revision);
    left.document["intention"] = "保留开门动作".into();
    let mut right = input(&first);
    right.record_id = Some(draft.id);
    right.expected_revision = Some(draft.revision);
    right.document["intention"] = "突出人物反应".into();
    let (left, right) = futures::join!(
        save_authoring_document(&db, left),
        save_authoring_document(&db, right)
    );
    assert_ne!(
        left.is_ok(),
        right.is_ok(),
        "one concurrent editor must conflict"
    );
    let saved = authoring_document(&db, first.requested_project_id, draft.id)
        .await
        .unwrap();
    let adopted =
        adopt_authoring_document(&db, first.requested_project_id, saved.id, saved.revision)
            .await
            .unwrap();
    assert_eq!(adopted.status, AuthoringDocumentStatus::Adopted);
    assert_eq!(
        adopted.revision, saved.revision,
        "adoption does not change the document content revision"
    );

    // A source-sync proposal can adopt a new lineage while existing scenes keep
    // their old lineage. It cannot supersede another project's plan.
    let mut target = second.clone();
    target.requested_project_id = first.requested_project_id;
    let next = save_authoring_document(&db, input(&target)).await.unwrap();
    adopt_authoring_document(&db, target.requested_project_id, next.id, next.revision)
        .await
        .unwrap();
    let old_current = current_authoring_document(
        &db,
        first.requested_project_id,
        first.source_snapshot_id.unwrap(),
        Some(chapter.id),
        AuthoringDocumentKind::AdaptationPlan,
        AuthoringDocumentStatus::Adopted,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(old_current.id, adopted.id);
    assert!(
        current_authoring_document(
            &db,
            projects[1].id,
            second.source_snapshot_id.unwrap(),
            Some(chapter.id),
            AuthoringDocumentKind::AdaptationPlan,
            AuthoringDocumentStatus::Adopted
        )
        .await
        .unwrap()
        .is_none()
    );

    let mut corrupt = input(&second);
    corrupt.inputs.sources[0].content_sha256 = "0".repeat(64);
    assert!(matches!(
        save_authoring_document(&db, corrupt).await,
        Err(AuthoringDocumentError::InvalidArgument(_))
    ));
}

#[tokio::test]
async fn deleting_adaptation_plan_previews_and_cascades_its_manga_page_and_image() {
    use a4n_db_models::models::{
        ai::generated_image,
        chapter::{chapter_manga_prompt, manga_pipeline_artifact},
        task::cleanup_outbox,
    };
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

    let db = a4n_migration::test_support::connect_test_database().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "改编计划级联删除".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id: root.id,
            chapter_number: 1,
            content: "她推开门。".into(),
            title: Some("门口".into()),
        },
        "media authoring cascade test",
    )
    .await
    .unwrap();
    let comic = create_derived_project_record(
        &db,
        1,
        CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: "comic".into(),
            title: "级联漫画".into(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let source = resolve_chapter_source(&db, comic.id, chapter.id)
        .await
        .unwrap();
    let draft = save_authoring_document(&db, input(&source)).await.unwrap();
    let adopted = adopt_authoring_document(&db, comic.id, draft.id, draft.revision)
        .await
        .unwrap();
    let now = chrono::Utc::now().naive_utc();
    let layout = json!({
        "schema_version": "manga_page_generation_v3",
        "page_plan": {
            "page_number": 1,
            "beat_refs": [{"unit_id": "door", "beat_id": "open"}],
            "role": "setup",
            "pacing": "medium",
            "importance": "normal",
            "reading_flow": "top_to_bottom_left_to_right",
            "page_goal": "推进场景",
            "visual_thesis": "打开房门",
            "entry_state": "门关闭",
            "exit_state": "门打开",
            "dominant_panel_number": 1,
            "panels": [{
                "panel_number": 1,
                "beat_refs": [{"unit_id": "door", "beat_id": "open"}],
                "function": "action",
                "size": "dominant",
                "shot_scale": "medium",
                "camera_angle": "eye_level",
                "participants": [],
                "composition": "门口",
                "observable_action": "推开门",
                "reaction": null,
                "spatial_relationship": "门前",
                "continuity_from_previous": null,
                "text_layers": []
            }]
        },
        "drawing_prompt": {
            "schema_version": "manga_page_drawing_prompt_v2",
            "page_number": 1,
            "image_prompt": "整页画面"
        },
        "reference_bindings": [],
        "prompt_fingerprint": {"hash": "sha256"},
        "source": {
            "root_project_id": source.root_project_id,
            "source_chapter_id": source.source_chapter_id,
            "source_snapshot_id": source.source_snapshot_id,
            "source_snapshot_chapter_id": source.source_snapshot_chapter_id,
            "source_revision_key": format!(
                "snapshot:{}:chapter:{}:{}",
                source.source_snapshot_id.unwrap(),
                source.source_snapshot_chapter_id.unwrap(),
                source.content_sha256,
            ),
            "content_sha256": source.content_sha256,
            "adaptation_plan_id": adopted.id,
            "adaptation_plan_revision": adopted.revision,
            "adaptation_plan_sha256": adopted.sha256,
        }
    });
    let page = chapter_manga_prompt::ActiveModel {
        project_id: Set(comic.id),
        chapter_id: Set(chapter.id),
        page_number: Set(1),
        prompt_type: Set("page_generation".into()),
        image_prompt: Set("整页画面".into()),
        dialogue_text: Set(None),
        characters_in_frame: Set(Some(json!([]))),
        layout: Set(Some(layout)),
        generated_image_id: Set(None),
        status: Set("completed".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let image = generated_image::ActiveModel {
        project_id: Set(comic.id),
        image_type: Set("manga_page".into()),
        file_path: Set("data/media-authoring-cascade-page.png".into()),
        prompt: Set("整页画面".into()),
        model: Set("test".into()),
        width: Set(768),
        height: Set(1024),
        file_size: Set(None),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(Some(chapter.id)),
        panel_id: Set(Some("page1".into())),
        metadata_json: Set(Some(json!({"page_number": 1}))),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    manga_pipeline_artifact::ActiveModel {
        project_id: Set(comic.id),
        chapter_id: Set(chapter.id),
        stage: Set("drawing_prompt".into()),
        page_number: Set(1),
        input_fingerprint: Set("input".into()),
        output_fingerprint: Set("output".into()),
        payload: Set(json!({"prompt_id": page.id})),
        stale: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let impact = preview_delete_authoring_document(
        &db,
        comic.id,
        adopted.id,
        adopted.kind,
        adopted.revision,
    )
    .await
    .unwrap();
    assert_eq!(impact.manga_pages.len(), 1);
    assert_eq!(impact.manga_pages[0].chapter_id, chapter.id);
    assert_eq!(impact.manga_pages[0].page_number, 1);
    assert_eq!(impact.manga_pages[0].image_count, 1);

    assert!(matches!(
        delete_authoring_document(
            &db,
            comic.id,
            adopted.id,
            adopted.kind,
            adopted.revision,
            &"0".repeat(64),
        )
        .await,
        Err(AuthoringDocumentError::DeletionImpactChanged)
    ));
    assert!(
        chapter_manga_prompt::Entity::find_by_id(page.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );

    delete_authoring_document(
        &db,
        comic.id,
        adopted.id,
        adopted.kind,
        adopted.revision,
        &impact.fingerprint,
    )
    .await
    .unwrap();
    assert!(authoring_document(&db, comic.id, adopted.id).await.is_err());
    assert!(
        chapter_manga_prompt::Entity::find_by_id(page.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        manga_pipeline_artifact::Entity::find()
            .filter(manga_pipeline_artifact::Column::ProjectId.eq(comic.id))
            .filter(manga_pipeline_artifact::Column::ChapterId.eq(chapter.id))
            .all(&db)
            .await
            .unwrap(),
        Vec::<manga_pipeline_artifact::Model>::new()
    );
    assert_eq!(
        cleanup_outbox::Entity::find()
            .filter(cleanup_outbox::Column::DedupeKey.starts_with(format!(
                "media-authoring-delete:{}:{}:",
                comic.id, adopted.id
            )))
            .count(&db)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn legacy_shared_director_plan_is_split_by_video_owner_without_changing_content() {
    use a4n_db_models::models::{chapter::chapter_visual_plan, project::novel_project};
    use sea_orm::{
        ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Set, Statement,
        TransactionTrait,
    };
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "历史导演归属".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id: root.id,
            chapter_number: 1,
            content: "她推开门。".into(),
            title: None,
        },
        "legacy film ownership test",
    )
    .await
    .unwrap();
    let mut videos = Vec::new();
    for title in ["视频甲", "视频乙"] {
        videos.push(
            create_derived_project_record(
                &db,
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: "video".into(),
                    title: title.into(),
                    description: String::new(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                },
            )
            .await
            .unwrap(),
        );
    }
    let source = resolve_chapter_source(&db, videos[0].id, chapter.id)
        .await
        .unwrap();
    let mut other: novel_project::ActiveModel = videos[1].clone().into();
    other.source_snapshot_id = Set(source.source_snapshot_id);
    sea_orm::ActiveModelTrait::update(other, &db).await.unwrap();
    let draft = a4n_domain::chapter_visual_plan::ChapterVisualPlanV3::starter();
    draft.validate_draft().unwrap();
    let hash = a4n_domain::generation::canonical_json_sha256(&draft).unwrap();
    let payload = serde_json::to_value(&draft).unwrap();
    db.execute_unprepared("DROP TRIGGER IF EXISTS video_visual_plan_owner ON chapter_visual_plan; ALTER TABLE chapter_visual_plan ALTER COLUMN project_id DROP NOT NULL").await.unwrap();
    db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO chapter_visual_plan(root_project_id, source_chapter_id, source_snapshot_id, source_snapshot_chapter_id, source_revision_key, source_content_sha256, contract_version, plan_version, revision, status, authoring_mode, plan_sha256, plan_json, author_user_id, created_at, updated_at) VALUES($1,$2,$3,$4,$5,$6,'chapter_visual_plan_v3',1,1,'draft','manual',$7,$8,1,LOCALTIMESTAMP,LOCALTIMESTAMP)",
        [root.id.into(), chapter.id.into(), source.source_snapshot_id.into(), source.source_snapshot_chapter_id.into(), source.source_revision_key.clone().into(), source.content_sha256.clone().into(), hash.clone().into(), payload.clone().into()],
    )).await.unwrap();
    let tx = db.begin().await.unwrap();
    a4n_migration::migrations::media_authoring::migrate_video_ownership(&tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let plans = chapter_visual_plan::Entity::find()
        .filter(chapter_visual_plan::Column::RootProjectId.eq(root.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(plans.len(), 2);
    assert_eq!(
        plans
            .iter()
            .map(|plan| plan.project_id)
            .collect::<std::collections::BTreeSet<_>>(),
        videos.iter().map(|project| project.id).collect()
    );
    for plan in &plans {
        assert_eq!(plan.plan_json, payload);
        assert_eq!(plan.plan_sha256, hash);
    }
    let tx = db.begin().await.unwrap();
    a4n_migration::migrations::media_authoring::migrate_video_ownership(&tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        chapter_visual_plan::Entity::find()
            .all(&db)
            .await
            .unwrap()
            .len(),
        2,
        "a completed ownership migration must not clone again"
    );
}

#[tokio::test]
async fn legacy_manga_preserves_edited_pages_and_all_images_without_its_original_producer() {
    use a4n_db_models::models::{
        ai::generated_image,
        chapter::chapter_manga_prompt,
        project::{media_authoring_document, media_authoring_migration_receipt, novel_project},
    };
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "人工改过的历史漫画".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id: root.id,
            chapter_number: 1,
            content: "她推开门。".into(),
            title: Some("门口".into()),
        },
        "historical comic migration",
    )
    .await
    .unwrap();
    let now = chrono::Utc::now().naive_utc();
    let source_hash = a4n_domain::source_text_hash(&chapter.content);
    let old_ref = json!({"scene_index":1,"shot_index":1,"beat_index":1});
    let layout = json!({
        "schema_version":"manga_page_generation_v2",
        "source":{"root_project_id":root.id,"source_chapter_id":chapter.id,"source_snapshot_id":null,"source_snapshot_chapter_id":null,
            "source_revision_key":format!("live:{}:{source_hash}",chapter.id),"content_sha256":source_hash,
            "shooting_script_id":9001,"shooting_script_revision":7,"shooting_script_sha256":"a".repeat(64)},
        "page_plan":{"page_number":1,"beat_refs":[old_ref],"role":"opening","pacing":"slow","importance":"high","reading_flow":"z_path",
            "page_goal":"保留用户修改后的停顿","visual_thesis":"门缝中的光","entry_state":"门关闭","exit_state":"门半开","dominant_panel_number":1,
            "panels":[{"panel_number":1,"beat_refs":[old_ref],"function":"action","size":"dominant","shot_scale":"medium","camera_angle":"eye_level",
                "participants":[],"composition":"门与人物的侧影","observable_action":"停在门前，随后推开门","reaction":"停顿","spatial_relationship":"门位于前方",
                "continuity_from_previous":null,"text_layers":[{"layer_index":1,"kind":"narration","speaker":null,"text":"人工编辑的文字","placement":"top_left","avoid_covering":["脸"]}]}]},
        "drawing_prompt":{"schema_version":"manga_page_drawing_prompt_v2","page_number":1,"image_prompt":"用户改过的整页画面提示词"},
        "reference_bindings":[],"prompt_fingerprint":{}
    });
    let prompt = chapter_manga_prompt::ActiveModel {
        project_id: Set(root.id),
        chapter_id: Set(chapter.id),
        page_number: Set(1),
        prompt_type: Set("page_generation".into()),
        image_prompt: Set("用户改过的整页画面提示词".into()),
        dialogue_text: Set(Some("人工编辑的文字".into())),
        characters_in_frame: Set(Some(json!([]))),
        layout: Set(Some(layout.clone())),
        generated_image_id: Set(None),
        status: Set("completed".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let mut image_ids = Vec::new();
    for index in 1..=2 {
        let image = generated_image::ActiveModel {
            project_id: Set(root.id),
            image_type: Set("manga_page".into()),
            file_path: Set(format!("data/history-page-{index}.png")),
            prompt: Set(prompt.image_prompt.clone()),
            model: Set("historical".into()),
            width: Set(768),
            height: Set(1024),
            file_size: Set(None),
            ai_score: Set(None),
            is_favorite: Set(index == 1),
            chapter_id: Set(Some(chapter.id)),
            panel_id: Set(Some("page_1".into())),
            metadata_json: Set(Some(json!({"page_prompt_id":prompt.id,"page_number":1}))),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        image_ids.push(image.id);
    }
    chapter_manga_prompt::ActiveModel {
        id: Set(prompt.id),
        generated_image_id: Set(Some(image_ids[1])),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap();
    a4n_migration::migrations::media_authoring::migrate_legacy_media(&db)
        .await
        .unwrap();
    let migrated = chapter_manga_prompt::Entity::find_by_id(prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(migrated.project_id, root.id);
    let comic = novel_project::Entity::find_by_id(migrated.project_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(comic.project_kind, "comic");
    assert_eq!(migrated.image_prompt, prompt.image_prompt);
    assert_eq!(migrated.dialogue_text, prompt.dialogue_text);
    let copied_images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(comic.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(
        copied_images.len(),
        2,
        "all saved image versions must remain available"
    );
    assert!(
        copied_images
            .iter()
            .any(|image| Some(image.id) == migrated.generated_image_id)
    );
    assert_eq!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(root.id))
            .all(&db)
            .await
            .unwrap()
            .len(),
        2,
        "existing root references remain valid"
    );
    let docs = media_authoring_document::Entity::find()
        .filter(media_authoring_document::Column::ProjectId.eq(comic.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(docs.len(), 2);
    assert!(docs.iter().all(|document| document.status == "adopted"));
    let adaptation = docs
        .iter()
        .find(|document| document.kind == "adaptation_plan")
        .unwrap();
    assert_eq!(
        adaptation.document_json["units"][0]["beats"][0]["origin"],
        "historical"
    );
    let receipts = media_authoring_migration_receipt::Entity::find()
        .filter(media_authoring_migration_receipt::Column::Kind.eq("manga_layout"))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(receipts[0].before_json["layout"], layout);
    a4n_migration::migrations::media_authoring::migrate_legacy_media(&db)
        .await
        .unwrap();
    assert_eq!(
        media_authoring_document::Entity::find()
            .filter(media_authoring_document::Column::ProjectId.eq(comic.id))
            .all(&db)
            .await
            .unwrap()
            .len(),
        2
    );
}
