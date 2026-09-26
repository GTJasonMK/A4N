use super::*;
use a4n_domain::chapter_visual_plan::SceneBindingV3;
use sea_orm::TransactionTrait;

fn source_script(plan: &ChapterVisualPlanV3) -> ChapterShootingScriptV1 {
    let director = project_visual_plan_for_shooting_script(plan, &[]).unwrap();
    let mut script: Value = serde_json::from_str(r#"{
        "schema_version": "chapter_shooting_script_v1", "directing_statement": null,
        "scenes": [{
            "scene_index": 1, "scene_identity": "unit:shot-1", "sequence_index": 1,
            "narrative_unit_index": 1, "timeline_position": 1, "editing_mode": "continuity",
            "transition_contracts": [], "title": "门口", "slugline": "内景 夜",
            "participants": [], "source_span_ids": [], "dramatic_question": "能否进入",
            "scene_objective": "进入", "scene_obstacle": "门锁", "relationship_before": "独自",
            "relationship_after": "独自", "audience_knowledge_before": [], "audience_knowledge_after": [],
            "withheld_information": [], "viewpoint": {"mode": "limited_third_person", "information_strategy": "跟随人物"},
            "spatial_continuity": {"continuity_intent": "transparent"}, "sound_strategy": "安静",
            "entry_state": "门口", "exit_state": "室内", "shots": [{
                "shot_index": 1, "shot_id": "shot-1", "title": "入门", "source_span_ids": [],
                "visual_constraints": null, "coverage": "must_show",
                "narrative_purpose": "入门", "subject": "人物", "shot_size": "medium",
                "camera_angle": "eye_level", "camera_height": "eye", "lens_intent": "空间关系",
                "camera_movement": "locked", "composition": "人物居中", "spatial_anchors": [],
                "blocking": {"start": "门口", "movement": "前进", "end": "室内"},
                "performance_beats": [], "dialogue_cues": [],
                "sound": {"dialogue": [], "environment": [], "effects": [], "music": [], "silence": []},
                "camera_start_state": "中景", "camera_end_state": "中景", "start_state": "门口", "end_state": "室内",
                "transition_in": "切入", "transition_out": "切出", "continuity_requirements": [], "estimated_natural_seconds": 8.0
            }]
        }]
    }"#).unwrap();
    script["scenes"][0]["shots"][0]["visual_constraints"] =
        json!(director.narrative_units[0].shots[0]);
    serde_json::from_value(script).unwrap()
}

#[tokio::test]
async fn migration_proves_exact_sources_keeps_unknown_sources_stale_and_preserves_media() {
    let db = crate::test_support::connect_test_database().await;
    let tx = db.begin().await.unwrap();
    tx.execute_unprepared(r#"
        CREATE TEMP TABLE chapter_visual_plan (id bigint, plan_json json, plan_sha256 text, plan_version int) ON COMMIT DROP;
        CREATE TEMP TABLE chapter_shooting_script (id bigint, revision int, source_revision_key text, source_content_sha256 text,
            script_sha256 text, script_json json, visual_plan_id bigint, visual_plan_version int, visual_plan_sha256 text) ON COMMIT DROP;
    "#).await.unwrap();
    let mut plan = ChapterVisualPlanV3::starter();
    plan.segments[0].prompt = "人物进入房间".into();
    plan.segments[0].scene_binding = Some(SceneBindingV3::Scene {
        scene_profile_id: 1,
    });
    let script = source_script(&plan);
    let fingerprints = script
        .shot_content_fingerprints(&project_visual_plan_for_shooting_script(&plan, &[]).unwrap())
        .unwrap();
    assert!(fingerprints.contains_key("shot-1"));
    tx.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO chapter_visual_plan VALUES (1, $1, 'plan-hash', 1)",
        [json!(plan).into()],
    ))
    .await
    .unwrap();
    tx.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO chapter_shooting_script VALUES (1, 2, 'source', 'body-hash', 'script-hash', $1, 1, 1, 'plan-hash')",
        [json!(script).into()],
    )).await.unwrap();
    for table in ["chapter_video_prompt", "generated_video", "generated_image"] {
        tx.execute_unprepared(&format!("CREATE TEMP TABLE {table} (id bigint, metadata_json json, status text, file_path text) ON COMMIT DROP")).await.unwrap();
        for id in 1..=4i64 {
            let mut metadata = json!({
                "video_execution_identity": {
                    "projection_sha256": "obsolete", "segment_plan_sha256": "obsolete", "unit_plan_sha256": "obsolete",
                    "segment_sha256": "keep-segment-hash",
                    "shooting_script_identity": {
                        "record_id": 1, "revision": 2, "source_revision_key": "source",
                        "source_content_sha256": "body-hash", "script_sha256": "script-hash"
                    }
                }
            });
            if id == 2 {
                metadata["video_execution_identity"]["shooting_script_identity"]["revision"] =
                    json!(1);
            } else if id == 3 {
                metadata["video_execution_identity"]["shooting_script_identity"]["shot_fingerprints"] =
                    json!({"shot-1": "older-shot"});
            } else if id == 4 {
                metadata = json!({"unrelated": true});
            }
            tx.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                format!("INSERT INTO {table} VALUES ($1, $2, 'completed', 'media/original.mp4')"),
                [id.into(), metadata.into()],
            ))
            .await
            .unwrap();
        }
    }
    migrate_metadata(&tx).await.unwrap();
    migrate_metadata(&tx).await.unwrap();
    for table in ["chapter_video_prompt", "generated_video", "generated_image"] {
        let rows = tx
            .query_all(Statement::from_string(
                DbBackend::Postgres,
                format!("SELECT * FROM {table} ORDER BY id"),
            ))
            .await
            .unwrap();
        for row in rows {
            let id: i64 = row.try_get("", "id").unwrap();
            let metadata: Value = row.try_get("", "metadata_json").unwrap();
            assert_eq!(row.try_get::<String>("", "status").unwrap(), "completed");
            assert_eq!(
                row.try_get::<String>("", "file_path").unwrap(),
                "media/original.mp4"
            );
            if id == 4 {
                assert_eq!(metadata, json!({"unrelated": true}));
                continue;
            }
            let identity = &metadata["video_execution_identity"];
            assert!(identity.get("projection_sha256").is_none());
            assert_eq!(identity["segment_sha256"], "keep-segment-hash");
            assert_eq!(
                identity["shooting_script_identity"]["shot_fingerprints"],
                match id {
                    1 => json!(fingerprints),
                    2 => json!({}),
                    3 => json!({"shot-1": "older-shot"}),
                    _ => unreachable!(),
                }
            );
        }
    }
    tx.rollback().await.unwrap();
}
