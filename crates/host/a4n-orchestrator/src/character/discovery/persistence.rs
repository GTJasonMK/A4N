use a4n_api_contract::character::{
    DiscoverCharacterProfilesResult, ExistingCharacterProfileDiscovery,
};
use a4n_domain::{
    character::CharacterDiscoveryCandidateRecordInput,
    generation::RecordCompletedGenerationRunInput,
};
use a4n_error::AppError;
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::{
    OrchestratorContext,
    character::json_field_rules::{required_string, required_value},
};

use super::{
    CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_KIND,
    contracts::{CharacterDiscoveryCallResult, CharacterDiscoveryRequestContext},
    identity::{discovered_character_keys, profile_for_result},
    output::discovered_character_base_attributes,
};

impl OrchestratorContext {
    pub(super) async fn persist_discovered_character_profiles(
        &self,
        project_id: i64,
        chapter_number: Option<i32>,
        request_context: CharacterDiscoveryRequestContext,
        call_result: CharacterDiscoveryCallResult,
        progress: Option<super::contracts::CharacterDiscoveryProgress<'_>>,
    ) -> Result<DiscoverCharacterProfilesResult, AppError> {
        let CharacterDiscoveryRequestContext {
            chapters,
            text_anchors,
            existing_keys,
            existing_key_profiles,
        } = request_context;
        let CharacterDiscoveryCallResult {
            discovered,
            response_content,
            llm_request,
            analyzed_chapters,
        } = call_result;
        let discovered_count = discovered.len();
        let request_json = serde_json::to_value(&llm_request)?;
        let messages_json = serde_json::to_value(&llm_request.messages)?;
        let discovered_total = discovered.len();
        let source_chapters = chapters
            .iter()
            .map(|chapter| chapter.source_snapshot())
            .collect::<Vec<_>>();
        let mut existing_keys = existing_keys;
        let mut new_candidate_index_by_key = HashMap::new();
        let mut candidates = Vec::with_capacity(discovered_total);
        let mut candidate_summaries = Vec::with_capacity(discovered_total);

        for (index, character) in discovered.into_iter().enumerate() {
            super::execution::report_character_discovery_progress(
                progress,
                90,
                &format!("正在准备写入发现角色 {}/{}", index + 1, discovered_total),
            )?;
            let keys = discovered_character_keys(&character).map_err(AppError::InvalidArgument)?;
            let name = required_string(&character, "name").map_err(AppError::InvalidArgument)?;
            let aliases = required_value(&character, "aliases")
                .map_err(AppError::InvalidArgument)?
                .clone();
            let evidence_anchor_id = character_evidence_anchor_id(&character)?;
            let matching_profile_id = keys
                .iter()
                .find_map(|key| existing_key_profiles.get(key).copied());
            let previous_candidate_index = keys
                .iter()
                .find_map(|key| new_candidate_index_by_key.get(key).copied());
            let matched = keys.iter().any(|key| existing_keys.contains(key));

            if matched {
                let candidate = if let Some(profile_id) = matching_profile_id {
                    CharacterDiscoveryCandidateRecordInput::ExistingProfile {
                        profile_id,
                        evidence_anchor_id,
                    }
                } else if let Some(candidate_index) = previous_candidate_index {
                    CharacterDiscoveryCandidateRecordInput::PreviouslyResolvedCandidate {
                        candidate_index,
                        evidence_anchor_id,
                    }
                } else {
                    CharacterDiscoveryCandidateRecordInput::ExistingWithoutEvidence
                };
                candidates.push(candidate);
                candidate_summaries.push((name, aliases));
                for key in keys {
                    existing_keys.insert(key);
                }
                continue;
            }

            let base_attributes = discovered_character_base_attributes(&character)
                .map_err(AppError::InvalidArgument)?;
            let generation_run = RecordCompletedGenerationRunInput {
                project_id,
                workflow: CHARACTER_DISCOVERY_KIND.to_string(),
                step_key: CHARACTER_DISCOVERY_CALL_KEY.to_string(),
                title: format!("角色发现: {name}"),
                model_kind: "llm".to_string(),
                provider: None,
                model: None,
                prompt_template_names_json: Some(json!([CHARACTER_DISCOVERY_KIND])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(messages_json.clone()),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "chapter_number": chapter_number,
                    "analyzed_chapters": &analyzed_chapters,
                    "character_name": &name,
                    "text_anchor_count": text_anchors.len(),
                })),
                source_refs_json: Some(json!({
                    "chapters": chapters.iter().map(|chapter| json!({
                        "id": chapter.id,
                        "chapter_number": chapter.chapter_number,
                        "title": &chapter.title,
                    })).collect::<Vec<_>>(),
                })),
                media_refs_json: None,
                request_json: Some(request_json.clone()),
                response_summary_json: Some(json!({
                    "character_name": &name,
                    "discovered_count": discovered_count,
                    "response_chars": response_content.chars().count(),
                })),
                targets: Vec::new(),
            };
            let candidate_index = candidates.len();
            candidate_summaries.push((name.clone(), aliases));
            candidates.push(CharacterDiscoveryCandidateRecordInput::Create {
                name,
                base_attributes,
                evidence_anchor_id,
                generation_run,
            });
            for key in keys {
                existing_keys.insert(key.clone());
                new_candidate_index_by_key.insert(key, candidate_index);
            }
        }

        let outcomes = a4n_db::character::persist_character_discovery_candidates(
            self.database.executor(),
            project_id,
            &source_chapters,
            candidates,
        )
        .await
        .map_err(crate::character::character_db_error)?;
        if outcomes.len() != candidate_summaries.len() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "角色发现持久化结果数量与候选数量不一致"
            )));
        }
        let mut created = Vec::new();
        let mut existing = Vec::new();
        for ((name, aliases), outcome) in candidate_summaries.into_iter().zip(outcomes) {
            if outcome.created {
                let profile = outcome.profile.ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!("角色发现新建结果缺少档案记录"))
                })?;
                created.push(profile_for_result(&profile).map_err(AppError::InvalidArgument)?);
            } else {
                existing.push(ExistingCharacterProfileDiscovery {
                    name,
                    aliases,
                    reason: "already_exists".into(),
                });
            }
        }

        Ok(DiscoverCharacterProfilesResult {
            status: "discovered".into(),
            chapters_analyzed: chapters.len(),
            chapter_numbers: chapters
                .iter()
                .map(|chapter| chapter.chapter_number)
                .collect(),
            discovered_count,
            created,
            existing,
        })
    }
}

fn character_evidence_anchor_id(character: &Value) -> Result<i64, AppError> {
    let id = required_value(character, "evidence_anchor_id")
        .map_err(AppError::InvalidArgument)?
        .as_i64()
        .ok_or_else(|| {
            AppError::InvalidArgument(
                "角色发现字段 evidence_anchor_id 必须是 JSON 整数数字".to_string(),
            )
        })?;
    Ok(id)
}
