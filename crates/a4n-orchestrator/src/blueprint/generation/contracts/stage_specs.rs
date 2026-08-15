use super::*;

pub const BLUEPRINT_GENERATION_STAGES: [BlueprintStageSpec; 4] = [
    BlueprintStageSpec {
        key: "story_core",
        title: "阶段 1/4：故事核心",
        validate: validate_story_core_stage,
        progress_current: 2,
        progress_message: "生成故事核心与完整梗概",
        restored_message: "已恢复故事核心断点",
        output_schema: r#"{"title":"string","target_audience":"string","genre":"string","style":"string","tone":"string","one_sentence_summary":"string","full_synopsis":"string"}"#,
        expected_top_level_keys: STORY_CORE_STAGE_KEYS,
    },
    BlueprintStageSpec {
        key: "world",
        title: "阶段 2/4：世界观",
        validate: validate_world_stage,
        progress_current: 3,
        progress_message: "生成结构化世界观",
        restored_message: "已恢复世界观断点",
        output_schema: r#"{"world_setting":{"premise":"string","core_rules":"string","technology_or_magic":{"source":"string","rules":["string"],"limits":["string"],"costs":["string"]},"key_locations":[{"name":"string","description":"string","story_function":"string"}],"factions":[{"name":"string","description":"string","agenda":"string","representative_characters":["string"]}],"history_timeline":[{"period":"string","event":"string","impact":"string"}],"culture_and_society":{"values":"string","class_structure":"string","daily_life":"string"},"conflict_sources":[{"name":"string","description":"string","stakes":"string"}],"mysteries_and_foreshadowing":[{"mystery":"string","early_clues":["string"],"payoff_direction":"string"}],"genre_promises":["string"]}}"#,
        expected_top_level_keys: WORLD_STAGE_KEYS,
    },
    BlueprintStageSpec {
        key: "characters",
        title: "阶段 3/4：角色",
        validate: validate_characters_stage,
        progress_current: 4,
        progress_message: "生成主要角色档案",
        restored_message: "已恢复角色档案断点",
        output_schema: r#"{"characters":[{"name":"string","character_type":"protagonist|antagonist|supporting|minor","gender":"string","age":"string","appearance":"string","personality":"string","background":"string","motivation":"string","arc_summary":"string","relationship_to_protagonist":"string","arc_stages":{"start":"string","core_wound":"string","desire":"string","need":"string","secret":"string","voice_style":"string","turning_point":"string","end":"string"}}]}"#,
        expected_top_level_keys: CHARACTERS_STAGE_KEYS,
    },
    BlueprintStageSpec {
        key: "relationships",
        title: "阶段 4/4：关系",
        validate: validate_relationships_stage,
        progress_current: 5,
        progress_message: "生成人物关系网络",
        restored_message: "已恢复人物关系断点",
        output_schema: r#"{"relationships":[{"relationship_type":"family|friend|enemy|lover|rival|mentor|other","character_a":"string","character_b":"string","description":"string","strength":1,"evolution":{"start":"string","tension":"string","turning_points":["string"],"hidden_truth":"string","story_function":"string","end":"string"}}]}"#,
        expected_top_level_keys: RELATIONSHIPS_STAGE_KEYS,
    },
];
