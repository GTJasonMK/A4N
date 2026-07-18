use crate::models::project::text_anchor;

use super::lookup::anchor_by_id;

fn anchor_quote_in_exact_chapter(
    path: &str,
    anchor_id: i64,
    text_anchors: &[text_anchor::Model],
    chapter_id: i64,
    scope_label: &str,
) -> Result<String, String> {
    let anchor = anchor_by_id(path, anchor_id, text_anchors)?;
    if anchor.source_chapter_id != Some(chapter_id) {
        return Err(format!(
            "{path} 无效：text_anchor_id={anchor_id} 不属于{scope_label}"
        ));
    }
    Ok(anchor.quote.clone())
}

pub fn anchor_quotes_in_exact_chapter(
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
