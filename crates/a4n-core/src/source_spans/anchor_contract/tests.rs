use chrono::Utc;

use crate::models::chapter::chapter;
use crate::models::project::text_anchor;

use super::{
    anchor_quote_with_chapter_number, anchor_quotes_in_exact_chapter,
    anchor_quotes_with_chapter_numbers,
};

fn chapter(id: i64, number: i32) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::Model {
        id,
        project_id: 1,
        outline_id: None,
        chapter_number: number,
        title: None,
        content: "正文".to_string(),
        word_count: 2,
        status: chapter::STATUS_COMPLETED.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

fn anchor(id: i64, chapter_id: Option<i64>, quote: &str) -> text_anchor::Model {
    text_anchor::Model {
        id,
        project_id: 1,
        source_chapter_id: chapter_id,
        source_snapshot_id: None,
        source_snapshot_chapter_id: None,
        source_kind: "chapter".to_string(),
        source_id: chapter_id.unwrap_or(0),
        paragraph_index: 1,
        start_offset: 0,
        end_offset: quote.chars().count() as i32,
        quote: quote.to_string(),
        quote_hash: "hash".to_string(),
        source_version_hash: "version".to_string(),
        created_at: Utc::now().naive_utc(),
    }
}

#[test]
fn projects_anchor_quotes_and_chapter_numbers_in_input_order() {
    let chapters = vec![chapter(10, 3), chapter(11, 4)];
    let anchors = vec![anchor(20, Some(10), "潮汐城"), anchor(21, Some(11), "白塔")];
    let projected = anchor_quote_with_chapter_number("evidence", 20, &chapters, &anchors)
        .expect("valid anchor");
    assert_eq!(projected.quote, "潮汐城");
    assert_eq!(projected.chapter_number, 3);

    let projected = anchor_quotes_with_chapter_numbers("evidence", &[21, 20], &chapters, &anchors)
        .expect("valid anchors");
    assert_eq!(
        projected
            .iter()
            .map(|item| (item.quote.as_str(), item.chapter_number))
            .collect::<Vec<_>>(),
        vec![("白塔", 4), ("潮汐城", 3)]
    );
}

#[test]
fn chapter_number_projection_preserves_candidate_and_chapter_errors() {
    let chapters = vec![chapter(10, 3)];
    let anchors = vec![anchor(20, Some(10), "潮汐城"), anchor(21, None, "外部资料")];
    assert_eq!(
        anchor_quote_with_chapter_number("evidence", 22, &chapters, &anchors),
        Err("evidence 无效：text_anchor_id 不在本次输入候选中: 22".to_string())
    );
    assert_eq!(
        anchor_quote_with_chapter_number("evidence", 21, &chapters, &anchors),
        Err("evidence 无效：text_anchor_id=21 不是章节正文锚点".to_string())
    );
    assert_eq!(
        anchor_quote_with_chapter_number("evidence", 20, &[], &anchors),
        Err("evidence 无效：text_anchor_id=20 不属于本次输入章节".to_string())
    );
}

#[test]
fn exact_chapter_projection_preserves_order_and_scope_errors() {
    let anchors = vec![
        anchor(20, Some(10), "潮汐城"),
        anchor(21, None, "外部资料"),
        anchor(22, Some(10), "白塔"),
    ];
    assert_eq!(
        anchor_quotes_in_exact_chapter("evidence", &[22, 20], &anchors, 10, "当前章节"),
        Ok(vec!["白塔".to_string(), "潮汐城".to_string()])
    );
    assert_eq!(
        anchor_quotes_in_exact_chapter("evidence", &[20], &anchors, 11, "当前章节"),
        Err("evidence 无效：text_anchor_id=20 不属于当前章节".to_string())
    );
    assert_eq!(
        anchor_quotes_in_exact_chapter("evidence", &[21], &anchors, 10, "当前章节"),
        Err("evidence 无效：text_anchor_id=21 不属于当前章节".to_string())
    );
    assert_eq!(
        anchor_quotes_in_exact_chapter("evidence", &[23], &anchors, 10, "当前章节"),
        Err("evidence 无效：text_anchor_id 不在本次输入候选中: 23".to_string())
    );
}
