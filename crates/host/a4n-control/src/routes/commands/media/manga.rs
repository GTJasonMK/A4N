mod pipeline;
mod preview;
mod records;

pub(in crate::routes::commands) use pipeline::{
    cancel_manga_pipeline, get_manga_pipeline_status, list_manga_pipeline_statuses,
    prepare_standalone_comic_source, resume_manga_pipeline, start_manga_pipeline,
};
pub(in crate::routes::commands) use preview::{
    preview_resume_manga_pipeline_request, preview_start_manga_pipeline_request,
};
pub(in crate::routes::commands) use records::{
    bind_manga_reference_image, delete_manga_by_chapter, delete_manga_page,
    delete_manga_pipeline_artifact, delete_manga_prompt, list_manga_page_prompts,
    list_manga_pipeline_artifacts, optimize_manga_prompt, preview_delete_manga_pipeline_artifact,
    save_manga_pipeline_artifact, update_manga_prompt,
};
