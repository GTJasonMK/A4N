use a4n_db_models::models::{
    galgame::{galgame_scene, galgame_script},
    generation::generation_spec,
};
use std::collections::HashMap;

pub(in super::super) fn validate_generation_spec_galgame_scene_context(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    scene: &galgame_scene::Model,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
) -> Result<(), String> {
    let script = galgame_scripts.get(&scene.script_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包生成意图 Galgame 目标场引用了不存在的剧本: generation_spec_id={}, scene_id={}, script_id={}",
            spec.id, scene.id, scene.script_id
        )
    })?;
    if script.project_id != spec.project_id
        || script.root_project_id != spec.root_project_id
        || script.source_snapshot_id != source_snapshot_id
    {
        return Err(format!(
            "项目族导入包生成意图 Galgame 目标上下文不一致: generation_spec_id={}, target_project_id={}, target_root_project_id={}, target_source_snapshot_id={}, spec_project_id={}, spec_root_project_id={}, spec_source_snapshot_id={}",
            spec.id,
            script.project_id,
            script.root_project_id,
            script.source_snapshot_id,
            spec.project_id,
            spec.root_project_id,
            source_snapshot_id
        ));
    }
    Ok(())
}
