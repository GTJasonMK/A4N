use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::validate_positive_id;
use a4n_db::project::ProjectFamilyContext;

use super::scope::GalgamePersistValidationScope;

impl GalgamePersistValidationScope {
    pub(in crate::galgame::persistence) fn validate_source_snapshot_chapter(
        &self,
        context: &ProjectFamilyContext,
        source_snapshot_chapter_id: i64,
    ) -> Result<(), AppError> {
        validate_positive_id(source_snapshot_chapter_id, "source_snapshot_chapter_id")?;
        let snapshot_chapter = self
            .source_snapshot_chapters
            .get(&source_snapshot_chapter_id)
            .ok_or_else(|| {
                AppError::NotFound(format!("源快照章节不存在: {source_snapshot_chapter_id}"))
            })?;
        if snapshot_chapter.snapshot_id != context.source_snapshot.id {
            return Err(AppError::InvalidArgument(format!(
                "源快照章节 {} 不属于当前 Galgame 源快照",
                snapshot_chapter.id
            )));
        }
        Ok(())
    }

    pub(in crate::galgame::persistence) fn validate_speaker_profile(
        &self,
        context: &ProjectFamilyContext,
        speaker_profile_id: i64,
    ) -> Result<(), AppError> {
        validate_positive_id(speaker_profile_id, "speaker_profile_id")?;
        let profile = self
            .speaker_profiles
            .get(&speaker_profile_id)
            .ok_or_else(|| AppError::NotFound(format!("角色档案不存在: {speaker_profile_id}")))?;
        if profile.project_id != context.root_project.id {
            return Err(AppError::InvalidArgument(format!(
                "角色档案 {} 不属于当前 Galgame 根项目",
                profile.id
            )));
        }
        Ok(())
    }

    pub(in crate::galgame::persistence) fn validate_scene_participants_json(
        &self,
        context: &ProjectFamilyContext,
        value: &Value,
    ) -> Result<(), AppError> {
        let items = value.as_array().ok_or_else(|| {
            AppError::InvalidArgument("participants_json 必须是 JSON 数组".to_string())
        })?;
        for item in items {
            let object = item.as_object().ok_or_else(|| {
                AppError::InvalidArgument("Galgame 场 participants_json 项必须是对象".to_string())
            })?;
            let Some(profile_id_value) = object.get("character_profile_id") else {
                continue;
            };
            let profile_id = profile_id_value
                .as_i64()
                .filter(|profile_id| *profile_id > 0)
                .ok_or_else(|| {
                    AppError::InvalidArgument(
                        "Galgame 场 participants_json.character_profile_id 必须是正整数"
                            .to_string(),
                    )
                })?;
            self.validate_speaker_profile(context, profile_id)?;
        }
        Ok(())
    }

    pub(in crate::galgame::persistence) fn validate_text_anchor(
        &self,
        context: &ProjectFamilyContext,
        text_anchor_id: i64,
        expected_source_snapshot_chapter_id: Option<i64>,
    ) -> Result<(), AppError> {
        validate_positive_id(text_anchor_id, "text_anchor_id")?;
        let anchor = self
            .text_anchors
            .get(&text_anchor_id)
            .ok_or_else(|| AppError::NotFound(format!("文本锚点不存在: {text_anchor_id}")))?;
        if anchor.project_id != context.root_project.id
            || anchor.source_snapshot_id != Some(context.source_snapshot.id)
            || anchor.source_snapshot_chapter_id.is_none()
        {
            return Err(AppError::InvalidArgument(format!(
                "文本锚点 {} 不属于当前 Galgame 源快照",
                anchor.id
            )));
        }
        if let Some(expected_source_snapshot_chapter_id) = expected_source_snapshot_chapter_id
            && anchor.source_snapshot_chapter_id != Some(expected_source_snapshot_chapter_id)
        {
            return Err(AppError::InvalidArgument(format!(
                "文本锚点 {} 不属于当前 Galgame 场源快照章节 {}",
                anchor.id, expected_source_snapshot_chapter_id
            )));
        }
        Ok(())
    }
}
