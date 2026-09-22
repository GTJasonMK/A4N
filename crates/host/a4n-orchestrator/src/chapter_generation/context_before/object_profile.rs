use super::shared::truncate_chars;
use crate::OrchestratorContext;
use a4n_db::project::ChapterObjectProfileContext;
use std::cmp::Reverse;

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
    profiles.sort_by_key(|profile| match profile {
        ChapterObjectProfileContext::Authored(profile) => (Reverse(0), profile.id),
        ChapterObjectProfileContext::Observed {
            profile_id,
            evidence,
            ..
        } => (
            Reverse(
                evidence
                    .last()
                    .expect("historical profile has source evidence")
                    .chapter_number,
            ),
            *profile_id,
        ),
    });
    Ok(profiles
        .iter()
        .map(object_profile_context_line)
        .take(OBJECT_PROFILE_CONTEXT_LIMIT)
        .collect())
}

fn object_profile_context_line(context: &ChapterObjectProfileContext) -> String {
    match context {
        ChapterObjectProfileContext::Authored(profile) => format!(
            "- {}（作者预设）：外观: {}；状态: {}；用途: {}",
            profile.name,
            truncate_chars(&profile.visual_description, 100),
            truncate_chars(&profile.material_and_state, 100),
            truncate_chars(&profile.story_usage, 120),
        ),
        ChapterObjectProfileContext::Observed { name, evidence, .. } => {
            let recent = evidence.len().saturating_sub(2);
            let selected = evidence
                .iter()
                .take(usize::from(recent > 0))
                .chain(evidence[recent..].iter());
            let facts = selected
                .map(|item| {
                    format!(
                        "第{}章正文：{}",
                        item.chapter_number,
                        truncate_chars(&item.quote, 240)
                    )
                })
                .collect::<Vec<_>>();
            format!("- {name}：{}", facts.join("；"))
        }
    }
}
