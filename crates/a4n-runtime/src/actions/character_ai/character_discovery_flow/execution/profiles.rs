use super::super::{
    contracts::{CHARACTER_DISCOVERY_PROGRESS_TOTAL, CharacterDiscoveryRequestContext},
    persistence::{bind_character_profile_evidence, existing_character_profile_by_name},
    rules::report_character_discovery_progress,
};
use super::call::CharacterDiscoveryCallResult;
use crate::{
    AppState,
    actions::character_ai::{
        CharacterAiProgress,
        character_discovery::{
            discovered_character_base_attributes, discovered_character_keys, profile_for_result,
        },
        json_utils::{required_string, required_value},
    },
};
use a4n_orchestrator::character::RecordCompletedGenerationRunInput;
use serde_json::{Value, json};

pub(super) async fn persist_discovered_character_profiles(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    request_context: CharacterDiscoveryRequestContext,
    call_result: CharacterDiscoveryCallResult,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<Value, String> {
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
    let request_json =
        serde_json::to_value(&llm_request).map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let messages_json = serde_json::to_value(&llm_request.messages)
        .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
    let mut created = Vec::new();
    let mut existing = Vec::new();

    let discovered_total = discovered.len().max(1);
    for (index, character) in discovered.into_iter().enumerate() {
        let message = format!("正在写入发现角色 {}/{}", index + 1, discovered_total);
        report_character_discovery_progress(
            progress,
            90,
            CHARACTER_DISCOVERY_PROGRESS_TOTAL,
            &message,
            "角色发现已停止",
        )?;
        let keys = discovered_character_keys(&character)?;
        let name = required_string(&character, "name")?;
        if keys.iter().any(|key| existing_keys.contains(key)) {
            if let Some(profile_id) = keys
                .iter()
                .find_map(|key| existing_key_profiles.get(key).copied())
            {
                bind_character_profile_evidence(state, project_id, profile_id, &character).await?;
            }
            existing.push(json!({
                "name": name,
                "aliases": required_value(&character, "aliases")?.clone(),
                "reason": "already_exists"}));
            for key in keys {
                existing_keys.insert(key);
            }
            continue;
        }

        let base_attributes = discovered_character_base_attributes(&character)?;
        let create_result = state
            .orchestrator
            .create_profile_with_generation_run(
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
                        "text_anchor_count": text_anchors.len()})),
                    source_refs_json: Some(json!({
                        "chapters": chapters.iter().map(|chapter| json!({
                            "id": chapter.id,
                            "chapter_number": chapter.chapter_number,
                            "title": &chapter.title})).collect::<Vec<_>>()})),
                    media_refs_json: None,
                    request_json: Some(request_json.clone()),
                    response_summary_json: Some(json!({
                        "character_name": &name,
                        "discovered_count": discovered_count,
                        "response_chars": response_content.chars().count()})),
                    targets: Vec::new(),
                },
            )
            .await;
        match create_result {
            Ok(profile) => {
                for key in keys {
                    existing_keys.insert(key.clone());
                    existing_key_profiles.insert(key, profile.id);
                }
                bind_character_profile_evidence(state, project_id, profile.id, &character).await?;
                created.push(profile_for_result(&profile)?);
            }
            Err(a4n_core::error::AppError::InvalidState { current, .. })
                if current == "character_profile_already_exists" =>
            {
                let profile = existing_character_profile_by_name(state, project_id, &name).await?;
                bind_character_profile_evidence(state, project_id, profile.id, &character).await?;
                for key in keys {
                    existing_keys.insert(key.clone());
                    existing_key_profiles.insert(key, profile.id);
                }
                existing.push(json!({
                    "name": name,
                    "aliases": required_value(&character, "aliases")?.clone(),
                    "reason": "already_exists"}));
            }
            Err(err) => return Err(format!("写入发现角色失败：{name}：{err}")),
        }
    }

    Ok(json!({
        "status": "discovered",
        "chapters_analyzed": chapters.len(),
        "chapter_numbers": chapters.iter().map(|chapter| chapter.chapter_number).collect::<Vec<_>>(),
        "discovered_count": discovered_count,
        "created": created,
        "existing": existing}))
}
