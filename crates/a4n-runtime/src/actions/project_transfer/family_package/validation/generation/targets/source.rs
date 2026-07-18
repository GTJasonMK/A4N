use a4n_core::models::{generation::generation_spec, project::source_snapshot_chapter};
use std::collections::HashMap;

pub(in super::super) fn required_generation_spec_source_snapshot_id(
    spec: &generation_spec::Model,
) -> Result<i64, String> {
    let context_kind = spec
        .source_context_kind
        .as_deref()
        .unwrap_or("snapshot_backed");
    if context_kind != "snapshot_backed" {
        return Err(format!(
            "项目族导入包生成意图只支持 snapshot_backed 上下文: generation_spec_id={}, source_context_kind={context_kind}",
            spec.id
        ));
    }
    spec.source_snapshot_id.ok_or_else(|| {
        format!(
            "项目族导入包生成意图缺少 source_snapshot_id: generation_spec_id={}",
            spec.id
        )
    })
}

pub(in super::super) fn validate_generation_spec_source_chapter(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
) -> Result<(), String> {
    if spec.source_chapter_id.is_some() != spec.source_snapshot_chapter_id.is_some() {
        return Err(format!(
            "项目族导入包生成意图 source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空: generation_spec_id={}",
            spec.id
        ));
    }
    let Some(snapshot_chapter_id) = spec.source_snapshot_chapter_id else {
        return Ok(());
    };
    let snapshot_chapter = snapshot_chapters.get(&snapshot_chapter_id).ok_or_else(|| {
        format!(
            "项目族导入包生成意图引用了不存在的源快照章节: generation_spec_id={}, source_snapshot_chapter_id={snapshot_chapter_id}",
            spec.id
        )
    })?;
    if snapshot_chapter.snapshot_id != source_snapshot_id {
        return Err(format!(
            "项目族导入包生成意图源快照章节不属于目标快照: generation_spec_id={}, source_snapshot_id={}, source_snapshot_chapter_id={snapshot_chapter_id}",
            spec.id, source_snapshot_id
        ));
    }
    if spec.source_chapter_id != Some(snapshot_chapter.source_chapter_id) {
        return Err(format!(
            "项目族导入包生成意图 source_chapter_id 与源快照章节不一致: generation_spec_id={}, source_chapter_id={:?}, expected={}",
            spec.id, spec.source_chapter_id, snapshot_chapter.source_chapter_id
        ));
    }
    Ok(())
}
