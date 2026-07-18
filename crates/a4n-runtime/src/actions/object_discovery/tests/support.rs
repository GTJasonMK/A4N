use a4n_core::models::chapter::chapter;
use a4n_core::models::project::text_anchor;
use a4n_core::source_spans::source_spans_for_chapters;

pub(super) fn chapter_for_test() -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: 1,
        project_id: 1,
        outline_id: None,
        chapter_number: 3,
        title: Some("旧书房".to_string()),
        content: "他在旧书房抽屉里摸到一枚青铜钥匙，钥匙柄部刻着残缺的星纹。".to_string(),
        word_count: 32,
        status: chapter::STATUS_DRAFT.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn anchors_for_test(chapters: &[chapter::Model]) -> Vec<text_anchor::Model> {
    let now = chrono::Utc::now().naive_utc();
    source_spans_for_chapters(chapters)
        .into_iter()
        .enumerate()
        .map(|(index, span)| text_anchor::Model {
            id: i64::try_from(index + 1).unwrap(),
            project_id: 1,
            source_chapter_id: Some(span.chapter_id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_kind: "chapter_content".to_string(),
            source_id: span.chapter_id,
            paragraph_index: i32::try_from(span.sentence_index).unwrap(),
            start_offset: i32::try_from(span.char_start).unwrap(),
            end_offset: i32::try_from(span.char_end).unwrap(),
            quote: span.text,
            quote_hash: format!("hash-{}", index + 1),
            source_version_hash: "source-version".to_string(),
            created_at: now,
        })
        .collect()
}

pub(super) fn anchor_id_containing(anchors: &[text_anchor::Model], text: &str) -> i64 {
    anchors
        .iter()
        .find(|anchor| anchor.quote.contains(text))
        .map(|anchor| anchor.id)
        .unwrap_or_else(|| panic!("missing text anchor containing {text}"))
}
