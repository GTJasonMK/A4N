mod context;
mod contracts;
mod conversations;
mod project;

pub use context::inspiration_project_context_rows;
pub use contracts::{
    InspirationBlueprintRows, InspirationDbError, InspirationProjectContextRows,
    InspirationRecentChapterRow,
};
pub use conversations::{
    INSPIRATION_HISTORY_LIMIT, ensure_inspiration_conversation,
    insert_inspiration_exchange_in_transaction, inspiration_messages,
    inspiration_messages_for_connection, inspiration_messages_llm_context,
};

#[cfg(test)]
use a4n_db_models::models::project::novel_project;

#[cfg(test)]
mod tests;
