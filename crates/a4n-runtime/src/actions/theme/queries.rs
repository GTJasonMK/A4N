use std::collections::HashMap;

use super::ThemeSummary;
use super::projections::to_summary_with_resources;
use crate::AppState;
use a4n_core::error::AppError;

pub async fn list_themes_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<ThemeSummary>, AppError> {
    let themes = state
        .orchestrator()
        .list_themes_for_user_validated(user_id)
        .await?;
    let theme_ids = themes.iter().map(|theme| theme.id).collect::<Vec<_>>();
    let resources = state
        .orchestrator()
        .list_theme_resources_for_themes(&theme_ids)
        .await?;
    let mut resources_by_theme = HashMap::<i64, Vec<_>>::new();
    for resource in resources {
        resources_by_theme
            .entry(resource.theme_id)
            .or_default()
            .push(resource);
    }
    let mut summaries = Vec::with_capacity(themes.len());
    for theme in themes {
        let theme_id = theme.id;
        summaries.push(to_summary_with_resources(
            theme,
            resources_by_theme.remove(&theme_id).unwrap_or_default(),
        )?);
    }
    Ok(summaries)
}
