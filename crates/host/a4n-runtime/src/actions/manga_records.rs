mod optimization;
mod pipeline;
mod records;

pub use optimization::optimize_manga_prompt;
pub use pipeline::{
    cancel_manga_pipeline, get_manga_pipeline_status, list_manga_pipeline_statuses,
};
pub use records::{
    bind_manga_reference_image, delete_manga_by_chapter, delete_manga_page,
    delete_manga_pipeline_artifact, delete_manga_prompt, list_manga_page_prompts,
    list_manga_pipeline_artifacts, preview_delete_manga_pipeline_artifact,
    save_manga_pipeline_artifact, update_manga_prompt,
};

#[cfg(test)]
mod tests;
