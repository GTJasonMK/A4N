use crate::OrchestratorContext;
use std::collections::{HashMap, HashSet};

use super::formatting::{character_profile_context_line, character_state_context_line};

const RECENT_CHARACTER_STATE_ROWS: u64 = 80;
const CHARACTER_STATE_CONTEXT_LIMIT: usize = 8;

pub(in crate::chapter_generation::context_before) async fn recent_character_state_context_lines(
    context: &OrchestratorContext,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<String>, String> {
    let profiles = context
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(|err| format!("构建角色状态上下文失败：{err}"))?;
    if profiles.is_empty() {
        return Ok(Vec::new());
    }

    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let profile_by_id = profiles
        .iter()
        .map(|profile| (profile.id, profile))
        .collect::<HashMap<_, _>>();

    let states = context
        .character_states_for_profiles_before_chapter(
            &profile_ids,
            chapter_number,
            RECENT_CHARACTER_STATE_ROWS,
        )
        .await
        .map_err(|err| format!("构建角色状态上下文失败：{err}"))?;

    let mut seen_profiles = HashSet::new();
    let mut lines = Vec::new();
    for state_index in states {
        if !seen_profiles.insert(state_index.profile_id) {
            continue;
        }
        let Some(profile) = profile_by_id.get(&state_index.profile_id) else {
            continue;
        };
        if let Some(line) = character_state_context_line(&state_index, profile) {
            lines.push(line);
        }
        if lines.len() >= CHARACTER_STATE_CONTEXT_LIMIT {
            break;
        }
    }
    for profile in &profiles {
        if lines.len() >= CHARACTER_STATE_CONTEXT_LIMIT {
            break;
        }
        if seen_profiles.contains(&profile.id) {
            continue;
        }
        if let Some(line) = character_profile_context_line(profile, chapter_number) {
            lines.push(line);
        }
    }
    Ok(lines)
}
