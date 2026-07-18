use super::super::types::{AllConfigsExport, PromptRow};
use crate::{
    AppState,
    actions::prompt_records::{prompt_get_runtime_entry, prompt_update_many_runtime},
};
use tera::Tera;

pub(super) async fn import_prompt_rows(
    state: &AppState,
    rows: Vec<PromptRow>,
) -> Result<usize, String> {
    if rows.is_empty() {
        return Ok(0);
    }
    let count = rows.len();
    let updates = rows
        .into_iter()
        .map(|row| (row.name, row.content))
        .collect::<Vec<_>>();
    prompt_update_many_runtime(state, updates, "settings_import")
        .await
        .map_err(|err| format!("提示词批量导入失败：{err}"))?;
    Ok(count)
}

#[cfg(test)]
pub(super) async fn import_prompt_rows_for_engine(
    engine: &dyn a4n_prompt::engine::PromptEngine,
    rows: Vec<PromptRow>,
) -> Result<usize, String> {
    if rows.is_empty() {
        return Ok(0);
    }
    let count = rows.len();
    let updates = rows
        .into_iter()
        .map(|row| (row.name, row.content))
        .collect::<Vec<_>>();
    engine
        .update_many(updates, "settings_import")
        .await
        .map_err(|err| format!("提示词批量导入失败：{err}"))?;
    Ok(count)
}

pub(super) async fn prompt_import_snapshots(
    state: &AppState,
    rows: &[PromptRow],
) -> Result<Vec<(String, String)>, String> {
    let mut snapshots = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = prompt_get_runtime_entry(state, &row.name)
            .await
            .map_err(|_| format!("提示词 {} 导入失败：注册表中不存在", row.name))?;
        snapshots.push((row.name.clone(), entry.content));
    }
    Ok(snapshots)
}

#[cfg(test)]
pub(super) fn prompt_import_snapshots_for_engine(
    engine: &dyn a4n_prompt::engine::PromptEngine,
    rows: &[PromptRow],
) -> Result<Vec<(String, String)>, String> {
    rows.iter()
        .map(|row| {
            let entry = engine
                .get(&row.name)
                .map_err(|_| format!("提示词 {} 导入失败：注册表中不存在", row.name))?;
            Ok((row.name.clone(), entry.content))
        })
        .collect()
}

pub(super) async fn validate_prompt_import_targets(
    state: &AppState,
    incoming: &AllConfigsExport,
) -> Result<(), String> {
    for row in &incoming.prompts_modified {
        prompt_get_runtime_entry(state, &row.name)
            .await
            .map_err(|_| format!("提示词 {} 导入失败：注册表中不存在", row.name))?;
        validate_prompt_template(&row.name, &row.content)?;
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn validate_prompt_import_targets_for_engine(
    incoming: &AllConfigsExport,
    engine: &dyn a4n_prompt::engine::PromptEngine,
) -> Result<(), String> {
    for row in &incoming.prompts_modified {
        engine
            .get(&row.name)
            .map_err(|_| format!("提示词 {} 导入失败：注册表中不存在", row.name))?;
        validate_prompt_template(&row.name, &row.content)?;
    }
    Ok(())
}

fn validate_prompt_template(name: &str, content: &str) -> Result<(), String> {
    let mut tera = Tera::default();
    tera.add_raw_template(name, content)
        .map_err(|err| format!("提示词 {name} 模板编译失败：{err}"))?;
    Ok(())
}
