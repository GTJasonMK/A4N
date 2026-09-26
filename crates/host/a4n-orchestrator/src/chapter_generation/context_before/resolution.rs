use super::{
    analysis::chapter_analysis_context_line, character_state::recent_character_state_context_lines,
    object_profile::recent_object_profile_context_lines,
    scene_profile::recent_scene_profile_context_lines, summaries::layered_chapter_summary_lines,
};
use crate::OrchestratorContext;
use a4n_db_models::models::chapter::chapter;

const RECENT_ANALYSIS_CHAPTERS: usize = 12;
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
        let (previous_result, character_result, object_result, scene_result) = futures::join!(
            async {
                if chapter_number > 1 {
                    self.chapters_before_number(project_id, chapter_number)
                        .await
                } else {
                    Ok(Vec::new())
                }
            },
            recent_character_state_context_lines(self, project_id, chapter_number),
            recent_object_profile_context_lines(self, project_id, chapter_number),
            recent_scene_profile_context_lines(self, project_id, chapter_number),
        );
        let previous_chapters = previous_result
            .map_err(|err| format!("构建章节前情上下文失败: {err}"))?
            .into_iter()
            .filter(chapter::Model::has_available_content)
            .collect::<Vec<_>>();

        let mut lines = Vec::new();
        let mut world_facts = std::collections::BTreeMap::new();
        let ready_sources =
            a4n_db::fact_feedback::current_feedback_sources(&self.database, project_id)
                .await
                .map_err(|error| error.to_string())?;
        for chapter in previous_chapters.iter().filter(|chapter| {
            ready_sources
                .iter()
                .any(|source| source.chapter.id == chapter.id)
        }) {
            if let Some(value) = chapter
                .analysis_data
                .as_ref()
                .and_then(|analysis| analysis.get("world_facts"))
            {
                let facts: Vec<a4n_domain::fact_feedback::WorldFact> =
                    serde_json::from_value(value.clone())
                        .map_err(|error| format!("章节世界事实无效: {error}"))?;
                for fact in facts {
                    if fact.assertion == a4n_domain::fact_feedback::FactAssertion::Observed {
                        world_facts.insert(
                            fact.identity(),
                            format!("- 第{}章确认：{}", chapter.chapter_number, fact.statement),
                        );
                    }
                }
            }
        }
        append_section(
            &mut lines,
            "最新正文事实（优先于尚未更新的蓝图）",
            world_facts.into_values().collect(),
        );
        append_section(&mut lines, "角色状态", character_result?);
        append_section(&mut lines, "关键物体档案", object_result?);
        append_section(&mut lines, "场景档案", scene_result?);
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

        if let Some(tail) = previous_chapter_tail(&previous_chapters) {
            append_section(&mut lines, "上一章结尾", vec![tail]);
        }

        let context = lines.join("\n");
        if context.trim().is_empty() {
            Ok(None)
        } else {
            // The request context ledger owns the budget; cutting the assembled text
            // here could silently discard freshly synchronized facts.
            Ok(Some(context))
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
