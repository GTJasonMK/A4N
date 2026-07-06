mod analysis;
mod character_state;
mod object_profile;
mod scene_profile;
mod shared;
mod summaries;

#[cfg(test)]
use self::character_state::{character_profile_context_line, character_state_context_line};
use self::{
    analysis::chapter_analysis_context_line, character_state::recent_character_state_context_lines,
    object_profile::recent_object_profile_context_lines,
    scene_profile::recent_scene_profile_context_lines, shared::truncate_chars,
    summaries::layered_chapter_summary_lines,
};
use crate::state::AppState;
use a4n_core::models::chapter::chapter;
#[cfg(test)]
use a4n_core::models::character::{character_state_index, protagonist_profile};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

const RECENT_ANALYSIS_CHAPTERS: u64 = 12;
const CHAPTER_CONTEXT_CHAR_LIMIT: usize = 16_000;

pub async fn resolve_chapter_context_before(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    explicit: Option<&str>,
) -> Result<Option<String>, String> {
    if let Some(text) = explicit.map(str::trim).filter(|text| !text.is_empty()) {
        return Ok(Some(text.to_string()));
    }
    if chapter_number <= 1 {
        return Ok(None);
    }

    let previous_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.lt(chapter_number))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("构建章节前情上下文失败: {err}"))?
        .into_iter()
        .filter(chapter::Model::has_available_content)
        .collect::<Vec<_>>();

    if previous_chapters.is_empty() {
        return Ok(None);
    }

    let mut lines = Vec::new();
    let summary_lines = layered_chapter_summary_lines(&previous_chapters, chapter_number);
    if !summary_lines.is_empty() {
        lines.push("## 前文摘要".to_string());
        lines.extend(summary_lines);
    }

    let recent_analysis = previous_chapters
        .iter()
        .rev()
        .take(RECENT_ANALYSIS_CHAPTERS as usize)
        .rev()
        .collect::<Vec<_>>();
    let analysis_lines = recent_analysis
        .iter()
        .filter_map(|chapter| chapter_analysis_context_line(chapter))
        .collect::<Vec<_>>();
    if !analysis_lines.is_empty() {
        lines.push(String::new());
        lines.push("## 前文结构化线索".to_string());
        lines.extend(analysis_lines);
    }

    let character_state_lines =
        recent_character_state_context_lines(state, project_id, chapter_number).await?;
    if !character_state_lines.is_empty() {
        lines.push(String::new());
        lines.push("## 角色状态".to_string());
        lines.extend(character_state_lines);
    }

    let object_profile_lines =
        recent_object_profile_context_lines(state, project_id, chapter_number).await?;
    if !object_profile_lines.is_empty() {
        lines.push(String::new());
        lines.push("## 关键物体档案".to_string());
        lines.extend(object_profile_lines);
    }

    let scene_profile_lines =
        recent_scene_profile_context_lines(state, project_id, chapter_number).await?;
    if !scene_profile_lines.is_empty() {
        lines.push(String::new());
        lines.push("## 场景档案".to_string());
        lines.extend(scene_profile_lines);
    }

    if let Some(tail) = previous_chapter_tail(state, project_id, chapter_number).await? {
        lines.push(String::new());
        lines.push("## 上一章结尾".to_string());
        lines.push(tail);
    }

    let context = lines.join("\n");
    if context.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(truncate_chars(&context, CHAPTER_CONTEXT_CHAR_LIMIT)))
    }
}

async fn previous_chapter_tail(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<Option<String>, String> {
    let previous = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.lt(chapter_number))
        .order_by_desc(chapter::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("查询上一章结尾失败: {err}"))?
        .into_iter()
        .find(chapter::Model::has_available_content);

    Ok(previous.and_then(|chapter| {
        let content = chapter.content;
        let mut tail_chars: Vec<char> = content.chars().rev().take(600).collect();
        tail_chars.reverse();
        let tail: String = tail_chars.into_iter().collect();
        if !tail.trim().is_empty() {
            Some(tail)
        } else {
            None
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chapter_for_context(
        chapter_number: i32,
        title: Option<&str>,
        summary: Option<&str>,
        analysis_data: Option<serde_json::Value>,
    ) -> chapter::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter::Model {
            id: i64::from(chapter_number),
            project_id: 1,
            outline_id: None,
            chapter_number,
            title: title.map(str::to_string),
            content: "正文".to_string(),
            word_count: 2,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: summary.map(str::to_string),
            style_tags: None,
            analysis_data,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn layered_chapter_summary_lines_omits_missing_summary_without_placeholders() {
        let chapters = vec![
            chapter_for_context(1, Some("已有标题"), None, None),
            chapter_for_context(2, None, Some("有效摘要。后续句子"), None),
        ];

        let lines = layered_chapter_summary_lines(&chapters, 3);
        let text = lines.join("\n");

        assert!(!text.contains("无标题"));
        assert!(!text.contains("无摘要"));
        assert!(!text.contains("已有标题"));
        assert!(text.contains("第2章：有效摘要"));
    }

    #[test]
    fn chapter_analysis_context_line_omits_missing_title_without_placeholder() {
        let chapter = chapter_for_context(
            7,
            None,
            None,
            Some(serde_json::json!({
                "key_events": [
                    { "description": "主角进入边境城" }
                ]
            })),
        );

        let line =
            chapter_analysis_context_line(&chapter).expect("analysis context line should exist");

        assert!(line.starts_with("第7章："));
        assert!(!line.contains("无标题"));
        assert!(line.contains("主角进入边境城"));
    }

    fn profile_for_context(last_synced_chapter: i32) -> protagonist_profile::Model {
        let now = chrono::Utc::now().naive_utc();
        protagonist_profile::Model {
            id: 1,
            project_id: 1,
            name: "林岚".to_string(),
            gender: None,
            age: None,
            base_attributes: serde_json::json!({}),
            current_state: serde_json::json!({
                "explicit_attributes": {
                    "appearance": "穿青色劲装，左腕有旧伤",
                    "weapon": "短刀"
                },
                "implicit_attributes": {
                    "personality": "谨慎但会为弱者冒险"
                },
                "social_attributes": {
                    "身份": "边城医馆学徒"
                }
            }),
            character_arc: None,
            voice_fingerprint: None,
            appearance: None,
            background: None,
            personality: None,
            last_synced_chapter,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn character_state_context_line_includes_profile_constraints() {
        let now = chrono::Utc::now().naive_utc();
        let state_index = character_state_index::Model {
            id: 1,
            profile_id: 1,
            chapter_number: 3,
            state_summary: "刚逃出城门，正在寻找线索".to_string(),
            key_attributes: serde_json::json!({
                "explicit_attributes": {
                    "appearance": "衣摆沾血",
                    "weapon": "短刀"
                },
                "implicit_attributes": {
                    "personality": "谨慎"
                },
                "social_attributes": {}
            }),
            emotional_state: Some("警惕".to_string()),
            current_location: Some("北城门".to_string()),
            current_goal: Some("找到失踪的商队".to_string()),
            created_at: now,
        };
        let profile = profile_for_context(3);

        let line = character_state_context_line(&state_index, &profile)
            .expect("state context should be rendered");

        assert!(line.contains("第3章后，林岚"));
        assert!(line.contains("显性特征: appearance: 衣摆沾血"));
        assert!(line.contains("性格特质: personality: 谨慎"));
        assert!(line.contains("目标: 找到失踪的商队"));
    }

    #[test]
    fn profile_context_line_skips_future_synced_state() {
        let profile = profile_for_context(8);

        assert!(character_profile_context_line(&profile, 8).is_none());
        assert!(character_profile_context_line(&profile, 7).is_none());
        assert!(
            character_profile_context_line(&profile, 9)
                .expect("past state should be usable")
                .contains("第8章后，林岚")
        );
    }
}
