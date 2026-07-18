mod evidence;
mod parsing;
mod selection;

pub(in crate::optimize) use evidence::materialize_optimization_issue_evidence;
pub(in crate::optimize) use parsing::{
    parse_content_optimization_issues, parse_optimization_issues,
};
pub(in crate::optimize) use selection::select_optimization_issues;
