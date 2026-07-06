use a4n_core::models::project::{novel_blueprint, novel_project, project_status_history};
use a4n_core::state::{status::ProjectStatus, validator::StateValidator};
use a4n_services::blueprint::{
    BlueprintCharacterDraft, BlueprintRelationshipDraft, sync_blueprint_characters,
    sync_blueprint_relationships, validate_blueprint_chapter_count,
};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use serde_json::Value;

#[derive(Debug)]
pub(super) struct RestoredBlueprintFields {
    title: String,
    genre: String,
    style: Option<String>,
    tone: Option<String>,
    target_audience: Option<String>,
    one_sentence_summary: Option<String>,
    full_synopsis: Option<String>,
    world_setting: Option<Value>,
    current_version: String,
    needs_part_outlines: bool,
    total_chapters: Option<i32>,
    chapters_per_part: Option<i32>,
    avatar_svg: Option<String>,
    avatar_animal: Option<String>,
}

impl RestoredBlueprintFields {
    pub(super) fn from_snapshot(data: &Value, version: &str) -> Result<Self, String> {
        Ok(Self {
            title: required_snapshot_string(data, "title", "蓝图版本")?,
            genre: required_snapshot_string(data, "genre", "蓝图版本")?,
            style: optional_snapshot_string(data, "style", "蓝图版本")?,
            tone: optional_snapshot_string(data, "tone", "蓝图版本")?,
            target_audience: optional_snapshot_string(data, "target_audience", "蓝图版本")?,
            one_sentence_summary: optional_snapshot_string(
                data,
                "one_sentence_summary",
                "蓝图版本",
            )?,
            full_synopsis: optional_snapshot_string(data, "full_synopsis", "蓝图版本")?,
            world_setting: optional_snapshot_json(data, "world_setting", "蓝图版本"),
            current_version: version.to_string(),
            needs_part_outlines: required_snapshot_bool(data, "needs_part_outlines", "蓝图版本")?,
            total_chapters: required_snapshot_total_chapters(data, "total_chapters", "蓝图版本")
                .map(Some)?,
            chapters_per_part: optional_positive_snapshot_i32(
                data,
                "chapters_per_part",
                "蓝图版本",
            )?,
            avatar_svg: optional_snapshot_string(data, "avatar_svg", "蓝图版本")?,
            avatar_animal: optional_snapshot_string(data, "avatar_animal", "蓝图版本")?,
        })
    }

    pub(super) fn apply_to_existing(
        &self,
        active: &mut novel_blueprint::ActiveModel,
        now: NaiveDateTime,
    ) {
        active.title = Set(self.title.clone());
        active.genre = Set(self.genre.clone());
        active.style = Set(self.style.clone());
        active.tone = Set(self.tone.clone());
        active.target_audience = Set(self.target_audience.clone());
        active.one_sentence_summary = Set(self.one_sentence_summary.clone());
        active.full_synopsis = Set(self.full_synopsis.clone());
        active.world_setting = Set(self.world_setting.clone());
        active.current_version = Set(self.current_version.clone());
        active.needs_part_outlines = Set(self.needs_part_outlines);
        active.total_chapters = Set(self.total_chapters);
        active.chapters_per_part = Set(self.chapters_per_part);
        active.avatar_svg = Set(self.avatar_svg.clone());
        active.avatar_animal = Set(self.avatar_animal.clone());
        active.updated_at = Set(now);
    }

    pub(super) fn into_active_model(
        self,
        project_id: i64,
        now: NaiveDateTime,
    ) -> novel_blueprint::ActiveModel {
        novel_blueprint::ActiveModel {
            project_id: Set(project_id),
            title: Set(self.title),
            genre: Set(self.genre),
            style: Set(self.style),
            tone: Set(self.tone),
            target_audience: Set(self.target_audience),
            one_sentence_summary: Set(self.one_sentence_summary),
            full_synopsis: Set(self.full_synopsis),
            world_setting: Set(self.world_setting),
            current_version: Set(self.current_version),
            needs_part_outlines: Set(self.needs_part_outlines),
            total_chapters: Set(self.total_chapters),
            chapters_per_part: Set(self.chapters_per_part),
            avatar_svg: Set(self.avatar_svg),
            avatar_animal: Set(self.avatar_animal),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
    }
}

pub(super) async fn restore_blueprint_character_snapshot<C>(
    db: &C,
    project_id: i64,
    snapshot: Option<&Value>,
    now: NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    if snapshot.is_null() {
        return Ok(());
    }
    let drafts = parse_character_drafts(snapshot)?;
    sync_blueprint_characters(db, project_id, drafts, now)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) async fn restore_blueprint_relationship_snapshot<C>(
    db: &C,
    project_id: i64,
    snapshot: Option<&Value>,
    now: NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    if snapshot.is_null() {
        return Ok(());
    }
    let drafts = parse_relationship_drafts(snapshot)?;
    sync_blueprint_relationships(db, project_id, drafts, now)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) async fn sync_project_after_blueprint_restore<C>(
    db: &C,
    project_id: i64,
    blueprint: &novel_blueprint::Model,
    data: &Value,
    now: NaiveDateTime,
) -> Result<Option<(String, String)>, String>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))?;
    let previous_status =
        StateValidator::parse_status(&project.status).map_err(|e| e.to_string())?;
    let next_status = if previous_status.ordinal() < ProjectStatus::BlueprintReady.ordinal() {
        ProjectStatus::BlueprintReady
    } else {
        previous_status
    };
    let total_chapters = blueprint
        .total_chapters
        .ok_or_else(|| "蓝图版本缺少 total_chapters，无法同步项目章节目标".to_string())?;

    let mut active: novel_project::ActiveModel = project.into();
    active.title = Set(blueprint.title.clone());
    active.status = Set(next_status.as_str().to_string());
    active.target_chapters = Set(total_chapters);
    active.total_chapters = Set(blueprint.total_chapters);
    active.chapters_per_part = Set(blueprint.chapters_per_part);
    active.raw_blueprint = Set(Some(
        serde_json::to_string_pretty(data).map_err(|e| e.to_string())?,
    ));
    active.updated_at = Set(now);
    active.update(db).await.map_err(|e| e.to_string())?;

    let status_change = if previous_status != next_status {
        let from = previous_status.as_str().to_string();
        let to = next_status.as_str().to_string();
        project_status_history::ActiveModel {
            project_id: Set(project_id),
            from_status: Set(from.clone()),
            to_status: Set(to.clone()),
            reason: Set(Some("蓝图版本恢复".to_string())),
            changed_by: Set(Some("system".to_string())),
            changed_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
        Some((from, to))
    } else {
        None
    };
    Ok(status_change)
}

fn parse_character_drafts(value: &Value) -> Result<Vec<BlueprintCharacterDraft>, String> {
    let array = value
        .as_array()
        .ok_or_else(|| "蓝图版本 characters 必须是数组".to_string())?;
    array
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let label = format!("蓝图版本 characters[{index}]");
            Ok(BlueprintCharacterDraft {
                name: required_snapshot_string(item, "name", &label)?,
                character_type: required_snapshot_string(item, "character_type", &label)?,
                gender: optional_snapshot_string(item, "gender", &label)?,
                age: optional_snapshot_string(item, "age", &label)?,
                appearance: optional_snapshot_string(item, "appearance", &label)?,
                personality: optional_snapshot_string(item, "personality", &label)?,
                background: optional_snapshot_string(item, "background", &label)?,
                motivation: optional_snapshot_string(item, "motivation", &label)?,
                arc_summary: optional_snapshot_string(item, "arc_summary", &label)?,
                relationship_to_protagonist: optional_snapshot_string(
                    item,
                    "relationship_to_protagonist",
                    &label,
                )?,
                arc_stages: optional_snapshot_json(item, "arc_stages", &label),
            })
        })
        .collect()
}

fn parse_relationship_drafts(value: &Value) -> Result<Vec<BlueprintRelationshipDraft>, String> {
    let array = value
        .as_array()
        .ok_or_else(|| "蓝图版本 relationships 必须是数组".to_string())?;
    array
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let label = format!("蓝图版本 relationships[{index}]");
            Ok(BlueprintRelationshipDraft {
                relationship_type: required_snapshot_string(item, "relationship_type", &label)?,
                character_a: required_snapshot_string(item, "character_a", &label)?,
                character_b: required_snapshot_string(item, "character_b", &label)?,
                description: optional_snapshot_string(item, "description", &label)?,
                strength: optional_snapshot_strength(item, "strength", &label)?,
                evolution: optional_snapshot_json(item, "evolution", &label),
            })
        })
        .collect()
}

fn required_snapshot_string(value: &Value, key: &str, label: &str) -> Result<String, String> {
    optional_snapshot_string(value, key, label)?
        .ok_or_else(|| format!("{label} 缺少必填字符串字段: {key}"))
}

fn optional_snapshot_string(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Option<String>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            let text = text.trim();
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(text.to_string()))
            }
        }
        Some(_) => Err(format!("{label}.{key} 必须是字符串")),
    }
}

fn required_snapshot_bool(value: &Value, key: &str, label: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{label} 缺少必填布尔字段: {key}"))
}

fn required_snapshot_i32(value: &Value, key: &str, label: &str) -> Result<i32, String> {
    optional_snapshot_i32(value, key, label)?
        .ok_or_else(|| format!("{label} 缺少必填整数字段: {key}"))
}

fn required_snapshot_total_chapters(value: &Value, key: &str, label: &str) -> Result<i32, String> {
    let total_chapters = required_snapshot_i32(value, key, label)?;
    validate_blueprint_chapter_count(total_chapters, &format!("{label}.{key}"))
        .map_err(|e| e.to_string())?;
    Ok(total_chapters)
}

fn optional_snapshot_i32(value: &Value, key: &str, label: &str) -> Result<Option<i32>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(raw) => {
            let number = raw
                .as_i64()
                .ok_or_else(|| format!("{label}.{key} 必须是整数"))?;
            i32::try_from(number)
                .map(Some)
                .map_err(|_| format!("{label}.{key} 超出 i32 范围: {number}"))
        }
    }
}

fn optional_positive_snapshot_i32(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Option<i32>, String> {
    let Some(number) = optional_snapshot_i32(value, key, label)? else {
        return Ok(None);
    };
    if number <= 0 {
        return Err(format!("{label}.{key} 必须为正整数"));
    }
    Ok(Some(number))
}

fn optional_snapshot_strength(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Option<i32>, String> {
    let Some(strength) = optional_snapshot_i32(value, key, label)? else {
        return Ok(None);
    };
    if !(1..=10).contains(&strength) {
        return Err(format!("{label}.{key} 必须在 1 到 10 之间"));
    }
    Ok(Some(strength))
}

fn optional_snapshot_json(value: &Value, key: &str, _label: &str) -> Option<Value> {
    value.get(key).filter(|item| !item.is_null()).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_restore_snapshot() -> Value {
        json!({
            "title": "星潮档案",
            "genre": "科幻奇幻",
            "needs_part_outlines": false,
            "total_chapters": 30,
            "chapters_per_part": 25
        })
    }

    #[test]
    fn restored_blueprint_fields_rejects_invalid_total_chapters() {
        let mut snapshot = valid_restore_snapshot();
        snapshot["total_chapters"] = json!(0);

        let err = RestoredBlueprintFields::from_snapshot(&snapshot, "1.0")
            .expect_err("invalid total_chapters must fail before restore");

        assert!(err.contains("5 到 10000"));
    }

    #[test]
    fn restored_blueprint_fields_rejects_non_positive_chapters_per_part() {
        let mut snapshot = valid_restore_snapshot();
        snapshot["chapters_per_part"] = json!(-1);

        let err = RestoredBlueprintFields::from_snapshot(&snapshot, "1.0")
            .expect_err("invalid chapters_per_part must fail before restore");

        assert!(err.contains("chapters_per_part 必须为正整数"));
    }

    #[test]
    fn parse_relationship_drafts_rejects_strength_out_of_range() {
        let err = parse_relationship_drafts(&json!([{
            "relationship_type": "friend",
            "character_a": "林澈",
            "character_b": "许灯",
            "strength": 11
        }]))
        .expect_err("relationship strength outside 1..=10 must fail");

        assert!(err.contains("strength 必须在 1 到 10 之间"));
    }
}
