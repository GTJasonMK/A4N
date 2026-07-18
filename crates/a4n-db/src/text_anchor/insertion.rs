use super::{
    SOURCE_KIND_CHAPTER_CONTENT, TextAnchorContentInsertInput, TextAnchorDbError,
    TextAnchorDbResult,
};
use a4n_core::{
    models::project::text_anchor,
    source_spans::{source_spans_for_content, source_text_hash},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub async fn insert_content_anchor_records<C>(
    db: &C,
    input: TextAnchorContentInsertInput<'_>,
) -> TextAnchorDbResult<Vec<text_anchor::Model>>
where
    C: ConnectionTrait,
{
    let TextAnchorContentInsertInput {
        project_id,
        source_chapter_id,
        source_snapshot_id,
        source_snapshot_chapter_id,
        source_id,
        chapter_number,
        content,
        source_version_hash,
    } = input;
    let now = Utc::now().naive_utc();
    let spans = source_spans_for_content(
        source_chapter_id.unwrap_or(source_id),
        chapter_number,
        content,
    );
    let mut anchors = Vec::with_capacity(spans.len());
    for span in spans {
        anchors.push(
            text_anchor::ActiveModel {
                project_id: Set(project_id),
                source_chapter_id: Set(source_chapter_id),
                source_snapshot_id: Set(source_snapshot_id),
                source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
                source_kind: Set(SOURCE_KIND_CHAPTER_CONTENT.to_string()),
                source_id: Set(source_id),
                paragraph_index: Set(to_i32(span.sentence_index, "paragraph_index")?),
                start_offset: Set(to_i32(span.char_start, "start_offset")?),
                end_offset: Set(to_i32(span.char_end, "end_offset")?),
                quote_hash: Set(source_text_hash(&span.text)),
                quote: Set(span.text),
                source_version_hash: Set(source_version_hash.to_string()),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await?,
        );
    }
    Ok(anchors)
}

fn to_i32(value: usize, field: &str) -> TextAnchorDbResult<i32> {
    i32::try_from(value)
        .map_err(|_| TextAnchorDbError::InvalidArgument(format!("{field} 超出 i32 范围: {value}")))
}
