mod fields;
mod page_pipeline;
mod page_plan;
mod page_prompts;
use a4n_domain::manga::MangaPreprocessingStep;

pub(crate) use fields::required_array_field;
pub(crate) use page_pipeline::{
    parse_manga_page_adaptation, parse_manga_page_slots, parse_manga_page_storyboard,
    parse_manga_page_storyboard_model_output,
};
pub(crate) use page_plan::parse_manga_page_plan;
pub(crate) use page_prompts::page_drawing_prompt_from_response;

#[cfg(test)]
mod tests;
pub(in crate::manga) fn output_schema(
    prompt_name: &str,
) -> Result<serde_json::Value, a4n_error::AppError> {
    use a4n_domain::{
        json_contract::typed_output_schema,
        manga::{MangaPageAdaptationV1, MangaPageDrawingPromptV2, MangaPageSlotsV1},
    };
    Ok(match prompt_name {
        "manga_preprocessing_facts" => MangaPreprocessingStep::Facts.output_schema(),
        "manga_preprocessing_speech" => MangaPreprocessingStep::Speech.output_schema(),
        "manga_preprocessing_visuals" => MangaPreprocessingStep::Visuals.output_schema(),
        "manga_preprocessing_synthesis" => MangaPreprocessingStep::Synthesis.output_schema(),
        "manga_page_slots" => typed_output_schema::<MangaPageSlotsV1>(),
        "manga_page_adaptation_plan" => typed_output_schema::<MangaPageAdaptationV1>(),
        "manga_page_storyboard" => page_pipeline::storyboard_output_schema(),
        "manga_page_reference_discovery" => {
            crate::manga::reference_assets::manga_reference_discovery_output_schema()
        }
        "manga_page_drawing_prompt" => typed_output_schema::<MangaPageDrawingPromptV2>(),
        _ => {
            return Err(a4n_error::AppError::InvalidArgument(format!(
                "未知漫画输出合同：{prompt_name}"
            )));
        }
    })
}

#[cfg(test)]
#[test]
fn model_contract_matrix() {
    for name in MangaPreprocessingStep::ALL
        .map(|step| step.prompt_name())
        .into_iter()
        .chain([
            "manga_page_slots",
            "manga_page_adaptation_plan",
            "manga_page_storyboard",
            "manga_page_reference_discovery",
            "manga_page_drawing_prompt",
        ])
    {
        a4n_test_support::model_contract::emit_schema(name, output_schema(name).unwrap());
    }
}
