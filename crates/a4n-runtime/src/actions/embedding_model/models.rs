mod deletion;
mod info;
mod listing;
mod scope;

pub use deletion::{delete_local_embedding_model, delete_local_embedding_model_in_cache};
pub use info::{get_embedding_model_download_info, get_embedding_model_download_info_in_cache};
pub use listing::{list_local_embedding_models, list_local_embedding_models_in_cache};
pub use scope::{
    EMBEDDING_MODEL_DOWNLOAD_INITIAL_MESSAGE, EMBEDDING_MODEL_DOWNLOAD_TASK_TOTAL,
    normalize_optional_embedding_download_task_id, required_task_id,
};
