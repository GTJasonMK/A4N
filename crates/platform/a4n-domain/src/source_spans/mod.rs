//! Source text evidence values and pure projection algorithms.
//!
//! Database rows implement the small input traits in their owning adapter
//! crate. The algorithms here never depend on ORM entities or persistence.

mod anchors;
mod evidence;
mod metadata;
mod spans;
mod types;

pub use anchors::{
    AnchorQuoteIndex, AnchorQuoteWithChapter, anchor_quote_with_chapter_number,
    anchor_quotes_in_exact_chapter, anchor_quotes_with_chapter_numbers, text_anchor_prompt_values,
};
pub use evidence::{
    source_span_context_excerpt, source_span_prompt_excerpt, source_span_prompt_text,
    source_span_prompt_value, source_span_prompt_values, text_anchor_prompt_text,
};
pub use metadata::{
    ChapterSourceContext, metadata_source_content_sha256, source_metadata,
    source_metadata_with_flags,
};
pub use spans::{source_paragraph_ranges, source_spans_for_chapters, source_spans_for_content};
pub use types::{SourceChapter, SourceSpan, SourceSpanIndex, SourceTextAnchor};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn paragraphs_and_evidence_share_indexes_for_all_line_endings() {
        for newline in ["\n", "\r\n", "\r"] {
            let content =
                format!("{newline}  第一段。{newline}第二段。{newline}  {newline}第三段。 ");
            let ranges = source_paragraph_ranges(&content);
            assert_eq!(
                ranges
                    .iter()
                    .map(|(start, end)| &content[*start..*end])
                    .collect::<Vec<_>>(),
                ["  第一段。", "第二段。", "第三段。"]
            );
            let spans = source_spans_for_content(7, 3, &content);
            assert_eq!(
                spans
                    .iter()
                    .map(|span| span.paragraph_index)
                    .collect::<Vec<_>>(),
                [1, 2, 3]
            );
        }
        assert!(source_paragraph_ranges(" \r\n\t").is_empty());
        assert_eq!(source_paragraph_ranges("正文。"), [(0, "正文。".len())]);
    }

    struct Chapter {
        id: i64,
        number: i32,
        content: String,
    }

    impl SourceChapter for Chapter {
        fn source_chapter_id(&self) -> i64 {
            self.id
        }

        fn source_chapter_number(&self) -> i32 {
            self.number
        }

        fn source_content(&self) -> &str {
            &self.content
        }
    }

    struct Anchor {
        id: i64,
        chapter_id: Option<i64>,
        quote: String,
    }

    impl SourceTextAnchor for Anchor {
        fn source_anchor_id(&self) -> i64 {
            self.id
        }

        fn source_anchor_chapter_id(&self) -> Option<i64> {
            self.chapter_id
        }

        fn source_anchor_quote(&self) -> &str {
            &self.quote
        }

        fn source_anchor_char_range(&self) -> (i32, i32) {
            (0, self.quote.chars().count() as i32)
        }
    }

    #[test]
    fn derives_spans_from_pure_chapter_input() {
        let chapter = Chapter {
            id: 7,
            number: 1,
            content: "他转身走向厨房，地上有一把军用匕首，刀身上还沾着血迹。".to_string(),
        };
        let spans = source_spans_for_chapters(&[chapter]);
        assert!(spans.iter().any(|span| span.text == "地上有一把军用匕首"));
        assert!(spans.iter().all(|span| span.char_start < span.char_end));
    }

    #[test]
    fn annotated_source_preserves_original_text_and_rejects_stale_ranges() {
        let content = "“别走！”她说。\n\n雨落下来，🌧️打湿窗台。";
        let mut spans = source_spans_for_content(7, 3, content);
        let annotated = source_span_prompt_text(content, &spans).expect("valid source");
        let restored = spans.iter().fold(annotated.clone(), |text, span| {
            text.replace(&format!("⟦{}⟧", span.id), "")
        });
        assert_eq!(restored, content);
        assert!(annotated.contains("！”"));
        assert!(annotated.contains("\n\n"));
        assert!(
            annotated.len()
                < serde_json::to_string(&source_span_prompt_values(&spans))
                    .unwrap()
                    .len()
        );
        spans[0].text = "已变更".to_string();
        assert!(source_span_prompt_text(content, &spans).is_err());
        spans[0].char_end = usize::MAX;
        assert!(source_span_prompt_text(content, &spans).is_err());
    }

    #[test]
    fn scoped_excerpt_keeps_quoted_paragraphs_without_unrelated_story() {
        let text = "“别走！”她说。\n\n不相关的往事。\n\n🌧️雨落下来，门关上了。";
        let spans = source_spans_for_content(7, 2, text);
        let selected = spans
            .iter()
            .filter(|span| span.text == "别走" || span.text.contains("雨落下来"))
            .cloned()
            .collect::<Vec<_>>();
        let excerpt = source_span_prompt_excerpt(text, &selected).unwrap();
        assert!(excerpt.contains("！”她说。"));
        assert!(excerpt.contains("门关上了。"));
        assert!(!excerpt.contains("不相关的往事"));
        let plain = source_span_context_excerpt(text, &selected).unwrap();
        assert!(plain.contains("“别走！”她说。"));
        assert!(!plain.contains("⟦"));
        assert!(!plain.contains("不相关的往事"));
        for span in &selected {
            assert_eq!(excerpt.matches(&format!("⟦{}⟧", span.id)).count(), 1);
        }
        assert_eq!(source_span_prompt_excerpt(text, &[]).unwrap(), "");
    }

    #[test]
    fn annotated_anchor_content_uses_only_current_chapter_and_valid_quotes() {
        let chapter = Chapter {
            id: 10,
            number: 3,
            content: "潮汐城。\n".into(),
        };
        let mut anchors = vec![
            Anchor {
                id: 20,
                chapter_id: Some(10),
                quote: "潮汐城".into(),
            },
            Anchor {
                id: 21,
                chapter_id: Some(11),
                quote: "其他章节".into(),
            },
        ];
        assert_eq!(
            text_anchor_prompt_text(&chapter, &anchors).unwrap(),
            "⟦20⟧潮汐城。\n"
        );
        anchors[0].quote = "旧正文".into();
        assert!(text_anchor_prompt_text(&chapter, &anchors).is_err());
    }

    #[test]
    fn excerpts_share_range_validation_and_respect_all_line_endings() {
        for newline in ["\n", "\r", "\r\n"] {
            let text = format!("前文。{newline}“别走！”她说。{newline}后文。");
            let spans = source_spans_for_content(7, 2, &text);
            let span = spans.iter().find(|span| span.text == "别走").unwrap();
            let selected = std::slice::from_ref(span);
            assert_eq!(
                source_span_context_excerpt(&text, selected).unwrap(),
                "“别走！”她说。"
            );
            assert_eq!(
                source_span_prompt_excerpt(&text, selected).unwrap(),
                format!("“⟦{}⟧别走！”她说。", span.id),
            );
            let overlapping = [span.clone(), span.clone()];
            assert!(source_span_context_excerpt(&text, &overlapping).is_err());
            assert!(source_span_prompt_excerpt(&text, &overlapping).is_err());
        }
    }

    #[test]
    fn prompt_projection_keeps_only_stable_fields() {
        let chapter = Chapter {
            id: 10,
            number: 3,
            content: "正文".to_string(),
        };
        let anchor = Anchor {
            id: 20,
            chapter_id: Some(10),
            quote: "潮汐城".to_string(),
        };
        let value = text_anchor_prompt_values(&[chapter], &[anchor]);
        assert_eq!(
            value[0],
            json!({"id": 20, "chapter_label": "第3章", "quote": "潮汐城"})
        );
    }

    #[test]
    fn anchor_index_reports_scope_errors() {
        let chapters = [Chapter {
            id: 10,
            number: 3,
            content: "正文".to_string(),
        }];
        let anchors = [Anchor {
            id: 20,
            chapter_id: Some(10),
            quote: "潮汐城".to_string(),
        }];
        assert_eq!(
            anchor_quote_with_chapter_number("evidence", 20, &chapters, &anchors)
                .expect("valid anchor"),
            AnchorQuoteWithChapter {
                quote: "潮汐城".to_string(),
                chapter_number: 3,
            }
        );
        assert_eq!(
            anchor_quotes_in_exact_chapter("evidence", &[20], &anchors, 11, "当前章节"),
            Err("evidence 无效：text_anchor_id=20 不属于当前章节".to_string())
        );
    }

    #[test]
    fn metadata_and_span_index_are_pure() {
        let metadata = source_metadata(ChapterSourceContext { content: "正文" });
        assert_eq!(metadata["content_chars"], 2);
        let spans = source_spans_for_content(1, 1, "绷带。");
        assert_eq!(
            SourceSpanIndex::new(&spans).resolve_text(&spans[0].id),
            Ok("绷带".to_string())
        );
    }

    #[test]
    fn nested_hash_extraction_requires_non_empty_hashes() {
        let nested = json!({ "source": { "content_sha256": "same" } });
        assert_eq!(metadata_source_content_sha256(Some(&nested)), Some("same"));
        assert_eq!(metadata_source_content_sha256(Some(&json!({}))), None);
    }
}
