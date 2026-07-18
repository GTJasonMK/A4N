mod completion;
mod copying;
mod deletion;
mod paths;
mod probe;
mod prompt;
mod reference_options;
mod upload;

pub(in crate::actions::video_records) use completion::mark_imported_video_completed;
#[cfg(test)]
pub(in crate::actions::video_records) use copying::copy_imported_video_file;
pub(super) use copying::persist_imported_video_file;
pub(in crate::actions::video_records) use deletion::{
    delete_video_record_error, delete_video_record_then_files, video_delete_file_paths,
};
#[cfg(test)]
pub(in crate::actions::video_records) use paths::import_video_source_path;
pub(super) use paths::{import_project_video_output_path, import_video_source};
pub(in crate::actions::video_records) use paths::{
    import_video_extension, import_video_output_path,
};
pub(in crate::actions::video_records) use probe::probe_video_properties;
#[cfg(test)]
pub(in crate::actions::video_records) use probe::video_probe_error_message;
pub(super) use prompt::{import_video_prompt, imported_video_prompt_text};
pub(in crate::actions::video_records) use reference_options::validate_reference_media_import_options;
#[cfg(test)]
pub(in crate::actions::video_records) use upload::persist_uploaded_video_source;
pub(in crate::actions::video_records) use upload::uploaded_video_source;
