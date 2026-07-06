use a4n_core::models::chapter::chapter;
use a4n_core::models::project::text_anchor;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) struct AnchorQuoteWithChapter {
    pub quote: String,
    pub chapter_number: i32,
}

pub(crate) fn reject_deprecated_output_fields(
    value: &Value,
    path: &str,
    fields: &[&str],
    expected_field: &str,
) -> Result<(), String> {
    for key in fields {
        if value.get(key).is_some() {
            return Err(format!("{path}.{key} 已废弃；必须输出 {expected_field}"));
        }
    }
    Ok(())
}

pub(crate) fn reject_deprecated_reference_fields(
    value: &Value,
    path: &str,
    fields: &[&str],
    expected_field: &str,
) -> Result<(), String> {
    for key in fields {
        if value.get(key).is_some() {
            return Err(format!("{path}.{key} 已废弃；必须引用 {expected_field}"));
        }
    }
    Ok(())
}

pub(crate) fn anchor_quote_with_chapter_number(
    path: &str,
    anchor_id: i64,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<AnchorQuoteWithChapter, String> {
    let chapter_numbers = chapters
        .iter()
        .map(|chapter| (chapter.id, chapter.chapter_number))
        .collect::<HashMap<_, _>>();
    let anchor = text_anchors
        .iter()
        .find(|anchor| anchor.id == anchor_id)
        .ok_or_else(|| format!("{path} 无效：text_anchor_id 不在本次输入候选中: {anchor_id}"))?;
    let source_chapter_id = anchor
        .source_chapter_id
        .ok_or_else(|| format!("{path} 无效：text_anchor_id={anchor_id} 不是章节正文锚点"))?;
    let chapter_number = chapter_numbers
        .get(&source_chapter_id)
        .copied()
        .ok_or_else(|| format!("{path} 无效：text_anchor_id={anchor_id} 不属于本次输入章节"))?;
    Ok(AnchorQuoteWithChapter {
        quote: anchor.quote.clone(),
        chapter_number,
    })
}

pub(crate) fn anchor_quotes_with_chapter_numbers(
    path: &str,
    anchor_ids: &[i64],
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<AnchorQuoteWithChapter>, String> {
    anchor_ids
        .iter()
        .map(|anchor_id| anchor_quote_with_chapter_number(path, *anchor_id, chapters, text_anchors))
        .collect()
}

fn anchor_quote_in_exact_chapter(
    path: &str,
    anchor_id: i64,
    text_anchors: &[text_anchor::Model],
    chapter_id: i64,
    scope_label: &str,
) -> Result<String, String> {
    let anchor = text_anchors
        .iter()
        .find(|anchor| anchor.id == anchor_id)
        .ok_or_else(|| format!("{path} 无效：text_anchor_id 不在本次输入候选中: {anchor_id}"))?;
    if anchor.source_chapter_id != Some(chapter_id) {
        return Err(format!(
            "{path} 无效：text_anchor_id={anchor_id} 不属于{scope_label}"
        ));
    }
    Ok(anchor.quote.clone())
}

pub(crate) fn anchor_quotes_in_exact_chapter(
    path: &str,
    anchor_ids: &[i64],
    text_anchors: &[text_anchor::Model],
    chapter_id: i64,
    scope_label: &str,
) -> Result<Vec<String>, String> {
    anchor_ids
        .iter()
        .map(|anchor_id| {
            anchor_quote_in_exact_chapter(path, *anchor_id, text_anchors, chapter_id, scope_label)
        })
        .collect()
}
