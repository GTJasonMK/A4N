mod completion;
mod copying;
mod deletion;
mod paths;
mod probe;
mod upload;

pub(in crate::actions::video_records) use completion::{
    enqueue_unadopted_video_file_cleanup, mark_imported_video_completed,
};
#[cfg(test)]
pub(in crate::actions::video_records) use copying::copy_imported_video_file;
pub(super) use copying::persist_imported_video_file;
pub(in crate::actions::video_records) use deletion::delete_video_record_error;
pub(in crate::actions::video_records) use paths::import_video_output_path;
#[cfg(test)]
pub(in crate::actions::video_records) use paths::import_video_source_path;
pub(super) use paths::{import_project_video_output_path, import_video_source};
pub(in crate::actions::video_records) use probe::probe_video_properties;
#[cfg(test)]
pub(in crate::actions::video_records) use probe::video_probe_error_message;
pub(crate) use upload::recover_expired_uploaded_video_sources;
pub(in crate::actions::video_records) use upload::uploaded_video_source;
#[cfg(test)]
pub(in crate::actions::video_records) use upload::{
    expired_uploaded_video_sources, persist_uploaded_video_source,
};
