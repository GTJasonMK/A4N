mod artifacts;
mod deletion;
mod persistence;
mod rules;

pub use a4n_core::files::paths::{
    CanonicalDataPathError, canonicalize_data_file_path, canonicalize_path_in_data_dir,
    data_dir_relative_file_path, path_to_db_string,
};
pub(crate) use artifacts::inspect_generated_media_file;
pub use deletion::{
    deletable_data_dir_path, deletable_data_file_path, remove_created_file_on_error,
    remove_created_file_on_error_sync, remove_data_dir_in_data_dir, remove_data_file_in_data_dir,
};
pub use persistence::{
    data_file_record_facts, persist_created_media_file, probe_created_audio_duration,
    write_new_data_file,
};

#[cfg(test)]
mod tests;
