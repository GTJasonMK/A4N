mod context;
mod contracts;
mod conversations;
mod project;

pub use context::inspiration_project_context_rows;
pub use contracts::{
    InspirationBlueprintRows, InspirationDbError, InspirationExchangeInsertInput,
    InspirationProjectContextRows,
};
pub use conversations::{
    get_or_create_inspiration_conversation, insert_inspiration_exchange, inspiration_messages,
};
pub use project::inspiration_project_by_id;

#[cfg(test)]
use a4n_core::models::project::novel_project;

#[cfg(test)]
mod tests;
