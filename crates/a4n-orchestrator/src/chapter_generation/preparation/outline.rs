use a4n_core::models::chapter::chapter_outline;
use serde_json::Value;

use super::{
    reference_context::json::is_meaningful_json_value,
    variables::non_empty_trimmed,
    word_count::{ChapterGenerationWordCountRange, format_generation_word_count_requirement},
};

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
    let writing_instruction = writing_instruction.and_then(non_empty_trimmed);
    let chapter_title = chapter_title
        .and_then(non_empty_trimmed)
        .map(str::to_string)
        .unwrap_or_else(|| format!("第{chapter_number}章"));

    if let Some(outline) = db_outline {
        sections.push(format_chapter_outline_for_generation(
            outline,
            &chapter_title,
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
        let Some(brief) = writing_instruction else {
            return Err(format!(
                "第 {chapter_number} 章缺少可用章节大纲或本章创作简报，无法生成正文"
            ));
        };
        sections.push(format!(
            "## 当前任务\n章节编号: 第{chapter_number}章\n章节标题: {chapter_title}\n本章创作简报:\n{brief}"
        ));
    }

    if has_outline_source && let Some(instruction) = writing_instruction {
        sections.push(format!("## 写作指令\n{instruction}"));
    }

    if let Some(range) = word_count_range {
        sections.push(format_generation_word_count_requirement(range));
    }

    Ok(sections.join("\n\n"))
}

pub fn generation_chapter_title(
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    title_hint: Option<&str>,
) -> String {
    title_hint
        .and_then(non_empty_trimmed)
        .map(str::to_string)
        .or_else(|| {
            outline
                .and_then(|outline| outline.title.as_deref())
                .and_then(non_empty_trimmed)
                .map(str::to_string)
        })
        .unwrap_or_else(|| format!("第{chapter_number}章"))
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
