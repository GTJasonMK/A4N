mod preview;
mod records;
mod task;

pub(in crate::routes::commands) use preview::{
    preview_apply_beat_mapping_request, preview_batch_generate_part_outlines_request,
    preview_continue_part_outlines_request, preview_generate_chapter_outlines_by_count_request,
    preview_generate_part_chapter_outlines_request, preview_generate_parts_request,
    preview_regenerate_chapter_outline_request, preview_regenerate_last_part_request,
    preview_regenerate_part_outline_request, preview_regenerate_parts_request,
};
pub(in crate::routes::commands) use records::{
    delete_latest_chapter_outlines, delete_latest_parts, get_chapter_outlines_pacing,
    get_generation_progress, list_chapter_outline_index, list_chapter_outlines, list_parts,
    save_chapter_outline, save_part_outline, sync_outline_materials, update_chapter_outline,
};
