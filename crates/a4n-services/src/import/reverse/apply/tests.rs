use super::*;
use crate::blueprint::CHAPTERS_PER_PART;
use a4n_core::models::{
    chapter::{chapter_outline, part_outline},
    project::{novel_blueprint, novel_project},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde_json::{Value, json};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn fixed_time() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 6, 5)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

async fn insert_project(db: &DatabaseConnection, title: &str) -> novel_project::Model {
    let now = fixed_time();
    novel_project::ActiveModel {
        user_id: Set(1),
        title: Set(title.to_string()),
        description: Set("反推 apply 测试".to_string()),
        project_type: Set("novel_long".to_string()),
        status: Set("draft".to_string()),
        target_chapters: Set(100),
        current_chapter: Set(0),
        total_chapters: Set(Some(100)),
        chapters_per_part: Set(Some(CHAPTERS_PER_PART)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_existing_blueprint(
    db: &DatabaseConnection,
    project_id: i64,
    total_chapters: i32,
    needs_part_outlines: bool,
) {
    let now = fixed_time();
    novel_blueprint::ActiveModel {
        project_id: Set(project_id),
        title: Set("旧蓝图".to_string()),
        genre: Set("科幻".to_string()),
        style: Set(Some("冷峻".to_string())),
        tone: Set(Some("悬疑".to_string())),
        target_audience: Set(Some("成人".to_string())),
        one_sentence_summary: Set(Some("旧概要".to_string())),
        full_synopsis: Set(Some("旧大纲".to_string())),
        world_setting: Set(Some(json!({"core_rules": ["旧规则"]}))),
        current_version: Set("1.0".to_string()),
        needs_part_outlines: Set(needs_part_outlines),
        total_chapters: Set(Some(total_chapters)),
        chapters_per_part: Set(if needs_part_outlines {
            Some(CHAPTERS_PER_PART)
        } else {
            None
        }),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

fn reverse_blueprint(total_chapters: i32) -> Value {
    json!({
        "title": "星潮档案",
        "genre": "科幻",
        "style": "冷峻",
        "tone": "悬疑",
        "target_audience": "成人",
        "one_sentence_summary": "调查星潮真相。",
        "full_synopsis": "主角追查星潮背后的阴谋。",
        "world_setting": {
            "premise": "星潮灾变后，记忆成为可被封存、审校和买卖的公共资源。",
            "core_rules": "星潮会周期性改写城市居民的短期记忆，只有少数审校员能发现异常。",
            "technology_or_magic": {
                "source": "星潮辐射与记忆审校装置共同作用",
                "rules": ["记忆可被读取、封存、比对"],
                "limits": ["无法无代价恢复完整真相"],
                "costs": ["审校过度会造成自我认知损耗"]
            },
            "key_locations": [
                {"name": "旧城档案馆", "description": "保存灾变前后的纸质档案", "story_function": "主角寻找真相的核心据点"}
            ],
            "factions": [
                {"name": "调查局", "description": "公开维护记忆秩序的官方机构", "agenda": "维持秩序并掩盖星潮源头", "representative_characters": ["林澈"]}
            ],
            "history_timeline": [
                {"period": "灾变元年", "event": "第一场星潮覆盖旧城", "impact": "记忆审校制度建立"}
            ],
            "culture_and_society": {
                "values": "稳定记忆被视为基本权利",
                "class_structure": "审校员与普通居民之间存在信息壁垒",
                "daily_life": "居民定期接受记忆校验"
            },
            "conflict_sources": [
                {"name": "真相封锁", "description": "调查局隐藏星潮成因", "stakes": "旧城居民可能再次失去自我"}
            ],
            "mysteries_and_foreshadowing": [
                {"mystery": "星潮源头", "early_clues": ["档案编号重复", "旧城钟楼停摆"], "payoff_direction": "源头与主角家族有关"}
            ],
            "genre_promises": ["记忆悬疑", "制度阴谋", "身份追寻"]
        },
        "total_chapters": total_chapters,
        "characters": [
            {
                "name": "林澈",
                "character_type": "protagonist",
                "gender": "男",
                "age": "27",
                "appearance": "黑发清瘦，常穿深色审校员外套，眼神疲惫但敏锐",
                "personality": "克制、执拗、擅长从细节中寻找异常",
                "background": "旧城记忆审校员，妹妹在星潮后失踪",
                "motivation": "查明妹妹失踪和星潮源头的关系",
                "arc_summary": "从服从制度的审校员成长为揭开真相的人",
                "relationship_to_protagonist": "本人",
                "arc_stages": {
                    "start": "相信审校制度能维持秩序",
                    "core_wound": "妹妹失踪时自己选择遵守流程而非救人",
                    "desire": "找回妹妹的真实记忆",
                    "need": "承认制度本身可能是伤害来源",
                    "secret": "他的家族与第一场星潮有关",
                    "voice_style": "话少，判断直接，常用档案编号确认事实",
                    "turning_point": "发现自己的审校记录被篡改",
                    "end": "主动公开真相并承担代价"
                }
            },
            {
                "name": "沈砚",
                "character_type": "antagonist",
                "gender": "女",
                "age": "41",
                "appearance": "银灰短发，调查局制服整洁，笑容克制",
                "personality": "冷静、强势、善于用秩序包装控制",
                "background": "调查局核心负责人，参与星潮封锁计划",
                "motivation": "阻止旧城因真相公开而崩溃",
                "arc_summary": "以守护秩序为名持续压制真相",
                "relationship_to_protagonist": "上级与阻碍者",
                "arc_stages": {
                    "start": "作为可靠上级引导林澈调查",
                    "core_wound": "第一场星潮中失去家人后只相信控制",
                    "desire": "让旧城维持可预测的稳定",
                    "need": "面对被她牺牲的个体",
                    "secret": "她批准过篡改林澈档案",
                    "voice_style": "礼貌、精确、带有命令感",
                    "turning_point": "林澈公开第一批证据",
                    "end": "被迫承认秩序建立在谎言上"
                }
            },
            {
                "name": "许南星",
                "character_type": "supporting",
                "gender": "女",
                "age": "24",
                "appearance": "长发，戴旧式圆框眼镜，常抱着档案夹",
                "personality": "敏感、谨慎、记忆力异常稳定",
                "background": "旧城档案馆管理员，保存灾变前纸档",
                "motivation": "保护档案馆中未被电子系统污染的记录",
                "arc_summary": "从旁观保管者变成主动协助者",
                "relationship_to_protagonist": "关键盟友",
                "arc_stages": {
                    "start": "只愿提供有限帮助",
                    "core_wound": "曾因泄露档案害朋友被带走",
                    "desire": "保住档案馆和自己的安全",
                    "need": "相信真相需要被看见",
                    "secret": "她保存着林澈妹妹最后的纸质记录",
                    "voice_style": "轻声、谨慎，常先确认周围是否安全",
                    "turning_point": "调查局查封档案馆",
                    "end": "公开纸档并成为证人"
                }
            }
        ],
        "relationships": [{
            "relationship_type": "rival",
            "character_a": "林澈",
            "character_b": "沈砚",
            "description": "林澈表面听命于沈砚，实际逐渐发现她是记忆封锁的核心执行者。",
            "strength": 8,
            "evolution": {
                "start": "上级与下属的信任关系",
                "tension": "调查方向不断被沈砚截断",
                "turning_points": ["林澈发现篡改记录", "沈砚下令封存档案馆"],
                "hidden_truth": "沈砚知道林澈妹妹失踪的真实原因",
                "story_function": "把个人寻亲线推向制度真相线",
                "end": "两人在公开真相前彻底对立"
            }
        }]
    })
}

fn reverse_outline(chapter_number: i32, title: &str) -> Value {
    json!({
        "chapter_number": chapter_number,
        "title": title,
        "summary": format!("{title}摘要")
    })
}

fn reverse_part(part_number: i32, start_chapter: i32, end_chapter: i32, title: &str) -> Value {
    json!({
        "part_number": part_number,
        "title": title,
        "start_chapter": start_chapter,
        "end_chapter": end_chapter,
        "summary": format!("{title}摘要"),
        "theme": "代价",
        "key_events": ["旧敌回归", "盟友背叛", "真相浮出"],
        "character_focus": {"主角": "从迟疑转为主动"}
    })
}

#[tokio::test]
async fn apply_reverse_blueprint_derives_part_outlines_only_above_threshold() {
    let db = setup_db().await;
    let short_project = insert_project(&db, "五十章项目").await;
    let long_project = insert_project(&db, "五十一章项目").await;

    apply_reverse_blueprint_data(&db, short_project.id, &reverse_blueprint(50))
        .await
        .unwrap();
    apply_reverse_blueprint_data(&db, long_project.id, &reverse_blueprint(51))
        .await
        .unwrap();

    let short_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(short_project.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let long_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(long_project.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(short_blueprint.total_chapters, Some(50));
    assert!(!short_blueprint.needs_part_outlines);
    assert_eq!(short_blueprint.chapters_per_part, None);
    assert_eq!(long_blueprint.total_chapters, Some(51));
    assert!(long_blueprint.needs_part_outlines);
    assert_eq!(long_blueprint.chapters_per_part, Some(CHAPTERS_PER_PART));
}

#[tokio::test]
async fn apply_reverse_blueprint_preserves_existing_chapter_plan_when_count_is_omitted() {
    let db = setup_db().await;
    let project = insert_project(&db, "既有五十一章项目").await;
    insert_existing_blueprint(&db, project.id, 51, true).await;
    let mut data = reverse_blueprint(51);
    data.as_object_mut().unwrap().remove("total_chapters");

    apply_reverse_blueprint_data(&db, project.id, &data)
        .await
        .unwrap();

    let blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let project = novel_project::Entity::find_by_id(project.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(blueprint.total_chapters, Some(51));
    assert!(blueprint.needs_part_outlines);
    assert_eq!(blueprint.chapters_per_part, Some(CHAPTERS_PER_PART));
    assert_eq!(project.total_chapters, Some(51));
    assert_eq!(project.target_chapters, 51);
    assert_eq!(project.chapters_per_part, Some(CHAPTERS_PER_PART));
}

#[tokio::test]
async fn apply_reverse_blueprint_rejects_total_chapters_conflicting_with_existing_plan() {
    let db = setup_db().await;
    let project = insert_project(&db, "章节数冲突项目").await;
    insert_existing_blueprint(&db, project.id, 50, false).await;

    let err = apply_reverse_blueprint_data(&db, project.id, &reverse_blueprint(51))
        .await
        .expect_err("conflicting total_chapters must fail");

    assert!(err.contains("total_chapters"));
    let blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(blueprint.total_chapters, Some(50));
    assert!(!blueprint.needs_part_outlines);
    assert_eq!(blueprint.chapters_per_part, None);
}

#[tokio::test]
async fn apply_reverse_blueprint_accepts_matching_part_outline_flag() {
    let db = setup_db().await;
    let project = insert_project(&db, "分部标记匹配项目").await;
    let mut data = reverse_blueprint(51);
    data["needs_part_outlines"] = json!(true);

    apply_reverse_blueprint_data(&db, project.id, &data)
        .await
        .unwrap();

    let blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(blueprint.needs_part_outlines);
    assert_eq!(blueprint.chapters_per_part, Some(CHAPTERS_PER_PART));
}

#[tokio::test]
async fn apply_reverse_blueprint_rejects_mismatched_part_outline_flag() {
    let db = setup_db().await;
    let project = insert_project(&db, "规则冲突项目").await;
    let mut data = reverse_blueprint(100);
    data["needs_part_outlines"] = json!(false);

    let err = apply_reverse_blueprint_data(&db, project.id, &data)
        .await
        .expect_err("mismatched needs_part_outlines must fail");

    assert!(err.contains("needs_part_outlines"));
    assert!(err.contains("必须与有效总章节数推导结果一致"));
    let count = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn apply_reverse_blueprint_rejects_non_bool_part_outline_flag() {
    let db = setup_db().await;
    let project = insert_project(&db, "分部标记类型错误项目").await;
    let mut data = reverse_blueprint(51);
    data["needs_part_outlines"] = json!("true");

    let err = apply_reverse_blueprint_data(&db, project.id, &data)
        .await
        .expect_err("non-bool needs_part_outlines must fail");

    assert!(err.contains("needs_part_outlines 必须是布尔值"));
    let count = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn apply_reverse_blueprint_rejects_non_canonical_blueprint_sections() {
    let db = setup_db().await;
    let project = insert_project(&db, "旧结构反推项目").await;
    let mut data = reverse_blueprint(51);
    data["world_setting"]
        .as_object_mut()
        .unwrap()
        .remove("premise");
    data["characters"][0]
        .as_object_mut()
        .unwrap()
        .remove("arc_stages");
    data["relationships"][0]
        .as_object_mut()
        .unwrap()
        .remove("strength");

    let err = apply_reverse_blueprint_data(&db, project.id, &data)
        .await
        .expect_err("non-canonical reverse blueprint must fail");

    assert!(
        err.contains("world_setting") || err.contains("arc_stages") || err.contains("strength")
    );
    let count = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn apply_reverse_outline_rejects_invalid_chapter_number_before_writes() {
    let db = setup_db().await;
    let project = insert_project(&db, "章号非法项目").await;
    let data = json!({
        "outlines": [
            reverse_outline(1, "第一章"),
            reverse_outline(0, "第零章")
        ]
    });

    let err = apply_reverse_outline_data(&db, project.id, &data)
        .await
        .expect_err("chapter_number 0 must fail");

    assert!(err.contains("chapter_number"));
    let count = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn apply_reverse_outline_updates_existing_word_count_target() {
    let db = setup_db().await;
    let project = insert_project(&db, "章纲更新项目").await;
    let now = fixed_time();
    chapter_outline::ActiveModel {
        project_id: Set(project.id),
        part_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("旧标题".to_string())),
        summary: Set(Some("旧摘要".to_string())),
        beats: Set(None),
        key_scenes: Set(None),
        pacing_score: Set(None),
        foreshadowing_setup: Set(None),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(None),
        word_count_target: Set(None),
        status: Set("draft".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let data = json!({
        "outlines": [{
            "chapter_number": 1,
            "title": "新标题",
            "summary": "新摘要",
            "word_count_target": 3200
        }]
    });

    apply_reverse_outline_data(&db, project.id, &data)
        .await
        .unwrap();

    let outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project.id))
        .filter(chapter_outline::Column::ChapterNumber.eq(1))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outline.title.as_deref(), Some("新标题"));
    assert_eq!(outline.summary.as_deref(), Some("新摘要"));
    assert_eq!(outline.word_count_target, Some(3200));
    assert_eq!(outline.foreshadowing_setup, None);
    assert_eq!(outline.foreshadowing_payoff, None);
}

#[tokio::test]
async fn apply_reverse_part_outline_rejects_invalid_ranges_and_shapes_before_writes() {
    let db = setup_db().await;
    let project = insert_project(&db, "分部非法项目").await;
    let reversed_range = json!({
        "parts": [
            reverse_part(1, 1, 10, "第一部"),
            reverse_part(2, 20, 11, "第二部")
        ]
    });
    let err = apply_reverse_part_outline_data(&db, project.id, &reversed_range)
        .await
        .expect_err("reversed chapter range must fail");
    assert!(err.contains("end_chapter"));

    let wrong_shape = json!({
        "parts": [{
            "part_number": 1,
            "title": "第一部",
            "start_chapter": 1,
            "end_chapter": 10,
            "summary": "第一部摘要",
            "theme": "代价",
            "key_events": "旧敌回归",
            "character_focus": {"主角": "成长"}
        }]
    });
    let err = apply_reverse_part_outline_data(&db, project.id, &wrong_shape)
        .await
        .expect_err("key_events string must fail");
    assert!(err.contains("key_events"));

    let count = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
