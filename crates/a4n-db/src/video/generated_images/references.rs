mod candidates;
mod context;
mod rules;
mod tail_frames;
mod validation;

pub use candidates::{
    ensure_image_chapter_belongs_to_project_record, video_reference_candidate_image_records,
};
pub(crate) use context::video_project_record;
pub use context::{
    VideoProjectIdentity, ensure_video_chapter_belongs_to_project_record,
    video_asset_project_ids_record, video_chapter_record,
};
pub use tail_frames::video_tail_frame_images;
pub(crate) use validation::validated_video_reference_image_records_batch;
pub use validation::{
    ensure_video_reference_images_available_record, validated_video_reference_image_records,
};

pub(super) use rules::{
    normalize_optional_image_record_text, required_image_record_text,
    should_delete_manga_page_image_fields, should_delete_manga_page_image_record,
    validate_generated_image_ai_score, validate_positive_image_i32, validate_positive_image_i64,
};
