use thiserror::Error;

use super::contracts::ChapterShootingScriptV1;

mod rules;
mod scene;
mod script;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("拍摄剧本字段 {path} 无效: {message}")]
pub struct ShootingScriptValidationError {
    pub path: String,
    pub message: String,
}

impl ChapterShootingScriptV1 {
    pub fn validate(&self) -> Result<(), ShootingScriptValidationError> {
        script::validate_script(self)
    }
}
