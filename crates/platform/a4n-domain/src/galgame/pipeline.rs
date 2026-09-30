use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgamePipelineStartStage {
    AdaptationPlan,
    BranchDesign,
    ScriptAdaptation,
    CharacterStaging,
    VisualPlanning,
    PersistScript,
    AssetGeneration,
}

impl GalgamePipelineStartStage {
    pub const ALL: [Self; 7] = [
        Self::AdaptationPlan,
        Self::BranchDesign,
        Self::ScriptAdaptation,
        Self::CharacterStaging,
        Self::VisualPlanning,
        Self::PersistScript,
        Self::AssetGeneration,
    ];

    pub const fn key(self) -> &'static str {
        self.status().0
    }

    pub const fn order(self) -> i32 {
        self.status().1
    }

    pub const fn message(self) -> &'static str {
        self.status().2
    }

    pub const fn checkpoint_keys_to_clear(self) -> &'static [&'static str] {
        match self {
            Self::AdaptationPlan => &[
                "scenes",
                "branch_plan",
                "lines_draft",
                "lines_staged",
                "visual_plan",
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

    const fn status(self) -> (&'static str, i32, &'static str) {
        match self {
            Self::AdaptationPlan => ("adaptation_plan", 1, "正在准备本项目的改编计划"),
            Self::BranchDesign => ("branch_design", 2, "正在基于导演节拍设计 Galgame 分支"),
            Self::ScriptAdaptation => ("script_adaptation", 3, "正在改编 Galgame 主线与支线剧本行"),
            Self::CharacterStaging => ("character_staging", 4, "正在绑定角色并标注演出状态"),
            Self::VisualPlanning => ("visual_planning", 5, "正在规划背景与事件 CG"),
            Self::PersistScript => ("persist_script", 6, "正在写入 Galgame 剧本"),
            Self::AssetGeneration => ("asset_generation", 7, "正在创建 Galgame 资产生成意图"),
        }
    }
}
