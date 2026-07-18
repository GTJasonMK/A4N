use super::visual_spec::{
    json_owned_string, required_visual_json_i64_array, required_visual_json_str,
    required_visual_json_string_array,
};
use super::{
    GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_EXPRESSIONS, GALGAME_TARGET_PORTRAIT_EXPRESSION,
    Value, generation_spec,
};

#[derive(Debug)]
pub(super) struct GalgamePortraitSpecPlan {
    pub(super) profile_id: i64,
    pub(super) expression: String,
    pub(super) pose: Option<String>,
    pub(super) position_samples: Vec<String>,
    pub(super) source_line_ids: Vec<i64>,
}

impl GalgamePortraitSpecPlan {
    pub(super) fn from_spec(spec: &generation_spec::Model) -> Result<Self, String> {
        let profile_id = spec
            .target_entity_id
            .ok_or_else(|| format!("Galgame 立绘差分生成意图缺少 target_entity_id: {}", spec.id))?;
        if profile_id <= 0 {
            return Err(format!(
                "Galgame 立绘差分 target_entity_id 无效: {}",
                spec.id
            ));
        }
        if spec.target_kind != GALGAME_TARGET_PORTRAIT_EXPRESSION
            || spec.target_entity_kind != GALGAME_ENTITY_CHARACTER_PROFILE
        {
            return Err(format!(
                "Galgame 立绘差分只支持 portrait_expression/character_profile spec: spec_id={}, target={}/{}",
                spec.id, spec.target_kind, spec.target_entity_kind
            ));
        }
        if let Some(param_profile_id) = spec
            .params_json
            .get("character_profile_id")
            .and_then(Value::as_i64)
            && param_profile_id != profile_id
        {
            return Err(format!(
                "Galgame 立绘差分参数 character_profile_id 与 target_entity_id 不一致: spec_id={}",
                spec.id
            ));
        }
        let expression = json_owned_string(&spec.params_json, "expression").ok_or_else(|| {
            format!(
                "Galgame 立绘差分生成意图缺少 expression: spec_id={}",
                spec.id
            )
        })?;
        let pose = json_owned_string(&spec.params_json, "pose");
        if !GALGAME_EXPRESSIONS.contains(&expression.as_str()) {
            return Err(format!(
                "Galgame 立绘差分生成意图 expression 无效: spec_id={}, expression={}",
                spec.id, expression
            ));
        }
        let variant_key = match pose.as_deref() {
            Some(pose) => format!("expression:{expression}:pose:{pose}"),
            None => format!("expression:{expression}"),
        };
        let expected_key = format!(
            "galgame:{}:portrait:{profile_id}:{variant_key}",
            spec.project_id
        );
        validate_galgame_portrait_canonical_key(spec, &expected_key)?;
        let source_line_ids =
            required_visual_json_i64_array(&spec.params_json, "source_line_ids", spec.id)?;
        let position_samples =
            required_visual_json_string_array(&spec.params_json, "position_samples", spec.id)?;
        Ok(Self {
            profile_id,
            expression,
            pose,
            position_samples,
            source_line_ids,
        })
    }
}

fn validate_galgame_portrait_canonical_key(
    spec: &generation_spec::Model,
    expected_key: &str,
) -> Result<(), String> {
    let canonical_key =
        required_visual_json_str(&spec.params_json, "canonical_key", "canonical_key", spec.id)?;
    if canonical_key != expected_key {
        return Err(format!(
            "Galgame 立绘差分生成意图 canonical_key 与目标不一致: spec_id={}, canonical_key={}, expected={}",
            spec.id, canonical_key, expected_key
        ));
    }
    Ok(())
}
