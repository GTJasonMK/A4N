use crate::workflow::workflow_checkpoint_contract_app_error;
use a4n_core::error::AppError;
use a4n_db::workflow;

use super::super::super::pipeline_prompt::{
    MangaLlmCallInput, load_manga_restored_llm_checkpoint, prepare_manga_llm_call,
};
use super::super::contracts::{MangaPreviewCall, MangaPreviewCallInput};

pub(in crate::manga::pipeline_preview) async fn build_manga_preview_call(
    input: MangaPreviewCallInput<'_>,
) -> Result<MangaPreviewCall, AppError> {
    let MangaPreviewCallInput {
        service,
        prompt_name,
        workflow_run_id,
        call_key,
        call_order,
        restore_from_checkpoint,
        prompt_renderer,
        project_id,
        variables,
        user_content,
        temperature,
    } = input;
    let prepared = prepare_manga_llm_call(MangaLlmCallInput {
        prompt_renderer,
        prompt_name,
        project_id,
        workflow_run_id,
        call_key,
        call_order,
        variables,
        user_content,
        temperature,
    })
    .await?;
    if restore_from_checkpoint {
        let workflow_run_id =
            workflow::required_llm_call_workflow_run_id(workflow_run_id, &prepared.call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        if let Some(restored) = load_manga_restored_llm_checkpoint(
            &service.database,
            workflow_run_id,
            &prepared.call_key,
            &prepared.input_fingerprint_json,
            prompt_name,
        )
        .await?
        {
            return Ok(MangaPreviewCall::Restored {
                parsed_output: restored.parsed_output,
            });
        }
    }
    Ok(MangaPreviewCall::Ready(Box::new(prepared.request)))
}
