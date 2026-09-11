use a4n_db::{
    asset::ObjectProfileUpsertRecordResult, generation::RecordCompletedGenerationRunInput,
};
use a4n_db_models::models::asset::object_profile;
use a4n_domain::object_profile::ObjectProfileEvidenceRecordInput;
use a4n_error::AppError;
use serde_json::{Value, json};

use crate::{OrchestratorContext, asset::object_profiles::normalize_object_name};

use super::contracts::{
    ObjectDiscoveryCallResult, ObjectDiscoveryProgress, ObjectDiscoveryRequestContext,
};
use super::{OBJECT_DISCOVERY_CALL_KEY, OBJECT_DISCOVERY_KIND};

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
        let profile_evidence = parsed_drafts
            .iter()
            .map(|item| {
                Ok(ObjectProfileEvidenceRecordInput {
                    normalized_name: normalize_object_name(&item.draft.name)?,
                    text_anchor_ids: item.evidence_anchor_ids.clone(),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let source_chapters = chapters
            .iter()
            .map(a4n_db::project::ChapterSourceSnapshot::from)
            .collect();
        let result = self
            .upsert_object_profiles_from_drafts_validated(
                project_id,
                drafts,
                source_chapters,
                profile_evidence,
                RecordCompletedGenerationRunInput {
                    project_id,
                    workflow: OBJECT_DISCOVERY_KIND.to_string(),
                    step_key: OBJECT_DISCOVERY_CALL_KEY.to_string(),
                    title: "物体档案发现".to_string(),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: None,
                    prompt_template_names_json: Some(json!([OBJECT_DISCOVERY_KIND])),
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
        Ok(object_discovery_result(chapters, discovered_count, result))
    }
}

fn object_discovery_result(
    chapters: Vec<a4n_db_models::models::chapter::chapter::Model>,
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
        "ownership_kind": profile.ownership_kind,
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
