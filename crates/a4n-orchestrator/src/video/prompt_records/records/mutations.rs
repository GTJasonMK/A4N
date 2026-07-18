use std::collections::BTreeSet;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
#[cfg(test)]
use serde_json::Value;

use crate::video::OrchestratorContext;
#[cfg(test)]
use crate::video::prompt_records::VideoPromptUpdateInput;
use crate::video::video_db_error;

#[cfg(test)]
use super::super::matching::db_video_prompt_update_input;

impl OrchestratorContext {
    #[cfg(test)]
    pub async fn update_prompt(
        &self,
        prompt_id: i64,
        visual_prompt: Option<&str>,
        motion_prompt: Option<Option<&str>>,
        negative_prompt: Option<Option<&str>>,
        duration_secs: Option<i32>,
        reference_image_ids_json: Option<Option<Value>>,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        self.update_prompt_inner(
            prompt_id,
            VideoPromptUpdateInput {
                visual_prompt,
                motion_prompt,
                negative_prompt,
                duration_secs,
                reference_image_ids_json,
            },
            false,
        )
        .await
    }

    #[cfg(test)]
    async fn update_prompt_inner(
        &self,
        prompt_id: i64,
        input: VideoPromptUpdateInput<'_>,
        allow_downstream: bool,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        a4n_db::video_prompt::update_chapter_video_prompt_with_policy(
            self.db.connection(),
            prompt_id,
            db_video_prompt_update_input(input),
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::video) async fn ensure_prompt_stage_mutable(
        &self,
        prompt: &chapter_video_prompt::Model,
        operation: &str,
    ) -> Result<(), AppError> {
        a4n_db::video_prompt::ensure_chapter_video_prompt_stage_mutable(
            self.db.connection(),
            prompt,
            operation,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::video) async fn ensure_prompt_stage_mutable_ignoring(
        &self,
        prompt: &chapter_video_prompt::Model,
        operation: &str,
        ignored_prompt_ids: &BTreeSet<i64>,
    ) -> Result<(), AppError> {
        a4n_db::video_prompt::ensure_chapter_video_prompt_stage_mutable_ignoring(
            self.db.connection(),
            prompt,
            operation,
            ignored_prompt_ids,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::video) async fn frame_prompt_has_generated_image(
        &self,
        prompt: &chapter_video_prompt::Model,
    ) -> Result<bool, AppError> {
        a4n_db::video_prompt::chapter_video_prompt_has_generated_frame_image(
            self.db.connection(),
            prompt,
        )
        .await
        .map_err(video_db_error)
    }
}
