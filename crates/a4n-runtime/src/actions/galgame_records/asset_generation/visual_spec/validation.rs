use super::super::{
    GALGAME_TARGET_BACKGROUND, GALGAME_TARGET_CG, GalgameTargetContext, generation_spec,
};
use super::json::{
    required_visual_json_object, required_visual_json_str, validate_galgame_visual_canonical_key,
    validate_optional_galgame_cg_importance, validate_visual_json_i64_equals,
};
use super::plan::GalgameVisualSpecPlan;

pub fn validate_galgame_visual_spec_params(
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
    target_context: &GalgameTargetContext,
) -> Result<(), String> {
    if spec.project_id != target_context.project_id
        || spec.root_project_id != target_context.root_project_id
        || spec.source_snapshot_id != Some(target_context.source_snapshot_id)
    {
        return Err(format!(
            "Galgame 视觉生成意图项目族上下文与目标不一致: spec_id={}, spec_project_id={}, spec_root_project_id={}, spec_source_snapshot_id={:?}, target_project_id={}, target_root_project_id={}, target_source_snapshot_id={}",
            spec.id,
            spec.project_id,
            spec.root_project_id,
            spec.source_snapshot_id,
            target_context.project_id,
            target_context.root_project_id,
            target_context.source_snapshot_id
        ));
    }
    if spec.source_chapter_id != Some(target_context.source_chapter_id)
        || spec.source_snapshot_chapter_id != Some(target_context.source_snapshot_chapter_id)
    {
        return Err(format!(
            "Galgame 视觉生成意图源章节上下文与目标不一致: spec_id={}, spec_source_chapter_id={:?}, spec_source_snapshot_chapter_id={:?}, target_source_chapter_id={}, target_source_snapshot_chapter_id={}",
            spec.id,
            spec.source_chapter_id,
            spec.source_snapshot_chapter_id,
            target_context.source_chapter_id,
            target_context.source_snapshot_chapter_id
        ));
    }
    match plan.target_kind {
        GALGAME_TARGET_BACKGROUND => {
            let expected_key = format!(
                "galgame:{}:background:scene:{}",
                spec.project_id, plan.target_entity_id
            );
            validate_galgame_visual_canonical_key(spec, &expected_key)?;
            let scene = required_visual_json_object(&spec.params_json, "scene", spec.id)?;
            validate_visual_json_i64_equals(spec, scene, "id", "scene.id", plan.target_entity_id)?;
            let source = required_visual_json_object(&spec.params_json, "source", spec.id)?;
            validate_visual_json_i64_equals(
                spec,
                source,
                "source_snapshot_chapter_id",
                "source.source_snapshot_chapter_id",
                target_context.source_snapshot_chapter_id,
            )?;
            validate_visual_json_i64_equals(
                spec,
                source,
                "source_chapter_id",
                "source.source_chapter_id",
                target_context.source_chapter_id,
            )?;
        }
        GALGAME_TARGET_CG => {
            let expected_line_id = target_context.line_id.ok_or_else(|| {
                format!(
                    "Galgame CG 生成意图目标上下文缺少行 ID: spec_id={}",
                    spec.id
                )
            })?;
            let expected_key = format!("galgame:{}:cg:line:{expected_line_id}", spec.project_id);
            validate_galgame_visual_canonical_key(spec, &expected_key)?;
            let line = required_visual_json_object(&spec.params_json, "line", spec.id)?;
            validate_visual_json_i64_equals(spec, line, "id", "line.id", expected_line_id)?;
            let scene = required_visual_json_object(&spec.params_json, "scene", spec.id)?;
            validate_visual_json_i64_equals(
                spec,
                scene,
                "id",
                "scene.id",
                target_context.scene_id,
            )?;
            required_visual_json_str(&spec.params_json, "cg_note", "cg_note", spec.id)?;
            validate_optional_galgame_cg_importance(&spec.params_json, spec.id)?;
        }
        other => {
            return Err(format!(
                "不支持的 Galgame 视觉 target_kind: spec_id={}, target_kind={other}",
                spec.id
            ));
        }
    }
    Ok(())
}
