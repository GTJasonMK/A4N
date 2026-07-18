mod cleanup;
mod prompt_artifacts;
mod references;
mod replacement;

pub use cleanup::{delete_generation_spec_records, delete_local_generation_spec_record};
pub use prompt_artifacts::{
    SourceSyncManualPromptArtifactRecordInput, SourceSyncPromptArtifactRecordInput,
    insert_manual_source_sync_prompt_artifact_record, insert_source_sync_prompt_artifact_record,
    render_and_insert_source_sync_prompt_artifact_record,
};
pub(in crate::source_sync) use references::source_snapshot_chapter_for_source;
pub use references::{
    SourceSyncAnchorMappingRecord, SourceSyncEntityRefRecordInput, SourceSyncMediaRefRecordInput,
    SourceSyncTextAnchorRefRecordInput, generation_spec_for_source_sync_run,
    insert_generation_spec_ref_records, rebind_generation_spec_anchor_records,
    replace_generation_spec_ref_records, update_generation_spec_params_record,
    update_generation_spec_to_snapshot_record,
};
pub(in crate::source_sync) use replacement::create_source_sync_replacement_spec_record;
pub use replacement::{
    SourceSyncReplacementSegmentRecordInput, SourceSyncReplacementSpecRecordInput,
    create_replacement_generation_spec_record,
};

use crate::source_sync::{SourceSyncDbError, source_sync_generation_db_error};
use serde_json::Value;

pub(in crate::source_sync) fn source_sync_params_with_prompt_kind(
    mut params_json: Value,
    prompt_kind: &str,
) -> Result<Value, SourceSyncDbError> {
    crate::generation::normalize_video_prompt_kind(prompt_kind)
        .map_err(source_sync_generation_db_error)?;
    let object = params_json.as_object_mut().ok_or_else(|| {
        SourceSyncDbError::InvalidArgument(
            "decision_payload_json.params_json 必须是 JSON 对象".to_string(),
        )
    })?;
    object.insert(
        "prompt_kind".to_string(),
        Value::String(prompt_kind.to_string()),
    );
    Ok(params_json)
}

#[cfg(test)]
mod tests;
