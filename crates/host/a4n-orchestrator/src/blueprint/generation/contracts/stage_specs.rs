use crate::blueprint::json::{
    validate_characters_stage, validate_relationships_stage, validate_story_core_stage,
    validate_world_stage,
};

use super::BlueprintStageSpec;
use a4n_domain::project::{
    BLUEPRINT_CHARACTERS_SHAPE, BLUEPRINT_RELATIONSHIPS_SHAPE, BLUEPRINT_STORY_CORE_SHAPE,
    BLUEPRINT_WORLD_SHAPE,
};

pub(in crate::blueprint) const BLUEPRINT_GENERATION_STAGES: [BlueprintStageSpec; 4] = [
    BlueprintStageSpec {
        key: "story_core",
        title: "阶段 1/4：故事核心",
        validate: validate_story_core_stage,
        progress_current: 2,
        progress_message: "生成故事核心与完整梗概",
        restored_message: "已恢复故事核心断点",
        output_schema: BLUEPRINT_STORY_CORE_SHAPE,
    },
    BlueprintStageSpec {
        key: "world",
        title: "阶段 2/4：世界观",
        validate: validate_world_stage,
        progress_current: 3,
        progress_message: "生成结构化世界观",
        restored_message: "已恢复世界观断点",
        output_schema: BLUEPRINT_WORLD_SHAPE,
    },
    BlueprintStageSpec {
        key: "characters",
        title: "阶段 3/4：角色",
        validate: validate_characters_stage,
        progress_current: 4,
        progress_message: "生成主要角色档案",
        restored_message: "已恢复角色档案断点",
        output_schema: BLUEPRINT_CHARACTERS_SHAPE,
    },
    BlueprintStageSpec {
        key: "relationships",
        title: "阶段 4/4：关系",
        validate: validate_relationships_stage,
        progress_current: 5,
        progress_message: "生成人物关系网络",
        restored_message: "已恢复人物关系断点",
        output_schema: BLUEPRINT_RELATIONSHIPS_SHAPE,
    },
];
