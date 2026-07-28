mod mutations;
mod query;

pub use mutations::{
    create_llm_config, delete_llm_config, set_default_llm_config, update_llm_config,
};
pub use query::list_llm_config;
