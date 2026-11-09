use std::collections::HashMap;

use a4n_domain::manga::*;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;

use super::{call::build_manga_preview_call, contracts::*};
use crate::{OrchestratorContext, manga::pipeline::prompt::MangaPreprocessingMessages};

pub(super) async fn preview_preprocessing(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    prompt_renderer: &dyn PromptRenderer,
    workflow_run_id: Option<i64>,
    resume: bool,
    messages: &MangaPreprocessingMessages<'_>,
) -> Result<Vec<MangaPipelineRequestPreviewStep>, AppError> {
    let mut steps = Vec::new();
    let mut facts: Option<MangaPreprocessingFacts> = None;
    let mut speech: Option<MangaPreprocessingSpeech> = None;
    let mut visuals: Option<MangaPreprocessingVisuals> = None;
    for step in MangaPreprocessingStep::ALL {
        if (step != MangaPreprocessingStep::Facts && facts.is_none())
            || (step == MangaPreprocessingStep::Synthesis
                && (speech.is_none() || visuals.is_none()))
        {
            steps.push(MangaPipelineRequestPreviewStep {
                key: step.prompt_name().into(),
                title: step.title().into(),
                prompt_name: Some(step.prompt_name().into()),
                state: "waiting_dependencies".into(),
                note: Some("等待上游步骤完成后，使用其真实结果构建请求".into()),
                request: None,
            });
            continue;
        }
        let call = build_manga_preview_call(MangaPreviewCallInput {
            service,
            prompt_name: step.prompt_name(),
            workflow_run_id,
            call_key: step.call_key(chapter_id),
            call_order: step.order(),
            restore_from_checkpoint: resume,
            prompt_renderer,
            project_id,
            variables: &HashMap::new(),
            user_content: messages.build(
                step,
                facts.as_ref(),
                speech.as_ref(),
                visuals.as_ref(),
            )?,
            temperature: 0.3,
        })
        .await?;
        match call {
            MangaPreviewCall::Ready(request) => steps.push(MangaPipelineRequestPreviewStep::ready(
                step.prompt_name(),
                step.title(),
                step.prompt_name(),
                None,
                *request,
            )),
            MangaPreviewCall::Restored { parsed_output } => {
                match step {
                    MangaPreprocessingStep::Facts => {
                        let value: MangaPreprocessingFacts = serde_json::from_value(parsed_output)?;
                        value.validate().map_err(AppError::InvalidArgument)?;
                        facts = Some(value);
                    }
                    MangaPreprocessingStep::Speech => {
                        let value: MangaPreprocessingSpeech =
                            serde_json::from_value(parsed_output)?;
                        value
                            .validate(facts.as_ref().expect("checked facts dependency"))
                            .map_err(AppError::InvalidArgument)?;
                        speech = Some(value);
                    }
                    MangaPreprocessingStep::Visuals => {
                        let value: MangaPreprocessingVisuals =
                            serde_json::from_value(parsed_output)?;
                        value
                            .validate(facts.as_ref().expect("checked facts dependency"))
                            .map_err(AppError::InvalidArgument)?;
                        visuals = Some(value);
                    }
                    MangaPreprocessingStep::Synthesis => {
                        let value: MangaPreprocessingSynthesis =
                            serde_json::from_value(parsed_output)?;
                        value.validate().map_err(AppError::InvalidArgument)?;
                    }
                }
                steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                    step.prompt_name(),
                    step.title(),
                    step.prompt_name(),
                ));
            }
        }
    }
    Ok(steps)
}
