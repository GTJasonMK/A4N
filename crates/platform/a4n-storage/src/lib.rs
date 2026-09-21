//! 应用数据目录、项目媒体文件和文件生命周期的唯一 owner。
//!
//! 本 crate 只处理本地文件系统边界，不依赖 AppConfig、数据库、Host 状态或业务编排。
//! 调用方负责把配置中的目录传入，并在数据库事务边界外完成业务决策。

pub mod files;

pub use files::paths;

pub use files::{
    create_unique_timestamped_file, data_file_record_facts, deletable_data_dir_path,
    deletable_data_file_path, inspect_data_file_facts, persist_created_media_file,
    promote_data_file, remove_created_file_on_error, remove_created_file_on_error_sync,
    remove_data_dir_in_data_dir, remove_data_file_in_data_dir, remove_existing_data_file,
    write_new_data_file,
};

pub use files::paths::{
    AppPaths, CanonicalDataPathError, ProjectPaths, canonicalize_data_file_path,
    canonicalize_path_in_data_dir, data_dir_relative_file_path, data_dir_relative_path,
    path_to_db_string,
};
