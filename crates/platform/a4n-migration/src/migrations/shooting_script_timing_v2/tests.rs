use super::*;
use a4n_domain::{
    chapter_visual_plan::{ChapterVisualPlanV3, SceneBindingV3},
    generation::canonical_json_sha256,
    shooting_script::{ShootingVisualPlanSourceV1, compat::serialized_sha256, source_hash},
};
use sea_orm::TransactionTrait;

async fn fixture(db: &impl ConnectionTrait) -> (String, String) {
    db.execute_unprepared(r#"
        CREATE TEMP TABLE chapter_visual_plan (id bigint, plan_json json, status text, contract_version text) ON COMMIT DROP;
        CREATE TEMP TABLE chapter_shooting_script (id bigint, revision int, script_json json, script_sha256 text, contract_version text) ON COMMIT DROP;
        CREATE TEMP TABLE workflow_run (id bigint, workflow_kind text, status text, can_resume bool, progress_json json) ON COMMIT DROP;
        CREATE TEMP TABLE workflow_checkpoint (id bigint, workflow_run_id bigint, step_key text, input_fingerprint_json json, input_fingerprint_sha256 text, checkpoint_json json, artifacts_json json, status text DEFAULT 'completed') ON COMMIT DROP;
        CREATE TEMP TABLE project_source_snapshot (id bigint, root_project_id bigint, source_version_hash text, blueprint_version_id bigint, chapter_scope_json json, chapter_hashes_json json, profile_hashes_json json) ON COMMIT DROP;
        CREATE TEMP TABLE generation_spec (id bigint, params_json json, source_version_hash text) ON COMMIT DROP;
        CREATE TEMP TABLE generation_run (id bigint, parsed_output_json json, artifact_refs_json json, source_refs_json json) ON COMMIT DROP;
        CREATE TEMP TABLE source_sync_run (id bigint, summary_json json) ON COMMIT DROP;
        CREATE TEMP TABLE source_sync_conflict (id bigint, source_summary_json json, local_summary_json json, affected_refs_json json, decision_json json) ON COMMIT DROP;
        CREATE TEMP TABLE source_sync_decision (id bigint, decision_payload_json json) ON COMMIT DROP;
        CREATE TEMP TABLE chapter_manga_prompt (id bigint, layout json) ON COMMIT DROP;
        CREATE TEMP TABLE generation_task_checkpoint (task_id text, status text, public_snapshot_json jsonb, public_snapshot_sha256 text, execution_checkpoint_json jsonb, execution_checkpoint_sha256 text, result_kind text, result_revision text) ON COMMIT DROP;
    "#).await.unwrap();
    for table in ["chapter_video_prompt", "generated_image", "generated_video"] {
        db.execute_unprepared(&format!("CREATE TEMP TABLE {table} (id bigint, metadata_json json, status text, file_path text) ON COMMIT DROP")).await.unwrap();
    }
    let mut plan = ChapterVisualPlanV3::starter();
    plan.segments[0].prompt = "建立空房间的空间关系".into();
    plan.segments[0].scene_binding = Some(SceneBindingV3::Scene {
        scene_profile_id: 1,
    });
    let director = project_visual_plan_for_shooting_script(&plan, &[]).unwrap();
    let mut script = super::super::video_shot_source_identity::tests::source_script(&plan);
    script.visual_plan_source = Some(ShootingVisualPlanSourceV1 {
        plan_id: 1,
        plan_version: 1,
        plan_sha256: "a".repeat(64),
    });
    script.user_additions = director.user_additions.clone();
    script.scenes[0].visual_narrative_unit_id = Some(director.narrative_units[0].id.clone());
    script.scenes[0].user_addition_ids =
        script.user_additions.iter().map(|a| a.id.clone()).collect();
    let additions = script.scenes[0].user_addition_ids.clone();
    let shot = &mut script.scenes[0].shots[0];
    shot.user_addition_ids = additions;
    shot.visual_shot_id = Some(director.narrative_units[0].shots[0].segment_id.clone());
    shot.planned_start_ms = Some(director.narrative_units[0].shots[0].start_ms);
    shot.planned_end_ms = Some(director.narrative_units[0].shots[0].end_ms);
    shot.estimated_natural_seconds =
        (shot.planned_end_ms.unwrap() - shot.planned_start_ms.unwrap()) as f64 / 1000.0;
    script.validate().unwrap();
    let old_hash = serialized_sha256(&script).unwrap();
    let old_fingerprints = script.shot_content_fingerprints(&director).unwrap();
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO chapter_visual_plan VALUES (1, $1, 'confirmed', 'chapter_visual_plan_v3')",
        [json!(plan).into()],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO chapter_shooting_script VALUES (17, 3, $1, $2, 'chapter_shooting_script_v1')",
        [json!(script).into(), old_hash.clone().into()],
    ))
    .await
    .unwrap();
    let chapter_hashes = json!([{"source_chapter_id": 1, "shooting_script_hash": source_hash(&json!(script)).unwrap()}]);
    let snapshot_payload = json!({"root_project_id": 3, "blueprint_version_id": null, "chapter_scope": {"start": 1,"end": 1}, "chapter_hashes": chapter_hashes, "profile_hashes": []});
    let snapshot_hash = serialized_sha256(&snapshot_payload).unwrap();
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO project_source_snapshot VALUES (3, 3, $1, NULL, $2, $3, '[]')",
        [
            snapshot_hash.clone().into(),
            snapshot_payload["chapter_scope"].clone().into(),
            chapter_hashes.into(),
        ],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO generation_spec VALUES (1, '{}', $1)",
        [snapshot_hash.into()],
    ))
    .await
    .unwrap();
    let progress = json!({"snapshot": {"version": 1, "source_content_sha256": "b".repeat(64), "progress": {"workflow_run_id": 81, "completed_unit_count": 1, "total_unit_count": 2, "script": script}}});
    db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO workflow_run VALUES (81, 'chapter_shooting_script', 'interrupted', true, $1), (82, 'chapter_shooting_script', 'cancelled', false, $2)",
        [progress.clone().into(), progress.clone().into()])).await.unwrap();
    let progress_hash = canonical_json_sha256(&progress).unwrap();
    db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO generation_task_checkpoint VALUES ('kept-task', 'interrupted', $1, $2, $1, $2, 'chapter_shooting_script_progress', $3)",
        [progress.into(), progress_hash.into(), canonical_json_sha256(&json!(script)).unwrap().into()])).await.unwrap();
    let raw = serde_json::to_string(&script.scenes[0]).unwrap();
    let input =
        json!({"messages": [{"role": "system", "content": "ShootingSceneV1 estimated_seconds"}]});
    let input_hash = canonical_json_sha256(&input).unwrap();
    let checkpoint = json!({"parsed_output": script.scenes[0], "response_content": raw, "usage": {"total_tokens": 23}, "finish_reason": "stop"});
    db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO workflow_checkpoint VALUES (5, 81, 'shooting_design_shot:1:shot-1', $1, $2, $3, NULL, 'completed')",
        [input.into(), input_hash.into(), checkpoint.into()])).await.unwrap();
    db.execute_unprepared("INSERT INTO workflow_checkpoint SELECT 6, 82, step_key, NULL, NULL, checkpoint_json, NULL, 'completed' FROM workflow_checkpoint WHERE id=5; INSERT INTO workflow_checkpoint SELECT 7, 81, step_key, NULL, NULL, checkpoint_json, NULL, 'failed' FROM workflow_checkpoint WHERE id=5").await.unwrap();
    for table in ["chapter_video_prompt", "generated_image", "generated_video"] {
        for id in [1_i64, 2] {
            let fingerprints = if id == 1 {
                json!(old_fingerprints)
            } else {
                json!({"shot-1": "already-stale"})
            };
            let value = json!({"video_execution_identity": {"contract_epoch": 5, "shooting_script_identity": {"record_id": 17, "revision": 3, "script_sha256": old_hash, "shot_fingerprints": fingerprints}}});
            db.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                format!("INSERT INTO {table} VALUES ($1, $2, 'completed', 'media/kept.mp4')"),
                [id.into(), value.into()],
            ))
            .await
            .unwrap();
        }
    }
    (old_hash, raw)
}

#[tokio::test]
async fn upgrades_preserve_media_and_partial_work_and_are_idempotent() {
    let db = crate::test_support::connect_test_database().await;
    let tx = db.begin().await.unwrap();
    let (old_hash, raw) = fixture(&tx).await;
    migrate(&tx).await.unwrap();
    migrate(&tx).await.unwrap();
    let script = tx
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM chapter_shooting_script".to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    let new_hash: String = script.try_get("", "script_sha256").unwrap();
    assert_ne!(new_hash, old_hash);
    assert_eq!(script.try_get::<i32>("", "revision").unwrap(), 3);
    let value: Value = script.try_get("", "script_json").unwrap();
    assert_eq!(value["schema_version"], "chapter_shooting_script_v2");
    assert!(
        value["scenes"][0]["shots"][0]
            .get("estimated_natural_seconds")
            .is_none()
    );
    let progress = tx
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT progress_json FROM workflow_run WHERE id=81".to_string(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<Value>("", "progress_json")
        .unwrap();
    assert_eq!(progress["snapshot"]["progress"]["completed_unit_count"], 1);
    assert_eq!(progress["snapshot"]["progress"]["total_unit_count"], 2);
    let archived = tx
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status, can_resume, progress_json FROM workflow_run WHERE id=82".to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        archived.try_get::<String>("", "status").unwrap(),
        "cancelled"
    );
    assert!(!archived.try_get::<bool>("", "can_resume").unwrap());
    assert_eq!(
        archived.try_get::<Value>("", "progress_json").unwrap()["snapshot"]["progress"]["script"]["schema_version"],
        "chapter_shooting_script_v1"
    );
    for row in tx
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            "SELECT checkpoint_json, artifacts_json FROM workflow_checkpoint WHERE id IN (6,7)"
                .to_string(),
        ))
        .await
        .unwrap()
    {
        assert!(
            row.try_get::<Option<Value>>("", "artifacts_json")
                .unwrap()
                .is_none()
        );
        assert!(
            row.try_get::<Value>("", "checkpoint_json").unwrap()["parsed_output"]["shots"][0]
                .get("estimated_natural_seconds")
                .is_some()
        );
    }
    let snapshot = tx.query_one(Statement::from_string(DbBackend::Postgres,
        "SELECT s.source_version_hash, s.chapter_hashes_json, g.source_version_hash AS spec_hash FROM project_source_snapshot s JOIN generation_spec g ON g.id=1 WHERE s.id=3".to_string())).await.unwrap().unwrap();
    assert_eq!(
        snapshot
            .try_get::<String>("", "source_version_hash")
            .unwrap(),
        snapshot.try_get::<String>("", "spec_hash").unwrap()
    );
    assert_eq!(
        snapshot
            .try_get::<Value>("", "chapter_hashes_json")
            .unwrap()[0]["shooting_script_hash"],
        source_hash(&value).unwrap()
    );
    let task = tx
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT * FROM generation_task_checkpoint WHERE task_id='kept-task'".to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    for (column, hash) in [
        ("public_snapshot_json", "public_snapshot_sha256"),
        ("execution_checkpoint_json", "execution_checkpoint_sha256"),
    ] {
        let upgraded = task.try_get::<Value>("", column).unwrap();
        assert_eq!(upgraded, progress);
        assert_eq!(
            task.try_get::<String>("", hash).unwrap(),
            canonical_json_sha256(&upgraded).unwrap()
        );
    }
    assert_eq!(task.try_get::<String>("", "status").unwrap(), "interrupted");
    assert_eq!(
        task.try_get::<String>("", "result_revision").unwrap(),
        canonical_json_sha256(&value).unwrap()
    );
    let checkpoint = tx
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT checkpoint_json, artifacts_json FROM workflow_checkpoint WHERE id=5"
                .to_string(),
        ))
        .await
        .unwrap()
        .unwrap();
    let checkpoint_value: Value = checkpoint.try_get("", "checkpoint_json").unwrap();
    assert_eq!(checkpoint_value["response_content"], raw);
    assert_eq!(checkpoint_value["usage"]["total_tokens"], 23);
    a4n_domain::shooting_script::compat::checkpoint::validate_checkpoint_upgrade(
        &checkpoint_value,
        &checkpoint.try_get::<Value>("", "artifacts_json").unwrap(),
    )
    .unwrap();
    for table in ["chapter_video_prompt", "generated_image", "generated_video"] {
        for row in tx
            .query_all(Statement::from_string(
                DbBackend::Postgres,
                format!("SELECT * FROM {table} ORDER BY id"),
            ))
            .await
            .unwrap()
        {
            let id: i64 = row.try_get("", "id").unwrap();
            let metadata: Value = row.try_get("", "metadata_json").unwrap();
            assert_eq!(
                row.try_get::<String>("", "file_path").unwrap(),
                "media/kept.mp4"
            );
            assert_eq!(row.try_get::<String>("", "status").unwrap(), "completed");
            assert_eq!(metadata["video_execution_identity"]["contract_epoch"], 5);
            assert_eq!(
                metadata["video_execution_identity"]["shooting_script_identity"]["script_sha256"],
                new_hash
            );
            if id == 2 {
                assert_eq!(
                    metadata["video_execution_identity"]["shooting_script_identity"]["shot_fingerprints"]
                        ["shot-1"],
                    "already-stale"
                );
            }
        }
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn corrupt_old_hash_aborts_without_committing_partial_conversion() {
    let db = crate::test_support::connect_test_database().await;
    let tx = db.begin().await.unwrap();
    let (old_hash, _) = fixture(&tx).await;
    tx.execute_unprepared("INSERT INTO chapter_shooting_script SELECT 18, revision, script_json, 'corrupt', contract_version FROM chapter_shooting_script WHERE id=17").await.unwrap();
    let attempt = tx.begin().await.unwrap();
    let failure = migrate(&attempt).await.unwrap_err();
    assert!(failure.to_string().contains("#18"));
    attempt.rollback().await.unwrap();
    let hash = tx
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT script_sha256 FROM chapter_shooting_script WHERE id=17".to_string(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "script_sha256")
        .unwrap();
    assert_eq!(hash, old_hash);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn missing_completed_call_fingerprint_aborts_the_upgrade() {
    let db = crate::test_support::connect_test_database().await;
    let tx = db.begin().await.unwrap();
    let (old_hash, _) = fixture(&tx).await;
    tx.execute_unprepared(
        "UPDATE workflow_checkpoint SET input_fingerprint_sha256 = NULL WHERE id=5",
    )
    .await
    .unwrap();
    let attempt = tx.begin().await.unwrap();
    let failure = migrate(&attempt).await.unwrap_err().to_string();
    assert!(failure.contains("workflow_checkpoint #5"), "{failure}");
    assert!(failure.contains("缺少输入指纹哈希"), "{failure}");
    attempt.rollback().await.unwrap();
    let hash = tx
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT script_sha256 FROM chapter_shooting_script WHERE id=17".to_string(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "script_sha256")
        .unwrap();
    assert_eq!(hash, old_hash);
    tx.rollback().await.unwrap();
}
