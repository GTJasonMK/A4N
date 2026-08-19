use super::shared::truncate_chars;
use crate::OrchestratorContext;
use a4n_core::models::asset::object_profile;

const OBJECT_PROFILE_CONTEXT_LIMIT: usize = 8;

pub(super) async fn recent_object_profile_context_lines(
    context: &OrchestratorContext,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<String>, String> {
    let mut profiles = context
        .object_profiles_before_chapter(project_id, chapter_number)
        .await
        .map_err(|err| format!("构建物体档案上下文失败：{err}"))?;
    profiles.sort_by_key(|profile| {
        (
            importance_rank(&profile.importance),
            std::cmp::Reverse(profile.last_seen_chapter),
            profile.id,
        )
    });
    Ok(profiles
        .iter()
        .map(object_profile_context_line)
        .take(OBJECT_PROFILE_CONTEXT_LIMIT)
        .collect())
}

fn object_profile_context_line(profile: &object_profile::Model) -> String {
    let mut details = Vec::new();
    details.push(format!(
        "外观: {}",
        truncate_chars(&profile.visual_description, 100)
    ));
    details.push(format!(
        "状态: {}",
        truncate_chars(&profile.material_and_state, 80)
    ));
    details.push(format!(
        "叙事用途: {}",
        truncate_chars(&profile.story_usage, 100)
    ));
    details.push(format!(
        "视觉用途: {}",
        truncate_chars(&profile.visual_usage, 100)
    ));
    details.push(format!(
        "生成用途: {}",
        truncate_chars(&profile.generation_usage, 100)
    ));
    if let Some(location) = profile
        .location_context
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        details.push(format!("语境: {}", truncate_chars(location, 80)));
    }
    format!(
        "- {}（第{}-{}章，{}）：{}",
        profile.name,
        profile.first_seen_chapter,
        profile.last_seen_chapter,
        profile.importance,
        details.join("；")
    )
}

fn importance_rank(value: &str) -> u8 {
    match value {
        "primary" => 0,
        "secondary" => 1,
        _ => 2,
    }
}
