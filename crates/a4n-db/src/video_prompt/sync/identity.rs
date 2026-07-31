use super::*;

pub(super) fn prompt_identity_for_sync_draft(
    draft: &ChapterVideoPromptSyncDraft,
) -> PromptIdentity {
    prompt_sync_identity(
        &draft.fields.workflow_mode,
        &draft.fields.prompt_type,
        draft.fields.shot_number,
        draft.fields.metadata_json.as_ref(),
    )
}

pub(super) fn prompt_identity_for_sync_row(row: &chapter_video_prompt::Model) -> PromptIdentity {
    prompt_sync_identity(
        &row.workflow_mode,
        &row.prompt_type,
        row.shot_number,
        row.metadata_json.as_ref(),
    )
}

fn prompt_sync_identity(
    workflow_mode: &str,
    prompt_type: &str,
    shot_number: i32,
    metadata: Option<&Value>,
) -> PromptIdentity {
    (
        workflow_mode.to_string(),
        prompt_type.to_string(),
        shot_number,
        prompt_sync_output_language_identity(prompt_type, metadata),
        prompt_execution_identity(metadata),
    )
}

pub(super) fn prompt_scope_for_sync_draft(draft: &ChapterVideoPromptSyncDraft) -> PromptScope {
    prompt_sync_scope(
        &draft.fields.workflow_mode,
        &draft.fields.prompt_type,
        draft.fields.metadata_json.as_ref(),
    )
}

fn prompt_scope_for_sync_row(row: &chapter_video_prompt::Model) -> PromptScope {
    prompt_sync_scope(
        &row.workflow_mode,
        &row.prompt_type,
        row.metadata_json.as_ref(),
    )
}

fn prompt_sync_scope(
    workflow_mode: &str,
    prompt_type: &str,
    metadata: Option<&Value>,
) -> PromptScope {
    (
        workflow_mode.to_string(),
        prompt_type.to_string(),
        prompt_sync_output_language_identity(prompt_type, metadata),
        prompt_execution_identity(metadata),
    )
}

fn prompt_sync_output_language_identity(
    prompt_type: &str,
    metadata: Option<&Value>,
) -> Option<String> {
    a4n_core::prompt_language::prompt_type_uses_prompt_output_language(prompt_type).then(|| {
        a4n_core::prompt_language::prompt_output_language_code_from_metadata(metadata).to_string()
    })
}

pub(super) fn should_prefer_existing_prompt(
    candidate: &chapter_video_prompt::Model,
    current: &chapter_video_prompt::Model,
) -> bool {
    let candidate_linked = candidate.generated_video_id.is_some();
    let current_linked = current.generated_video_id.is_some();
    if candidate_linked != current_linked {
        return candidate_linked;
    }
    candidate.updated_at > current.updated_at || candidate.id < current.id
}

pub(super) fn should_keep_unmatched_prompt(
    row: &chapter_video_prompt::Model,
    draft_scopes: &BTreeSet<PromptScope>,
) -> bool {
    if a4n_core::prompt_language::prompt_type_uses_prompt_output_language(&row.prompt_type)
        && !draft_scopes.contains(&prompt_scope_for_sync_row(row))
    {
        return true;
    }
    row.prompt_type == "video_prompt"
        && (row.generated_video_id.is_some()
            || matches!(row.status.as_str(), "running" | "completed"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn prompt_sync_identity_separates_card_execution_chains() {
        let first = json!({
            "prompt_output_language": "zh_cn",
            "video_execution_identity": {
                "segment_plan_prompt_id": 10,
                "segment_number": 1,
                "segment_recipe_fingerprint": "recipe-1"
            }
        });
        let second = json!({
            "prompt_output_language": "zh_cn",
            "video_execution_identity": {
                "segment_plan_prompt_id": 10,
                "segment_number": 2,
                "segment_recipe_fingerprint": "recipe-2"
            }
        });

        assert_ne!(
            prompt_sync_identity("boundary_frames", "frame_prompt", 2, Some(&first)),
            prompt_sync_identity("boundary_frames", "frame_prompt", 2, Some(&second)),
        );
        assert_ne!(
            prompt_sync_scope("boundary_frames", "frame_prompt", Some(&first)),
            prompt_sync_scope("boundary_frames", "frame_prompt", Some(&second)),
        );
    }
}
