mod cleanup;
mod creation;
mod path_lifecycle;
pub mod paths;
mod persistence;

pub use cleanup::{
    remove_created_file_on_error, remove_created_file_on_error_sync, remove_existing_data_file,
};
pub use creation::create_unique_timestamped_file;
pub use path_lifecycle::{
    deletable_data_dir_path, deletable_data_file_path, remove_data_dir_in_data_dir,
    remove_data_file_in_data_dir, write_new_data_file,
};
pub use persistence::{
    data_file_record_facts, inspect_data_file_facts, persist_created_media_file, promote_data_file,
};
