pub(super) mod error_codes;
pub const GENERATION_EVENT_SCHEMA_VERSION: u16 = 1;
pub const JS_SAFE_INTEGER_U64: u64 = 9_007_199_254_740_991;
pub const JS_SAFE_INTEGER_I64: i64 = 9_007_199_254_740_991;
pub const MAX_GENERATION_EVENT_BYTES: usize = 64 * 1024;
pub const MAX_TEXT_DELTA_BYTES: usize = 16 * 1024;
pub const MAX_GENERATION_SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;
const MAX_GENERATION_STREAMS: usize = 64;
const MAX_GENERATION_ARTIFACTS: usize = 4_096;
pub const DEFAULT_GENERATION_ACTIVE_SESSION_MAX: usize = 64;

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_STREAM_ID_BYTES: usize = 64;
const MAX_KEY_BYTES: usize = 256;
const MAX_MESSAGE_BYTES: usize = 2 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_FAILED_ITEM_KEYS: usize = 256;

#[cfg(test)]
mod tests;
mod types;
mod validation;

pub use types::*;
pub use validation::GenerationContractError;
