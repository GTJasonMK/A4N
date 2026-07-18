use super::super::*;
use super::fixtures::{canonical_character_sync_value, parse_sync_value};
use serde_json::{Value, json};

include!("sync_response/parsing.rs");
include!("sync_response/chapter_drafts.rs");
include!("sync_response/deletion_candidates.rs");
