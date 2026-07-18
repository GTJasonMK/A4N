use super::visual_spec::{
    required_visual_json_object, required_visual_json_str, validate_optional_galgame_cg_importance,
};
use super::*;
use std::collections::HashMap;

pub(super) async fn upsert_galgame_prompt_artifact(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<prompt_artifact::Model, String> {
    let existing = state
        .orchestrator
        .galgame_prompt_artifact_by_generation_spec(spec.id)
        .await
        .map_err(crate::actions::app_error_message)?;
    if let Some(existing) = existing {
        if existing.edited_by_user {
            return Ok(existing);
        }
    }
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let prompt_text = render_galgame_image_prompt(&prompt_renderer, spec, plan).await?;
    state
        .orchestrator
        .upsert_galgame_prompt_artifact_record(GalgamePromptArtifactUpsertInput {
            generation_spec_id: spec.id,
            prompt_kind: plan.prompt_kind.to_string(),
            prompt_text,
            renderer_version: GALGAME_ASSET_RENDERER_VERSION.to_string(),
        })
        .await
        .map_err(crate::actions::app_error_message)
}

pub(super) async fn render_galgame_image_prompt(
    prompt_renderer: &dyn PromptRenderer,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<String, String> {
    let (template_name, variables) = galgame_image_prompt_variables(spec, plan)?;
    let rendered = prompt_renderer
        .render_prompt(template_name, &variables, Some(spec.project_id))
        .await
        .map_err(|err| err.to_string())?
        .trim()
        .to_string();
    if rendered.is_empty() {
        return Err(format!(
            "Galgame 图像提示词模板渲染为空: spec_id={}, template={template_name}",
            spec.id
        ));
    }
    Ok(rendered)
}

pub(super) fn galgame_image_prompt_variables(
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<PromptVariables, String> {
    match plan.target_kind {
        GALGAME_TARGET_BACKGROUND => galgame_background_prompt_variables(spec),
        GALGAME_TARGET_CG => galgame_event_cg_prompt_variables(spec),
        _ => Err(format!(
            "不支持的 Galgame 视觉 target_kind: {}",
            plan.target_kind
        )),
    }
}

pub(super) fn galgame_background_prompt_variables(
    spec: &generation_spec::Model,
) -> Result<PromptVariables, String> {
    let scene = required_visual_json_object(&spec.params_json, "scene", spec.id)?;
    required_visual_json_str(scene, "title", "scene.title", spec.id)?;

    let mut variables = HashMap::new();
    variables.insert("params".to_string(), spec.params_json.clone());
    variables.insert("scene".to_string(), scene.clone());
    variables.insert(
        "source".to_string(),
        spec.params_json
            .get("source")
            .cloned()
            .unwrap_or(Value::Null),
    );
    variables.insert(
        "canonical_key".to_string(),
        spec.params_json
            .get("canonical_key")
            .cloned()
            .unwrap_or(Value::Null),
    );
    Ok((GALGAME_BACKGROUND_IMAGE_PROMPT_TEMPLATE, variables))
}

pub(super) fn galgame_event_cg_prompt_variables(
    spec: &generation_spec::Model,
) -> Result<PromptVariables, String> {
    let scene = required_visual_json_object(&spec.params_json, "scene", spec.id)?;
    required_visual_json_str(scene, "title", "scene.title", spec.id)?;
    let line = required_visual_json_object(&spec.params_json, "line", spec.id)?;
    required_visual_json_str(line, "text", "line.text", spec.id)?;
    let cg_note = required_visual_json_str(&spec.params_json, "cg_note", "cg_note", spec.id)?;
    validate_optional_galgame_cg_importance(&spec.params_json, spec.id)?;

    let mut variables = HashMap::new();
    variables.insert("params".to_string(), spec.params_json.clone());
    variables.insert("scene".to_string(), scene.clone());
    variables.insert("line".to_string(), line.clone());
    variables.insert("cg_note".to_string(), Value::String(cg_note.to_string()));
    variables.insert(
        "cg_importance".to_string(),
        spec.params_json
            .get("cg_importance")
            .cloned()
            .unwrap_or(Value::Null),
    );
    variables.insert(
        "canonical_key".to_string(),
        spec.params_json
            .get("canonical_key")
            .cloned()
            .unwrap_or(Value::Null),
    );
    Ok((GALGAME_EVENT_CG_PROMPT_TEMPLATE, variables))
}

type PromptVariables = (&'static str, HashMap<String, Value>);
