use a4n_core::error::AppError;

pub const GALGAME_PIPELINE_TOTAL_STEPS: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalgamePipelineStartStage {
    SceneSegmentation,
    ScriptAdaptation,
    CharacterStaging,
    VisualPlanning,
    BranchDesign,
    PersistScript,
    AssetGeneration,
}

impl GalgamePipelineStartStage {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "scene_segmentation" => Some(Self::SceneSegmentation),
            "script_adaptation" => Some(Self::ScriptAdaptation),
            "character_staging" => Some(Self::CharacterStaging),
            "visual_planning" => Some(Self::VisualPlanning),
            "branch_design" => Some(Self::BranchDesign),
            "persist_script" => Some(Self::PersistScript),
            "asset_generation" => Some(Self::AssetGeneration),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        self.status().0
    }

    pub fn order(self) -> i32 {
        self.status().1
    }

    pub fn message(self) -> &'static str {
        self.status().2
    }

    pub fn checkpoint_keys_to_clear(self) -> &'static [&'static str] {
        match self {
            Self::SceneSegmentation => &[
                "scenes",
                "lines_draft",
                "lines_staged",
                "visual_plan",
                "branch_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::ScriptAdaptation => &[
                "lines_draft",
                "lines_staged",
                "visual_plan",
                "branch_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::CharacterStaging => &[
                "lines_staged",
                "visual_plan",
                "branch_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::VisualPlanning => &[
                "visual_plan",
                "branch_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::BranchDesign => &["branch_plan", "persist_script", "asset_generation"],
            Self::PersistScript => &["persist_script", "asset_generation"],
            Self::AssetGeneration => &["asset_generation"],
        }
    }

    fn status(self) -> (&'static str, i32, &'static str) {
        match self {
            Self::SceneSegmentation => ("scene_segmentation", 1, "正在切分 Galgame 场景"),
            Self::ScriptAdaptation => ("script_adaptation", 2, "正在改编 Galgame 剧本行"),
            Self::CharacterStaging => ("character_staging", 3, "正在绑定角色并标注演出状态"),
            Self::VisualPlanning => ("visual_planning", 4, "正在规划背景与事件 CG"),
            Self::BranchDesign => ("branch_design", 5, "正在设计 Galgame 分支"),
            Self::PersistScript => ("persist_script", 6, "正在写入 Galgame 剧本"),
            Self::AssetGeneration => ("asset_generation", 7, "正在创建 Galgame 资产生成意图"),
        }
    }
}

pub fn parse_galgame_start_stage(
    value: Option<&str>,
) -> Result<Option<GalgamePipelineStartStage>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    GalgamePipelineStartStage::parse(value)
        .map(Some)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "未知 Galgame 管线起始阶段: {value}; 可用阶段: {}",
                galgame_pipeline_stage_keys().join(", ")
            ))
        })
}

pub fn should_restore_galgame_stage_checkpoint(
    resume_from_checkpoint: bool,
    start_from_stage: Option<GalgamePipelineStartStage>,
    stage_order: i32,
) -> bool {
    if let Some(start_stage) = start_from_stage {
        return stage_order < start_stage.order();
    }
    resume_from_checkpoint
}

pub fn galgame_pipeline_stage_keys() -> Vec<&'static str> {
    [
        GalgamePipelineStartStage::SceneSegmentation,
        GalgamePipelineStartStage::ScriptAdaptation,
        GalgamePipelineStartStage::CharacterStaging,
        GalgamePipelineStartStage::VisualPlanning,
        GalgamePipelineStartStage::BranchDesign,
        GalgamePipelineStartStage::PersistScript,
        GalgamePipelineStartStage::AssetGeneration,
    ]
    .into_iter()
    .map(GalgamePipelineStartStage::key)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_galgame_pipeline_start_stages() {
        for key in galgame_pipeline_stage_keys() {
            assert!(
                GalgamePipelineStartStage::parse(key).is_some(),
                "stage should parse: {key}"
            );
        }
    }

    #[test]
    fn rejects_unknown_galgame_pipeline_start_stage() {
        let err = parse_galgame_start_stage(Some("unknown_stage")).expect_err("unknown stage");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("unknown_stage"));
        assert!(err.to_string().contains("scene_segmentation"));
    }

    #[test]
    fn clears_downstream_checkpoints_when_restarting_from_staging() {
        let keys = GalgamePipelineStartStage::CharacterStaging.checkpoint_keys_to_clear();

        assert!(!keys.contains(&"scenes"));
        assert!(!keys.contains(&"lines_draft"));
        assert!(keys.contains(&"lines_staged"));
        assert!(keys.contains(&"visual_plan"));
        assert!(keys.contains(&"asset_generation"));
    }

    #[test]
    fn restores_only_upstream_checkpoints_when_start_stage_is_set() {
        assert!(should_restore_galgame_stage_checkpoint(
            false,
            Some(GalgamePipelineStartStage::VisualPlanning),
            GalgamePipelineStartStage::CharacterStaging.order(),
        ));
        assert!(!should_restore_galgame_stage_checkpoint(
            false,
            Some(GalgamePipelineStartStage::VisualPlanning),
            GalgamePipelineStartStage::VisualPlanning.order(),
        ));
        assert!(should_restore_galgame_stage_checkpoint(true, None, 1));
    }
}
