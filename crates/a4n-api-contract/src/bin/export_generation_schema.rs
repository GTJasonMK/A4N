use a4n_api_contract::blueprint::{StartBlueprintGenerationInput, StartBlueprintRefinementInput};
use a4n_api_contract::chapter::StartChapterGenerationInput;
use a4n_api_contract::generation_tasks::{
    CancelGenerationTaskInput, GenerationSnapshotState, GenerationStartRequestMeta,
    GenerationSubscriptionHandle, GenerationTaskHandle, GenerationTaskHistoryPage,
    GenerationTaskHistoryQuery, GenerationTaskSummary, HostErrorResponse,
};
use a4n_api_contract::image_generation::StartImageGenerationInput;
use a4n_api_contract::inspiration::StartInspirationGenerationInput;
use a4n_api_contract::optimize::{
    StartOptimizeAnalysisGenerationInput, StartOptimizeChatGenerationInput,
    StartOptimizePatchGenerationInput,
};
use a4n_api_contract::outline::StartOutlineGenerationInput;
use a4n_api_contract::portrait::StartPortraitGenerationInput;
use a4n_api_contract::video::StartVideoGenerationInput;
use a4n_service_protocol::{GenerationTaskEvent, GenerationTaskSnapshot};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde_json::json;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct GenerationContractBundle {
    event: GenerationTaskEvent,
    snapshot: GenerationTaskSnapshot,
    start_request_meta: GenerationStartRequestMeta,
    start_blueprint_generation: StartBlueprintGenerationInput,
    start_blueprint_refinement: StartBlueprintRefinementInput,
    task_handle: GenerationTaskHandle,
    subscription_handle: GenerationSubscriptionHandle,
    snapshot_state: GenerationSnapshotState,
    task_summary: GenerationTaskSummary,
    history_query: GenerationTaskHistoryQuery,
    history_page: GenerationTaskHistoryPage,
    cancel_input: CancelGenerationTaskInput,
    host_error: HostErrorResponse,
    start_chapter_generation: StartChapterGenerationInput,
    start_inspiration_generation: StartInspirationGenerationInput,
    start_optimize_analysis_generation: StartOptimizeAnalysisGenerationInput,
    start_optimize_chat_generation: StartOptimizeChatGenerationInput,
    start_optimize_patch_generation: StartOptimizePatchGenerationInput,
    start_outline_generation: StartOutlineGenerationInput,
    start_portrait_generation: StartPortraitGenerationInput,
    start_image_generation: StartImageGenerationInput,
    start_video_generation: StartVideoGenerationInput,
}

fn main() {
    let schema = SchemaGenerator::new(SchemaSettings::draft2020_12())
        .into_root_schema_for::<GenerationContractBundle>();
    let mut schema = serde_json::to_value(schema).expect("serialize generation schema");
    schema
        .as_object_mut()
        .expect("generation schema is an object")
        .insert(
            "$comment".to_string(),
            json!("Generated from Rust contracts; do not edit."),
        );
    let output = format!(
        "{}\n",
        serde_json::to_string_pretty(&schema).expect("serialize generation schema")
    );
    let path = generated_path();

    match env::args().nth(1).as_deref() {
        Some("--write") => write_generated(&path, &output),
        Some("--check") => check_generated(&path, &output),
        _ => print!("{output}"),
    }
}

fn generated_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../frontend/src/api/generated/generation-contract.schema.json")
}

fn write_generated(path: &Path, output: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!("create {}: {error}", parent.display());
        });
    }
    fs::write(path, output).unwrap_or_else(|error| {
        panic!("write {}: {error}", path.display());
    });
}

fn check_generated(path: &Path, output: &str) {
    let existing = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("read {}: {error}", path.display());
    });
    assert_eq!(existing, output, "generation schema is stale; run --write");
}
