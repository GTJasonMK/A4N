use super::{
    analysis::chapter_analysis_context_line, character_state::recent_character_state_context_lines,
    object_profile::recent_object_profile_context_lines,
    scene_profile::recent_scene_profile_context_lines, shared::truncate_chars,
    summaries::layered_chapter_summary_lines,
};
use crate::OrchestratorContext;
use a4n_core::models::chapter::chapter;

const RECENT_ANALYSIS_CHAPTERS: usize = 12;
const CHAPTER_CONTEXT_CHAR_LIMIT: usize = 16_000;
const PREVIOUS_CHAPTER_TAIL_CHAR_LIMIT: usize = 600;

impl OrchestratorContext {
    pub async fn resolve_chapter_context_before(
        &self,
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

        let previous_chapters = self
            .chapters_before_number(project_id, chapter_number)
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
            .take(RECENT_ANALYSIS_CHAPTERS)
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

        append_section(
            &mut lines,
            "角色状态",
            recent_character_state_context_lines(self, project_id, chapter_number).await?,
        );
        append_section(
            &mut lines,
            "关键物体档案",
            recent_object_profile_context_lines(self, project_id, chapter_number).await?,
        );
        append_section(
            &mut lines,
            "场景档案",
            recent_scene_profile_context_lines(self, project_id, chapter_number).await?,
        );

        if let Some(tail) = previous_chapter_tail(&previous_chapters) {
            append_section(&mut lines, "上一章结尾", vec![tail]);
        }

        let context = lines.join("\n");
        if context.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(truncate_chars(&context, CHAPTER_CONTEXT_CHAR_LIMIT)))
        }
    }
}

fn append_section(lines: &mut Vec<String>, title: &str, section_lines: Vec<String>) {
    if section_lines.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("## {title}"));
    lines.extend(section_lines);
}

fn previous_chapter_tail(previous_chapters: &[chapter::Model]) -> Option<String> {
    previous_chapters.iter().rev().find_map(|chapter| {
        let mut tail_chars: Vec<char> = chapter
            .content
            .chars()
            .rev()
            .take(PREVIOUS_CHAPTER_TAIL_CHAR_LIMIT)
            .collect();
        tail_chars.reverse();
        let tail: String = tail_chars.into_iter().collect();
        (!tail.trim().is_empty()).then_some(tail)
    })
}
