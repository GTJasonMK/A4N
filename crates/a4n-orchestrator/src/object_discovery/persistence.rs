use std::collections::{BTreeSet, HashMap};

use a4n_core::{error::AppError, models::asset::object_profile};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    asset::{ObjectProfileUpsertRecordResult, normalize_object_name},
    generation::RecordCompletedGenerationRunInput,
    text_anchor::{EVIDENCE_ROLE_FIRST_SEEN, TextAnchorEvidenceInput, TextAnchorProfileKind},
};

use super::contracts::{
    ObjectDiscoveryCallResult, ObjectDiscoveryProgress, ObjectDiscoveryRequestContext,
    ParsedObjectProfileDraft,
};

impl OrchestratorContext {
    pub(super) async fn persist_discovered_object_profiles(
        &self,
        project_id: i64,
        chapter_number: Option<i32>,
        request_context: ObjectDiscoveryRequestContext,
        call_result: ObjectDiscoveryCallResult,
        progress: Option<ObjectDiscoveryProgress<'_>>,
    ) -> Result<Value, AppError> {
        let ObjectDiscoveryRequestContext {
            chapters,
            text_anchors,
            request: _,
        } = request_context;
        let ObjectDiscoveryCallResult {
            parsed_drafts,
            response_content,
            llm_request,
            analyzed_chapters,
        } = call_result;
        let discovered_count = parsed_drafts.len();
        super::execution::report_object_discovery_progress(progress, 90, "正在写入物体档案")?;
        let request_json = serde_json::to_value(&llm_request)?;
        let messages_json = serde_json::to_value(&llm_request.messages)?;
        let drafts = parsed_drafts
            .iter()
            .map(|item| item.draft.clone())
            .collect();
        let result = self
            .upsert_object_profiles_from_drafts_validated(
                project_id,
                drafts,
                RecordCompletedGenerationRunInput {
                    project_id,
                    workflow: "object_discovery".to_string(),
                    step_key: "discover_object_profiles_for_chapters".to_string(),
                    title: "物体档案发现".to_string(),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: None,
                    prompt_template_names_json: Some(json!(["object_discovery"])),
                    prompt_template_hashes_json: None,
                    rendered_messages_json: Some(messages_json),
                    rendered_prompts_json: None,
                    input_variables_json: Some(json!({
                        "chapter_number": chapter_number,
                        "analyzed_chapters": analyzed_chapters,
                        "text_anchor_count": text_anchors.len(),
                    })),
                    source_refs_json: Some(json!({
                        "chapters": chapters.iter().map(|chapter| json!({
                            "id": chapter.id,
                            "chapter_number": chapter.chapter_number,
                            "title": chapter.title,
                        })).collect::<Vec<_>>(),
                    })),
                    media_refs_json: None,
                    request_json: Some(request_json),
                    response_summary_json: Some(json!({
                        "response_chars": response_content.chars().count(),
                    })),
                    targets: Vec::new(),
                },
            )
            .await?;
        self.bind_object_profile_evidence(project_id, &parsed_drafts, &result)
            .await?;
        Ok(object_discovery_result(chapters, discovered_count, result))
    }

    async fn bind_object_profile_evidence(
        &self,
        project_id: i64,
        drafts: &[ParsedObjectProfileDraft],
        result: &ObjectProfileUpsertRecordResult,
    ) -> Result<(), AppError> {
        let profiles = result
            .created
            .iter()
            .chain(result.updated.iter())
            .map(|profile| (profile.normalized_name.clone(), profile.id))
            .collect::<HashMap<_, _>>();
        for draft in drafts {
            let normalized = normalize_object_name(&draft.draft.name)?;
            let profile_id =
                profiles
                    .get(&normalized)
                    .copied()
                    .ok_or_else(|| AppError::InvalidState {
                        current: format!("物体档案写入结果缺少对象: {}", draft.draft.name),
                        operation: "绑定物体档案证据".to_string(),
                    })?;
            let mut refs = Vec::new();
            let mut seen = BTreeSet::new();
            for anchor_id in &draft.evidence_anchor_ids {
                if seen.insert((*anchor_id, EVIDENCE_ROLE_FIRST_SEEN)) {
                    refs.push(TextAnchorEvidenceInput {
                        text_anchor_id: *anchor_id,
                        evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
                    });
                }
            }
            if refs.is_empty() {
                return Err(AppError::InvalidArgument(
                    "profile_evidence 不能为空".to_string(),
                ));
            }
            self.merge_profile_evidence_records(
                project_id,
                None,
                TextAnchorProfileKind::Object,
                profile_id,
                refs,
            )
            .await?;
        }
        Ok(())
    }
}

fn object_discovery_result(
    chapters: Vec<a4n_core::models::chapter::chapter::Model>,
    discovered_count: usize,
    result: ObjectProfileUpsertRecordResult,
) -> Value {
    json!({
        "status": "discovered",
        "chapters_analyzed": chapters.len(),
        "chapter_numbers": chapters.iter().map(|chapter| chapter.chapter_number).collect::<Vec<_>>(),
        "discovered_count": discovered_count,
        "created": result.created.iter().map(object_profile_result).collect::<Vec<_>>(),
        "updated": result.updated.iter().map(object_profile_result).collect::<Vec<_>>(),
    })
}

fn object_profile_result(profile: &object_profile::Model) -> Value {
    json!({
        "id": profile.id,
        "project_id": profile.project_id,
        "name": profile.name,
        "normalized_name": profile.normalized_name,
        "aliases_json": profile.aliases_json,
        "object_type": profile.object_type,
        "importance": profile.importance,
        "visual_description": profile.visual_description,
        "material_and_state": profile.material_and_state,
        "story_usage": profile.story_usage,
        "visual_usage": profile.visual_usage,
        "generation_usage": profile.generation_usage,
        "location_context": profile.location_context,
        "first_seen_chapter": profile.first_seen_chapter,
        "last_seen_chapter": profile.last_seen_chapter,
        "evidence_json": profile.evidence_json,
        "image_prompt_seed": profile.image_prompt_seed,
        "created_at": profile.created_at.to_string(),
        "updated_at": profile.updated_at.to_string(),
    })
}
