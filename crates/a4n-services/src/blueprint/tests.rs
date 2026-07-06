use super::generation::{BlueprintStageRequestInput, build_blueprint_stage_request};
use super::generation_contract::{
    assemble_staged_blueprint_json, blueprint_refine_manifest_user_message, parse_stage_json_object,
};
use super::json::parse_blueprint_json;
use super::*;
use a4n_core::models::character::protagonist_profile;
use a4n_engine::prompt::engine::PromptEngine;
use a4n_engine::prompt::tera_engine::TeraPromptEngine;
use chrono::NaiveDate;
use serde_json::json;
use std::path::Path;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn fixed_time(day: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 5, day)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

fn prompts_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts")
}

fn prompt_engine() -> TeraPromptEngine {
    TeraPromptEngine::from_directory(&prompts_dir()).expect("prompts should load")
}

#[test]
fn blueprint_llm_context_contains_story_fields_without_database_metadata() {
    let now = fixed_time(1);
    let blueprint = novel_blueprint::Model {
        id: 10,
        project_id: 20,
        title: "星潮档案".to_string(),
        genre: "科幻".to_string(),
        style: Some("冷峻".to_string()),
        tone: Some("悬疑".to_string()),
        target_audience: Some("成人".to_string()),
        one_sentence_summary: Some("调查星潮真相".to_string()),
        full_synopsis: Some("完整主线".to_string()),
        world_setting: Some(json!({"premise": "星潮复苏"})),
        current_version: "1.0.0".to_string(),
        needs_part_outlines: true,
        total_chapters: Some(100),
        chapters_per_part: Some(25),
        avatar_svg: Some("<svg />".to_string()),
        avatar_animal: Some("crest".to_string()),
        created_at: now,
        updated_at: now,
    };
    let character = blueprint_character::Model {
        id: 11,
        project_id: 20,
        name: "林烬".to_string(),
        character_type: "protagonist".to_string(),
        gender: Some("男".to_string()),
        age: Some("二十岁".to_string()),
        appearance: Some("灰色制服".to_string()),
        personality: Some("克制".to_string()),
        background: Some("调查员".to_string()),
        motivation: Some("查明真相".to_string()),
        arc_summary: Some("从逃避到承担".to_string()),
        relationship_to_protagonist: Some("本人".to_string()),
        arc_stages: Some(json!({"start": "逃避"})),
        created_at: now,
        updated_at: now,
    };
    let relationship = blueprint_relationship::Model {
        id: 12,
        project_id: 20,
        relationship_type: "rival".to_string(),
        character_a: "林烬".to_string(),
        character_b: "沈澜".to_string(),
        description: Some("互相试探".to_string()),
        strength: Some(7),
        evolution: Some(json!({"start": "敌意"})),
        created_at: now,
    };

    let context = blueprint_llm_context(&blueprint, &[character], &[relationship]);
    let text = serde_json::to_string(&context).expect("context should serialize");

    assert_eq!(context["blueprint"]["title"], json!("星潮档案"));
    assert_eq!(context["characters"][0]["motivation"], json!("查明真相"));
    assert_eq!(context["relationships"][0]["strength"], json!(7));
    assert!(!text.contains("\"project_id\""));
    assert!(!text.contains("\"created_at\""));
    assert!(!text.contains("\"updated_at\""));
    assert!(!text.contains("\"avatar_svg\""));
}

#[tokio::test]
async fn sync_blueprint_characters_preserves_existing_identity() {
    let db = setup_db().await;
    let project_id = 7301;
    let old_time = fixed_time(1);
    let now = fixed_time(2);
    let kept = blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("林烬".to_string()),
        character_type: Set("protagonist".to_string()),
        personality: Set(Some("旧性格".to_string())),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let stale = blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("旧人".to_string()),
        character_type: Set("supporting".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let stats = sync_blueprint_characters(
        &db,
        project_id,
        vec![
            BlueprintCharacterDraft {
                name: "林烬".to_string(),
                character_type: "protagonist".to_string(),
                gender: None,
                age: None,
                appearance: None,
                personality: Some("新性格".to_string()),
                background: None,
                motivation: None,
                arc_summary: None,
                relationship_to_protagonist: None,
                arc_stages: None,
            },
            BlueprintCharacterDraft {
                name: "沈白".to_string(),
                character_type: "antagonist".to_string(),
                gender: None,
                age: None,
                appearance: None,
                personality: Some("冷静".to_string()),
                background: None,
                motivation: None,
                arc_summary: None,
                relationship_to_protagonist: None,
                arc_stages: None,
            },
        ],
        now,
    )
    .await
    .unwrap();

    assert_eq!(
        stats,
        BlueprintIncrementalSyncStats {
            inserted: 1,
            updated: 1,
            unchanged: 0,
            deleted: 1,
        }
    );
    let rows = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let updated = rows.iter().find(|row| row.name == "林烬").unwrap();
    assert_eq!(updated.id, kept.id);
    assert_eq!(updated.created_at, old_time);
    assert_eq!(updated.updated_at, now);
    assert_eq!(updated.personality.as_deref(), Some("新性格"));
    assert!(!rows.iter().any(|row| row.id == stale.id));
}

#[tokio::test]
async fn sync_blueprint_relationships_preserves_unchanged_identity() {
    let db = setup_db().await;
    let project_id = 7302;
    let old_time = fixed_time(3);
    let now = fixed_time(4);
    let kept = blueprint_relationship::ActiveModel {
        project_id: Set(project_id),
        relationship_type: Set("friend".to_string()),
        character_a: Set("甲".to_string()),
        character_b: Set("乙".to_string()),
        description: Set(Some("同盟".to_string())),
        strength: Set(Some(7)),
        evolution: Set(Some(json!({"start": "同盟"}))),
        created_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let stale = blueprint_relationship::ActiveModel {
        project_id: Set(project_id),
        relationship_type: Set("enemy".to_string()),
        character_a: Set("旧甲".to_string()),
        character_b: Set("旧乙".to_string()),
        created_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let stats = sync_blueprint_relationships(
        &db,
        project_id,
        vec![
            BlueprintRelationshipDraft {
                relationship_type: "friend".to_string(),
                character_a: "甲".to_string(),
                character_b: "乙".to_string(),
                description: Some("同盟".to_string()),
                strength: Some(7),
                evolution: Some(json!({"start": "同盟"})),
            },
            BlueprintRelationshipDraft {
                relationship_type: "rival".to_string(),
                character_a: "甲".to_string(),
                character_b: "丙".to_string(),
                description: Some("竞争".to_string()),
                strength: Some(5),
                evolution: None,
            },
        ],
        now,
    )
    .await
    .unwrap();

    assert_eq!(
        stats,
        BlueprintIncrementalSyncStats {
            inserted: 1,
            updated: 0,
            unchanged: 1,
            deleted: 1,
        }
    );
    let rows = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_relationship::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let unchanged = rows
        .iter()
        .find(|row| row.relationship_type == "friend")
        .unwrap();
    assert_eq!(unchanged.id, kept.id);
    assert_eq!(unchanged.created_at, old_time);
    assert!(!rows.iter().any(|row| row.id == stale.id));
}

#[tokio::test]
async fn sync_character_profiles_from_blueprint_creates_initial_profiles_and_relationships() {
    let db = setup_db().await;
    let project_id = 7303;
    let now = fixed_time(5);
    blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("林烬".to_string()),
        character_type: Set("protagonist".to_string()),
        gender: Set(Some("男".to_string())),
        age: Set(Some("二十岁".to_string())),
        appearance: Set(Some("灰色制服".to_string())),
        personality: Set(Some("克制".to_string())),
        background: Set(Some("调查员".to_string())),
        motivation: Set(Some("查明星潮真相".to_string())),
        arc_summary: Set(Some("从逃避到承担".to_string())),
        relationship_to_protagonist: Set(Some("本人".to_string())),
        arc_stages: Set(Some(json!({"start": "逃避", "end": "承担"}))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("沈澜".to_string()),
        character_type: Set("supporting".to_string()),
        gender: Set(Some("女".to_string())),
        personality: Set(Some("敏锐".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    blueprint_relationship::ActiveModel {
        project_id: Set(project_id),
        relationship_type: Set("rival".to_string()),
        character_a: Set("林烬".to_string()),
        character_b: Set("沈澜".to_string()),
        description: Set(Some("互相试探".to_string())),
        strength: Set(Some(7)),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    sync_character_profiles_from_blueprint(&db, project_id, now)
        .await
        .unwrap();

    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(profiles.len(), 2);
    let lin = profiles
        .iter()
        .find(|profile| profile.name == "林烬")
        .unwrap();
    assert_eq!(lin.gender.as_deref(), Some("男"));
    assert_eq!(lin.appearance.as_deref(), Some("灰色制服"));
    assert_eq!(lin.personality.as_deref(), Some("克制"));
    assert_eq!(lin.last_synced_chapter, 0);
    assert_eq!(
        lin.current_state["explicit_attributes"]["motivation"],
        json!("查明星潮真相")
    );
    assert_eq!(
        lin.current_state["social_attributes"]["沈澜"],
        json!("rival：互相试探 强度 7/10")
    );
    assert_eq!(
        lin.character_arc.as_ref().unwrap()["summary"],
        json!("从逃避到承担")
    );

    sync_character_profiles_from_blueprint(&db, project_id, now)
        .await
        .unwrap();
    let count_after_second_sync = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count_after_second_sync, 2);
}

#[tokio::test]
async fn sync_character_profiles_from_blueprint_supplements_synced_profiles_without_overwrite() {
    let db = setup_db().await;
    let project_id = 7304;
    let now = fixed_time(6);
    blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("林烬".to_string()),
        character_type: Set("protagonist".to_string()),
        gender: Set(Some("男".to_string())),
        motivation: Set(Some("查明星潮真相".to_string())),
        personality: Set(Some("克制".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let synced_state = json!({
        "explicit_attributes": {"gender": "女", "appearance": "章节中的伪装"},
        "implicit_attributes": {"personality": "已经变得果断"},
        "social_attributes": {"沈澜": "正式同盟"}
    });
    protagonist_profile::ActiveModel {
        project_id: Set(project_id),
        name: Set("林烬".to_string()),
        gender: Set(Some("女".to_string())),
        age: Set(None),
        base_attributes: Set(synced_state.clone()),
        current_state: Set(synced_state.clone()),
        character_arc: Set(Some(json!({"summary": "正文弧光"}))),
        voice_fingerprint: Set(None),
        appearance: Set(Some("章节中的伪装".to_string())),
        background: Set(None),
        personality: Set(Some("已经变得果断".to_string())),
        last_synced_chapter: Set(3),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    sync_character_profiles_from_blueprint(&db, project_id, now)
        .await
        .unwrap();

    let profile = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq("林烬"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(profile.gender.as_deref(), Some("女"));
    assert_eq!(
        profile.current_state["social_attributes"]["沈澜"],
        json!("正式同盟")
    );
    assert_eq!(
        profile.current_state["explicit_attributes"]["character_type"],
        json!("protagonist")
    );
    assert_eq!(
        profile.current_state["explicit_attributes"]["motivation"],
        json!("查明星潮真相")
    );
    assert_eq!(
        profile.character_arc.as_ref().unwrap()["summary"],
        json!("正文弧光")
    );
}

#[test]
fn blueprint_generation_status_never_rolls_project_back() {
    assert_eq!(
        project_status_after_blueprint_generation(ProjectStatus::Draft),
        ProjectStatus::BlueprintReady
    );
    assert_eq!(
        project_status_after_blueprint_generation(ProjectStatus::BlueprintReady),
        ProjectStatus::BlueprintReady
    );
    assert_eq!(
        project_status_after_blueprint_generation(ProjectStatus::Writing),
        ProjectStatus::Writing
    );
    assert_eq!(
        project_status_after_blueprint_generation(ProjectStatus::Completed),
        ProjectStatus::Completed
    );
}

#[test]
fn assemble_staged_blueprint_json_builds_canonical_blueprint() {
    let core_output = json!({
        "title": "星潮档案",
        "target_audience": "成年幻想读者",
        "genre": "科幻奇幻",
        "style": "冷峻写实",
        "tone": "悬疑",
        "one_sentence_summary": "档案员追查星潮真相并重建失落记忆。",
        "full_synopsis": "主角在星潮灾变后进入档案城，逐步发现记忆交易背后的统治机制，并在同伴与敌手的夹击中选择公开真相。"
    });
    let world_output = json!({
        "world_setting": {
            "premise": "星潮灾变后，记忆成为可被封存、审校和买卖的公共资源。",
            "core_rules": "记忆可被封存和交易。",
            "technology_or_magic": {
                "source": "星潮残留物与档案术共同塑造记忆存储技术",
                "rules": ["记忆封存必须绑定原主情绪"],
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
                "values": "秩序高于个人记忆",
                "class_structure": "审校局、档案员和普通居民形成等级",
                "daily_life": "居民用记忆信用换取服务"
            },
            "conflict_sources": [{
                "name": "真相公开",
                "description": "公开灾变真相会动摇城市秩序",
                "stakes": "主角必须在稳定和自由之间选择"
            }],
            "mysteries_and_foreshadowing": [{
                "mystery": "星潮是否由审校局制造",
                "early_clues": ["旧档案编号被抹除"],
                "payoff_direction": "揭示灾变背后的权力交易"
            }],
            "genre_promises": ["记忆谜案", "制度反转"]
        }
    });
    let characters_output = json!({
        "characters": [
            {
                "name": "林烬",
                "character_type": "protagonist",
                "gender": "男",
                "age": "28",
                "appearance": "常穿灰色档案员制服",
                "personality": "克制、敏锐、抗拒亲密",
                "background": "灾变幸存者，失去童年关键记忆",
                "motivation": "找回自己的记忆并查明星潮真相",
                "arc_summary": "从维护秩序的档案员转为公开真相的行动者",
                "relationship_to_protagonist": "本人",
                "arc_stages": {
                    "start": "相信审校制度能保护城市",
                    "core_wound": "害怕找回记忆后失去现在的自我",
                    "desire": "恢复个人记忆",
                    "need": "承认真相比稳定更重要",
                    "secret": "他的记忆里藏有灾变源头",
                    "voice_style": "短句、审慎、常用档案术语",
                    "turning_point": "发现自己的档案被审校局篡改",
                    "end": "主动公开记忆并承担后果"
                }
            },
            {
                "name": "沈白",
                "character_type": "antagonist",
                "gender": "女",
                "age": "41",
                "appearance": "银发，佩戴审校局徽章",
                "personality": "冷静、强势、相信必要牺牲",
                "background": "审校局现任负责人",
                "motivation": "维持档案城秩序",
                "arc_summary": "逐步暴露她守护秩序背后的罪责",
                "relationship_to_protagonist": "调查对象与压迫者",
                "arc_stages": {
                    "start": "以城市守护者身份出现",
                    "core_wound": "曾因真相公开失去亲人",
                    "desire": "永久控制危险记忆",
                    "need": "承认被压制者也有选择权",
                    "secret": "参与掩盖星潮起因",
                    "voice_style": "精准、冷淡、权威",
                    "turning_point": "发现林烬掌握不可销毁的原始档案",
                    "end": "被迫面对制度崩塌"
                }
            },
            {
                "name": "周弦",
                "character_type": "supporting",
                "gender": "女",
                "age": "25",
                "appearance": "背着旧式录音匣",
                "personality": "机敏、热情、敢冒险",
                "background": "黑市记忆修复师",
                "motivation": "证明普通人能夺回自己的历史",
                "arc_summary": "从旁观交易者成长为反抗网络组织者",
                "relationship_to_protagonist": "盟友",
                "arc_stages": {
                    "start": "只相信交易和自保",
                    "core_wound": "亲人记忆被审校局没收",
                    "desire": "买回亲人的记忆",
                    "need": "建立真正的信任关系",
                    "secret": "她能修复被污染的原始档案",
                    "voice_style": "快节奏、带黑市俚语",
                    "turning_point": "救下林烬并公开加入调查",
                    "end": "成为新档案公约的推动者"
                }
            }
        ]
    });
    let relationships_output = json!({
        "relationships": [{
            "relationship_type": "rival",
            "character_a": "林烬",
            "character_b": "沈白",
            "description": "林烬追查真相，沈白代表制度阻拦他。",
            "strength": 8,
            "evolution": {
                "start": "上下级式的审查与服从",
                "tension": "林烬发现档案篡改后开始对抗沈白",
                "turning_points": ["沈白销毁周弦的修复档案", "林烬公开原始记忆"],
                "hidden_truth": "沈白曾保护过幼年的林烬",
                "story_function": "体现秩序与真相的核心冲突",
                "end": "沈白失败但保留复杂动机"
            }
        }]
    });

    let assembled = assemble_staged_blueprint_json(
        "星潮档案",
        "科幻奇幻",
        60,
        &core_output,
        &world_output,
        &characters_output,
        &relationships_output,
    );
    let parsed = parse_blueprint_json(&assembled.to_string(), 60).expect("assembled blueprint");

    assert_eq!(parsed.title, "星潮档案");
    assert!(parsed.needs_part_outlines);
    assert_eq!(parsed.chapters_per_part, CHAPTERS_PER_PART);
    assert_eq!(parsed.characters.len(), 3);
    assert_eq!(parsed.relationships.len(), 1);
}

#[test]
fn parse_stage_json_object_rejects_non_object() {
    let err = parse_stage_json_object("[]", "阶段测试", &["result"]).expect_err("non-object");

    assert!(err.to_string().contains("必须是 JSON 对象"));
}

#[test]
fn parse_stage_json_object_rejects_unexpected_top_level_key() {
    let err = parse_stage_json_object(r#"{"result":{},"extra":true}"#, "阶段测试", &["result"])
        .expect_err("unexpected key");

    assert!(err.to_string().contains("未声明顶层字段 extra"));
}

#[test]
fn blueprint_stage_request_moves_source_context_into_manifest() {
    let engine = prompt_engine();
    let previous_context = json!({"story_core":{"title":"星潮档案"}});
    let request = build_blueprint_stage_request(BlueprintStageRequestInput {
        prompt_engine: &engine,
        project_id: 42,
        base_prompt: "基础蓝图规则",
        user_payload_json:
            r#"{"novel_title":"星潮档案","genre":"科幻","requested_chapter_count":80}"#,
        stage_key: "world",
        stage_name: "阶段 2/4：世界观",
        output_schema: r#"{"world_setting":{"premise":"string"}}"#,
        previous_context: Some(&previous_context),
    })
    .expect("stage request should build");

    let manifest = request
        .context_manifest_json
        .as_ref()
        .expect("context manifest should be attached");
    let pack_kinds = manifest["packs"]
        .as_array()
        .expect("packs should be array")
        .iter()
        .filter_map(|pack| pack["pack_kind"].as_str())
        .collect::<Vec<_>>();
    assert!(pack_kinds.contains(&"blueprint_project_seed_context"));
    assert!(pack_kinds.contains(&"blueprint_previous_stage_outputs"));

    let user_message = request
        .messages
        .iter()
        .find(|message| matches!(message.role, a4n_engine::llm::provider::MessageRole::User))
        .expect("user message should exist");
    assert!(user_message.content.contains("context_refs"));
    assert!(
        user_message
            .content
            .contains("blueprint_project_seed_context")
    );
    assert!(user_message.content.contains("每个 factions[] 必须包含"));
    assert!(user_message.content.contains("representative_characters"));
    assert!(!user_message.content.contains("星潮档案"));
    assert!(!user_message.content.contains(r#""story_core""#));
}

#[test]
fn blueprint_stage_request_rejects_invalid_schema() {
    let engine = prompt_engine();
    let err = build_blueprint_stage_request(BlueprintStageRequestInput {
        prompt_engine: &engine,
        project_id: 42,
        base_prompt: "基础蓝图规则",
        user_payload_json: r#"{"novel_title":"星潮档案"}"#,
        stage_key: "story_core",
        stage_name: "阶段测试",
        output_schema: "not-json",
        previous_context: None,
    })
    .expect_err("stage schema must be JSON");

    assert!(err.to_string().contains("输出 schema 不是合法 JSON"));
}

#[test]
fn blueprint_refine_manifest_user_message_uses_context_refs_and_omits_missing_focus() {
    let engine = prompt_engine();
    let message =
        blueprint_refine_manifest_user_message(&engine, None, None).expect("message should render");

    assert!(message.contains(r#""context_refs""#));
    assert!(message.contains("existing_blueprint_context"));
    assert!(!message.contains("星潮档案"));
    assert!(!message.contains(r#""focus""#));
    assert!(!message.contains("全面优化"));
}

#[test]
fn blueprint_refine_manifest_user_message_omits_blank_focus() {
    let engine = prompt_engine();
    let message = blueprint_refine_manifest_user_message(&engine, None, Some(" \n\t "))
        .expect("message should render");

    assert!(!message.contains(r#""focus""#));
    assert!(!message.contains("全面优化"));
}

#[test]
fn blueprint_refine_manifest_user_message_includes_explicit_focus() {
    let engine = prompt_engine();
    let message = blueprint_refine_manifest_user_message(&engine, None, Some(" 强化主角动机 "))
        .expect("message should render");

    assert!(message.contains(r#""focus": "强化主角动机""#));
}

#[test]
fn blueprint_refine_request_moves_existing_blueprint_into_manifest() {
    let engine = prompt_engine();
    let system_prompt = engine
        .render(
            "blueprint_optimization",
            &std::collections::HashMap::new(),
            Some(42),
        )
        .expect("system prompt should render");
    let user_message =
        blueprint_refine_manifest_user_message(&engine, Some(42), Some("强化主角动机"))
            .expect("user message should render");
    let blueprint_context = json!({
        "blueprint": {
            "title": "星潮档案",
            "genre": "科幻",
            "full_synopsis": "完整主线",
        },
        "characters": [{"name": "林烬", "motivation": "查明真相"}],
        "relationships": [],
    });
    let request =
        build_blueprint_refine_request(42, system_prompt, user_message, &blueprint_context)
            .expect("refine request should build");

    let manifest = request
        .context_manifest_json
        .as_ref()
        .expect("context manifest should be attached");
    let pack_kinds = manifest["packs"]
        .as_array()
        .expect("packs should be array")
        .iter()
        .filter_map(|pack| pack["pack_kind"].as_str())
        .collect::<Vec<_>>();
    assert!(pack_kinds.contains(&"existing_blueprint_context"));

    let user_message = request
        .messages
        .iter()
        .find(|message| matches!(message.role, a4n_engine::llm::provider::MessageRole::User))
        .expect("user message should exist");
    assert!(user_message.content.contains("context_refs"));
    assert!(user_message.content.contains("existing_blueprint_context"));
    assert!(!user_message.content.contains("星潮档案"));
    assert!(!user_message.content.contains("完整主线"));

    let trace_context = request
        .trace_context
        .as_ref()
        .expect("trace context should be attached");
    assert_eq!(trace_context.workflow, "blueprint_refine");
    assert_eq!(trace_context.call_key.as_deref(), Some("blueprint_refine"));
}
