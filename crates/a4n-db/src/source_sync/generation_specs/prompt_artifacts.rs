use super::generation_spec_for_source_sync_run;
use crate::source_sync::{
    SourceSyncDbError, required_text, source_sync_generation_db_error, validate_positive_id,
};
use a4n_core::models::{
    generation::{generation_spec, prompt_artifact},
    source_sync::source_sync_run,
};
use sea_orm::ConnectionTrait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SourceSyncPromptArtifactRecordInput {
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
    pub renderer_version: String,
    pub edited_by_user: bool,
}

#[derive(Debug, Clone)]
pub struct SourceSyncManualPromptArtifactRecordInput {
    pub expected_generation_spec_id: i64,
    pub prompt_artifact_id: i64,
    pub prompt_text: String,
    pub renderer_version: String,
}

pub async fn insert_source_sync_prompt_artifact_record<C>(
    db: &C,
    input: SourceSyncPromptArtifactRecordInput,
) -> Result<prompt_artifact::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    crate::generation::normalize_video_prompt_kind(&input.prompt_kind)
        .map_err(source_sync_generation_db_error)?;
    crate::generation::insert_prompt_artifact(
        db,
        crate::generation::PromptArtifactInsertInput {
            generation_spec_id: input.generation_spec_id,
            prompt_kind: input.prompt_kind,
            prompt_text: input.prompt_text,
            renderer_version: input.renderer_version,
            edited_by_user: input.edited_by_user,
        },
    )
    .await
    .map_err(source_sync_generation_db_error)
}

pub async fn render_and_insert_source_sync_prompt_artifact_record<C>(
    db: &C,
    spec: &generation_spec::Model,
    renderer_version: &str,
) -> Result<prompt_artifact::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let prompt_kind = required_json_str(&spec.params_json, "prompt_kind")?;
    let prompt_text = crate::generation::render_prompt_text(spec, prompt_kind)
        .map_err(source_sync_generation_db_error)?;
    insert_source_sync_prompt_artifact_record(
        db,
        SourceSyncPromptArtifactRecordInput {
            generation_spec_id: spec.id,
            prompt_kind: prompt_kind.to_string(),
            prompt_text,
            renderer_version: renderer_version.to_string(),
            edited_by_user: false,
        },
    )
    .await
}

pub async fn insert_manual_source_sync_prompt_artifact_record<C>(
    db: &C,
    run: &source_sync_run::Model,
    input: SourceSyncManualPromptArtifactRecordInput,
) -> Result<prompt_artifact::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(
        input.expected_generation_spec_id,
        "expected_generation_spec_id",
    )?;
    validate_positive_id(input.prompt_artifact_id, "prompt_artifact_id")?;
    let prompt_artifact = crate::generation::prompt_artifact_by_id(db, input.prompt_artifact_id)
        .await
        .map_err(source_sync_generation_db_error)?;
    if prompt_artifact.generation_spec_id != input.expected_generation_spec_id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "manual 决策目标生成意图不匹配：冲突影响 generation_spec={}，prompt_artifact 指向 generation_spec={}",
            input.expected_generation_spec_id, prompt_artifact.generation_spec_id
        )));
    }
    generation_spec_for_source_sync_run(db, run, prompt_artifact.generation_spec_id).await?;
    insert_source_sync_prompt_artifact_record(
        db,
        SourceSyncPromptArtifactRecordInput {
            generation_spec_id: prompt_artifact.generation_spec_id,
            prompt_kind: prompt_artifact.prompt_kind,
            prompt_text: input.prompt_text,
            renderer_version: input.renderer_version,
            edited_by_user: true,
        },
    )
    .await
}

fn required_json_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, SourceSyncDbError> {
    let text = value.get(field).and_then(Value::as_str).ok_or_else(|| {
        SourceSyncDbError::InvalidArgument(format!("params_json.{field} 必须是非空字符串"))
    })?;
    required_text(text, &format!("params_json.{field}"))
}
