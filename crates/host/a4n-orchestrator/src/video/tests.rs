use a4n_db_models::models::ai::{generated_image, generated_video};
use a4n_db_models::models::canvas::{canvas_graph, canvas_node};
use a4n_db_models::models::chapter::{chapter, chapter_video_prompt};
use a4n_db_models::models::generation::{generation_run, generation_run_target, workflow_run};
use a4n_db_models::models::project::{
    novel_project, project_source_snapshot, source_snapshot_chapter,
};
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
    IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
};
use a4n_domain::media::GeneratedImageRecord;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use a4n_llm_provider_contract::provider::LlmProvider;
use a4n_prompt::tera_engine::TeraPromptEngine;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;

use super::prompts::context::{video_context_character_names, video_profile_context};
use super::prompts::records::PromptWorkflowRowsContext;
use super::prompts::store::{NewChapterVideoPromptInput, PromptWorkflowMode, new_prompt_draft};
use super::*;
use a4n_db::generation::RecordStartedGenerationRunInput;
use a4n_db::project::{
    CreateDerivedProjectParams, DerivedChapterScope, PROJECT_KIND_VIDEO,
    create_derived_project_record,
};
use a4n_test_support::llm::{SequenceLlmProvider, SwitchableLlmProvider};

include!("tests/shooting_script_fixture.rs");

mod context_contract;
mod generation_flow;
pub(in crate::video) use generation_flow::segment_plan_json_for_shot;
mod prompt_json_contract;
mod prompt_store_flow;
mod prompt_update_flow;
mod provider_generation_application;
mod record_flow;
mod runtime_contract;
mod shooting_script_source;

async fn setup_db() -> DatabaseConnection {
    let db = a4n_migration::test_support::connect_test_database().await;
    crate::test_support::create_test_user(&db).await;
    db
}

pub(in crate::video) fn video_execution_identity_fixture(
    source_hash: &str,
    duration_secs: i32,
    shot_number: i32,
) -> Value {
    json!({
        "contract_epoch": a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH,
        "source_content_sha256": source_hash,
        "style_sha256": "test-style",
        "duration_secs": duration_secs,
        "aspect_ratio": "16:9",
        "segment_plan_prompt_id": 100,
        "source_shot_id": format!("shot-{shot_number}"),
        "segment_index_in_shot": 1,
        "segment_sha256": format!("segment-{shot_number}"),
        "segment_recipe_fingerprint": format!("recipe-{shot_number}"),
    })
}

fn prompts_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../prompts")
}

fn fixed_time(day: u32) -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 5, day)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

pub(in crate::video) async fn insert_video_project(
    db: &DatabaseConnection,
    title: &str,
) -> novel_project::Model {
    let now = fixed_time(3);
    let root = novel_project::ActiveModel {
        user_id: Set(1),
        title: Set(title.to_string()),
        description: Set("视频测试项目".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set("text".to_string()),
        source_sync_status: Set("current".to_string()),
        status: Set("writing".to_string()),
        target_chapters: Set(10),
        current_chapter: Set(0),
        total_chapters: Set(Some(10)),
        chapters_per_part: Set(Some(5)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let scope = json!({"start":1,"end":1});
    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(root.id), chapter_scope_json: Set(scope.clone()), chapter_hashes_json: Set(json!([])), profile_hashes_json: Set(json!([])),
        source_version_hash: Set(a4n_domain::shooting_script::compat::serialized_sha256(&json!({"root_project_id":root.id,"chapter_scope":scope,"chapter_hashes":[],"profile_hashes":[],"blueprint_version_id":null})).unwrap()),
        created_at: Set(now), ..Default::default()
    }.insert(db).await.unwrap();
    let mut video: novel_project::ActiveModel = root.clone().into();
    video.id = sea_orm::NotSet;
    video.project_kind = Set("video".into());
    video.root_project_id = Set(Some(root.id));
    video.source_snapshot_id = Set(Some(snapshot.id));
    video.insert(db).await.unwrap()
}

pub(in crate::video) async fn insert_derived_video_project(
    db: Arc<DatabaseConnection>,
    root_project_id: i64,
    title: &str,
) -> novel_project::Model {
    let source_project = novel_project::Entity::find_by_id(root_project_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let root_project_id = source_project.root_project_id.unwrap_or(source_project.id);
    let video = create_derived_project_record(
        db.as_ref(),
        1,
        CreateDerivedProjectParams {
            root_project_id,
            derived_kind: PROJECT_KIND_VIDEO.to_string(),
            title: title.to_string(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let chapters = a4n_db::project::source_snapshot_chapters(db.as_ref(), video.id)
        .await
        .unwrap();
    for source in chapters {
        let chapter = chapter::Entity::find_by_id(source.source_chapter_id)
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        insert_confirmed_visual_plan_and_shooting_script(db.as_ref(), video.id, &chapter).await;
    }
    video
}

async fn insert_video_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> chapter::Model {
    let chapter =
        insert_video_chapter_without_shooting_script(db, project_id, chapter_number).await;
    insert_confirmed_visual_plan_and_shooting_script(db, project_id, &chapter).await;
    chapter
}

pub(super) async fn insert_confirmed_visual_plan_and_shooting_script(
    db: &DatabaseConnection,
    project_id: i64,
    chapter: &chapter::Model,
) -> (
    a4n_db::chapter_visual_plan::ParsedChapterVisualPlan,
    a4n_db::shooting_script::ParsedChapterShootingScript,
) {
    let source = a4n_db::project::resolve_chapter_source(db, project_id, chapter.id)
        .await
        .expect("video test chapter source should resolve");
    let source_spans = a4n_domain::source_spans::source_spans_for_content(
        chapter.id,
        chapter.chapter_number,
        &chapter.content,
    );
    let mut plan = crate::shooting_script::visual_plan_fixture(&source).plan;
    let scene_id =
        crate::shooting_script::seed_visual_plan_scene(db, source.requested_project_id).await;
    for segment in &mut plan.segments {
        segment.scene_binding = Some(a4n_domain::chapter_visual_plan::SceneBindingV3::Scene {
            scene_profile_id: scene_id,
        });
    }
    let draft = a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
        db,
        a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
            source: source.clone(),
            author_user_id: 1,
            authoring_mode: a4n_domain::chapter_visual_plan::ChapterVisualPlanAuthoringMode::Manual,
            expected: None,
            expected_source_hash: None,
            plan,
        },
    )
    .await
    .expect("video test visual plan should insert");
    let plan = a4n_db::chapter_visual_plan::confirm_chapter_visual_plan(
        db,
        a4n_domain::chapter_visual_plan::ConfirmChapterVisualPlanInput {
            record_id: draft.record.id,
            expected_revision: draft.record.revision,
        },
    )
    .await
    .expect("video test visual plan should confirm")
    .visual_plan;
    let projected_plan = crate::chapter_visual_plan::project_visual_plan_for_shooting_script(
        &plan.plan,
        &source_spans,
    )
    .expect("video test visual plan should project");
    let source_span = source_spans
        .first()
        .expect("video test chapter should have a source span");
    let mut script = test_chapter_shooting_script(&source_span.id);
    script.visual_plan_source = Some(ShootingVisualPlanSourceV1 {
        plan_id: plan.record.id,
        plan_version: plan.record.plan_version,
        plan_sha256: plan.record.plan_sha256.clone(),
    });
    script.user_additions = projected_plan.user_additions.clone();
    for (scene, visual_unit) in script
        .scenes
        .iter_mut()
        .zip(&projected_plan.narrative_units)
    {
        scene.visual_narrative_unit_id = Some(visual_unit.id.clone());
        scene.user_addition_ids = visual_unit
            .shots
            .iter()
            .map(|shot| format!("director:{}", shot.segment_id))
            .collect();
        for (shot, visual_shot) in scene.shots.iter_mut().zip(&visual_unit.shots) {
            shot.shot_id = visual_shot.segment_id.clone();
            shot.visual_shot_id = Some(visual_shot.segment_id.clone());
            shot.planned_start_ms = Some(visual_shot.start_ms);
            shot.planned_end_ms = Some(visual_shot.end_ms);
            shot.visual_constraints = Some(visual_shot.clone());
            shot.user_addition_ids = vec![format!("director:{}", visual_shot.segment_id)];
            for beat in &mut shot.performance_beats {
                beat.user_addition_ids = shot.user_addition_ids.clone();
            }
        }
    }
    let script = a4n_db::shooting_script::insert_chapter_shooting_script(
        db,
        a4n_db::shooting_script::InsertChapterShootingScriptInput {
            source,
            input_fingerprint_sha256: "1".repeat(64),
            visual_plan: a4n_db::shooting_script::ChapterVisualPlanIdentity {
                id: plan.record.id,
                version: plan.record.plan_version,
                sha256: plan.record.plan_sha256.clone(),
            },
            script,
            workflow_run_id: None,
        },
    )
    .await
    .expect("video test shooting script should insert");
    (plan, script)
}

pub(in crate::video) async fn insert_video_chapter_without_shooting_script(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> chapter::Model {
    const TEST_WORKFLOW_RUN_POOL_SIZE: i64 = 64;
    let now = fixed_time(3);
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    let root_project_id = project.root_project_id.unwrap_or(project_id);
    let chapter = chapter::ActiveModel {
        project_id: Set(root_project_id),
        outline_id: Set(None),
        chapter_number: Set(chapter_number),
        title: Set(Some(format!("视频章节 {chapter_number}"))),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set("draft".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    if let Some(snapshot_id) = project.source_snapshot_id {
        // Assemble a complete frozen source fixture before exercising the
        // application. The real source-capture API is covered separately.
        source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(snapshot_id),
            root_project_id: Set(root_project_id),
            source_chapter_id: Set(chapter.id),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title.clone()),
            content: Set(chapter.content.clone()),
            summary: Set(chapter.summary.clone()),
            analysis_data_json: Set(chapter.analysis_data.clone()),
            content_hash: Set(a4n_domain::source_text_hash(&chapter.content)),
            analysis_hash: Set(None),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
        let chapters = source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
            .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
            .all(db)
            .await
            .unwrap();
        let scope = json!({"start": chapters.first().unwrap().chapter_number, "end": chapters.last().unwrap().chapter_number});
        let hashes = json!(chapters.iter().map(|row| json!({"source_chapter_id":row.source_chapter_id,"chapter_number":row.chapter_number,"content_hash":row.content_hash,"analysis_hash":row.analysis_hash})).collect::<Vec<_>>());
        project_source_snapshot::ActiveModel { id: Set(snapshot_id), chapter_scope_json: Set(scope.clone()), chapter_hashes_json: Set(hashes.clone()),
            source_version_hash: Set(a4n_domain::shooting_script::compat::serialized_sha256(&json!({"root_project_id":root_project_id,"chapter_scope":scope,"chapter_hashes":hashes,"profile_hashes":[],"blueprint_version_id":null})).unwrap()),
            ..Default::default() }.update(db).await.unwrap();
    }
    let workflow_runs = (1..=TEST_WORKFLOW_RUN_POOL_SIZE).map(|slot| workflow_run::ActiveModel {
        id: Set(chapter.id * 100 + slot),
        business_key: Set(format!("video-test:{project_id}:{}:{slot}", chapter.id)),
        workflow_kind: Set("chapter_video_prompt_generation".to_string()),
        project_id: Set(Some(project_id)),
        root_project_id: Set(Some(root_project_id)),
        source_snapshot_id: Set(project.source_snapshot_id),
        scope_table: Set(Some("chapter".to_string())),
        scope_id: Set(Some(chapter.id)),
        chapter_id: Set(Some(chapter.id)),
        mode: Set(Some("test".to_string())),
        input_fingerprint_json: Set(None),
        status: Set("running".to_string()),
        current_step_key: Set(None),
        progress_json: Set(None),
        active_task_id: Set(None),
        can_resume: Set(true),
        resume_block_reason: Set(None),
        error_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(None),
    });
    workflow_run::Entity::insert_many(workflow_runs)
        .exec(db)
        .await
        .expect("video test workflow run pool should insert");
    chapter
}

async fn insert_video_prompt(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    generated_video_id: Option<i64>,
) -> ChapterVideoPromptRecord {
    insert_video_prompt_with_metadata(db, project_id, chapter_id, generated_video_id, None).await
}

async fn insert_video_prompt_with_metadata(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    generated_video_id: Option<i64>,
    metadata_json: Option<Value>,
) -> ChapterVideoPromptRecord {
    let now = fixed_time(3);
    let metadata_json = metadata_json.unwrap_or_else(|| json!({}));
    let metadata_json = Some(metadata_json).map(|mut metadata| {
        if let Some(object) = metadata.as_object_mut() {
            object
                .entry("prompt_output_language".to_string())
                .or_insert_with(|| Value::String("zh_cn".to_string()));
            let source_content_sha256 = object
                .get("source")
                .and_then(Value::as_object)
                .and_then(|source| source.get("content_sha256"))
                .cloned()
                .unwrap_or_else(|| Value::String("same".to_string()));
            object
                .entry("video_execution_identity".to_string())
                .or_insert_with(|| {
                    json!({
                        "contract_epoch": 1,
                        "source_content_sha256": source_content_sha256,
                        "style_sha256": "style",
                        "duration_secs": 6,
                        "aspect_ratio": "16:9",
                        "segment_plan_prompt_id": 1,
                        "source_shot_id": "shot-1",
                        "segment_index_in_shot": 1,
                        "segment_sha256": "segment-1",
                        "segment_recipe_fingerprint": "recipe-1"
                    })
                });
        } else {
            metadata = json!({
                "prompt_output_language": "zh_cn",
                "video_execution_identity": {
                    "contract_epoch": 1,
                    "source_content_sha256": "same",
                    "style_sha256": "style",
                    "duration_secs": 6,
                    "aspect_ratio": "16:9",
                    "segment_plan_prompt_id": 1,
                    "source_shot_id": "shot-1",
                    "segment_index_in_shot": 1,
                    "segment_sha256": "segment-1",
                    "segment_recipe_fingerprint": "recipe-1"
                }
            });
        }
        metadata
    });
    let prompt = chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("boundary_frames".to_string()),
        shot_number: Set(1),
        title: Set(Some("镜头一".to_string())),
        scene_summary: Set(Some("雨夜对峙".to_string())),
        visual_prompt: Set("雨夜街巷，人物对峙".to_string()),
        motion_prompt: Set(Some("慢推镜头".to_string())),
        negative_prompt: Set(None),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(generated_video_id),
        metadata_json: Set(metadata_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    a4n_db::video::read_models::chapter_video_prompt_from_model(prompt)
}

async fn insert_generated_image(db: &DatabaseConnection, project_id: i64) -> GeneratedImageRecord {
    insert_generated_image_with_binding(db, project_id, None, None).await
}

async fn insert_generated_image_with_binding(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: Option<i64>,
    metadata_json: Option<Value>,
) -> GeneratedImageRecord {
    let image = generated_image::ActiveModel {
        project_id: Set(project_id),
        image_type: Set("scene".to_string()),
        file_path: Set("images/source.png".to_string()),
        prompt: Set("source frame".to_string()),
        model: Set("image-model".to_string()),
        width: Set(512),
        height: Set(512),
        file_size: Set(None),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(chapter_id),
        panel_id: Set(None),
        metadata_json: Set(metadata_json),
        created_at: Set(fixed_time(3)),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    a4n_db::video::read_models::generated_image_by_id(db, image.id)
        .await
        .unwrap()
        .expect("test image should exist")
}

fn video_record_input(project_id: i64) -> a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
    a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
        project_id,
        chapter_id: None,
        video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
        workflow_mode: Some("boundary_frames".to_string()),
        prompt: "雨夜街巷，人物对峙".to_string(),
        negative_prompt: None,
        model: "video-model".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 6.0,
        fps: 24,
        source_video_id: None,
        source_image_id: None,
        provider_job_id: None,
        metadata_json: None,
    }
}

async fn generated_video_count(db: &DatabaseConnection) -> u64 {
    generated_video::Entity::find().count(db).await.unwrap()
}
