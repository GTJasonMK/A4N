use a4n_core::models::{
    galgame::{galgame_scene, galgame_script},
    generation::generation_spec,
    project::source_snapshot_chapter,
};
use std::collections::HashMap;

use crate::project::{
    generation_spec_params_object, required_generation_spec_target_id, required_params_i64,
    required_params_object, validate_generation_spec_canonical_key, validate_params_id_equals,
};

use super::super::targets::validate_generation_spec_galgame_scene_context;

pub(super) fn validate_galgame_background_spec_params(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
) -> Result<(), String> {
    let target_id = required_generation_spec_target_id(spec)?;
    let target_scene = galgame_scenes.get(&target_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 背景参数目标场不存在: generation_spec_id={}, target_entity_id={target_id}",
            spec.id
        )
    })?;
    validate_generation_spec_galgame_scene_context(
        spec,
        source_snapshot_id,
        target_scene,
        galgame_scripts,
    )?;
    let params = generation_spec_params_object(spec)?;
    let expected_key = format!("galgame:{}:background:scene:{target_id}", spec.project_id);
    validate_generation_spec_canonical_key(spec, params, &expected_key)?;
    let scene = required_params_object(params, "scene", spec.id)?;
    validate_params_id_equals(spec, scene, "scene.id", target_id)?;
    let source = required_params_object(params, "source", spec.id)?;
    let source_snapshot_chapter_id =
        required_params_i64(source, "source.source_snapshot_chapter_id", spec.id)?;
    ensure_package!(
        spec.source_snapshot_chapter_id == Some(source_snapshot_chapter_id),
        "项目族导入包 Galgame 背景参数 source_snapshot_chapter_id 与生成意图不一致: generation_spec_id={}, params_source_snapshot_chapter_id={}, spec_source_snapshot_chapter_id={:?}",
        spec.id,
        source_snapshot_chapter_id,
        spec.source_snapshot_chapter_id
    );
    let snapshot_chapter = snapshot_chapters
        .get(&source_snapshot_chapter_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 背景参数 source_snapshot_chapter_id 不存在: generation_spec_id={}, source_snapshot_chapter_id={source_snapshot_chapter_id}",
                spec.id
            )
        })?;
    ensure_package!(
        snapshot_chapter.snapshot_id == source_snapshot_id,
        "项目族导入包 Galgame 背景参数 source_snapshot_chapter_id 不属于源快照: generation_spec_id={}, source_snapshot_id={}, source_snapshot_chapter_id={}",
        spec.id,
        source_snapshot_id,
        source_snapshot_chapter_id
    );
    let source_chapter_id = required_params_i64(source, "source.source_chapter_id", spec.id)?;
    ensure_package!(
        spec.source_chapter_id == Some(source_chapter_id)
            && snapshot_chapter.source_chapter_id == source_chapter_id,
        "项目族导入包 Galgame 背景参数 source_chapter_id 与源快照章节不一致: generation_spec_id={}, params_source_chapter_id={}, spec_source_chapter_id={:?}, expected={}",
        spec.id,
        source_chapter_id,
        spec.source_chapter_id,
        snapshot_chapter.source_chapter_id
    );
    Ok(())
}
