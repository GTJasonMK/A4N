pub const GALGAME_PIPELINE_TOTAL_STEPS: u32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalgamePipelineStartStage {
    ShootingScriptProjection,
    BranchDesign,
    ScriptAdaptation,
    CharacterStaging,
    VisualPlanning,
    PersistScript,
    AssetGeneration,
}

impl GalgamePipelineStartStage {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "shooting_script_projection" => Some(Self::ShootingScriptProjection),
            "branch_design" => Some(Self::BranchDesign),
            "script_adaptation" => Some(Self::ScriptAdaptation),
            "character_staging" => Some(Self::CharacterStaging),
            "visual_planning" => Some(Self::VisualPlanning),
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
            Self::ShootingScriptProjection => &[
                "scenes",
                "branch_plan",
                "lines_draft",
                "lines_staged",
                "visual_plan",
                "branch_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::BranchDesign => &[
                "branch_plan",
                "lines_draft",
                "lines_staged",
                "visual_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::ScriptAdaptation => &[
                "lines_draft",
                "lines_staged",
                "visual_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::CharacterStaging => &[
                "lines_staged",
                "visual_plan",
                "persist_script",
                "asset_generation",
            ],
            Self::VisualPlanning => &["visual_plan", "persist_script", "asset_generation"],
            Self::PersistScript => &["persist_script", "asset_generation"],
            Self::AssetGeneration => &["asset_generation"],
        }
    }

    fn status(self) -> (&'static str, i32, &'static str) {
        match self {
            Self::ShootingScriptProjection => {
                ("shooting_script_projection", 1, "正在投影结构化拍摄剧本")
            }
            Self::BranchDesign => ("branch_design", 2, "正在基于导演节拍设计 Galgame 分支"),
            Self::ScriptAdaptation => ("script_adaptation", 3, "正在改编 Galgame 主线与支线剧本行"),
            Self::CharacterStaging => ("character_staging", 4, "正在绑定角色并标注演出状态"),
            Self::VisualPlanning => ("visual_planning", 5, "正在规划背景与事件 CG"),
            Self::PersistScript => ("persist_script", 6, "正在写入 Galgame 剧本"),
            Self::AssetGeneration => ("asset_generation", 7, "正在创建 Galgame 资产生成意图"),
        }
    }
}

pub fn galgame_pipeline_stage_keys() -> Vec<&'static str> {
    [
        GalgamePipelineStartStage::ShootingScriptProjection,
        GalgamePipelineStartStage::BranchDesign,
        GalgamePipelineStartStage::ScriptAdaptation,
        GalgamePipelineStartStage::CharacterStaging,
        GalgamePipelineStartStage::VisualPlanning,
        GalgamePipelineStartStage::PersistScript,
        GalgamePipelineStartStage::AssetGeneration,
    ]
    .into_iter()
    .map(GalgamePipelineStartStage::key)
    .collect()
}
