use crate::generation::{
    GenerationDbError, GenerationRunCheckpointContext, normalize_optional_owned_text,
    validate_positive_id,
};

pub(super) fn normalize_checkpoint_context(
    input: GenerationRunCheckpointContext,
) -> Result<GenerationRunCheckpointContext, GenerationDbError> {
    if let Some(workflow_run_id) = input.workflow_run_id {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
    }
    if let Some(attempt) = input.attempt
        && attempt <= 0
    {
        return Err(GenerationDbError::InvalidArgument(
            "attempt 必须大于 0".to_string(),
        ));
    }
    Ok(GenerationRunCheckpointContext {
        workflow_run_id: input.workflow_run_id,
        call_key: normalize_optional_owned_text(input.call_key),
        input_fingerprint_sha256: normalize_optional_owned_text(input.input_fingerprint_sha256),
        input_fingerprint_json: input.input_fingerprint_json,
        raw_response_json: input.raw_response_json,
        parsed_output_json: input.parsed_output_json,
        validation_json: input.validation_json,
        artifact_refs_json: input.artifact_refs_json,
        attempt: input.attempt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_run_checkpoint_rejects_invalid_workflow_or_attempt() {
        let error = normalize_checkpoint_context(GenerationRunCheckpointContext {
            workflow_run_id: Some(0),
            ..Default::default()
        })
        .unwrap_err();
        assert!(matches!(
            error,
            GenerationDbError::InvalidArgument(message)
                if message == "workflow_run_id 必须为正数"
        ));

        let error = normalize_checkpoint_context(GenerationRunCheckpointContext {
            attempt: Some(0),
            ..Default::default()
        })
        .unwrap_err();
        assert!(matches!(
            error,
            GenerationDbError::InvalidArgument(message) if message == "attempt 必须大于 0"
        ));
    }

    #[test]
    fn generation_run_checkpoint_normalizes_optional_identity_fields() {
        let checkpoint = normalize_checkpoint_context(GenerationRunCheckpointContext {
            workflow_run_id: Some(7),
            call_key: Some("  provider_call  ".to_string()),
            input_fingerprint_sha256: Some("  abc123  ".to_string()),
            attempt: Some(2),
            ..Default::default()
        })
        .expect("valid checkpoint");

        assert_eq!(checkpoint.workflow_run_id, Some(7));
        assert_eq!(checkpoint.call_key.as_deref(), Some("provider_call"));
        assert_eq!(
            checkpoint.input_fingerprint_sha256.as_deref(),
            Some("abc123")
        );
        assert_eq!(checkpoint.attempt, Some(2));
    }
}
