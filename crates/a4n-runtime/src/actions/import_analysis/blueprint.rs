use super::chapter_outline::{collect_analysis_strings, collect_chapter_summaries};
use crate::actions::import_reverse::{ReverseEngineeringTarget, build_reverse_request_from_sample};
use crate::state::AppState;
use a4n_core::models::{
    chapter::chapter,
    project::{novel_blueprint, novel_project},
};
use a4n_services::import::reverse::{
    ReverseContentRequest, ReverseContextKind, apply_reverse_blueprint_data,
    resolve_reverse_content_sample,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
};
use serde_json::{Value, json};

fn is_blank_text(value: Option<&str>) -> bool {
    value.map(str::trim).is_none_or(str::is_empty)
}

fn is_import_generated_world_setting(value: Option<&Value>) -> bool {
    value
        .and_then(|v| v.get("source"))
        .and_then(Value::as_str)
        .is_some_and(|source| source == "import_analysis")
}

fn import_blueprint_genre(project_type: &str) -> Result<&'static str, String> {
    match project_type.trim() {
        "novel_long" => Ok("长篇小说"),
        "novel_short" => Ok("短篇小说"),
        "script" => Ok("剧本"),
        "prose" => Ok("散文"),
        "" => Err("项目类型不能为空，无法生成导入蓝图题材".to_string()),
        other => Err(format!("不支持的项目类型：{other}，无法生成导入蓝图题材")),
    }
}

pub(super) async fn upsert_import_blueprint(
    db: &impl ConnectionTrait,
    project: &novel_project::Model,
    chapters: &[chapter::Model],
    needs_part_outlines: bool,
) -> Result<(), String> {
    let now = Utc::now().naive_utc();
    let summaries = collect_chapter_summaries(chapters, 80)?;
    if summaries.is_empty() {
        return Err("项目没有可用于生成导入蓝图的章节摘要".to_string());
    }
    let synopsis = summaries.join("\n");
    let world_setting = json!({
        "locations": collect_analysis_strings(chapters, "locations", 40),
        "items": collect_analysis_strings(chapters, "items", 40),
        "tags": collect_analysis_strings(chapters, "tags", 40),
        "source": "import_analysis",
    });
    let one_sentence = summaries
        .first()
        .cloned()
        .ok_or_else(|| "项目没有可用于生成一句话概要的章节摘要".to_string())?;
    let chapters_per_part = if needs_part_outlines { Some(25) } else { None };

    if let Some(existing) = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    {
        let mut active: novel_blueprint::ActiveModel = existing.into();
        active.title = Set(project.title.clone());
        if is_blank_text(active.one_sentence_summary.as_ref().as_deref()) {
            active.one_sentence_summary = Set(Some(one_sentence));
        }
        if is_blank_text(active.full_synopsis.as_ref().as_deref()) {
            active.full_synopsis = Set(Some(synopsis));
        }
        if active.world_setting.as_ref().is_none()
            || is_import_generated_world_setting(active.world_setting.as_ref().as_ref())
        {
            active.world_setting = Set(Some(world_setting));
        }
        active.total_chapters = Set(Some(chapters.len() as i32));
        active.needs_part_outlines = Set(needs_part_outlines);
        active.chapters_per_part = Set(chapters_per_part);
        active.updated_at = Set(now);
        active.update(db).await.map_err(|e| e.to_string())?;
    } else {
        novel_blueprint::ActiveModel {
            project_id: Set(project.id),
            title: Set(project.title.clone()),
            genre: Set(import_blueprint_genre(&project.project_type)?.to_string()),
            style: Set(None),
            tone: Set(None),
            target_audience: Set(None),
            one_sentence_summary: Set(Some(one_sentence)),
            full_synopsis: Set(Some(synopsis)),
            world_setting: Set(Some(world_setting)),
            current_version: Set("1.0".to_string()),
            needs_part_outlines: Set(needs_part_outlines),
            total_chapters: Set(Some(chapters.len() as i32)),
            chapters_per_part: Set(chapters_per_part),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) async fn apply_reverse_blueprint(
    state: &AppState,
    project_id: i64,
) -> Result<(), String> {
    let content_sample = resolve_reverse_content_sample(
        state.db.as_ref(),
        ReverseContentRequest {
            project_id,
            manual_sample: "",
            auto_context: true,
            context_limit: Some(50),
        },
        ReverseContextKind::Blueprint,
    )
    .await?;
    let variables: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
    let target = ReverseEngineeringTarget::Blueprint;
    let request =
        build_reverse_request_from_sample(state, project_id, target, &content_sample, &variables)
            .map_err(|e| e.to_string())?;
    let result = state
        .import
        .reverse_engineer_request(target.prompt_name(), request, state.llm.as_ref(), None)
        .await
        .map_err(|e| e.to_string())?;
    let txn = state.db.begin().await.map_err(|e| e.to_string())?;
    let applied = apply_reverse_blueprint_data(&txn, project_id, &result).await?;
    let character_count = required_applied_count(&applied, "character_count")?;
    let relationship_count = required_applied_count(&applied, "relationship_count")?;
    txn.commit().await.map_err(|e| e.to_string())?;
    tracing::info!(
        project_id,
        character_count,
        relationship_count,
        "项目蓝图反推完成"
    );
    Ok(())
}

fn required_applied_count(value: &Value, field: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("项目蓝图反推结果缺少非负整数字段: {field}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_blueprint_genre_maps_known_project_types() {
        assert_eq!(import_blueprint_genre("novel_long").unwrap(), "长篇小说");
        assert_eq!(import_blueprint_genre("novel_short").unwrap(), "短篇小说");
        assert_eq!(import_blueprint_genre("script").unwrap(), "剧本");
        assert_eq!(import_blueprint_genre("prose").unwrap(), "散文");
    }

    #[test]
    fn import_blueprint_genre_rejects_unknown_project_type() {
        let err = import_blueprint_genre("game")
            .expect_err("unknown project type must not become placeholder genre");

        assert!(err.contains("不支持的项目类型"));
    }

    #[test]
    fn is_blank_text_does_not_treat_placeholder_as_blank() {
        assert!(is_blank_text(None));
        assert!(is_blank_text(Some("  ")));
        assert!(!is_blank_text(Some("待补充")));
        assert!(!is_blank_text(Some("（待补充）")));
    }

    #[test]
    fn required_applied_count_rejects_missing_or_invalid_count() {
        assert_eq!(
            required_applied_count(&json!({"character_count": 2}), "character_count").unwrap(),
            2
        );
        assert!(
            required_applied_count(&json!({}), "character_count")
                .expect_err("missing count must fail")
                .contains("缺少非负整数字段")
        );
        assert!(
            required_applied_count(&json!({"character_count": -1}), "character_count")
                .expect_err("negative count must fail")
                .contains("缺少非负整数字段")
        );
    }
}
