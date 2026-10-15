use super::*;
use crate::MangaRecordWorker;
use a4n_api_contract::manga::MangaPagePromptSummary;
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::chapter::{chapter, chapter_manga_prompt, manga_pipeline_artifact};
use a4n_domain::manga::{MangaPageSlotsV1, MangaPipelineArtifactStage};
use a4n_manga_record_worker_contract::{
    MANGA_RECORD_LIST_OPERATION, MANGA_RECORD_WORKER_ID, MangaChapterInput,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};
use serde_json::{Value, json};
use std::time::Instant;

const PAGE_COUNT: i32 = 100;
const PAGE_ARTIFACTS_PER_PAGE: usize = 7;
const ARTIFACT_PAYLOAD_BYTES: usize = 8 * 1024;
const WARMUPS: usize = 2;
const SAMPLES: usize = 15;

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

fn request(project_id: i64, chapter_id: i64, user_id: i64) -> WorkerEnvelope<MangaChapterInput> {
    WorkerEnvelope {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: "manga-prompt-performance".into(),
        trace_id: "manga-prompt-performance".into(),
        principal: WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: user_id.to_string(),
        },
        operation_id: MANGA_RECORD_LIST_OPERATION.into(),
        idempotency_key: "manga-prompt-performance".into(),
        project_id: Some(project_id),
        root_project_id: None,
        target_revision: None,
        runtime_config_snapshot_ref: None,
        lease: WorkerLease {
            worker_id: MANGA_RECORD_WORKER_ID.into(),
            epoch: 1,
            fence_token: "manga-prompt-performance".into(),
            expires_at: "2099-01-01T00:00:00Z".into(),
        },
        payload: MangaChapterInput {
            project_id,
            chapter_id,
        },
    }
}

fn page_layout(page_number: i32) -> Value {
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

fn page_slots_payload() -> Value {
    json!({
        "schema_version": "manga_page_slots_v1",
        "pages": (1..PAGE_COUNT).map(|page_number| json!({
            "page_number": page_number,
            "content": {
                "summary": format!("页面 {page_number}"),
                "source_span_ids": [],
                "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
                "action": "角色继续行动",
                "reaction": null,
                "dialogue_intent": null,
                "entry_state": "连续状态",
                "exit_state": "连续状态"
            }
        })).collect::<Vec<_>>()
    })
}

async fn fixture() -> (WorkerState, WorkerEnvelope<MangaChapterInput>, usize, usize) {
    let (db, database_url) = a4n_migration::test_support::connect_test_database_with_url().await;
    let user = a4n_db::auth::create_user_summary(
        &db,
        &format!("manga-performance-{}", std::process::id()),
        &format!("manga-performance-{}@example.test", std::process::id()),
        "benchmark-password",
        "user",
    )
    .await
    .expect("benchmark user must be created");
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: user.id,
            title: format!("Manga prompt performance {}", std::process::id()),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("benchmark project must be created");
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("性能章节".into())),
        content: Set("性能基准正文".into()),
        word_count: Set(6),
        status: Set(chapter::STATUS_COMPLETED.into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("benchmark chapter must be created");

    let prompt_rows = (1..=PAGE_COUNT)
        .map(|page_number| chapter_manga_prompt::ActiveModel {
            chapter_id: Set(chapter.id),
            project_id: Set(project.id),
            page_number: Set(page_number),
            prompt_type: Set(a4n_db::manga::MANGA_PAGE_PROMPT_TYPE.into()),
            image_prompt: Set(format!("第 {page_number} 页 {}", "p".repeat(1024))),
            dialogue_text: Set(Some(format!("对白 {page_number}"))),
            characters_in_frame: Set(Some(json!(["hero"]))),
            layout: Set(Some(page_layout(page_number))),
            generated_image_id: Set(None),
            status: Set("pending".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    chapter_manga_prompt::Entity::insert_many(prompt_rows)
        .exec(&db)
        .await
        .expect("benchmark page prompts must be created");

    let payload_padding = "x".repeat(ARTIFACT_PAYLOAD_BYTES);
    let page_stages = MangaPipelineArtifactStage::all()
        .iter()
        .filter(|stage| stage.page_scoped())
        .count();
    assert_eq!(page_stages, PAGE_ARTIFACTS_PER_PAGE);
    let padded_payload = json!({"data": payload_padding});
    let padded_payload_bytes = serde_json::to_vec(&padded_payload)
        .expect("payload serialization")
        .len();
    let slots_payload = page_slots_payload();
    let old_artifact_payload_bytes = padded_payload_bytes * (1 + PAGE_COUNT as usize * page_stages)
        + serde_json::to_vec(&slots_payload)
            .expect("payload serialization")
            .len();
    let mut artifact_rows = vec![
        artifact_row(
            project.id,
            chapter.id,
            MangaPipelineArtifactStage::Preprocessing,
            0,
            false,
            padded_payload.clone(),
            now,
        ),
        artifact_row(
            project.id,
            chapter.id,
            MangaPipelineArtifactStage::PageSlots,
            0,
            false,
            slots_payload.clone(),
            now,
        ),
    ];
    for page_number in 1..=PAGE_COUNT {
        for stage in MangaPipelineArtifactStage::all()
            .iter()
            .copied()
            .filter(|stage| stage.page_scoped())
        {
            artifact_rows.push(artifact_row(
                project.id,
                chapter.id,
                stage,
                page_number,
                stage == MangaPipelineArtifactStage::PageContent && page_number % 10 == 0,
                padded_payload.clone(),
                now,
            ));
        }
    }
    let artifact_rows_count = artifact_rows.len();
    for rows in artifact_rows.chunks(250) {
        manga_pipeline_artifact::Entity::insert_many(rows.iter().cloned())
            .exec(&db)
            .await
            .expect("benchmark artifacts must be created");
    }

    let database = a4n_db::manga::connect_manga_record_worker_database(database_url)
        .await
        .expect("worker database pool must connect");
    let state = MangaRecordWorker::new(database, MANGA_RECORD_WORKER_ID, "benchmark-token").state;
    (
        state,
        request(project.id, chapter.id, user.id),
        artifact_rows_count,
        old_artifact_payload_bytes,
    )
}

fn artifact_row(
    project_id: i64,
    chapter_id: i64,
    stage: MangaPipelineArtifactStage,
    page_number: i32,
    stale: bool,
    payload: Value,
    now: chrono::NaiveDateTime,
) -> manga_pipeline_artifact::ActiveModel {
    manga_pipeline_artifact::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        stage: Set(stage.key().into()),
        page_number: Set(page_number),
        input_fingerprint: Set(format!("input-{page_number}-{}", stage.key())),
        output_fingerprint: Set(format!("output-{page_number}-{}", stage.key())),
        payload: Set(payload),
        stale: Set(stale),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

async fn legacy_list_page_prompts(
    state: &WorkerState,
    request: WorkerEnvelope<MangaChapterInput>,
) -> Result<Vec<MangaPagePromptSummary>, WorkerFailure> {
    validate_request(&request, MANGA_RECORD_LIST_OPERATION)?;
    let user_id = request.principal.require_user_id()?;
    let input = request.payload;
    WorkerFailure::require_positive_id(input.project_id, "project_id")?;
    WorkerFailure::require_positive_id(input.chapter_id, "chapter_id")?;
    verify_manga_viewer(&state.database, input.project_id, user_id).await?;
    let prompts = a4n_db::manga::manga_page_prompts_by_chapter(
        state.database.connection(),
        input.project_id,
        input.chapter_id,
    )
    .await
    .map_err(manga_error)?;
    let artifacts = a4n_db::manga::manga_pipeline_artifacts_by_chapter(
        state.database.connection(),
        input.project_id,
        input.chapter_id,
    )
    .await
    .map_err(manga_error)?;
    let current_page_numbers = match artifacts
        .iter()
        .find(|artifact| artifact.stage == MangaPipelineArtifactStage::PageSlots && !artifact.stale)
    {
        Some(artifact) => {
            let slots = serde_json::from_value::<MangaPageSlotsV1>(artifact.payload.clone())
                .map_err(|error| {
                    WorkerFailure::invalid_request(format!("当前漫画分页产物无效: {error}"))
                })?;
            slots.validate().map_err(WorkerFailure::invalid_request)?;
            Some(
                slots
                    .pages
                    .into_iter()
                    .map(|page| page.page_number)
                    .collect::<std::collections::HashSet<_>>(),
            )
        }
        None => None,
    };
    prompts
        .iter()
        .map(|prompt| {
            let mut summary = a4n_manga_core::manga_page_prompt_summary(prompt)
                .map_err(WorkerFailure::invalid_request)?;
            summary.stale_stages = a4n_manga_core::manga_page_prompt_stale_stages(
                prompt.id,
                prompt.page_number,
                &artifacts,
            )
            .map_err(WorkerFailure::invalid_request)?;
            summary.orphaned = current_page_numbers
                .as_ref()
                .is_some_and(|pages| !pages.contains(&(prompt.page_number as u32)));
            Ok(summary)
        })
        .collect()
}

async fn time_legacy(
    state: &WorkerState,
    request: WorkerEnvelope<MangaChapterInput>,
) -> (f64, Vec<u8>) {
    let started = Instant::now();
    let summaries = legacy_list_page_prompts(state, request)
        .await
        .expect("legacy list request must succeed");
    let response = serde_json::to_vec(&summaries).expect("legacy response must serialize");
    (started.elapsed().as_secs_f64() * 1000.0, response)
}

async fn time_optimized(
    state: &WorkerState,
    request: WorkerEnvelope<MangaChapterInput>,
) -> (f64, Vec<u8>) {
    let started = Instant::now();
    let summaries = state
        .list_page_prompts(request)
        .await
        .expect("optimized list request must succeed");
    let response = serde_json::to_vec(&summaries).expect("optimized response must serialize");
    (started.elapsed().as_secs_f64() * 1000.0, response)
}

#[tokio::test]
#[ignore = "manual PostgreSQL end-to-end performance benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn performance_manga_prompt_list_artifact_projection() {
    let (state, request, artifacts, old_artifact_payload_bytes) = fixture().await;
    let optimized_artifacts = a4n_db::manga::manga_pipeline_artifact_metadata_by_chapter(
        state.database.connection(),
        request.payload.project_id,
        request.payload.chapter_id,
    )
    .await
    .expect("optimized artifact query must succeed");
    let new_artifact_payload_bytes = optimized_artifacts
        .iter()
        .filter_map(|artifact| artifact.page_slots_payload.as_ref())
        .map(|payload| {
            serde_json::to_vec(payload)
                .expect("payload serialization")
                .len()
        })
        .sum::<usize>();
    let expected_slots = serde_json::from_value::<MangaPageSlotsV1>(
        optimized_artifacts
            .iter()
            .find(|artifact| {
                artifact.stage == MangaPipelineArtifactStage::PageSlots && !artifact.stale
            })
            .and_then(|artifact| artifact.page_slots_payload.clone())
            .expect("current page slots must be present"),
    )
    .expect("page slots must be valid");
    expected_slots.validate().expect("page slots must validate");

    let mut response_bytes = 0;
    for _ in 0..WARMUPS {
        let (legacy, old_body) = time_legacy(&state, request.clone()).await;
        let (optimized, new_body) = time_optimized(&state, request.clone()).await;
        assert_eq!(old_body, new_body);
        response_bytes = old_body.len();
        let _ = (legacy, optimized);
    }

    let mut legacy_samples = Vec::with_capacity(SAMPLES);
    let mut optimized_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let (elapsed, old_body) = time_legacy(&state, request.clone()).await;
            legacy_samples.push(elapsed);
            let (elapsed, new_body) = time_optimized(&state, request.clone()).await;
            optimized_samples.push(elapsed);
            assert_eq!(old_body, new_body);
        } else {
            let (elapsed, new_body) = time_optimized(&state, request.clone()).await;
            optimized_samples.push(elapsed);
            let (elapsed, old_body) = time_legacy(&state, request.clone()).await;
            legacy_samples.push(elapsed);
            assert_eq!(old_body, new_body);
        }
    }

    let legacy_p50 = percentile(&legacy_samples, 0.50);
    let legacy_p95 = percentile(&legacy_samples, 0.95);
    let optimized_p50 = percentile(&optimized_samples, 0.50);
    let optimized_p95 = percentile(&optimized_samples, 0.95);
    println!(
        "PERF-064 manga page prompt list: pages={PAGE_COUNT} artifacts={artifacts} artifact_payload_bytes={old_artifact_payload_bytes}->{new_artifact_payload_bytes} response_bytes={response_bytes}; full worker projection+serialization p50/p95={legacy_p50:.3}/{legacy_p95:.3}->{optimized_p50:.3}/{optimized_p95:.3}ms; reductions p50/p95={:.2}%/{:.2}%",
        (1.0 - optimized_p50 / legacy_p50) * 100.0,
        (1.0 - optimized_p95 / legacy_p95) * 100.0,
    );
    assert!(
        (1.0 - optimized_p50 / legacy_p50) >= 0.30,
        "Manga prompt full path p50 must improve at least 30%"
    );

    let mut page_slots = manga_pipeline_artifact::Entity::find()
        .filter(manga_pipeline_artifact::Column::ProjectId.eq(request.payload.project_id))
        .filter(manga_pipeline_artifact::Column::ChapterId.eq(request.payload.chapter_id))
        .filter(
            manga_pipeline_artifact::Column::Stage.eq(MangaPipelineArtifactStage::PageSlots.key()),
        )
        .one(state.database.connection())
        .await
        .expect("page slots artifact query must succeed")
        .expect("page slots artifact must exist")
        .into_active_model();
    page_slots.payload = Set(json!({
        "schema_version": "manga_page_slots_v1",
        "pages": []
    }));
    page_slots
        .update(state.database.connection())
        .await
        .expect("invalid page slots fixture must be written");
    let legacy_error = legacy_list_page_prompts(&state, request.clone())
        .await
        .expect_err("legacy path must reject invalid page slots");
    let optimized_error = state
        .list_page_prompts(request)
        .await
        .expect_err("optimized path must reject invalid page slots");
    assert_eq!(legacy_error.message(), optimized_error.message());
}
