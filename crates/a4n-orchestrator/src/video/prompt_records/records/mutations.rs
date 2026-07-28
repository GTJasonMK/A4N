//! 视频提示词更新的测试辅助实现（仅 `cfg(test)`）。

#[cfg(test)]
use a4n_core::error::AppError;
#[cfg(test)]
use a4n_core::models::chapter::chapter_video_prompt;
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
use crate::video::OrchestratorContext;
#[cfg(test)]
use crate::video::prompt_records::VideoPromptUpdateInput;
#[cfg(test)]
use crate::video::video_db_error;

#[cfg(test)]
use super::super::matching::db_video_prompt_update_input;

#[cfg(test)]
impl OrchestratorContext {
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

    async fn update_prompt_inner(
        &self,
        prompt_id: i64,
        input: VideoPromptUpdateInput<'_>,
        allow_downstream: bool,
    ) -> Result<chapter_video_prompt::Model, AppError> {
        a4n_db::video_prompt::update_chapter_video_prompt_with_policy(
            self.database.connection(),
            prompt_id,
            db_video_prompt_update_input(input),
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }
}
