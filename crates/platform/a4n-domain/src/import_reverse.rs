pub const WORKFLOW_KIND: &str = "import_reverse_engineer";
pub const WORKFLOW_MODE: &str = "reverse";

pub fn business_key(project_id: i64, prompt_name: &str) -> Result<String, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    let prompt_name = prompt_name.trim();
    if prompt_name.is_empty() {
        return Err("反推目标不能为空".to_string());
    }
    Ok(format!(
        "{WORKFLOW_KIND}:project:{project_id}:{prompt_name}:{WORKFLOW_MODE}"
    ))
}
