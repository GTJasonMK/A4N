use super::*;
use a4n_core::error::AppError;
use serde_json::json;

fn valid_blueprint() -> String {
    json!({
        "title": "星潮档案",
        "target_audience": "成年幻想读者",
        "genre": "科幻奇幻",
        "style": "冷峻写实",
        "tone": "悬疑",
        "one_sentence_summary": "档案员追查星潮真相并重建失落记忆。",
        "full_synopsis": "主角在星潮灾变后进入档案城，逐步发现记忆交易背后的统治机制，并在同伴与敌手的夹击中选择公开真相。",
        "world_setting": {
            "premise": "星潮灾变后，记忆成为可被封存、审校和买卖的公共资源。",
            "core_rules": "记忆可被封存和交易。",
            "technology_or_magic": {
                "source": "星潮残留物与档案术共同塑造记忆存储技术",
                "rules": ["记忆封存必须绑定原主情绪", "公开记忆会改变城市信用"],
                "limits": ["被污染的记忆无法直接读取"],
                "costs": ["频繁读取会造成自我认知剥离"]
            },
            "key_locations": [{
                "name": "档案城",
                "description": "保存灾变记忆的城市",
                "story_function": "主角调查和权力审查的核心舞台"
            }],
            "factions": [{
                "name": "审校局",
                "description": "管控记忆流通",
                "agenda": "公开维护秩序，暗中垄断灾变真相",
                "representative_characters": ["裴玄"]
            }],
            "history_timeline": [{
                "period": "星潮灾变",
                "event": "大批居民失去关键记忆",
                "impact": "催生档案城与审校制度"
            }],
            "culture_and_society": {
                "values": "稳定高于真实",
                "class_structure": "记忆完整者拥有更高社会信用",
                "daily_life": "居民以记忆凭证兑换工作、住所和医疗"
            },
            "conflict_sources": [{
                "name": "真相公开与秩序维持",
                "description": "公开灾变真相可能瓦解审校局统治",
                "stakes": "失败会让全城继续活在伪造记忆里"
            }],
            "mysteries_and_foreshadowing": [{
                "mystery": "林澈妹妹的失踪是否与第一份星潮档案有关",
                "early_clues": ["妹妹留下空白记忆匣", "裴玄回避旧档案编号"],
                "payoff_direction": "她是审校局隐藏第一批实验的关键证人"
            }],
            "genre_promises": ["记忆悬疑", "制度阴谋", "同伴背叛与救赎"]
        },
        "characters": [
            {
                "name": "林澈",
                "character_type": "protagonist",
                "gender": "男",
                "age": "27",
                "appearance": "黑发灰眼，穿旧档案员制服",
                "personality": "谨慎但执拗",
                "background": "灾变幸存档案员",
                "motivation": "找回妹妹失踪真相",
                "arc_summary": "从旁观记录者成长为公开真相的人",
                "relationship_to_protagonist": "本人",
                "arc_stages": {
                    "start": "逃避真相，只相信档案记录",
                    "core_wound": "妹妹失踪时自己选择遵守流程而非救人",
                    "desire": "找回妹妹并证明自己没有失职",
                    "need": "承认制度本身正在制造伤害",
                    "secret": "他曾亲手封存过妹妹的求救片段",
                    "voice_style": "短句、克制，常用档案术语描述情绪",
                    "turning_point": "发现自己的签名出现在禁档上",
                    "end": "承担公开真相的代价"
                }
            },
            {
                "name": "许灯",
                "character_type": "supporting",
                "gender": "女",
                "age": "25",
                "appearance": "短发，机械义眼",
                "personality": "直率机敏",
                "background": "黑市记忆修补师",
                "motivation": "偿还旧债",
                "arc_summary": "从逐利者转向主动保护他人",
                "relationship_to_protagonist": "盟友",
                "arc_stages": {
                    "start": "只为报酬修补记忆",
                    "core_wound": "曾因救人失败而拒绝长期承诺",
                    "desire": "还清债务后离开档案城",
                    "need": "重新相信自己能保护别人",
                    "secret": "她修补过裴玄删改的证词",
                    "voice_style": "语速快，带黑市俚语，用玩笑掩饰恐惧",
                    "turning_point": "主动留下帮助林澈破解禁档",
                    "end": "成为公开档案的共同守护者"
                }
            },
            {
                "name": "裴玄",
                "character_type": "antagonist",
                "gender": "男",
                "age": "42",
                "appearance": "白发，常戴银框眼镜",
                "personality": "温和而控制欲强",
                "background": "审校局首席",
                "motivation": "维持灾后秩序",
                "arc_summary": "从秩序守护者暴露为记忆垄断者",
                "relationship_to_protagonist": "对手",
                "arc_stages": {
                    "start": "以温和姿态维护审校秩序",
                    "core_wound": "亲历灾变混乱后恐惧失控",
                    "desire": "让所有人相信秩序比真相更重要",
                    "need": "面对自己用谎言延续灾难",
                    "secret": "他保留着星潮源头的完整记忆",
                    "voice_style": "礼貌、缓慢，常把威胁包装成建议",
                    "turning_point": "亲自抹除证人记忆失败",
                    "end": "在真相公开后失去审校局合法性"
                }
            }
        ],
        "relationships": [
            {
                "relationship_type": "rival",
                "character_a": "林澈",
                "character_b": "裴玄",
                "description": "真相公开与秩序维持的核心对立",
                "strength": 8,
                "evolution": {
                    "start": "试探",
                    "tension": "林澈越接近真相，裴玄越需要证明封锁是必要的",
                    "turning_points": ["裴玄归还残缺记忆匣", "林澈发现禁档签名"],
                    "hidden_truth": "裴玄知道林澈妹妹的真实去向",
                    "story_function": "把个人寻亲线推向制度真相线",
                    "end": "决裂"
                }
            }
        ],
        "chapter_outline": [],
        "needs_part_outlines": false,
        "total_chapters": 30,
        "chapters_per_part": 25
    })
    .to_string()
}

#[test]
fn parses_canonical_blueprint_response() {
    let parsed = parse_blueprint_json(&valid_blueprint(), 30).expect("valid blueprint");

    assert_eq!(parsed.title, "星潮档案");
    assert_eq!(parsed.total_chapters, 30);
    assert_eq!(parsed.characters.len(), 3);
    assert_eq!(parsed.relationships[0].relationship_type, "rival");
}

#[test]
fn increment_version_accepts_major_minor_version() {
    assert_eq!(increment_version("1.0").unwrap(), "1.1");
    assert_eq!(increment_version("1.9").unwrap(), "1.10");
    assert_eq!(increment_version("2.0").unwrap(), "2.1");
}

#[test]
fn increment_version_rejects_patch_or_non_numeric_versions() {
    for current in ["1.0.0", "v1.0", "1.x", "-1.0", "1.-1", "1"] {
        let err = increment_version(current).expect_err("invalid version must fail");
        assert!(matches!(err, AppError::InvalidArgument(_)));
    }
}

#[test]
fn rejects_markdown_wrapped_json() {
    let raw = format!("```json\n{}\n```", valid_blueprint());

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_string_number_chapter_count() {
    let raw = valid_blueprint().replace("\"total_chapters\":30", "\"total_chapters\":\"30\"");

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_mismatched_requested_chapter_count() {
    let err = parse_blueprint_json(&valid_blueprint(), 40).expect_err("mismatch");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn rejects_old_character_alias_fields() {
    let raw = valid_blueprint().replace(
        "\"background\":\"灾变幸存档案员\"",
        "\"identity\":\"灾变幸存档案员\"",
    );

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_unknown_top_level_blueprint_field() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    value["legacy_notes"] = json!("旧字段");

    let err = parse_blueprint_json(&value.to_string(), 30)
        .expect_err("unknown top-level fields must fail");

    assert!(err.to_string().contains("未知字段 legacy_notes"));
}

#[test]
fn rejects_unknown_character_field() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    value["characters"][0]["power_tags"] = json!(["记忆"]);

    let err = parse_blueprint_json(&value.to_string(), 30)
        .expect_err("unknown character fields must fail");

    assert!(err.to_string().contains("未知字段 power_tags"));
}

#[test]
fn rejects_old_relationship_alias_fields() {
    let raw = valid_blueprint().replace("\"character_a\":\"林澈\"", "\"character_from\":\"林澈\"");

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_unknown_world_setting_nested_field() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    value["world_setting"]["key_locations"][0]["legacy_function"] = json!("旧字段");

    let err = parse_blueprint_json(&value.to_string(), 30)
        .expect_err("unknown world setting fields must fail");

    assert!(err.to_string().contains("未知字段 legacy_function"));
}

#[test]
fn rejects_thin_world_setting_schema() {
    let raw = valid_blueprint().replace(
        r#""premise":"星潮灾变后，记忆成为可被封存、审校和买卖的公共资源。","#,
        "",
    );

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_character_without_rich_arc_stages() {
    let raw =
        valid_blueprint().replace(r#""core_wound":"妹妹失踪时自己选择遵守流程而非救人","#, "");

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_character_without_canonical_identity_fields() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    let character = value["characters"][1].as_object_mut().unwrap();
    character.remove("gender");
    character.remove("age");
    let raw = value.to_string();

    let err = parse_blueprint_json(&raw, 30).expect_err("missing gender and age must fail");

    assert!(err.to_string().contains("gender"));
}

#[test]
fn rejects_relationship_without_dynamic_evolution() {
    let raw = valid_blueprint().replace(r#""story_function":"把个人寻亲线推向制度真相线","#, "");

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_relationship_without_strength() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    value["relationships"][0]
        .as_object_mut()
        .unwrap()
        .remove("strength");
    let raw = value.to_string();

    let err = parse_blueprint_json(&raw, 30).expect_err("missing strength must fail");

    assert!(err.to_string().contains("strength"));
}

#[test]
fn parse_blueprint_optimization_json_accepts_applicable_fields() {
    let value = parse_blueprint_optimization_json(
        r#"{"style":"冷峻写实","full_synopsis":"主角在压力中完成成长。"}"#,
    )
    .expect("valid optimization json");

    assert_eq!(value.style.as_deref(), Some("冷峻写实"));
    assert_eq!(
        value.full_synopsis.as_deref(),
        Some("主角在压力中完成成长。")
    );
}

#[test]
fn parse_blueprint_optimization_json_rejects_plain_text() {
    let err = parse_blueprint_optimization_json("整体结构不错，但建议加强冲突。")
        .expect_err("plain text should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn parse_blueprint_optimization_json_rejects_empty_applicable_fields() {
    let err = parse_blueprint_optimization_json(r#"{"style":" ","full_synopsis":null}"#)
        .expect_err("empty fields should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn parse_blueprint_optimization_json_rejects_unknown_field() {
    let err = parse_blueprint_optimization_json(
        r#"{"style":"冷峻写实","legacy_notes":"不要进入正式蓝图"}"#,
    )
    .expect_err("unknown optimization fields must fail");

    assert!(err.to_string().contains("未知字段 legacy_notes"));
}
