use super::validation::{optional_positive_i32, required_object_clone};
use crate::blueprint::{
    BlueprintCharacterDraft, BlueprintRelationshipDraft, CHAPTERS_PER_PART,
    needs_part_outlines_for_chapters, sync_blueprint_characters, sync_blueprint_relationships,
    validate_blueprint_characters_payload, validate_blueprint_relationships_payload,
    validate_blueprint_world_setting_payload,
};
use crate::import::reverse_json::{
    optional_field_clone, optional_i32, reject_unknown_keys, required_array, required_string,
};
use a4n_core::models::project::{novel_blueprint, novel_project};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};

pub async fn apply_reverse_blueprint_data(
    db: &impl ConnectionTrait,
    project_id: i64,
    data: &Value,
) -> Result<Value, String> {
    validate_reverse_blueprint_top_level_keys(data)?;
    let now = Utc::now().naive_utc();
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "项目不存在".to_string())?;

    let title = required_string(data, "title", "reverse_blueprint")?;
    let genre = required_string(data, "genre", "reverse_blueprint")?;
    let style = required_string(data, "style", "reverse_blueprint")?;
    let tone = required_string(data, "tone", "reverse_blueprint")?;
    let target_audience = required_string(data, "target_audience", "reverse_blueprint")?;
    let one_sentence_summary = required_string(data, "one_sentence_summary", "reverse_blueprint")?;
    let full_synopsis = required_string(data, "full_synopsis", "reverse_blueprint")?;
    let world_setting = required_object_clone(data, "world_setting", "reverse_blueprint")?;
    validate_reverse_blueprint_sections(data, &world_setting)?;
    let incoming_total_chapters =
        optional_positive_i32(data, "total_chapters", "reverse_blueprint")?;
    let incoming_needs_part_outlines = optional_needs_part_outlines(data)?;
    let existing_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    let existing_total_chapters = existing_blueprint
        .as_ref()
        .and_then(|blueprint| blueprint.total_chapters.filter(|total| *total > 0));
    if let (Some(existing_total), Some(incoming_total)) =
        (existing_total_chapters, incoming_total_chapters)
        && existing_total != incoming_total
    {
        return Err(format!(
            "reverse_blueprint.total_chapters 必须与现有蓝图总章节数一致（existing={existing_total}, incoming={incoming_total}）"
        ));
    }
    let effective_total_chapters = existing_total_chapters
        .or(incoming_total_chapters)
        .or_else(|| project.total_chapters.filter(|total| *total > 0));
    let needs_part_outlines = resolve_reverse_blueprint_part_outline_need(effective_total_chapters);
    if let Some(incoming) = incoming_needs_part_outlines
        && incoming != needs_part_outlines
    {
        return Err(format!(
            "reverse_blueprint.needs_part_outlines 必须与有效总章节数推导结果一致（expected={needs_part_outlines}, incoming={incoming}）"
        ));
    }
    if let Some(total_chapters) = effective_total_chapters {
        let mut active: novel_project::ActiveModel = project.into();
        active.target_chapters = Set(total_chapters);
        active.total_chapters = Set(Some(total_chapters));
        active.chapters_per_part = Set(if needs_part_outlines {
            Some(CHAPTERS_PER_PART)
        } else {
            None
        });
        active.updated_at = Set(now);
        active.update(db).await.map_err(|e| e.to_string())?;
    }

    if let Some(existing) = existing_blueprint {
        let mut active: novel_blueprint::ActiveModel = existing.into();
        active.title = Set(title);
        active.genre = Set(genre);
        active.style = Set(Some(style));
        active.tone = Set(Some(tone));
        active.target_audience = Set(Some(target_audience));
        active.one_sentence_summary = Set(Some(one_sentence_summary));
        active.full_synopsis = Set(Some(full_synopsis));
        active.world_setting = Set(Some(world_setting));
        if let Some(total_chapters) = effective_total_chapters {
            active.total_chapters = Set(Some(total_chapters));
        }
        active.needs_part_outlines = Set(needs_part_outlines);
        active.chapters_per_part = Set(if needs_part_outlines {
            Some(CHAPTERS_PER_PART)
        } else {
            None
        });
        active.updated_at = Set(now);
        active.update(db).await.map_err(|e| e.to_string())?;
    } else {
        novel_blueprint::ActiveModel {
            project_id: Set(project_id),
            title: Set(title),
            genre: Set(genre),
            style: Set(Some(style)),
            tone: Set(Some(tone)),
            target_audience: Set(Some(target_audience)),
            one_sentence_summary: Set(Some(one_sentence_summary)),
            full_synopsis: Set(Some(full_synopsis)),
            world_setting: Set(Some(world_setting)),
            current_version: Set("1.0".to_string()),
            needs_part_outlines: Set(needs_part_outlines),
            total_chapters: Set(effective_total_chapters),
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
        .map_err(|e| e.to_string())?;
    }

    let character_count = apply_reverse_blueprint_characters(db, project_id, data, now).await?;
    let relationship_count =
        apply_reverse_blueprint_relationships(db, project_id, data, now).await?;

    Ok(json!({
        "blueprint_applied": true,
        "character_count": character_count,
        "relationship_count": relationship_count,
    }))
}

fn validate_reverse_blueprint_top_level_keys(data: &Value) -> Result<(), String> {
    reject_unknown_keys(
        data,
        &[
            "title",
            "genre",
            "style",
            "tone",
            "target_audience",
            "one_sentence_summary",
            "full_synopsis",
            "world_setting",
            "characters",
            "relationships",
            "total_chapters",
            "needs_part_outlines",
        ],
        "reverse_blueprint",
    )
}

fn optional_needs_part_outlines(data: &Value) -> Result<Option<bool>, String> {
    let Some(value) = data.get("needs_part_outlines") else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| "reverse_blueprint.needs_part_outlines 必须是布尔值".to_string())
}

fn validate_reverse_blueprint_sections(data: &Value, world_setting: &Value) -> Result<(), String> {
    validate_blueprint_world_setting_payload(world_setting).map_err(|err| err.to_string())?;
    let characters = data
        .get("characters")
        .ok_or_else(|| "reverse_blueprint 缺少必填数组字段 characters".to_string())?;
    validate_blueprint_characters_payload(characters).map_err(|err| err.to_string())?;
    let relationships = data
        .get("relationships")
        .ok_or_else(|| "reverse_blueprint 缺少必填数组字段 relationships".to_string())?;
    validate_blueprint_relationships_payload(relationships).map_err(|err| err.to_string())?;
    Ok(())
}

async fn apply_reverse_blueprint_characters(
    db: &impl ConnectionTrait,
    project_id: i64,
    data: &Value,
    now: NaiveDateTime,
) -> Result<usize, String> {
    let characters = required_array(data, "characters", "reverse_blueprint")?;

    let mut drafts = Vec::with_capacity(characters.len());
    for (index, item) in characters.iter().enumerate() {
        let context = format!("reverse_blueprint.characters[{index}]");
        let name = required_string(item, "name", &context)?;
        let character_type = required_string(item, "character_type", &context)?;
        let gender = required_string(item, "gender", &context)?;
        let age = required_string(item, "age", &context)?;
        let appearance = required_string(item, "appearance", &context)?;
        let personality = required_string(item, "personality", &context)?;
        let background = required_string(item, "background", &context)?;
        let motivation = required_string(item, "motivation", &context)?;
        let arc_summary = required_string(item, "arc_summary", &context)?;
        let relationship_to_protagonist =
            required_string(item, "relationship_to_protagonist", &context)?;
        drafts.push(BlueprintCharacterDraft {
            name,
            character_type,
            gender: Some(gender),
            age: Some(age),
            appearance: Some(appearance),
            personality: Some(personality),
            background: Some(background),
            motivation: Some(motivation),
            arc_summary: Some(arc_summary),
            relationship_to_protagonist: Some(relationship_to_protagonist),
            arc_stages: optional_field_clone(item, "arc_stages"),
        });
    }

    let count = drafts.len();
    sync_blueprint_characters(db, project_id, drafts, now)
        .await
        .map_err(|e| e.to_string())?;
    Ok(count)
}

async fn apply_reverse_blueprint_relationships(
    db: &impl ConnectionTrait,
    project_id: i64,
    data: &Value,
    now: NaiveDateTime,
) -> Result<usize, String> {
    let relationships = required_array(data, "relationships", "reverse_blueprint")?;

    let mut drafts = Vec::with_capacity(relationships.len());
    for (index, item) in relationships.iter().enumerate() {
        let context = format!("reverse_blueprint.relationships[{index}]");
        let character_a = required_string(item, "character_a", &context)?;
        let character_b = required_string(item, "character_b", &context)?;
        let relationship_type = required_string(item, "relationship_type", &context)?;
        let description = required_string(item, "description", &context)?;
        drafts.push(BlueprintRelationshipDraft {
            relationship_type,
            character_a,
            character_b,
            description: Some(description),
            strength: optional_i32(item, "strength", &context)?,
            evolution: optional_field_clone(item, "evolution"),
        });
    }

    let count = drafts.len();
    sync_blueprint_relationships(db, project_id, drafts, now)
        .await
        .map_err(|e| e.to_string())?;
    Ok(count)
}

fn resolve_reverse_blueprint_part_outline_need(total_chapters: Option<i32>) -> bool {
    total_chapters.is_some_and(needs_part_outlines_for_chapters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn world_setting() -> Value {
        json!({
            "premise": "旧城被星潮改写记忆。",
            "core_rules": "记忆会被周期性校验和封存。",
            "technology_or_magic": {
                "source": "星潮辐射",
                "rules": ["记忆可比对"],
                "limits": ["被污染记忆不可直接读取"],
                "costs": ["审校会损耗自我认知"]
            },
            "key_locations": [
                {"name": "旧城档案馆", "description": "保存纸档", "story_function": "调查据点"}
            ],
            "factions": [
                {"name": "调查局", "description": "管理记忆秩序", "agenda": "封锁真相", "representative_characters": ["林澈"]}
            ],
            "history_timeline": [
                {"period": "灾变元年", "event": "第一场星潮", "impact": "审校制度建立"}
            ],
            "culture_and_society": {
                "values": "稳定高于真实",
                "class_structure": "审校员掌握信息优势",
                "daily_life": "居民定期接受校验"
            },
            "conflict_sources": [
                {"name": "真相封锁", "description": "调查局隐藏源头", "stakes": "居民继续失去自我"}
            ],
            "mysteries_and_foreshadowing": [
                {"mystery": "星潮源头", "early_clues": ["重复档案"], "payoff_direction": "源头与主角家族有关"}
            ],
            "genre_promises": ["记忆悬疑"]
        })
    }

    fn character(name: &str, character_type: &str) -> Value {
        json!({
            "name": name,
            "character_type": character_type,
            "gender": "未知",
            "age": "未知",
            "appearance": "有明确视觉识别点",
            "personality": "谨慎、执拗",
            "background": "旧城居民",
            "motivation": "查明真相",
            "arc_summary": "从旁观者变为行动者",
            "relationship_to_protagonist": "本人或重要关系人",
            "arc_stages": {
                "start": "相信秩序",
                "core_wound": "失去重要记忆",
                "desire": "找回真相",
                "need": "面对代价",
                "secret": "未知",
                "voice_style": "克制直接",
                "turning_point": "发现档案被篡改",
                "end": "公开真相"
            }
        })
    }

    fn canonical_payload() -> Value {
        json!({
            "world_setting": world_setting(),
            "characters": [
                character("林澈", "protagonist"),
                character("沈砚", "antagonist"),
                character("许南星", "supporting")
            ],
            "relationships": [{
                "relationship_type": "rival",
                "character_a": "林澈",
                "character_b": "沈砚",
                "description": "一方追查真相，一方维护封锁。",
                "strength": 8,
                "evolution": {
                    "start": "上下级信任",
                    "tension": "调查方向被阻断",
                    "turning_points": ["档案馆被封"],
                    "hidden_truth": "沈砚知道失踪真相",
                    "story_function": "推动制度冲突",
                    "end": "彻底对立"
                }
            }]
        })
    }

    #[test]
    fn validate_reverse_blueprint_sections_accepts_canonical_payload() {
        let data = canonical_payload();

        validate_reverse_blueprint_sections(&data, data.get("world_setting").unwrap())
            .expect("canonical reverse blueprint sections should pass");
    }

    #[test]
    fn validate_reverse_blueprint_top_level_keys_rejects_unknown_fields() {
        let mut data = canonical_payload();
        data["legacy_notes"] = json!("旧字段");

        let err = validate_reverse_blueprint_top_level_keys(&data)
            .expect_err("unknown reverse blueprint fields must fail");

        assert!(err.contains("未知字段 legacy_notes"));
    }

    #[test]
    fn validate_reverse_blueprint_top_level_keys_accepts_part_outline_flag() {
        let mut data = canonical_payload();
        data["needs_part_outlines"] = json!(true);

        validate_reverse_blueprint_top_level_keys(&data)
            .expect("documented needs_part_outlines field should pass top-level key validation");
    }

    #[test]
    fn optional_needs_part_outlines_rejects_non_bool_values() {
        let data = json!({"needs_part_outlines": null});

        let err = optional_needs_part_outlines(&data)
            .expect_err("needs_part_outlines must be an explicit bool when present");

        assert!(err.contains("必须是布尔值"));
    }

    #[test]
    fn validate_reverse_blueprint_sections_rejects_thin_character_payload() {
        let mut data = canonical_payload();
        data["characters"][0]
            .as_object_mut()
            .unwrap()
            .remove("arc_stages");

        let err = validate_reverse_blueprint_sections(&data, data.get("world_setting").unwrap())
            .expect_err("missing arc_stages must fail");

        assert!(err.contains("arc_stages"));
    }

    #[test]
    fn validate_reverse_blueprint_sections_rejects_missing_relationship_strength() {
        let mut data = canonical_payload();
        data["relationships"][0]
            .as_object_mut()
            .unwrap()
            .remove("strength");

        let err = validate_reverse_blueprint_sections(&data, data.get("world_setting").unwrap())
            .expect_err("missing strength must fail");

        assert!(err.contains("strength"));
    }

    #[test]
    fn validate_reverse_blueprint_top_level_keys_rejects_derived_fields() {
        let mut data = canonical_payload();
        data["needs_part_outlines"] = json!(false);
        data["chapters_per_part"] = json!(25);
        data["chapter_outline"] = json!([]);

        let err = validate_reverse_blueprint_top_level_keys(&data)
            .expect_err("derived reverse blueprint fields must stay system-owned");

        assert!(err.contains("未知字段"));
    }
}
