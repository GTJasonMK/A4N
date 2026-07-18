use a4n_core::models::project::novel_blueprint;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::Value;

use super::super::outline_records::ProjectOutlineRecordError;
use super::contracts::ImportBlueprintUpsertInput;

fn is_blank_import_blueprint_text(value: Option<&str>) -> bool {
    value.map(str::trim).is_none_or(str::is_empty)
}

fn is_import_generated_world_setting(value: Option<&Value>) -> bool {
    value
        .and_then(|v| v.get("source"))
        .and_then(Value::as_str)
        .is_some_and(|source| source == "import_analysis")
}

pub async fn upsert_import_blueprint(
    db: &DatabaseConnection,
    input: ImportBlueprintUpsertInput,
) -> Result<novel_blueprint::Model, ProjectOutlineRecordError> {
    let now = Utc::now().naive_utc();
    let existing = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(input.project_id))
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;

    if let Some(existing) = existing {
        let should_update_one_sentence =
            is_blank_import_blueprint_text(existing.one_sentence_summary.as_deref());
        let should_update_full_synopsis =
            is_blank_import_blueprint_text(existing.full_synopsis.as_deref());
        let should_update_world_setting = existing.world_setting.is_none()
            || is_import_generated_world_setting(existing.world_setting.as_ref());

        let mut active: novel_blueprint::ActiveModel = existing.into();
        active.title = Set(input.title);
        if should_update_one_sentence {
            active.one_sentence_summary = Set(Some(input.one_sentence_summary));
        }
        if should_update_full_synopsis {
            active.full_synopsis = Set(Some(input.full_synopsis));
        }
        if should_update_world_setting {
            active.world_setting = Set(Some(input.world_setting));
        }
        active.total_chapters = Set(Some(input.total_chapters));
        active.needs_part_outlines = Set(input.needs_part_outlines);
        active.chapters_per_part = Set(input.chapters_per_part);
        active.updated_at = Set(now);
        active
            .update(db)
            .await
            .map_err(ProjectOutlineRecordError::from)
    } else {
        novel_blueprint::ActiveModel {
            project_id: Set(input.project_id),
            title: Set(input.title),
            genre: Set(input.genre),
            style: Set(None),
            tone: Set(None),
            target_audience: Set(None),
            one_sentence_summary: Set(Some(input.one_sentence_summary)),
            full_synopsis: Set(Some(input.full_synopsis)),
            world_setting: Set(Some(input.world_setting)),
            current_version: Set("1.0".to_string()),
            needs_part_outlines: Set(input.needs_part_outlines),
            total_chapters: Set(Some(input.total_chapters)),
            chapters_per_part: Set(input.chapters_per_part),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectOutlineRecordError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn import_blueprint_blank_text_keeps_placeholders_as_user_content() {
        assert!(is_blank_import_blueprint_text(None));
        assert!(is_blank_import_blueprint_text(Some("  ")));
        assert!(!is_blank_import_blueprint_text(Some("待补充")));
        assert!(!is_blank_import_blueprint_text(Some("（待补充）")));
    }

    #[test]
    fn import_blueprint_world_setting_only_overwrites_import_generated_value() {
        assert!(is_import_generated_world_setting(Some(&json!({
            "source": "import_analysis"
        }))));
        assert!(!is_import_generated_world_setting(Some(&json!({
            "source": "manual"
        }))));
        assert!(!is_import_generated_world_setting(Some(&json!({}))));
        assert!(!is_import_generated_world_setting(None));
    }
}
