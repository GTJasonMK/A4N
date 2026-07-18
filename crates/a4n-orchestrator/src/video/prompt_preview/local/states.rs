mod boundary;
mod storyboard;

use a4n_core::error::AppError;

pub(super) use boundary::{BoundaryFramePromptPreviewState, BoundaryFramePromptPreviewStateInput};
pub(super) use storyboard::StoryboardPromptPreviewState;

fn one_based_preview_number(index: usize, label: &str) -> Result<i32, AppError> {
    i32::try_from(index)
        .map_err(|_| AppError::InvalidArgument(format!("{label}索引超过 i32 范围：{index}")))?
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidArgument(format!("{label}编号超过 i32 范围")))
}
