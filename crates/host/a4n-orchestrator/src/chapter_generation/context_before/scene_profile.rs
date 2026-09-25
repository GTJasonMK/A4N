use super::shared::truncate_chars;
use crate::OrchestratorContext;
use crate::chapter_generation::non_empty_trimmed;
use a4n_db::project::ChapterSceneProfileContext;
use serde_json::Value;

const SCENE_PROFILE_CONTEXT_LIMIT: usize = 8;

pub(super) async fn recent_scene_profile_context_lines(
    context: &OrchestratorContext,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<String>, String> {
    let mut profiles = context
        .scene_profiles_before_chapter(project_id, chapter_number)
        .await
        .map_err(|err| format!("构建场景档案上下文失败：{err}"))?;
    profiles.sort_by_key(|profile| match profile {
        ChapterSceneProfileContext::Authored(profile) => (std::cmp::Reverse(0), profile.id),
        ChapterSceneProfileContext::Observed {
            profile_id,
            occurrence,
            ..
        } => (std::cmp::Reverse(occurrence.chapter_number), *profile_id),
    });
    Ok(profiles
        .iter()
        .filter_map(scene_profile_context_line)
        .take(SCENE_PROFILE_CONTEXT_LIMIT)
        .collect())
}

fn scene_profile_context_line(context: &ChapterSceneProfileContext) -> Option<String> {
    let mut details = Vec::new();
    let label = match context {
        ChapterSceneProfileContext::Authored(profile) => {
            push_text_detail(&mut details, "视觉", &profile.visual_description, 100);
            push_text_detail(&mut details, "布局", &profile.spatial_layout, 100);
            push_text_detail(&mut details, "氛围", &profile.atmosphere, 80);
            format!("{}（作者预设）", profile.name)
        }
        ChapterSceneProfileContext::Observed {
            name,
            occurrence,
            fact_state,
            ..
        } => {
            if let Some(state) = fact_state {
                for (key, label) in [("spatial_layout", "布局"), ("visual_description", "视觉")]
                {
                    if let Some(value) = state.get(key).and_then(Value::as_str) {
                        push_text_detail(&mut details, label, value, 180);
                    }
                }
            }
            push_text_detail(&mut details, "本次出场", &occurrence.summary, 160);
            if let Some(mood) = occurrence.mood.as_deref() {
                push_text_detail(&mut details, "氛围", mood, 80);
            }
            if let Some(weather) = occurrence.weather_or_lighting.as_deref() {
                push_text_detail(&mut details, "环境", weather, 80);
            }
            if let Some(state) = compact_json_detail(&occurrence.exit_state_json, 180) {
                details.push(format!("退出状态: {state}"));
            }
            format!("{name}（第{}章后）", occurrence.chapter_number)
        }
    };
    if details.is_empty() {
        None
    } else {
        Some(format!("- {label}：{}", details.join("；")))
    }
}

fn push_text_detail(details: &mut Vec<String>, label: &str, value: &str, limit: usize) {
    if let Some(text) = non_empty_trimmed(value) {
        details.push(format!("{label}: {}", truncate_chars(text, limit)));
    }
}

fn compact_json_detail(value: &Value, limit: usize) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Array(items) if items.is_empty() => None,
        Value::Object(map) if map.is_empty() => None,
        _ => {
            let text = value.to_string();
            non_empty_trimmed(&text).map(|text| truncate_chars(text, limit))
        }
    }
}
