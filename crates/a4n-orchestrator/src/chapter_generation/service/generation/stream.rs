mod chunks;
mod completion;
mod persistence;
mod prompt;
mod provider;
mod request;
mod restoration;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;

use super::super::ChapterGenerationService;
use super::ChapterGenerationStreamInput;
use completion::{StreamCompletionInput, finish_stream_chapter};
use persistence::{StreamCheckpointPersistenceInput, validate_and_persist_stream_checkpoint};
use prompt::build_stream_chapter_request;
use provider::stream_chapter_response;
use request::prepare_chapter_generation_request;
use restoration::restore_completed_stream_checkpoint;

impl ChapterGenerationService {
    /// 流式生成章节
    pub async fn generate_stream<F>(
        &self,
        input: ChapterGenerationStreamInput<'_, F>,
    ) -> Result<chapter::Model, AppError>
    where
        F: FnMut(&str, u32),
    {
        let ChapterGenerationStreamInput {
            project_id,
            chapter_number,
            chapter_outline,
            context_before,
            llm,
            prompt_renderer,
            variables,
            workflow_run_id,
            cancel_token,
            mut on_chunk,
        } = input;
        let session = self.begin_generation(project_id, chapter_number).await?;
        let request = match build_stream_chapter_request(
            project_id,
            chapter_number,
            chapter_outline,
            context_before,
            prompt_renderer,
            variables,
        )
        .await
        {
            Ok(request) => request,
            Err(err) => {
                self.restore_generation_status(session).await;
                return Err(err);
            }
        };
        let (request, request_trace, call_key, input_fingerprint_json, model_name) =
            prepare_chapter_generation_request(
                project_id,
                chapter_number,
                workflow_run_id,
                request,
                llm,
            )?;

        let (session, restored) = restore_completed_stream_checkpoint(
            self,
            session,
            workflow_run_id,
            &call_key,
            &input_fingerprint_json,
            chapter_number,
            chapter_outline,
            &mut on_chunk,
        )
        .await?;
        let restored_from_checkpoint = restored.is_some();
        let (accumulated, finish_reason) = if let Some(restored) = restored {
            restored
        } else {
            match stream_chapter_response(llm, request, cancel_token.as_ref(), &mut on_chunk).await
            {
                Ok(result) => result,
                Err(err) => {
                    self.restore_generation_status(session).await;
                    return Err(err);
                }
            }
        };
        if let Err(err) = validate_and_persist_stream_checkpoint(StreamCheckpointPersistenceInput {
            service: self,
            session: &session,
            workflow_run_id,
            call_key,
            chapter_number,
            chapter_outline,
            input_fingerprint_json,
            response_content: &accumulated,
            finish_reason: finish_reason.as_deref(),
            restored_from_checkpoint,
        })
        .await
        {
            self.restore_generation_status(session).await;
            return Err(err);
        }

        match finish_stream_chapter(StreamCompletionInput {
            service: self,
            session: &session,
            content: &accumulated,
            chapter_outline,
            request_trace,
            finish_reason,
            model_name,
        })
        .await
        {
            Ok(chapter) => Ok(chapter),
            Err(err) => {
                self.restore_generation_status(session).await;
                Err(err)
            }
        }
    }
}
