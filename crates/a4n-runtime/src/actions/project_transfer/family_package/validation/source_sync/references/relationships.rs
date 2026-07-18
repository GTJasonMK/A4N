use super::super::contracts::SourceSyncReferenceSets;

pub(super) fn validate_source_sync_galgame_line_scene_pair(
    galgame_line_id: Option<i64>,
    galgame_scene_id: Option<i64>,
    json_label: &str,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let (Some(line_id), Some(scene_id)) = (galgame_line_id, galgame_scene_id) else {
        return Ok(());
    };
    let line_scene_id = refs
        .galgame_line_scene_ids
        .get(&line_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突 {json_label}.galgame_line_id 引用的 Galgame 行不存在: source_sync_conflict_id={}, galgame_line_id={}",
                conflict_id, line_id
            )
        })?;
    ensure_package!(
        line_scene_id == scene_id,
        "项目族导入包源同步冲突 {json_label}.galgame_line_id 不属于 galgame_scene_id: source_sync_conflict_id={}, galgame_line_id={}, scene_id={}, expected={}",
        conflict_id,
        line_id,
        line_scene_id,
        scene_id
    );
    Ok(())
}

pub(super) fn validate_source_sync_snapshot_chapter_source_pair(
    source_snapshot_chapter_id: Option<i64>,
    source_chapter_id: Option<i64>,
    json_label: &str,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let (Some(snapshot_chapter_id), Some(source_chapter_id)) =
        (source_snapshot_chapter_id, source_chapter_id)
    else {
        return Ok(());
    };
    let (expected_source_chapter_id, _) = refs
        .source_snapshot_chapter_contexts
        .get(&snapshot_chapter_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突 {json_label}.source_snapshot_chapter_id 引用的源快照章节不存在: source_sync_conflict_id={}, source_snapshot_chapter_id={}",
                conflict_id, snapshot_chapter_id
            )
        })?;
    ensure_package!(
        source_chapter_id == expected_source_chapter_id,
        "项目族导入包源同步冲突 {json_label}.source_chapter_id 与 source_snapshot_chapter_id 不一致: source_sync_conflict_id={}, source_chapter_id={}, source_snapshot_chapter_id={}, expected_source_chapter_id={}",
        conflict_id,
        source_chapter_id,
        snapshot_chapter_id,
        expected_source_chapter_id
    );
    Ok(())
}

pub(super) fn validate_source_sync_anchor_snapshot_chapter_pair(
    text_anchor_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
    anchor_key: &str,
    snapshot_chapter_key: &str,
    json_label: &str,
    conflict_id: i64,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let (Some(anchor_id), Some(snapshot_chapter_id)) = (text_anchor_id, source_snapshot_chapter_id)
    else {
        return Ok(());
    };
    let anchor_snapshot_chapter_id = refs
        .text_anchor_source_snapshot_chapter_ids
        .get(&anchor_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包源同步冲突 {json_label}.{anchor_key} 不属于源快照章节: source_sync_conflict_id={}, {anchor_key}={}",
                conflict_id, anchor_id
            )
        })?;
    ensure_package!(
        anchor_snapshot_chapter_id == snapshot_chapter_id,
        "项目族导入包源同步冲突 {json_label}.{anchor_key} 不属于 {snapshot_chapter_key}: source_sync_conflict_id={}, {anchor_key}={}, anchor_source_snapshot_chapter_id={}, {snapshot_chapter_key}={}",
        conflict_id,
        anchor_id,
        anchor_snapshot_chapter_id,
        snapshot_chapter_id
    );
    Ok(())
}
