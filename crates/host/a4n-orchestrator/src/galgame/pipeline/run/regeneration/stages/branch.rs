use a4n_error::AppError;
use serde_json::{Value, json};

use crate::galgame::{GalgameBranchDesign, branch_design_from_value};

pub(super) fn run_branch_design_stage() -> Result<(Value, GalgameBranchDesign), AppError> {
    let branch_plan_json = json!({"choices": []});
    let branch_plan = branch_design_from_value(branch_plan_json.clone(), "branch_plan")?;
    Ok((branch_plan_json, branch_plan))
}
