use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};
use a4n_core::models::character::{foreshadowing_index, protagonist_profile};
use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, novel_blueprint, novel_project,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Value, json};

use super::OptimizeAgentPermissions;
use crate::blueprint::blueprint_llm_context;
use crate::chapter::analysis::summary_text;
use crate::chapter::source_spans::{source_span_prompt_values, source_spans_for_chapters};

pub(super) async fn build_content_optimization_context(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    permissions: &OptimizeAgentPermissions,
) -> Result<Value, AppError> {
    let current = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
    let mut context = serde_json::Map::new();
    context.insert(
        "current_chapter".to_string(),
        optimization_chapter_context(&current),
    );

    let outline = match current.outline_id {
        Some(outline_id) => chapter_outline::Entity::find_by_id(outline_id)
            .one(db)
            .await
            .map_err(AppError::Database)?,
        None => chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .filter(chapter_outline::Column::ChapterNumber.eq(current.chapter_number))
            .one(db)
            .await
            .map_err(AppError::Database)?,
    };
    if let Some(outline) = outline {
        context.insert(
            "current_chapter_outline".to_string(),
            optimization_outline_context(&outline),
        );
    }

    if permissions.read_project_detail
        && let Some(project) = novel_project::Entity::find_by_id(project_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
    {
        context.insert(
            "project_detail".to_string(),
            optimization_project_context(&project),
        );
    }

    if permissions.read_blueprint
        && let Some(blueprint) = novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(project_id))
            .one(db)
            .await
            .map_err(AppError::Database)?
    {
        let characters = blueprint_character::Entity::find()
            .filter(blueprint_character::Column::ProjectId.eq(project_id))
            .all(db)
            .await
            .map_err(AppError::Database)?;
        let relationships = blueprint_relationship::Entity::find()
            .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
            .all(db)
            .await
            .map_err(AppError::Database)?;
        context.insert(
            "blueprint".to_string(),
            blueprint_llm_context(&blueprint, &characters, &relationships),
        );
    }

    if permissions.read_outlines {
        let parts = part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .order_by_asc(part_outline::Column::PartNumber)
            .all(db)
            .await
            .map_err(AppError::Database)?
            .iter()
            .map(optimization_part_outline_context)
            .collect::<Vec<_>>();
        if !parts.is_empty() {
            context.insert("part_outlines".to_string(), Value::Array(parts));
        }

        let outlines = chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .order_by_asc(chapter_outline::Column::ChapterNumber)
            .all(db)
            .await
            .map_err(AppError::Database)?
            .iter()
            .map(optimization_outline_context)
            .collect::<Vec<_>>();
        if !outlines.is_empty() {
            context.insert("chapter_outlines".to_string(), Value::Array(outlines));
        }
    }

    if permissions.read_character_profiles {
        let profiles = protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(protagonist_profile::Column::Id)
            .all(db)
            .await
            .map_err(AppError::Database)?
            .iter()
            .map(optimization_character_profile_context)
            .collect::<Vec<_>>();
        if !profiles.is_empty() {
            context.insert("character_profiles".to_string(), Value::Array(profiles));
        }
    }

    if permissions.read_foreshadowing {
        let foreshadowing = foreshadowing_index::Entity::find()
            .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
            .order_by_asc(foreshadowing_index::Column::SetupChapter)
            .all(db)
            .await
            .map_err(AppError::Database)?
            .iter()
            .map(optimization_foreshadowing_context)
            .collect::<Vec<_>>();
        if !foreshadowing.is_empty() {
            context.insert(
                "foreshadowing_index".to_string(),
                Value::Array(foreshadowing),
            );
        }
    }

    let mut previous = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .filter(chapter::Column::ChapterNumber.lt(current.chapter_number))
        .order_by_desc(chapter::Column::ChapterNumber)
        .limit(3)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    previous.reverse();
    let previous = previous
        .iter()
        .map(optimization_chapter_summary_context)
        .collect::<Vec<_>>();
    if !previous.is_empty() {
        context.insert("previous_chapters".to_string(), Value::Array(previous));
    }

    let next = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .filter(chapter::Column::ChapterNumber.gt(current.chapter_number))
        .order_by_asc(chapter::Column::ChapterNumber)
        .limit(2)
        .all(db)
        .await
        .map_err(AppError::Database)?
        .iter()
        .map(optimization_chapter_summary_context)
        .collect::<Vec<_>>();
    if !next.is_empty() {
        context.insert("next_chapters".to_string(), Value::Array(next));
    }

    Ok(Value::Object(context))
}

pub(super) fn optimization_context_manifest(context: &Value) -> Value {
    let Some(object) = context.as_object() else {
        return json!({
            "sections": [],
            "counts": {},
            "section_details": [],
        });
    };
    let mut section_names = object.keys().cloned().collect::<Vec<_>>();
    section_names.sort();
    let mut counts = serde_json::Map::new();
    for key in [
        "part_outlines",
        "chapter_outlines",
        "character_profiles",
        "foreshadowing_index",
        "previous_chapters",
        "next_chapters",
    ] {
        if let Some(items) = object.get(key).and_then(Value::as_array) {
            counts.insert(key.to_string(), json!(items.len()));
        }
    }
    for key in [
        "current_chapter",
        "current_chapter_outline",
        "project_detail",
        "blueprint",
    ] {
        if object.contains_key(key) {
            counts.insert(key.to_string(), json!(1));
        }
    }
    let section_details = section_names
        .iter()
        .filter_map(|name| optimization_context_section_detail(name, counts.get(name)))
        .collect::<Vec<_>>();

    json!({
        "sections": section_names,
        "counts": counts,
        "section_details": section_details,
    })
}

fn optimization_context_section_detail(name: &str, count: Option<&Value>) -> Option<Value> {
    let description = match name {
        "current_chapter" => {
            "当前章节元信息；get_project_information 读取后会返回当前章节正文和元信息"
        }
        "current_chapter_outline" => "当前章节章纲、节拍、角色、伏笔和字数目标",
        "project_detail" => "项目标题、类型、简介、目标章节数和原始项目配置",
        "blueprint" => "项目蓝图、主要角色关系和整体设定",
        "part_outlines" => "分卷/分部大纲，用于判断章节在整体结构中的位置",
        "chapter_outlines" => "全书章节大纲列表，用于判断前后承接和长期铺垫",
        "character_profiles" => "角色档案，用于判断动机、性格、口吻和关系一致性",
        "foreshadowing_index" => "伏笔索引，用于判断埋设、回收和遗漏",
        "previous_chapters" => "当前章之前的章节摘要，用于判断上下文承接",
        "next_chapters" => "当前章之后的章节摘要，用于判断后续铺垫和走向",
        _ => return None,
    };
    Some(json!({
        "name": name,
        "count": count.cloned(),
        "description": description,
    }))
}

fn optimization_project_context(project: &novel_project::Model) -> Value {
    json!({
        "id": project.id,
        "title": project.title,
        "description": project.description,
        "project_type": project.project_type,
        "status": project.status,
        "target_chapters": project.target_chapters,
        "current_chapter": project.current_chapter,
        "total_chapters": project.total_chapters,
        "chapters_per_part": project.chapters_per_part,
        "raw_blueprint": project.raw_blueprint.as_deref().map(str::trim).filter(|text| !text.is_empty()),
        "export_config": project.export_config.as_deref().map(str::trim).filter(|text| !text.is_empty()),
    })
}

fn optimization_chapter_context(ch: &chapter::Model) -> Value {
    let source_spans = source_spans_for_chapters(std::slice::from_ref(ch));
    json!({
        "chapter_number": ch.chapter_number,
        "title": ch.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": optimization_chapter_summary(ch),
        "word_count": ch.word_count,
        "status": ch.status,
        "source_spans": source_span_prompt_values(&source_spans),
    })
}

fn optimization_chapter_summary_context(ch: &chapter::Model) -> Value {
    json!({
        "chapter_number": ch.chapter_number,
        "title": ch.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": optimization_chapter_summary(ch),
        "word_count": ch.word_count,
    })
}

fn optimization_chapter_summary(ch: &chapter::Model) -> Option<String> {
    ch.analysis_data
        .as_ref()
        .and_then(summary_text)
        .or_else(|| {
            ch.summary
                .as_deref()
                .map(str::trim)
                .filter(|summary| !summary.is_empty())
                .map(ToOwned::to_owned)
        })
}

fn optimization_outline_context(outline: &chapter_outline::Model) -> Value {
    json!({
        "chapter_number": outline.chapter_number,
        "title": outline.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": outline.summary.as_deref().map(str::trim).filter(|summary| !summary.is_empty()),
        "beats": outline.beats,
        "key_scenes": outline.key_scenes,
        "pacing_score": outline.pacing_score,
        "foreshadowing_setup": outline.foreshadowing_setup,
        "foreshadowing_payoff": outline.foreshadowing_payoff,
        "characters_involved": outline.characters_involved,
        "word_count_target": outline.word_count_target,
    })
}

fn optimization_part_outline_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary.as_deref().map(str::trim).filter(|summary| !summary.is_empty()),
        "theme": part.theme.as_deref().map(str::trim).filter(|theme| !theme.is_empty()),
        "key_events": part.key_events,
        "character_focus": part.character_focus,
    })
}

fn optimization_character_profile_context(profile: &protagonist_profile::Model) -> Value {
    json!({
        "name": profile.name,
        "gender": profile.gender,
        "age": profile.age,
        "base_attributes": profile.base_attributes,
        "current_state": profile.current_state,
        "character_arc": profile.character_arc,
        "voice_fingerprint": profile.voice_fingerprint,
        "appearance": profile.appearance.as_deref().map(str::trim).filter(|text| !text.is_empty()),
        "background": profile.background.as_deref().map(str::trim).filter(|text| !text.is_empty()),
        "personality": profile.personality.as_deref().map(str::trim).filter(|text| !text.is_empty()),
        "last_synced_chapter": profile.last_synced_chapter,
    })
}

fn optimization_foreshadowing_context(item: &foreshadowing_index::Model) -> Value {
    json!({
        "description": item.description,
        "setup_chapter": item.setup_chapter,
        "payoff_chapter": item.payoff_chapter,
        "status": item.status,
        "related_characters": item.related_characters,
        "importance": item.importance,
    })
}
