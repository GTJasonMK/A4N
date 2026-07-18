use a4n_core::models::generation::generation_spec;

use crate::galgame::GalgameGenerationRecordError;

pub fn validate_galgame_portrait_spec_family_context(
    spec: &generation_spec::Model,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Result<(), GalgameGenerationRecordError> {
    if spec.root_project_id != root_project_id
        || spec.source_snapshot_id != Some(source_snapshot_id)
    {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分生成意图项目族上下文与项目不一致: spec_id={}, spec_root_project_id={}, spec_source_snapshot_id={:?}, project_root_project_id={}, project_source_snapshot_id={}",
            spec.id,
            spec.root_project_id,
            spec.source_snapshot_id,
            root_project_id,
            source_snapshot_id
        )));
    }
    Ok(())
}

pub fn validate_galgame_portrait_spec_source_chapter(
    spec: &generation_spec::Model,
) -> Result<(), GalgameGenerationRecordError> {
    match (spec.source_chapter_id, spec.source_snapshot_chapter_id) {
        (None, None) => Ok(()),
        _ => Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分生成意图必须使用源快照级上下文，source_chapter_id 和 source_snapshot_chapter_id 必须为空: spec_id={}",
            spec.id
        ))),
    }
}
