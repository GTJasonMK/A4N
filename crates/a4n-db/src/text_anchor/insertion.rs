use super::{
    SOURCE_KIND_CHAPTER_CONTENT, TextAnchorContentInsertInput, TextAnchorDbError,
    TextAnchorDbResult,
};
use a4n_core::{
    models::project::text_anchor,
    source_spans::{source_spans_for_content, source_text_hash},
};
use chrono::Utc;
use sea_orm::{ConnectionTrait, EntityTrait, Set};
use std::collections::HashMap;

const TEXT_ANCHOR_INSERT_BATCH_SIZE: usize = 500;

pub async fn insert_content_anchor_records<C>(
    db: &C,
    input: TextAnchorContentInsertInput<'_>,
) -> TextAnchorDbResult<Vec<text_anchor::Model>>
where
    C: ConnectionTrait,
{
    let mut inserted =
        insert_content_anchor_records_batch(db, std::slice::from_ref(&input)).await?;
    Ok(inserted.pop().unwrap_or_default())
}

pub(crate) async fn insert_content_anchor_records_batch<C>(
    db: &C,
    inputs: &[TextAnchorContentInsertInput<'_>],
) -> TextAnchorDbResult<Vec<Vec<text_anchor::Model>>>
where
    C: ConnectionTrait,
{
    if inputs.is_empty() {
        return Ok(Vec::new());
    }

    let now = Utc::now().naive_utc();
    let mut rows = Vec::new();
    let mut keys_by_input = Vec::with_capacity(inputs.len());
    for input in inputs {
        let source_chapter_id = input.source_chapter_id;
        let source_id = input.source_id;
        let mut input_keys = Vec::new();
        for span in source_spans_for_content(
            source_chapter_id.unwrap_or(source_id),
            input.chapter_number,
            input.content,
        ) {
            let paragraph_index = to_i32(span.sentence_index, "paragraph_index")?;
            let start_offset = to_i32(span.char_start, "start_offset")?;
            let end_offset = to_i32(span.char_end, "end_offset")?;
            input_keys.push((source_id, paragraph_index, start_offset, end_offset));
            rows.push(text_anchor::ActiveModel {
                project_id: Set(input.project_id),
                source_chapter_id: Set(source_chapter_id),
                source_snapshot_id: Set(input.source_snapshot_id),
                source_snapshot_chapter_id: Set(input.source_snapshot_chapter_id),
                source_kind: Set(SOURCE_KIND_CHAPTER_CONTENT.to_string()),
                source_id: Set(source_id),
                paragraph_index: Set(paragraph_index),
                start_offset: Set(start_offset),
                end_offset: Set(end_offset),
                quote_hash: Set(source_text_hash(&span.text)),
                quote: Set(span.text),
                source_version_hash: Set(input.source_version_hash.to_string()),
                created_at: Set(now),
                ..Default::default()
            });
        }
        keys_by_input.push(input_keys);
    }

    if rows.is_empty() {
        return Ok(vec![Vec::new(); inputs.len()]);
    }

    let expected = rows.len();
    let mut inserted = Vec::with_capacity(expected);
    for chunk in rows.chunks(TEXT_ANCHOR_INSERT_BATCH_SIZE) {
        inserted.extend(
            text_anchor::Entity::insert_many(chunk.iter().cloned())
                .exec_with_returning_many(db)
                .await?,
        );
    }
    if inserted.len() != expected {
        return Err(TextAnchorDbError::InvalidArgument(
            "文本锚点批量写入返回行数不一致".to_string(),
        ));
    }

    let mut inserted_by_key = HashMap::<(i64, i32, i32, i32), Vec<_>>::new();
    for anchor in inserted {
        inserted_by_key
            .entry((
                anchor.source_id,
                anchor.paragraph_index,
                anchor.start_offset,
                anchor.end_offset,
            ))
            .or_default()
            .push(anchor);
    }
    for anchors in inserted_by_key.values_mut() {
        anchors.reverse();
    }

    keys_by_input
        .into_iter()
        .map(|keys| {
            keys.into_iter()
                .map(|key| {
                    inserted_by_key
                        .get_mut(&key)
                        .and_then(Vec::pop)
                        .ok_or_else(|| {
                            TextAnchorDbError::InvalidArgument(
                                "文本锚点批量写入缺少返回记录".to_string(),
                            )
                        })
                })
                .collect()
        })
        .collect()
}

fn to_i32(value: usize, field: &str) -> TextAnchorDbResult<i32> {
    i32::try_from(value)
        .map_err(|_| TextAnchorDbError::InvalidArgument(format!("{field} 超出 i32 范围: {value}")))
}
