use std::collections::HashMap;

use serde_json::{Value, json};

use crate::models::{chapter::chapter, project::text_anchor};

/// Projects persisted text anchors into the minimal evidence shape exposed to
/// prompts, omitting internal offsets, hashes, and persistence metadata.
pub fn text_anchor_prompt_values(
    chapters: &[chapter::Model],
    anchors: &[text_anchor::Model],
) -> Vec<Value> {
    let chapter_numbers = chapters
        .iter()
        .map(|chapter| (chapter.id, chapter.chapter_number))
        .collect::<HashMap<_, _>>();

    anchors
        .iter()
        .map(|anchor| {
            let chapter_number = anchor
                .source_chapter_id
                .and_then(|chapter_id| chapter_numbers.get(&chapter_id).copied());
            json!({
                "id": anchor.id,
                "chapter_label": chapter_number.map(|value| format!("第{value}章")),
                "quote": anchor.quote,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::Value;

    use super::*;

    fn chapter_for_prompt_projection() -> chapter::Model {
        let now = Utc::now().naive_utc();
        chapter::Model {
            id: 42,
            project_id: 1,
            outline_id: None,
            chapter_number: 3,
            title: Some("第三章".to_string()),
            content: "他在旧书房抽屉里摸到一枚青铜钥匙。".to_string(),
            word_count: 20,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn anchor_for_prompt_projection(source_chapter_id: Option<i64>) -> text_anchor::Model {
        text_anchor::Model {
            id: 99,
            project_id: 1,
            source_chapter_id,
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_kind: "chapter_content".to_string(),
            source_id: 42,
            paragraph_index: 1,
            start_offset: 0,
            end_offset: 10,
            quote: "他在旧书房抽屉里摸到一枚青铜钥匙".to_string(),
            quote_hash: "hash".to_string(),
            source_version_hash: "source-version".to_string(),
            created_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn exposes_only_prompt_evidence_fields() {
        let chapter = chapter_for_prompt_projection();
        let anchor = anchor_for_prompt_projection(Some(chapter.id));

        let values = text_anchor_prompt_values(&[chapter], &[anchor]);
        let object = values[0].as_object().expect("prompt anchor object");

        assert_eq!(object.get("id").and_then(Value::as_i64), Some(99));
        assert_eq!(
            object.get("chapter_label").and_then(Value::as_str),
            Some("第3章")
        );
        assert_eq!(
            object.get("quote").and_then(Value::as_str),
            Some("他在旧书房抽屉里摸到一枚青铜钥匙")
        );
        assert_eq!(object.len(), 3);
        assert!(object.get("source_chapter_id").is_none());
        assert!(object.get("start_offset").is_none());
        assert!(object.get("end_offset").is_none());
        assert!(object.get("quote_hash").is_none());
    }

    #[test]
    fn keeps_anchor_order_and_uses_null_label_without_matching_chapter() {
        let chapter = chapter_for_prompt_projection();
        let matched = anchor_for_prompt_projection(Some(chapter.id));
        let mut unmatched = anchor_for_prompt_projection(Some(404));
        unmatched.id = 100;
        unmatched.quote = "未知章节证据".to_string();

        let values = text_anchor_prompt_values(&[chapter], &[matched, unmatched]);

        assert_eq!(values[0]["id"], 99);
        assert_eq!(values[1]["id"], 100);
        assert!(values[1]["chapter_label"].is_null());
        assert_eq!(values[1]["quote"], "未知章节证据");
    }
}
