use a4n_core::error::AppError;
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    character::{
        RecordCompletedGenerationRunInput,
        json_field_rules::{required_string, required_value},
    },
    text_anchor::{
        EVIDENCE_ROLE_FIRST_SEEN, TextAnchorEvidenceInput, TextAnchorProfileKind,
        text_anchor_db_error,
    },
};

use super::{
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
    ) -> Result<Value, AppError> {
        let CharacterDiscoveryRequestContext {
            chapters,
            text_anchors,
            mut existing_keys,
            mut existing_key_profiles,
            request: _,
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
        let mut created = Vec::new();
        let mut existing = Vec::new();
        let discovered_total = discovered.len().max(1);
        let source_chapters = chapters
            .iter()
            .map(a4n_db::project::ChapterSourceSnapshot::from)
            .collect::<Vec<_>>();
        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        if !a4n_db::chapter_content::chapter_source_snapshots_match_for_update(
            &tx,
            &source_chapters,
        )
        .await
        .map_err(AppError::Database)?
        {
            return Err(AppError::InvalidState {
                current: "chapter_source_changed".to_string(),
                operation: "保存角色发现结果；章节正文已变化，请重新分析".to_string(),
            });
        }

        for (index, character) in discovered.into_iter().enumerate() {
            super::execution::report_character_discovery_progress(
                progress,
                90,
                &format!("正在写入发现角色 {}/{}", index + 1, discovered_total),
            )?;
            let keys = discovered_character_keys(&character).map_err(AppError::InvalidArgument)?;
            let name = required_string(&character, "name").map_err(AppError::InvalidArgument)?;
            if keys.iter().any(|key| existing_keys.contains(key)) {
                if let Some(profile_id) = keys
                    .iter()
                    .find_map(|key| existing_key_profiles.get(key).copied())
                {
                    bind_character_profile_evidence_in(&tx, project_id, profile_id, &character)
                        .await?;
                }
                existing.push(json!({
                    "name": name,
                    "aliases": required_value(&character, "aliases")
                        .map_err(AppError::InvalidArgument)?
                        .clone(),
                    "reason": "already_exists",
                }));
                for key in keys {
                    existing_keys.insert(key);
                }
                continue;
            }

            let base_attributes = discovered_character_base_attributes(&character)
                .map_err(AppError::InvalidArgument)?;
            match a4n_db::character::create_profile_with_generation_run_in(
                &tx,
                project_id,
                &name,
                base_attributes,
                RecordCompletedGenerationRunInput {
                    project_id,
                    workflow: "character_discovery".to_string(),
                    step_key: "discover_character_profiles_for_chapters".to_string(),
                    title: format!("角色发现: {name}"),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: None,
                    prompt_template_names_json: Some(json!(["character_discovery"])),
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
                },
            )
            .await
            {
                Ok(profile) => {
                    for key in keys {
                        existing_keys.insert(key.clone());
                        existing_key_profiles.insert(key, profile.id);
                    }
                    bind_character_profile_evidence_in(&tx, project_id, profile.id, &character)
                        .await?;
                    created.push(profile_for_result(&profile).map_err(AppError::InvalidArgument)?);
                }
                Err(a4n_db::character::CharacterProfileCreateError::AlreadyExists { .. }) => {
                    let profile = a4n_db::character::profile_by_project_name(
                        &tx, project_id, &name,
                    )
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| {
                        AppError::NotFound(format!(
                            "角色档案已存在但无法重新读取: project_id={project_id}, name={name}"
                        ))
                    })?;
                    bind_character_profile_evidence_in(&tx, project_id, profile.id, &character)
                        .await?;
                    for key in keys {
                        existing_keys.insert(key.clone());
                        existing_key_profiles.insert(key, profile.id);
                    }
                    existing.push(json!({
                        "name": name,
                        "aliases": required_value(&character, "aliases")
                            .map_err(AppError::InvalidArgument)?
                            .clone(),
                        "reason": "already_exists",
                    }));
                }
                Err(error) => {
                    return Err(
                        crate::character::profiles::errors::character_profile_create_error(error),
                    );
                }
            }
        }
        tx.commit().await.map_err(AppError::Database)?;

        Ok(json!({
            "status": "discovered",
            "chapters_analyzed": chapters.len(),
            "chapter_numbers": chapters.iter().map(|chapter| chapter.chapter_number).collect::<Vec<_>>(),
            "discovered_count": discovered_count,
            "created": created,
            "existing": existing,
        }))
    }
}

async fn bind_character_profile_evidence_in<C>(
    db: &C,
    project_id: i64,
    profile_id: i64,
    character: &Value,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let text_anchor_id = required_value(character, "evidence_anchor_id")
        .map_err(AppError::InvalidArgument)?
        .as_i64()
        .ok_or_else(|| {
            AppError::InvalidArgument(
                "角色发现字段 evidence_anchor_id 必须是 JSON 整数数字".to_string(),
            )
        })?;
    a4n_db::text_anchor::merge_profile_evidence_records(
        db,
        project_id,
        None,
        TextAnchorProfileKind::Character,
        profile_id,
        vec![TextAnchorEvidenceInput {
            text_anchor_id,
            evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
        }],
    )
    .await
    .map_err(text_anchor_db_error)
    .map(|_| ())
}
