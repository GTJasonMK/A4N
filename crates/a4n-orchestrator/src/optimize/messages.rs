mod analysis_prompt;
mod chat_prompt;
mod contracts;
mod issues;
mod patch_prompt;

pub(in crate::optimize) use analysis_prompt::{
    build_content_optimization_user_message, is_content_optimization_analysis_prompt,
    optimization_dimensions,
};
pub(in crate::optimize) use chat_prompt::{
    build_content_optimization_chat_user_message, optimization_chat_history,
};
pub(in crate::optimize) use contracts::OptimizeIssue;
pub(in crate::optimize) use issues::{
    materialize_optimization_issue_evidence, parse_content_optimization_issues,
    parse_optimization_issues, select_optimization_issues,
};
pub(in crate::optimize) use patch_prompt::build_content_optimization_patch_user_message;
