use a4n_db::project::{
    CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope,
    create_derived_project_record, create_project_record, resolve_chapter_source,
};
use a4n_db_models::models::{
    chapter::chapter_visual_plan,
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::{
        generation_run, generation_run_target, generation_task_checkpoint, workflow_checkpoint,
        workflow_run,
    },
    project::{media_authoring_document, media_authoring_migration_receipt},
    task::task_queue,
};
use a4n_domain::{chapter::ImportChapterContentInput, generation::canonical_json_sha256};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Set,
    Statement,
};
use serde_json::json;

async fn source(
    db: &sea_orm::DatabaseConnection,
    kind: &str,
) -> a4n_domain::chapter::ResolvedChapterSource {
    let root = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "历史媒体回归".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let chapter = a4n_db::chapter_content::import_chapter_content_record(
        db,
        ImportChapterContentInput {
            project_id: root.id,
            chapter_number: 1,
            content: "她推开门。".into(),
            title: None,
        },
        "historical media fixture",
    )
    .await
    .unwrap();
    let project = create_derived_project_record(
        db,
        1,
        CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: kind.into(),
            title: "历史派生".into(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    resolve_chapter_source(db, project.id, chapter.id)
        .await
        .unwrap()
}

#[tokio::test]
async fn legacy_film_with_deleted_snapshot_recovers_from_exact_live_body() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "丢失快照的历史媒体".into(),
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
        "missing snapshot migration fixture",
    )
    .await
    .unwrap();
    let plan = a4n_domain::chapter_visual_plan::ChapterVisualPlanV3::starter();
    plan.validate_draft().unwrap();
    let plan_json = serde_json::to_value(&plan).unwrap();
    let plan_hash = a4n_domain::generation::canonical_json_sha256(&plan).unwrap();
    let content_hash = a4n_domain::source_text_hash(&chapter.content);

    db.execute_unprepared(
        "DROP TRIGGER IF EXISTS video_visual_plan_owner ON chapter_visual_plan; ALTER TABLE chapter_visual_plan ALTER COLUMN project_id DROP NOT NULL",
    )
    .await
    .unwrap();
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO chapter_visual_plan(root_project_id, source_chapter_id, source_snapshot_id, source_snapshot_chapter_id, source_revision_key, source_content_sha256, contract_version, plan_version, revision, status, authoring_mode, plan_sha256, plan_json, author_user_id, created_at, updated_at) VALUES($1,$2,999,999,$3,$4,'chapter_visual_plan_v3',1,1,'draft','manual',$5,$6,1,LOCALTIMESTAMP,LOCALTIMESTAMP) RETURNING id",
            [
                root.id.into(),
                chapter.id.into(),
                format!("snapshot:999:chapter:999:{content_hash}").into(),
                content_hash.clone().into(),
                plan_hash.into(),
                plan_json.into(),
            ],
        ))
        .await
        .unwrap()
        .unwrap();
    let plan_id: i64 = row.try_get("", "id").unwrap();

    a4n_migration::migrations::media_authoring::migrate_legacy_media(&db)
        .await
        .unwrap();

    let migrated = chapter_visual_plan::Entity::find_by_id(plan_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(migrated.source_snapshot_id, Some(999));
    assert_ne!(migrated.source_snapshot_chapter_id, Some(999));
    assert_eq!(migrated.source_content_sha256, content_hash);
    assert!(migrated.project_id > 0);
}

#[tokio::test]
async fn legacy_game_keeps_main_and_branch_edits_and_enforces_local_beat_identity() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let source = source(&db, "galgame").await;
    let now = chrono::Utc::now().naive_utc();
    let script = galgame_script::ActiveModel {
        project_id: Set(source.requested_project_id),
        root_project_id: Set(source.root_project_id),
        source_snapshot_id: Set(source.source_snapshot_id.unwrap()),
        title: Set("手改游戏".into()),
        style_config_json: Set(json!({})),
        status: Set("draft".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let old = json!({
        "schema_version":"galgame_line_presentation_v2", "visible_cast":[], "enter":[], "exit":[],
        "focus_character_profile_id":null, "hold":false, "player_knowledge":[], "cg":null, "cg_note":null,
        "cg_importance":null, "transition":null, "branch":null, "branch_id":null, "returns_to_scene_index":null, "after_beat_ref":null
    });
    let mut scenes = Vec::new();
    let mut lines = Vec::new();
    // Recreate persisted v2 rows before the new constraint is installed by migration.
    db.execute_unprepared("ALTER TABLE galgame_line DISABLE TRIGGER USER")
        .await
        .unwrap();
    for (index, kind) in ["main", "branch"].into_iter().enumerate() {
        let scene = galgame_scene::ActiveModel {
            script_id: Set(script.id),
            source_snapshot_chapter_id: Set(source.source_snapshot_chapter_id.unwrap()),
            scene_index: Set(index as i32 + 1),
            title: Set(format!("人工编辑的{kind}")),
            participants_json: Set(json!([])),
            entry_kind: Set(kind.into()),
            next_scene_id: Set(scenes.first().copied()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        scenes.push(scene.id);
        lines.push(
            galgame_line::ActiveModel {
                scene_id: Set(scene.id),
                line_index: Set(1),
                line_type: Set("narration".into()),
                text: Set(format!("人工编辑的{kind}旁白")),
                presentation_json: Set(old.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&db)
            .await
            .unwrap(),
        );
    }
    db.execute_unprepared("ALTER TABLE galgame_line ENABLE TRIGGER USER")
        .await
        .unwrap();
    use a4n_db_models::models::source_sync::{
        source_sync_conflict, source_sync_decision, source_sync_run,
    };
    let sync = source_sync_run::ActiveModel {
        derived_project_id: Set(source.requested_project_id),
        root_project_id: Set(source.root_project_id),
        from_snapshot_id: Set(source.source_snapshot_id.unwrap()),
        to_snapshot_id: Set(source.source_snapshot_id.unwrap()),
        mode: Set("apply".into()),
        status: Set("completed".into()),
        summary_json: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let old_stages = json!({"shooting_script_projection":{"scenes":[{"shooting_script_id":77}]},
        "script_adaptation":{"saved_text":"旧模型原始结果"},"character_staging":{},"visual_planning":{},"branch_design":{}});
    let decision_payload =
        json!({"galgame_scene_id":scenes[0],"result":{"stage_outputs":old_stages}});
    let conflict = source_sync_conflict::ActiveModel {
        sync_run_id: Set(sync.id),
        derived_project_id: Set(source.requested_project_id),
        root_project_id: Set(source.root_project_id),
        from_snapshot_id: Set(source.source_snapshot_id.unwrap()),
        to_snapshot_id: Set(source.source_snapshot_id.unwrap()),
        conflict_scope: Set("text_anchor".into()),
        target_kind: Set("galgame_scene".into()),
        target_id: Set(scenes[0]),
        source_change_kind: Set("updated".into()),
        local_state_kind: Set("user_edited".into()),
        severity: Set("warning".into()),
        status: Set("resolved".into()),
        resolution_strategy: Set(Some("regenerate_scene".into())),
        decision_json: Set(Some(decision_payload.clone())),
        source_summary_json: Set(json!({})),
        local_summary_json: Set(json!({})),
        affected_refs_json: Set(json!({})),
        created_at: Set(now),
        resolved_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let decision = source_sync_decision::ActiveModel {
        conflict_id: Set(conflict.id),
        decided_by: Set("user".into()),
        strategy: Set("regenerate_scene".into()),
        decision_payload_json: Set(decision_payload),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    a4n_migration::migrations::media_authoring::migrate_legacy_media(&db)
        .await
        .unwrap();
    let archived = source_sync_decision::Entity::find_by_id(decision.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap()
        .decision_payload_json;
    assert_eq!(archived["result"]["stage_outputs"]["original"], old_stages);
    assert!(
        a4n_domain::media_authoring::legacy::source_sync::archived_stage_outputs(
            &archived["result"]["stage_outputs"]
        )
        .unwrap()
    );
    assert_eq!(
        source_sync_conflict::Entity::find_by_id(conflict.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .decision_json,
        Some(archived)
    );
    for original in &lines {
        let line = galgame_line::Entity::find_by_id(original.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let scene = galgame_scene::Entity::find_by_id(line.scene_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let plan = media_authoring_document::Entity::find_by_id(scene.adaptation_plan_id.unwrap())
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(line.text, original.text);
        assert_eq!(line.presentation_json["origin"], "historical");
        assert_eq!(
            line.presentation_json["schema_version"],
            "galgame_line_presentation_v3"
        );
        assert_eq!(
            line.presentation_json["beat_ref"]["unit_id"],
            scene.adaptation_unit_id.unwrap()
        );
        assert_eq!(plan.project_id, source.requested_project_id);
        assert_eq!(
            plan.document_json["units"][0]["beats"][0]["action"],
            original.text
        );
        assert!(matches!(plan.status.as_str(), "adopted" | "superseded"));
        let receipt = media_authoring_migration_receipt::Entity::find()
            .filter(media_authoring_migration_receipt::Column::Kind.eq("galgame_presentation"))
            .filter(media_authoring_migration_receipt::Column::OldId.eq(original.id))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(receipt.before_json["presentation_json"], old);
    }
    let line = galgame_line::Entity::find_by_id(lines[0].id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let mut presentation = line.presentation_json.clone();
    presentation["origin"] = "added".into();
    galgame_line::ActiveModel {
        id: Set(line.id),
        presentation_json: Set(presentation.clone()),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap();
    presentation["origin"] = "source".into();
    let error = galgame_line::ActiveModel {
        id: Set(line.id),
        presentation_json: Set(presentation),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap_err();
    assert!(error.to_string().contains("本地改编版本"));
    let mut foreign = line.presentation_json.clone();
    foreign["beat_ref"]["unit_id"] = format!("legacy-scene-{}", scenes[1]).into();
    assert!(
        galgame_line::ActiveModel {
            id: Set(line.id),
            presentation_json: Set(foreign),
            ..Default::default()
        }
        .update(&db)
        .await
        .is_err()
    );
    a4n_migration::migrations::media_authoring::migrate_legacy_media(&db)
        .await
        .unwrap();
    assert_eq!(
        media_authoring_document::Entity::find()
            .all(&db)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn legacy_partial_workflow_and_task_keep_evidence_and_stay_nonresumable_after_transfer() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let source = source(&db, "comic").await;
    let now = chrono::Utc::now().naive_utc();
    let task_id = uuid::Uuid::new_v4().to_string();
    let fingerprint = json!({"source":{"root_project_id":source.root_project_id,"source_chapter_id":source.source_chapter_id,"content_sha256":source.content_sha256}});
    let workflow = workflow_run::ActiveModel {
        business_key: Set(format!(
            "manga_pipeline:{}:{}",
            source.root_project_id, source.source_chapter_id
        )),
        workflow_kind: Set("manga_pipeline".into()),
        project_id: Set(Some(source.root_project_id)),
        root_project_id: Set(Some(source.root_project_id)),
        chapter_id: Set(Some(source.source_chapter_id)),
        scope_table: Set(Some("chapter".into())),
        scope_id: Set(Some(source.source_chapter_id)),
        input_fingerprint_json: Set(Some(fingerprint.clone())),
        status: Set("running".into()),
        active_task_id: Set(Some(task_id.clone())),
        can_resume: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let raw = json!({"schema_version":"old", "text":"未完成的模型输出", "project_id":source.root_project_id});
    let checkpoint = workflow_checkpoint::ActiveModel {
        workflow_run_id: Set(workflow.id),
        step_key: Set("page_planning".into()),
        step_order: Set(1),
        status: Set("completed".into()),
        input_fingerprint_json: Set(Some(fingerprint.clone())),
        input_fingerprint_sha256: Set(Some(canonical_json_sha256(&fingerprint).unwrap())),
        checkpoint_json: Set(Some(raw.clone())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let generation = generation_run::ActiveModel {
        project_id: Set(source.root_project_id),
        root_project_id: Set(source.root_project_id),
        workflow: Set("manga_pipeline".into()),
        workflow_run_id: Set(Some(workflow.id)),
        step_key: Set("page_planning".into()),
        title: Set("历史模型调用".into()),
        model_kind: Set("llm".into()),
        status: Set("completed".into()),
        attempt: Set(1),
        request_json: Set(Some(fingerprint.clone())),
        raw_response_json: Set(Some(raw.clone())),
        created_at: Set(now),
        completed_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    generation_run_target::ActiveModel {
        generation_run_id: Set(generation.id),
        target_kind: Set("record".into()),
        target_table: Set("chapter_manga_prompt".into()),
        target_id: Set(999999),
        role: Set("output".into()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    task_queue::ActiveModel {
        id: Set(task_id.clone()),
        task_type: Set("manga_pipeline".into()),
        priority: Set("medium".into()),
        status: Set("running".into()),
        project_id: Set(Some(source.root_project_id)),
        root_project_id: Set(Some(source.root_project_id)),
        workflow_run_id: Set(Some(workflow.id)),
        payload: Set(Some(raw.clone())),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let draft = "尚未保存的漫画草稿";
    let snapshot = json!({
        "schema_version":1, "task_id":task_id, "attempt":1, "covers_through_sequence":3, "status":"running",
        "started":{"task_kind":"manga_pipeline", "delivery_mode":"stream", "project_id":source.root_project_id, "root_project_id":source.root_project_id,
            "workflow_run_id":workflow.id, "generation_run_id":generation.id, "target":null, "batch_policy":null, "root_task_id":task_id,
            "parent_task_id":null, "item_key":null, "retry_of_task_id":null},
        "cancel_reason":null, "progress":null, "streams":[{"stream_id":"draft", "text":draft, "next_offset":draft.len(), "sha256":a4n_domain::source_text_hash(draft)}],
        "artifacts":[], "usage":null, "latest_checkpoint":{"checkpoint_key":"page_planning", "resumable":true, "summary":null},
        "terminal_event":null, "updated_at_ms":now.and_utc().timestamp_millis()
    });
    serde_json::from_value::<a4n_service_protocol::GenerationTaskSnapshot>(snapshot.clone())
        .unwrap()
        .validate()
        .unwrap();
    generation_task_checkpoint::ActiveModel {
        task_id: Set(task_id.clone()),
        business_key: Set(workflow.business_key.clone()),
        attempt: Set(1),
        requested_by_user_id: Set(1),
        task_kind: Set("manga_pipeline".into()),
        delivery_mode: Set("stream".into()),
        project_id: Set(Some(source.root_project_id)),
        root_project_id: Set(Some(source.root_project_id)),
        workflow_run_id: Set(Some(workflow.id)),
        generation_run_id: Set(Some(generation.id)),
        root_task_id: Set(task_id.clone()),
        status: Set("running".into()),
        checkpoint_revision: Set(1),
        covers_through_sequence: Set(3),
        reserved_through_sequence: Set(5),
        public_snapshot_json: Set(Some(snapshot.clone())),
        public_snapshot_sha256: Set(Some(canonical_json_sha256(&snapshot).unwrap())),
        provider_kind: Set(Some("openai".into())),
        provider_job_id: Set(Some("original-provider-job".into())),
        provider_idempotency_key_sha256: Set(Some("a".repeat(64))),
        resume_metadata_version: Set(Some(1)),
        execution_checkpoint_json: Set(Some(raw.clone())),
        execution_checkpoint_sha256: Set(Some(canonical_json_sha256(&raw).unwrap())),
        input_fingerprint_sha256: Set(canonical_json_sha256(&fingerprint).unwrap()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    a4n_migration::migrations::media_authoring::migrate_legacy_media(&db)
        .await
        .unwrap();
    let migrated = workflow_run::Entity::find_by_id(workflow.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(migrated.project_id, Some(source.root_project_id));
    assert_eq!(migrated.mode.as_deref(), Some("legacy_media_authoring"));
    assert_eq!(migrated.status, "interrupted");
    assert!(!migrated.can_resume);
    assert!(migrated.active_task_id.is_none());
    assert_eq!(
        workflow_checkpoint::Entity::find_by_id(checkpoint.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .checkpoint_json,
        Some(raw.clone())
    );
    let migrated_call = generation_run::Entity::find_by_id(generation.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(Some(migrated_call.project_id), migrated.project_id);
    assert_eq!(migrated_call.raw_response_json, Some(raw.clone()));
    assert_eq!(migrated_call.request_json, Some(fingerprint));
    let task = generation_task_checkpoint::Entity::find_by_id(task_id.clone())
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(task.project_id, migrated.project_id);
    assert_eq!(task.status, "interrupted");
    assert_eq!(task.execution_checkpoint_json, Some(raw.clone()));
    assert_eq!(
        task.provider_job_id.as_deref(),
        Some("original-provider-job")
    );
    let public = task.public_snapshot_json.unwrap();
    assert_eq!(public["streams"], snapshot["streams"]);
    assert_eq!(public["latest_checkpoint"]["resumable"], false);
    assert_eq!(public["covers_through_sequence"], 6);
    serde_json::from_value::<a4n_service_protocol::GenerationTaskSnapshot>(public)
        .unwrap()
        .validate()
        .unwrap();
    assert_eq!(
        task_queue::Entity::find_by_id(task_id.clone())
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .payload,
        Some(raw)
    );

    use a4n_db::project::bundle::{
        ProjectBundleEntityIdMaps, export_project_bundle_history, import_project_bundle_history,
    };
    use std::collections::HashMap;
    let history = export_project_bundle_history(&db, &[migrated.project_id.unwrap()])
        .await
        .unwrap();
    let maps = ProjectBundleEntityIdMaps {
        project_ids: HashMap::from([
            (source.root_project_id, source.root_project_id),
            (migrated.project_id.unwrap(), source.requested_project_id),
        ]),
        root_project_ids: HashMap::from([(source.root_project_id, source.root_project_id)]),
        chapter_ids: HashMap::from([(source.source_chapter_id, source.source_chapter_id)]),
        source_snapshot_ids: HashMap::from([(
            migrated.source_snapshot_id.unwrap(),
            source.source_snapshot_id.unwrap(),
        )]),
        entity_ids: HashMap::from([("chapter_manga_prompt".into(), HashMap::new())]),
        ..Default::default()
    };
    import_project_bundle_history(&db, &history, &maps, "legacy-transfer", 1)
        .await
        .unwrap();
    let restored = export_project_bundle_history(&db, &[source.requested_project_id])
        .await
        .unwrap();
    assert_eq!(restored.workflow_runs.len(), 1);
    assert!(!restored.workflow_runs[0].can_resume);
    assert_eq!(
        restored.generation_run_targets[0].target_table,
        "source_archive:chapter_manga_prompt"
    );
    assert_eq!(restored.generation_run_targets[0].target_id, 999999);
    assert_eq!(
        restored.workflow_checkpoints[0].checkpoint_json,
        history.workflow_checkpoints[0].checkpoint_json
    );
    a4n_migration::migrations::media_authoring::migrate_legacy_media(&db)
        .await
        .unwrap();
    assert_eq!(
        generation_task_checkpoint::Entity::find_by_id(task_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .checkpoint_revision,
        task.checkpoint_revision
    );
}
