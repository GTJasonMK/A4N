mod collection;
mod metadata;
mod positioning;

pub(in crate::actions::video_records) use collection::{
    assembly_clips, is_assembly_clip_video_type,
};
pub(super) use metadata::optional_positive_i64;
#[cfg(test)]
pub(in crate::actions::video_records) use metadata::{
    assembly_clip_shot_number, assembly_clip_sort_order,
};
#[cfg(test)]
pub(in crate::actions::video_records) use positioning::assembly_insert_metadata_with_position;
pub(in crate::actions::video_records) use positioning::{
    next_assembly_insert_sort_order, resolve_assembly_insert_position_metadata,
};
