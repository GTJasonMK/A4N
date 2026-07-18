use a4n_core::{error::AppError, source_spans::source_span_prompt_values};
use a4n_llm::context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
    LlmContextManifest, context_aware_system_user_request,
};
use a4n_llm::provider::{ChatRequest, LlmTraceContext};
use serde::Serialize;
use serde_json::json;

use super::contracts::PreparedChapterShootingScriptRequest;

pub(super) fn shooting_context_request<T: Serialize>(
    prepared: &PreparedChapterShootingScriptRequest,
    workflow_run_id: i64,
    prompt_name: &str,
    call_key: String,
    system_prompt: String,
    shared_plan: Option<(&str, &T)>,
    dynamic_task: String,
    max_tokens_profile: String,
) -> Result<ChatRequest, AppError> {
    let manifest = shooting_context_manifest(prepared, shared_plan, dynamic_task)?;
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.2),
        None,
        Some(max_tokens_profile),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(prepared.source.requested_project_id),
        root_project_id: Some(prepared.source.root_project_id),
        workflow: super::contracts::CHAPTER_SHOOTING_SCRIPT_WORKFLOW.to_string(),
        step_key: prompt_name.to_string(),
        generation_run_id: None,
        workflow_run_id: Some(workflow_run_id),
        call_key: Some(call_key),
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

pub(super) fn shooting_context_manifest<T: Serialize>(
    prepared: &PreparedChapterShootingScriptRequest,
    shared_plan: Option<(&str, &T)>,
    dynamic_task: String,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: "chapter".into(),
        pack_kind: "shooting_chapter_source".into(),
        source_refs_json: Some(json!({
            "source_revision_key": prepared.source.source_revision_key,
            "content_sha256": prepared.source.content_sha256,
        })),
        config_identity_json: json!({"contract":"shooting_context_v1","kind":"chapter_source"}),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({
                "root_project_id": prepared.source.root_project_id,
                "source_chapter_id": prepared.source.source_chapter_id,
                "chapter_number": prepared.source.chapter_number,
                "title": prepared.source.title,
                "summary": prepared.source.summary,
                "analysis": prepared.source.analysis_data_json,
                "content_sha256": prepared.source.content_sha256,
                "source_revision_key": prepared.source.source_revision_key,
            }),
        )],
    })?;
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: "chapter".into(),
        pack_kind: "shooting_source_spans".into(),
        source_refs_json: Some(json!({
            "source_revision_key": prepared.source.source_revision_key,
            "span_count": prepared.source_spans.len(),
        })),
        config_identity_json: json!({"contract":"shooting_context_v1","kind":"source_spans"}),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({"source_spans": source_span_prompt_values(&prepared.source_spans)}),
        )],
    })?;
    if let Some((pack_kind, plan)) = shared_plan {
        ledger.add_pack(LlmContextLedgerPackInput {
            scope_kind: "chapter".into(),
            pack_kind: pack_kind.into(),
            source_refs_json: Some(
                json!({"source_revision_key": prepared.source.source_revision_key}),
            ),
            config_identity_json: json!({"contract":"shooting_context_v1","kind":pack_kind}),
            required: true,
            stable_prefix: true,
            variants: vec![LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                serde_json::to_value(plan)?,
            )],
        })?;
    }
    Ok(ledger.into_manifest(dynamic_task, None))
}

#[cfg(test)]
mod tests {
    use a4n_core::source_spans::source_spans_for_content;
    use a4n_db::project::{ResolvedChapterSource, ResolvedChapterSourceKind};

    use super::*;

    fn prepared() -> PreparedChapterShootingScriptRequest {
        let content = "甲进入教室。乙抬头看他。".to_string();
        PreparedChapterShootingScriptRequest {
            source: ResolvedChapterSource {
                source_kind: ResolvedChapterSourceKind::Live,
                requested_project_id: 1,
                root_project_id: 1,
                source_chapter_id: 3,
                source_snapshot_id: None,
                source_snapshot_chapter_id: None,
                chapter_number: 1,
                title: Some("测试".into()),
                summary: None,
                analysis_data_json: None,
                content: content.clone(),
                content_sha256: "content-hash".into(),
                source_revision_key: "revision-1".into(),
            },
            source_spans: source_spans_for_content(3, 1, &content),
            input_fingerprint_sha256: "fingerprint".into(),
            existing: None,
            skeleton_system_prompt: "skeleton".into(),
            layout_unit_system_prompt: "layout".into(),
            unit_adaptation_system_prompt: "adaptation".into(),
            design_unit_system_prompt: "design".into(),
        }
    }

    #[test]
    fn unit_requests_change_only_dynamic_task_after_stable_packs() {
        let prepared = prepared();
        let skeleton = serde_json::json!({"units":[1,2]});
        let first = shooting_context_manifest(
            &prepared,
            Some(("shooting_layout_skeleton", &skeleton)),
            "unit=1".into(),
        )
        .expect("first manifest");
        let second = shooting_context_manifest(
            &prepared,
            Some(("shooting_layout_skeleton", &skeleton)),
            "unit=2".into(),
        )
        .expect("second manifest");

        assert_eq!(first.packs, second.packs);
        assert!(first.packs.iter().all(|pack| pack.stable_prefix));
        assert_eq!(first.dynamic_user_content, "unit=1");
        assert_eq!(second.dynamic_user_content, "unit=2");
    }
}
