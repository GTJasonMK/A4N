use a4n_core::models::project::novel_blueprint;
use chrono::NaiveDateTime;
use sea_orm::Set;
use serde_json::Value;

use crate::project::blueprint_restore::BlueprintRestoreError;

use super::scalars::{
    optional_blueprint_snapshot_json, optional_blueprint_snapshot_string,
    optional_positive_blueprint_snapshot_i32, required_blueprint_snapshot_bool,
    required_blueprint_snapshot_string, required_blueprint_snapshot_total_chapters,
};

#[derive(Debug)]
pub(in crate::project::blueprint_restore) struct RestoredBlueprintFields {
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
    pub(in crate::project::blueprint_restore) fn from_snapshot(
        data: &Value,
        version: &str,
    ) -> Result<Self, BlueprintRestoreError> {
        Ok(Self {
            title: required_blueprint_snapshot_string(data, "title", "蓝图版本")?,
            genre: required_blueprint_snapshot_string(data, "genre", "蓝图版本")?,
            style: optional_blueprint_snapshot_string(data, "style", "蓝图版本")?,
            tone: optional_blueprint_snapshot_string(data, "tone", "蓝图版本")?,
            target_audience: optional_blueprint_snapshot_string(
                data,
                "target_audience",
                "蓝图版本",
            )?,
            one_sentence_summary: optional_blueprint_snapshot_string(
                data,
                "one_sentence_summary",
                "蓝图版本",
            )?,
            full_synopsis: optional_blueprint_snapshot_string(data, "full_synopsis", "蓝图版本")?,
            world_setting: optional_blueprint_snapshot_json(data, "world_setting"),
            current_version: version.to_string(),
            needs_part_outlines: required_blueprint_snapshot_bool(
                data,
                "needs_part_outlines",
                "蓝图版本",
            )?,
            total_chapters: required_blueprint_snapshot_total_chapters(
                data,
                "total_chapters",
                "蓝图版本",
            )
            .map(Some)?,
            chapters_per_part: optional_positive_blueprint_snapshot_i32(
                data,
                "chapters_per_part",
                "蓝图版本",
            )?,
            avatar_svg: optional_blueprint_snapshot_string(data, "avatar_svg", "蓝图版本")?,
            avatar_animal: optional_blueprint_snapshot_string(data, "avatar_animal", "蓝图版本")?,
        })
    }

    pub(in crate::project::blueprint_restore) fn apply_to_existing(
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

    pub(in crate::project::blueprint_restore) fn into_active_model(
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
