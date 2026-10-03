use a4n_domain::generation::GenerationSpecRecord;

use crate::galgame::GalgameGenerationRecordError;

pub(in crate::galgame) fn validate_galgame_portrait_spec_family_context(
    spec: &GenerationSpecRecord,
    root_project_id: i64,
) -> Result<i64, GalgameGenerationRecordError> {
    if spec.root_project_id != root_project_id {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分生成意图项目族上下文与项目不一致: spec_id={}, spec_root_project_id={}, project_root_project_id={}",
            spec.id, spec.root_project_id, root_project_id
        )));
    }
    spec.source_snapshot_id.filter(|id| *id > 0).ok_or_else(|| {
        GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分生成意图缺少有效 source_snapshot_id: spec_id={}",
            spec.id
        ))
    })
}

pub(in crate::galgame) fn validate_galgame_portrait_spec_source_chapter(
    spec: &GenerationSpecRecord,
) -> Result<(), GalgameGenerationRecordError> {
    match (spec.source_chapter_id, spec.source_snapshot_chapter_id) {
        (None, None) => Ok(()),
        _ => Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分生成意图必须使用源快照级上下文，source_chapter_id 和 source_snapshot_chapter_id 必须为空: spec_id={}",
            spec.id
        ))),
    }
}
