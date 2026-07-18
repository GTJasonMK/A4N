use super::super::{
    GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE, GALGAME_PROMPT_KIND_BACKGROUND,
    GALGAME_PROMPT_KIND_EVENT_CG, GALGAME_TARGET_BACKGROUND, GALGAME_TARGET_CG,
    MEDIA_BINDING_ROLE_BACKGROUND, MEDIA_BINDING_ROLE_CG, generation_spec,
};

pub struct GalgameVisualSpecPlan {
    pub(in super::super) target_entity_id: i64,
    pub(in super::super) target_kind: &'static str,
    pub(in super::super) target_entity_kind: &'static str,
    pub(in super::super) role: &'static str,
    pub(in super::super) prompt_kind: &'static str,
    pub(in super::super) image_type: &'static str,
}

impl GalgameVisualSpecPlan {
    pub(in super::super) fn from_spec(spec: &generation_spec::Model) -> Result<Self, String> {
        let target_entity_id = spec
            .target_entity_id
            .ok_or_else(|| format!("Galgame 视觉生成意图缺少 target_entity_id: {}", spec.id))?;
        if target_entity_id <= 0 {
            return Err(format!(
                "Galgame 视觉生成意图 target_entity_id 无效: {}",
                spec.id
            ));
        }
        match (spec.target_kind.as_str(), spec.target_entity_kind.as_str()) {
            (GALGAME_TARGET_BACKGROUND, GALGAME_ENTITY_SCENE) => Ok(Self {
                target_entity_id,
                target_kind: GALGAME_TARGET_BACKGROUND,
                target_entity_kind: GALGAME_ENTITY_SCENE,
                role: MEDIA_BINDING_ROLE_BACKGROUND,
                prompt_kind: GALGAME_PROMPT_KIND_BACKGROUND,
                image_type: "galgame_background",
            }),
            (GALGAME_TARGET_CG, GALGAME_ENTITY_LINE) => Ok(Self {
                target_entity_id,
                target_kind: GALGAME_TARGET_CG,
                target_entity_kind: GALGAME_ENTITY_LINE,
                role: MEDIA_BINDING_ROLE_CG,
                prompt_kind: GALGAME_PROMPT_KIND_EVENT_CG,
                image_type: "galgame_cg",
            }),
            _ => Err(format!(
                "Galgame 视觉生成只支持背景/CG spec: spec_id={}, target={}/{}",
                spec.id, spec.target_kind, spec.target_entity_kind
            )),
        }
    }
}
