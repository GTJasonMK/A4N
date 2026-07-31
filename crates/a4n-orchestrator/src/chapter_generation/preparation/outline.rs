use a4n_core::models::chapter::chapter_outline;
use serde_json::Value;

use super::{
    reference_context::json::is_meaningful_json_value,
    variables::non_empty_trimmed,
    word_count::{ChapterGenerationWordCountRange, format_generation_word_count_requirement},
};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn required_chapter_generation_outline(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<chapter_outline::Model, String> {
        self.chapter_outline_by_project_number(project_id, chapter_number)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("项目 {project_id} 第 {chapter_number} 章大纲未找到"))
    }
}

fn format_chapter_outline_for_generation(
    outline: &chapter_outline::Model,
    chapter_title: &str,
    word_count_range: Option<ChapterGenerationWordCountRange>,
) -> Result<String, String> {
    let mut lines = vec![
        "## 当前任务".to_string(),
        format!("章节编号: 第{}章", outline.chapter_number),
        format!("章节标题: {chapter_title}"),
    ];

    push_optional_text(&mut lines, "大纲摘要", outline.summary.as_deref());
    if word_count_range.is_none() {
        push_optional_i32(&mut lines, "目标字数", outline.word_count_target);
    }
    push_optional_f32(&mut lines, "节奏评分", outline.pacing_score);
    push_optional_json(&mut lines, "节拍安排", outline.beats.as_ref())?;
    push_optional_json(&mut lines, "关键场景", outline.key_scenes.as_ref())?;
    push_optional_json(&mut lines, "涉及角色", outline.characters_involved.as_ref())?;
    push_optional_json(
        &mut lines,
        "本章埋设伏笔",
        outline.foreshadowing_setup.as_ref(),
    )?;
    push_optional_json(
        &mut lines,
        "本章回收伏笔",
        outline.foreshadowing_payoff.as_ref(),
    )?;

    Ok(lines.join("\n"))
}

pub fn merge_generation_outline_text(
    chapter_number: i32,
    db_outline: Option<&chapter_outline::Model>,
    provided_outline: Option<&str>,
    writing_instruction: Option<&str>,
    chapter_title: Option<&str>,
    word_count_range: Option<ChapterGenerationWordCountRange>,
) -> Result<String, String> {
    let mut sections = Vec::new();
    let mut has_outline_source = false;

    if let Some(outline) = db_outline {
        let title = chapter_title
            .and_then(non_empty_trimmed)
            .ok_or_else(|| format!("第 {chapter_number} 章缺少标题，无法构建章节生成大纲"))?;
        sections.push(format_chapter_outline_for_generation(
            outline,
            title,
            word_count_range,
        )?);
        has_outline_source = true;
    }

    if let Some(outline) = provided_outline.map(str::trim).filter(|s| !s.is_empty()) {
        let is_duplicate = db_outline
            .and_then(|db| db.summary.as_deref())
            .is_some_and(|summary| summary.trim() == outline);
        if !is_duplicate {
            sections.push(format!("## 用户补充章纲\n{outline}"));
        }
        has_outline_source = true;
    }

    if !has_outline_source {
        return Err(format!("第 {chapter_number} 章缺少章节大纲，无法生成正文"));
    }

    if let Some(instruction) = writing_instruction.map(str::trim).filter(|s| !s.is_empty()) {
        sections.push(format!("## 写作指令\n{instruction}"));
    }

    if let Some(range) = word_count_range {
        sections.push(format_generation_word_count_requirement(range));
    }

    Ok(sections.join("\n\n"))
}

pub fn required_generation_chapter_title(
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    title_hint: Option<&str>,
) -> Result<String, String> {
    title_hint
        .and_then(non_empty_trimmed)
        .map(str::to_string)
        .or_else(|| {
            outline
                .and_then(|outline| outline.title.as_deref())
                .and_then(non_empty_trimmed)
                .map(str::to_string)
        })
        .ok_or_else(|| format!("第 {chapter_number} 章缺少标题，无法生成正文"))
}

pub(super) fn push_optional_text(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|s| !s.is_empty()) {
        lines.push(format!("{label}: {value}"));
    }
}

fn push_optional_i32(lines: &mut Vec<String>, label: &str, value: Option<i32>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {value}"));
    }
}

fn push_optional_f32(lines: &mut Vec<String>, label: &str, value: Option<f32>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {:.2}", value));
    }
}

fn push_optional_json(
    lines: &mut Vec<String>,
    label: &str,
    value: Option<&Value>,
) -> Result<(), String> {
    let Some(value) = value.filter(|value| is_meaningful_json_value(value)) else {
        return Ok(());
    };
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("序列化章节生成{label}失败: {error}"))?;
    lines.push(format!("{label}:\n{text}"));
    Ok(())
}
