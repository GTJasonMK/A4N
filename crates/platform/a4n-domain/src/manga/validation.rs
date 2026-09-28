//! 漫画计划领域不变量校验。

use thiserror::Error;

use super::contracts::{MangaPageDrawingPromptV2, MangaPagePlanV4, MangaPageV4};

mod page;
mod plan;
mod rules;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("漫画计划字段 {path} 无效: {message}")]
pub struct MangaPlanValidationError {
    pub path: String,
    pub message: String,
}

impl MangaPagePlanV4 {
    pub fn validate(&self) -> Result<(), MangaPlanValidationError> {
        plan::validate_plan(self)
    }
}

impl MangaPageV4 {
    pub fn validate(&self) -> Result<(), MangaPlanValidationError> {
        page::validate_page(self, "page")
    }
}

impl MangaPageDrawingPromptV2 {
    pub fn validate(&self, expected_page_number: u32) -> Result<(), MangaPlanValidationError> {
        plan::validate_drawing_prompt(self, expected_page_number)
    }
}
