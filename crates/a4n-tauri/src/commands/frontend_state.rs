#[path = "frontend_state/commands.rs"]
pub(crate) mod commands;
#[path = "frontend_state/file.rs"]
mod file;
#[path = "frontend_state/rules.rs"]
mod rules;

use std::path::PathBuf;

#[derive(Clone)]
pub struct FrontendStatePath(pub PathBuf);
