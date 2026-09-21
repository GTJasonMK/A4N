mod generic;
mod images;
mod json_records;
mod manga;
mod video;

pub(in crate::project_dependency_graph) use generic::{
    generic_node_detail_records, lineage_ref, optional_prompt_sources, prompt_source,
    structural_source_refs_for_node,
};
pub(in crate::project_dependency_graph) use images::generated_image_record;
pub(in crate::project_dependency_graph) use json_records::{
    db_record_result, db_record_result_with_related, db_related_record,
};
pub(in crate::project_dependency_graph) use manga::manga_prompt_record;
pub(in crate::project_dependency_graph) use video::{
    VideoPromptSourceRecord, video_prompt_record, video_prompt_sources,
    video_prompt_sources_from_record,
};

#[cfg(test)]
mod tests;
