use super::*;
use a4n_db_models::models::{
    chapter::{chapter, chapter_outline, part_outline},
    generation::{generation_spec, generation_spec_text_anchor, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
        novel_project, project_source_snapshot, source_snapshot_chapter, text_anchor,
    },
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_db::project::{
    ProjectDuplicateError, duplicate_derived_project_record, duplicate_text_project_record,
};
use a4n_domain::project::{ProjectRecord, ProjectStatus};
use chrono::NaiveDate;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::json;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn fixed_time() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

async fn insert_project(db: &DatabaseConnection, status: &str) -> novel_project::Model {
    novel_project::ActiveModel {
        user_id: Set(1),
        title: Set("源项目".to_string()),
        description: Set("用于分支复制测试".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set("text".to_string()),
        source_sync_status: Set("current".to_string()),
        status: Set(status.to_string()),
        target_chapters: Set(80),
        current_chapter: Set(12),
        total_chapters: Set(Some(80)),
        chapters_per_part: Set(Some(20)),
        avatar_svg: Set(Some("<svg>avatar</svg>".to_string())),
        avatar_animal: Set(Some("wolf".to_string())),
        raw_blueprint: Set(Some("{\"source\":true}".to_string())),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn project_record(db: &DatabaseConnection, project_id: i64) -> ProjectRecord {
    a4n_db::project::project_by_id_record(db, project_id)
        .await
        .unwrap()
        .expect("project record")
}

async fn insert_blueprint_graph(db: &DatabaseConnection, project_id: i64) {
    novel_blueprint::ActiveModel {
        project_id: Set(project_id),
        title: Set("旧城疑云".to_string()),
        genre: Set("悬疑".to_string()),
        style: Set(Some("冷峻".to_string())),
        tone: Set(Some("紧张".to_string())),
        target_audience: Set(Some("成人".to_string())),
        one_sentence_summary: Set(Some("一封信引出真相。".to_string())),
        full_synopsis: Set(Some("完整梗概".to_string())),
        world_setting: Set(Some(json!({"city": "旧城"}))),
        current_version: Set("1.2".to_string()),
        needs_part_outlines: Set(true),
        total_chapters: Set(Some(80)),
        chapters_per_part: Set(Some(20)),
        avatar_svg: Set(Some("<svg>blueprint</svg>".to_string())),
        avatar_animal: Set(Some("fox".to_string())),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    blueprint_version::ActiveModel {
        project_id: Set(project_id),
        version: Set("1.1".to_string()),
        change_description: Set(Some("保存旧版本".to_string())),
        blueprint_data: Set(json!({
            "title": "旧城疑云",
            "full_synopsis": "旧梗概",
            "world_setting": {"city": "旧城"},
            "characters": [{"name": "林岚"}],
            "relationships": [],
        })),
        created_at: Set(fixed_time()),
        created_by: Set(Some("user".to_string())),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("林岚".to_string()),
        character_type: Set("protagonist".to_string()),
        gender: Set(Some("女".to_string())),
        age: Set(Some("28".to_string())),
        appearance: Set(Some("黑色风衣".to_string())),
        personality: Set(Some("克制".to_string())),
        background: Set(Some("记者".to_string())),
        motivation: Set(Some("寻找真相".to_string())),
        arc_summary: Set(Some("从旁观者到揭露者".to_string())),
        relationship_to_protagonist: Set(None),
        arc_stages: Set(Some(json!(["怀疑", "行动"]))),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    blueprint_relationship::ActiveModel {
        project_id: Set(project_id),
        relationship_type: Set("rival".to_string()),
        character_a: Set("林岚".to_string()),
        character_b: Set("沈默".to_string()),
        description: Set(Some("互相试探".to_string())),
        strength: Set(Some(7)),
        evolution: Set(Some(json!(["冲突", "合作"]))),
        created_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn insert_outline_and_chapter(
    db: &DatabaseConnection,
    project_id: i64,
) -> (part_outline::Model, chapter_outline::Model, chapter::Model) {
    let part = part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(1),
        title: Set("疑云".to_string()),
        start_chapter: Set(1),
        end_chapter: Set(20),
        summary: Set(Some("第一部".to_string())),
        theme: Set(Some("真相".to_string())),
        key_events: Set(Some(json!(["收到密信"]))),
        character_focus: Set(Some(json!({"林岚": "追查"}))),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let outline = chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(Some(part.id)),
        chapter_number: Set(1),
        title: Set(Some("密信".to_string())),
        summary: Set(Some("林岚收到密信。".to_string())),
        beats: Set(Some(json!(["收到信", "前往旧城"]))),
        key_scenes: Set(Some(json!(["报社"]))),
        pacing_score: Set(Some(7.5)),
        foreshadowing_setup: Set(Some(json!(["邮戳"]))),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(Some(json!(["林岚"]))),
        word_count_target: Set(Some(3000)),
        status: Set("completed".to_string()),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let chapter = chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(Some(outline.id)),
        chapter_number: Set(1),
        title: Set(Some("密信".to_string())),
        content: Set("林岚收到一封没有署名的信。".to_string()),
        word_count: Set(14),
        status: Set("completed".to_string()),
        summary: Set(Some("密信出现。".to_string())),
        style_tags: Set(Some(json!(["悬疑"]))),
        analysis_data: Set(Some(json!({"foreshadowing": {"planted": ["邮戳"]}}))),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    (part, outline, chapter)
}
