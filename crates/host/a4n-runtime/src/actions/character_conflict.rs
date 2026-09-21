mod execution;
mod request;

pub use a4n_api_contract::character::{CharacterConflictCheckInput, CharacterConflictSummary};
pub use execution::{character_conflict_task_scope, check_character_conflict};
pub use request::preview_character_conflict_request;
