use crate::actions::project_transfer::family_package::ProjectFamilyPackage;

use super::super::super::{
    galgame_params::validate_galgame_generation_spec_params,
    targets::{
        required_generation_spec_source_snapshot_id, validate_generation_spec_source_chapter,
        validate_generation_spec_target_entity,
    },
};
use super::context::GenerationReferenceContext;

pub(super) fn validate_generation_specs(
    package: &ProjectFamilyPackage,
    refs: &GenerationReferenceContext<'_>,
) -> Result<(), String> {
    for spec in &package.generation_specs {
        let source_snapshot_id = required_generation_spec_source_snapshot_id(spec)?;
        let snapshot = refs
            .source_snapshots
            .get(&source_snapshot_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包生成意图引用了不存在的源文本快照: generation_spec_id={}, source_snapshot_id={}",
                    spec.id, source_snapshot_id
                )
            })?;
        if spec.source_version_hash != snapshot.source_version_hash {
            return Err(format!(
                "项目族导入包生成意图 source_version_hash 与源快照不一致: generation_spec_id={}, source_snapshot_id={}",
                spec.id, source_snapshot_id
            ));
        }
        validate_generation_spec_source_chapter(spec, source_snapshot_id, &refs.snapshot_chapters)?;
        validate_generation_spec_target_entity(
            spec,
            source_snapshot_id,
            &refs.snapshot_chapters,
            &refs.snapshot_profiles,
            &refs.video_segment_ids,
            &refs.assembly_timeline_ids,
            &refs.galgame_scripts,
            &refs.galgame_scenes,
            &refs.galgame_lines,
        )?;
        validate_galgame_generation_spec_params(
            spec,
            source_snapshot_id,
            &refs.snapshot_chapters,
            &refs.snapshot_profiles,
            &refs.galgame_scripts,
            &refs.galgame_scenes,
            &refs.galgame_lines,
        )?;
    }
    Ok(())
}
