use super::pipeline_state::{pipeline_progress_from_layout, pipeline_status_from_layout};
use super::{MANGA_PAGE_PROMPT_TYPE, MangaService, PIPELINE_STATUS_PROMPT_TYPE};
use crate::generation::delete_generation_run_targets_for;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use chrono::Utc;
use sea_orm::*;
use serde_json::{Map, Value, json};

#[derive(Clone, Copy)]
pub(super) struct MangaPipelineStatusInput<'a> {
    pub chapter_id: i64,
    pub project_id: i64,
    pub pipeline_status: &'a str,
    pub stage: &'a str,
    pub current: i32,
    pub total: i32,
    pub message: &'a str,
    pub task_id: Option<&'a str>,
    pub allow_cancel_override: bool,
}

#[derive(Clone, Copy)]
pub(super) struct MangaPipelineCheckpointInput<'a> {
    pub status: MangaPipelineStatusInput<'a>,
    pub source_metadata: &'a Value,
    pub reference_assets_metadata: &'a Value,
    pub prompt_fingerprint: &'a Value,
    pub checkpoint_data: &'a Value,
}

impl MangaService {
    pub(super) async fn set_pipeline_status(
        &self,
        input: MangaPipelineStatusInput<'_>,
    ) -> Result<(), AppError> {
        let now = Utc::now().naive_utc();
        let existing_rows = chapter_manga_prompt::Entity::find()
            .filter(chapter_manga_prompt::Column::ProjectId.eq(input.project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(input.chapter_id))
            .filter(chapter_manga_prompt::Column::PromptType.eq(PIPELINE_STATUS_PROMPT_TYPE))
            .order_by_desc(chapter_manga_prompt::Column::UpdatedAt)
            .order_by_desc(chapter_manga_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        if !input.allow_cancel_override
            && let Some(existing) = existing_rows.first()
        {
            let existing_status =
                pipeline_status_from_layout(existing.layout.as_ref(), "漫画流水线状态")?;
            if existing_status == "cancelled" || existing.status == "cancelled" {
                return Err(AppError::Cancelled);
            }
        }

        let can_cancel = input.pipeline_status == "running";
        let can_resume = matches!(input.pipeline_status, "failed" | "cancelled");
        let prompt_status = match input.pipeline_status {
            "running" => "generating",
            other => other,
        };
        let existing_layout = existing_rows.first().and_then(|row| row.layout.as_ref());
        let mut layout_map = Map::new();
        layout_map.insert("pipeline_status".into(), json!(input.pipeline_status));
        layout_map.insert("stage".into(), json!(input.stage));
        layout_map.insert("current".into(), json!(input.current));
        layout_map.insert("total".into(), json!(input.total));
        layout_map.insert("message".into(), json!(input.message));
        layout_map.insert("task_id".into(), json!(input.task_id));
        layout_map.insert("can_cancel".into(), json!(can_cancel));
        layout_map.insert("can_resume".into(), json!(can_resume));
        layout_map.insert("updated_at".into(), json!(now.to_string()));
        if let Some(existing_layout) = existing_layout {
            for key in [
                "checkpoint_data",
                "source",
                "reference_assets",
                "prompt_fingerprint",
                "checkpoint_updated_at",
            ] {
                if let Some(value) = existing_layout.get(key) {
                    layout_map.insert(key.into(), value.clone());
                }
            }
            if input.task_id.is_none()
                && let Some(value) = existing_layout.get("task_id")
            {
                layout_map.insert("task_id".into(), value.clone());
            }
        }
        let layout = Some(Value::Object(layout_map));

        if let Some(existing) = existing_rows.first() {
            let keep_id = existing.id;
            let mut active: chapter_manga_prompt::ActiveModel = existing.clone().into();
            active.project_id = Set(input.project_id);
            active.page_number = Set(0);
            active.panel_number = Set(0);
            active.image_prompt = Set(input.message.to_string());
            active.dialogue_text = Set(None);
            active.characters_in_frame = Set(None);
            active.layout = Set(layout);
            active.generated_image_id = Set(None);
            active.camera_angle = Set(None);
            active.dialogue_position = Set(None);
            active.status = Set(prompt_status.to_string());
            active.updated_at = Set(now);
            active
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;

            let duplicate_ids = existing_rows
                .iter()
                .skip(1)
                .map(|row| row.id)
                .collect::<Vec<_>>();
            if !duplicate_ids.is_empty() {
                delete_generation_run_targets_for(
                    self.db.as_ref(),
                    "chapter_manga_prompt",
                    &duplicate_ids,
                )
                .await?;
                chapter_manga_prompt::Entity::delete_many()
                    .filter(chapter_manga_prompt::Column::Id.is_in(duplicate_ids))
                    .filter(chapter_manga_prompt::Column::Id.ne(keep_id))
                    .exec(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?;
            }
            return Ok(());
        }

        chapter_manga_prompt::ActiveModel {
            chapter_id: Set(input.chapter_id),
            project_id: Set(input.project_id),
            page_number: Set(0),
            panel_number: Set(0),
            prompt_type: Set(PIPELINE_STATUS_PROMPT_TYPE.to_string()),
            image_prompt: Set(input.message.to_string()),
            dialogue_text: Set(None),
            characters_in_frame: Set(None),
            layout: Set(layout),
            generated_image_id: Set(None),
            camera_angle: Set(None),
            dialogue_position: Set(None),
            status: Set(prompt_status.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn pipeline_status_row(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Option<chapter_manga_prompt::Model>, AppError> {
        chapter_manga_prompt::Entity::find()
            .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
            .filter(chapter_manga_prompt::Column::PromptType.eq(PIPELINE_STATUS_PROMPT_TYPE))
            .order_by_desc(chapter_manga_prompt::Column::UpdatedAt)
            .order_by_desc(chapter_manga_prompt::Column::Id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub(super) async fn ensure_pipeline_not_cancelled(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let status_row = self.pipeline_status_row(project_id, chapter_id).await?;

        let Some(status_row) = status_row else {
            return Ok(());
        };
        let pipeline_status =
            pipeline_status_from_layout(status_row.layout.as_ref(), "漫画流水线状态")?;
        if pipeline_status == "cancelled" || status_row.status == "cancelled" {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }

    pub(super) async fn pipeline_progress(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(i32, i32), AppError> {
        let status_row = self
            .pipeline_status_row(project_id, chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound("漫画流水线状态记录不存在".to_string()))?;
        let progress = pipeline_progress_from_layout(status_row.layout.as_ref(), "漫画流水线状态")?;
        Ok((progress.current, progress.total))
    }

    pub(super) async fn clear_pipeline_checkpoint(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        let Some(status_row) = self.pipeline_status_row(project_id, chapter_id).await? else {
            return Ok(());
        };
        let Some(Value::Object(mut layout)) = status_row.layout.clone() else {
            return Err(AppError::InvalidArgument(
                "漫画流水线状态.layout 必须是 JSON 对象".to_string(),
            ));
        };
        let mut changed = false;
        for key in [
            "checkpoint_data",
            "source",
            "reference_assets",
            "prompt_fingerprint",
            "checkpoint_updated_at",
        ] {
            changed |= layout.remove(key).is_some();
        }
        if !changed {
            return Ok(());
        }

        let mut active: chapter_manga_prompt::ActiveModel = status_row.into();
        active.layout = Set(Some(Value::Object(layout)));
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub(super) async fn save_pipeline_checkpoint(
        &self,
        input: MangaPipelineCheckpointInput<'_>,
    ) -> Result<(), AppError> {
        self.set_pipeline_status(input.status).await?;

        let status_row = self
            .pipeline_status_row(input.status.project_id, input.status.chapter_id)
            .await?
            .ok_or_else(|| {
                AppError::InvalidArgument("漫画流水线状态记录写入后不可读取".to_string())
            })?;
        let mut layout = match status_row.layout.clone() {
            Some(Value::Object(map)) => map,
            _ => {
                return Err(AppError::InvalidArgument(
                    "漫画流水线状态.layout 必须是 JSON 对象".to_string(),
                ));
            }
        };
        layout.insert("source".into(), input.source_metadata.clone());
        layout.insert(
            "reference_assets".into(),
            input.reference_assets_metadata.clone(),
        );
        layout.insert(
            "prompt_fingerprint".into(),
            input.prompt_fingerprint.clone(),
        );
        layout.insert("checkpoint_data".into(), input.checkpoint_data.clone());
        layout.insert(
            "checkpoint_updated_at".into(),
            json!(Utc::now().to_rfc3339()),
        );

        let mut active: chapter_manga_prompt::ActiveModel = status_row.into();
        active.layout = Set(Some(Value::Object(layout)));
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn cancel_pipeline(&self, project_id: i64, chapter_id: i64) -> Result<i64, AppError> {
        let prompts = chapter_manga_prompt::Entity::find()
            .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
            .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
            .filter(chapter_manga_prompt::Column::Status.is_in(["pending", "generating"]))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        let count = prompts.len() as i64;
        let now = Utc::now().naive_utc();
        for prompt in prompts {
            let mut active: chapter_manga_prompt::ActiveModel = prompt.into();
            active.status = Set("cancelled".to_string());
            active.updated_at = Set(now);
            active
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
        }

        let (current, total) = self.pipeline_progress(project_id, chapter_id).await?;

        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "cancelled",
            stage: "cancelled",
            current,
            total,
            message: "已请求取消漫画流水线",
            task_id: None,
            allow_cancel_override: true,
        })
        .await?;

        Ok(count)
    }
}
