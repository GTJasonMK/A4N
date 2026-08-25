mod candidates;
mod mutations;
mod queries;
mod types;

pub use candidates::{
    chapter_video_boundary_frame_prompt_candidates,
    chapter_video_first_frame_chain_prompt_candidates, chapter_video_first_frame_prompt_candidates,
    chapter_video_prompts_for_batch_generation, chapter_video_storyboard_prompt_candidates,
};
pub use mutations::update_chapter_video_prompt_record;
pub(crate) use mutations::{
    delete_chapter_video_prompt_records, ensure_chapter_video_prompt_not_running,
    replace_chapter_video_prompt_record, update_chapter_video_prompt_metadata_record,
};
pub use queries::{
    chapter_video_prompt_by_id, chapter_video_prompt_ids_for_types,
    chapter_video_prompt_records_for_types, chapter_video_prompt_scope_by_id,
    chapter_video_prompt_segment_plan_records, chapter_video_prompt_timeline_records_for_chapter,
    chapter_video_prompt_video_binding_records_for_chapter, chapter_video_prompt_workflow_records,
    chapter_video_prompt_workflow_shot_records, chapter_video_prompts_for_chapter,
    completed_shared_segment_plan_id_for_source_hash,
};
pub(crate) use queries::{
    chapter_video_prompt_dependency_records, chapter_video_prompt_previous_frame_candidates,
    chapter_video_prompt_record, chapter_video_prompt_sibling_dependency_records,
    chapter_video_prompt_sibling_records, chapter_video_prompt_sync_candidates,
    frame_chapter_video_prompt_record,
};
pub(crate) use types::PromptDependencyRecord;
pub use types::{
    ChapterVideoPromptCandidateRecord, ChapterVideoPromptRecordFields,
    ChapterVideoPromptTimelineRecord, UpdateChapterVideoPromptRecordInput,
};

#[cfg(test)]
mod tests;
