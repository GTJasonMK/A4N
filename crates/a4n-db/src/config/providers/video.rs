mod activity;
mod contracts;
mod records;
mod resolution;

pub use activity::active_generated_video_ids_for_config;
pub use contracts::VideoConfigRecordInput;
pub(crate) use records::insert_video_config_record;
pub use records::{
    create_video_config_record, delete_video_config_record, list_video_config_records,
    update_video_config_record, video_config_by_id,
};
pub use resolution::{default_video_config, resolve_video_config};
