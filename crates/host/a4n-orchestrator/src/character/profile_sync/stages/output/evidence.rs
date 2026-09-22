use serde_json::Value;
use std::collections::HashMap;

use crate::character::required_i32_at;

pub(super) struct ChapterEvidenceIndex<'a> {
    chapters: HashMap<i32, ChapterEvidence<'a>>,
}

enum ChapterEvidence<'a> {
    MissingAnchors,
    Anchors(HashMap<i64, Option<&'a str>>),
}

impl<'a> ChapterEvidenceIndex<'a> {
    pub(super) fn new(chapter_data: &'a [Value]) -> Self {
        let mut chapters = HashMap::with_capacity(chapter_data.len());
        for chapter in chapter_data {
            let Some(chapter_number) = chapter
                .get("chapter_number")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
            else {
                continue;
            };
            if chapters.contains_key(&chapter_number) {
                continue;
            }
            let evidence = match chapter.get("text_anchors").and_then(Value::as_array) {
                Some(anchors) => ChapterEvidence::Anchors(anchors.iter().fold(
                    HashMap::with_capacity(anchors.len()),
                    |mut by_id, anchor| {
                        let Some(anchor_id) = anchor.get("id").and_then(Value::as_i64) else {
                            return by_id;
                        };
                        let quote = anchor
                            .get("quote")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|quote| !quote.is_empty());
                        by_id.entry(anchor_id).or_insert(quote);
                        by_id
                    },
                )),
                None => ChapterEvidence::MissingAnchors,
            };
            chapters.insert(chapter_number, evidence);
        }
        Self { chapters }
    }

    fn get(&self, chapter_number: i32) -> Option<&ChapterEvidence<'a>> {
        self.chapters.get(&chapter_number)
    }
}

pub(super) fn ensure_stage_anchor_exists(
    item: &Value,
    label: &str,
    index: usize,
    anchor_field: &str,
    chapter_index: &ChapterEvidenceIndex<'_>,
) -> Result<(), String> {
    let chapter_number = required_i32_at(
        item,
        "chapter_number",
        &format!("{label}[{index}].chapter_number"),
    )?;
    let anchor_id = item
        .get(anchor_field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}[{index}].{anchor_field} 必须是 JSON 整数数字"))?;
    if anchor_id <= 0 {
        return Err(format!("{label}[{index}].{anchor_field} 必须大于 0"));
    }
    let chapter = chapter_index
        .get(chapter_number)
        .ok_or_else(|| format!("{label}[{index}] 缺少可核对证据的第 {chapter_number} 章正文"))?;
    let exists = matches!(
        chapter,
        ChapterEvidence::Anchors(anchors) if anchors.contains_key(&anchor_id)
    );
    if !exists {
        return Err(format!(
            "{label}[{index}].{anchor_field} 不在第 {chapter_number} 章 text_anchors 中：{anchor_id}"
        ));
    }
    Ok(())
}

pub(super) fn validate_sync_evidence_against_chapters(
    changes: &mut [Value],
    behaviors: &mut [Value],
    deletion_candidates: &mut [Value],
    chapter_data: &[Value],
) -> Result<(), String> {
    if chapter_data.is_empty() {
        return Ok(());
    }

    let chapter_index = ChapterEvidenceIndex::new(chapter_data);
    for (index, change) in changes.iter_mut().enumerate() {
        materialize_item_evidence(
            "attribute_changes",
            index,
            change,
            "evidence_anchor_id",
            "evidence",
            &chapter_index,
        )?;
    }
    for (index, behavior) in behaviors.iter_mut().enumerate() {
        materialize_item_evidence(
            "behaviors",
            index,
            behavior,
            "original_text_anchor_id",
            "original_text",
            &chapter_index,
        )?;
    }
    for (index, candidate) in deletion_candidates.iter_mut().enumerate() {
        materialize_item_evidence(
            "deletion_candidates",
            index,
            candidate,
            "evidence_anchor_id",
            "evidence",
            &chapter_index,
        )?;
    }
    Ok(())
}

fn materialize_item_evidence(
    label: &str,
    index: usize,
    item: &mut Value,
    anchor_field: &str,
    target_field: &str,
    chapter_index: &ChapterEvidenceIndex<'_>,
) -> Result<(), String> {
    let chapter_number = required_i32_at(
        item,
        "chapter_number",
        &format!("{label}[{index}].chapter_number"),
    )?;
    let anchor_id = required_anchor_id(
        item,
        anchor_field,
        &format!("{label}[{index}].{anchor_field}"),
    )?;
    let chapter = chapter_index
        .get(chapter_number)
        .ok_or_else(|| format!("{label}[{index}] 缺少可核对证据的第 {chapter_number} 章正文"))?;
    let text = match chapter {
        ChapterEvidence::MissingAnchors => {
            return Err(format!(
                "{label}[{index}] 第 {chapter_number} 章上下文缺少 text_anchors，无法核对证据"
            ));
        }
        ChapterEvidence::Anchors(anchors) => anchors.get(&anchor_id).copied().flatten(),
    }
        .ok_or_else(|| {
            format!("{label}[{index}].{anchor_field} 不在第 {chapter_number} 章 text_anchors 中：{anchor_id}")
        })?
        .to_string();

    if let Some(object) = item.as_object_mut() {
        object.insert(target_field.to_string(), Value::String(text));
        object.remove(anchor_field);
    }
    Ok(())
}

fn required_anchor_id(item: &Value, field: &str, path: &str) -> Result<i64, String> {
    let id = item
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("角色 AI 字段 {path} 必须是 JSON 整数数字"))?;
    if id <= 0 {
        return Err(format!("角色 AI 字段 {path} 必须大于 0"));
    }
    Ok(id)
}
