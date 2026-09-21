mod pipeline;
mod profiles;
mod script;

#[cfg(test)]
pub(in crate::galgame) use pipeline::galgame_pipeline_prompt_context_from_record;
pub(in crate::galgame) use pipeline::galgame_pipeline_status_summary;
pub(crate) use script::galgame_line_from_model;
#[cfg(test)]
pub(in crate::galgame) use script::galgame_media_binding_summaries;
#[cfg(test)]
pub(in crate::galgame) use script::galgame_script_detail_from_read_model;
pub(in crate::galgame) use script::{
    galgame_choice_from_model, galgame_media_binding_summary, galgame_scene_from_model,
};
