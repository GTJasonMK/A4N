use super::super::contracts::{RegenerateSceneStageOutputContext, SourceSyncReferenceSets};
use serde_json::{Map, Value};

pub(in super::super) fn validate_source_span_ids(
    object: &Map<String, Value>,
    refs: &SourceSyncReferenceSets,
    context: &RegenerateSceneStageOutputContext,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    let Some(value) = object.get("source_span_ids") else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let items = value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 必须是字符串数组: {owner_kind}_id={owner_id}"
        )
    })?;
    for item in items {
        let span_id = item.as_str().ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.source_span_ids 必须是字符串数组: {owner_kind}_id={owner_id}"
            )
        })?;
        let (chapter_number, text_anchor_id) =
            parse_galgame_source_span_id(span_id, owner_kind, owner_id, path)?;
        ensure_package!(
            refs.source_snapshot_chapter_numbers
                .contains(&chapter_number),
            "项目族导入包 regenerate_scene {path}.source_span_ids 章节不存在: {owner_kind}_id={}, source_span_id={}",
            owner_id,
            span_id
        );
        ensure_package!(
            chapter_number == context.expected_chapter_number,
            "项目族导入包 regenerate_scene {path}.source_span_ids 章节编号与重建目标章节不一致: {owner_kind}_id={}, source_span_id={}, chapter_number={}, expected={}",
            owner_id,
            span_id,
            chapter_number,
            context.expected_chapter_number
        );
        ensure_package!(
            refs.text_anchor_ids.contains(&text_anchor_id),
            "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点不存在: {owner_kind}_id={}, source_span_id={}",
            owner_id,
            span_id
        );
        let actual_snapshot_chapter_id = refs
            .text_anchor_source_snapshot_chapter_ids
            .get(&text_anchor_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点不属于源快照章节: {owner_kind}_id={}, source_span_id={span_id}",
                    owner_id
                )
            })?;
        ensure_package!(
            actual_snapshot_chapter_id == context.expected_source_snapshot_chapter_id,
            "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点不属于重建目标源快照章节: {owner_kind}_id={}, source_span_id={}, source_snapshot_chapter_id={}, expected={}",
            owner_id,
            span_id,
            actual_snapshot_chapter_id,
            context.expected_source_snapshot_chapter_id
        );
        let actual_chapter_number = refs
            .source_span_anchor_chapter_numbers
            .get(&text_anchor_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点不属于源快照章节: {owner_kind}_id={}, source_span_id={span_id}",
                    owner_id
                )
            })?;
        ensure_package!(
            actual_chapter_number == chapter_number,
            "项目族导入包 regenerate_scene {path}.source_span_ids 章节编号与文本锚点不一致: {owner_kind}_id={}, source_span_id={}, anchor_chapter_number={}",
            owner_id,
            span_id,
            actual_chapter_number
        );
    }
    Ok(())
}

fn parse_galgame_source_span_id(
    span_id: &str,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(i32, i64), String> {
    let Some((chapter_prefix, anchor_id)) = span_id.rsplit_once(":ta") else {
        return Err(format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 格式无效: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        ));
    };
    let Some(chapter_number) = chapter_prefix.strip_prefix("ch") else {
        return Err(format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 格式无效: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        ));
    };
    let chapter_number = chapter_number.parse::<i32>().map_err(|_| {
        format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 章节编号无效: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        )
    })?;
    let text_anchor_id = anchor_id.parse::<i64>().map_err(|_| {
        format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids 文本锚点 ID 无效: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        )
    })?;
    if chapter_number <= 0 || text_anchor_id <= 0 {
        return Err(format!(
            "项目族导入包 regenerate_scene {path}.source_span_ids ID 必须为正数: {owner_kind}_id={}, source_span_id={span_id}",
            owner_id
        ));
    }
    Ok((chapter_number, text_anchor_id))
}
