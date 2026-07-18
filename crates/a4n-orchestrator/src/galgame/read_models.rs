mod pipeline;
mod profiles;
mod script;

#[cfg(test)]
pub(in crate::galgame) use pipeline::galgame_pipeline_prompt_context_from_record;
#[cfg(test)]
pub(in crate::galgame) use script::galgame_media_binding_summaries;
#[cfg(test)]
pub(in crate::galgame) use script::galgame_script_detail_from_read_model;
