//! Pure theme projections shared by the host and theme read worker.

use a4n_api_contract::theme::{ThemePreferenceSummary, ThemeResourceSummary, ThemeSummary};
use a4n_domain::theme_records::{ThemePreferenceRecord, ThemeRecord, ThemeResourceRecord};
use a4n_domain::{ThemeAppearance, ThemeDocument, ThemeMode};
use a4n_error::AppError;

pub fn theme_update_changes(
    input: a4n_api_contract::theme::UpdateThemeInput,
    resources: Vec<a4n_domain::theme_edit::PersistThemeResourceInput>,
) -> Result<a4n_domain::theme_edit::ThemeUpdateChanges, AppError> {
    if !input.resource_uploads.is_empty() {
        return Err(AppError::InvalidArgument(
            "主题资源必须先由 Runtime 准备".into(),
        ));
    }
    if input.is_empty() && resources.is_empty() {
        return Err(AppError::InvalidArgument("主题更新至少需要一个字段".into()));
    }
    let name = input
        .name
        .map(|name| {
            let name = name.trim().to_string();
            if name.is_empty() {
                Err(AppError::InvalidArgument("主题名称不能为空".into()))
            } else {
                Ok(name)
            }
        })
        .transpose()?;
    if let Some(document) = &input.document {
        a4n_domain::validate_theme_document(document).map_err(AppError::InvalidArgument)?;
    }
    Ok(a4n_domain::theme_edit::ThemeUpdateChanges {
        name,
        mode: input.mode,
        document: input.document,
        resources,
        delete_resource_ids: input.delete_resource_ids,
    })
}

pub fn theme_summary_with_resources(
    theme: ThemeRecord,
    resources: Vec<ThemeResourceRecord>,
) -> Result<ThemeSummary, AppError> {
    let mode = theme.mode.parse::<ThemeMode>().map_err(|error| {
        AppError::ConfigError(format!("主题 {} 的持久化 mode 无效：{error}", theme.id))
    })?;
    let document =
        serde_json::from_value::<ThemeDocument>(theme.theme_document.clone()).map_err(|error| {
            AppError::ConfigError(format!(
                "主题 {} 的持久化 theme_document 无效：{error}",
                theme.id
            ))
        })?;
    Ok(ThemeSummary {
        id: theme.id,
        user_id: theme.user_id,
        name: theme.name,
        mode,
        document,
        resources: resources.into_iter().map(theme_resource_summary).collect(),
        is_system_default: theme.is_system_default,
        is_active: theme.is_active,
    })
}

pub fn theme_preference_summary(
    user_id: i64,
    preference: Option<ThemePreferenceRecord>,
) -> Result<ThemePreferenceSummary, AppError> {
    let appearance = preference
        .map(|preference| preference.appearance.parse::<ThemeAppearance>())
        .transpose()
        .map_err(|error| AppError::ConfigError(format!("主题外观偏好无效：{error}")))?
        .unwrap_or(ThemeAppearance::System);
    Ok(ThemePreferenceSummary {
        user_id,
        appearance,
    })
}

fn theme_resource_summary(resource: ThemeResourceRecord) -> ThemeResourceSummary {
    ThemeResourceSummary {
        id: resource.id,
        theme_id: resource.theme_id,
        resource_key: resource.resource_key,
        file_name: resource.file_name,
        mime_type: resource.mime_type,
        file_path: resource.file_path,
        byte_size: resource.byte_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme(mode: &str, document: serde_json::Value) -> ThemeRecord {
        ThemeRecord {
            id: 7,
            user_id: Some(11),
            name: "测试主题".to_string(),
            mode: mode.to_string(),
            theme_document: document,
            is_system_default: false,
            is_active: true,
        }
    }

    #[test]
    fn projection_rejects_invalid_persisted_mode() {
        let error = theme_summary_with_resources(
            theme(
                "sepia",
                serde_json::to_value(ThemeDocument::default()).unwrap(),
            ),
            Vec::new(),
        )
        .expect_err("invalid mode must remain visible");
        assert!(error.to_string().contains("持久化 mode 无效"));
    }

    #[test]
    fn preference_projection_defaults_to_system_and_rejects_invalid_values() {
        let summary = theme_preference_summary(11, None).expect("missing preference defaults");
        assert_eq!(summary.appearance, ThemeAppearance::System);

        let invalid = ThemePreferenceRecord {
            appearance: "sepia".to_string(),
        };
        assert!(theme_preference_summary(11, Some(invalid)).is_err());
    }

    #[test]
    fn projection_preserves_resource_metadata_without_checksum() {
        let resource = ThemeResourceRecord {
            id: 3,
            theme_id: 7,
            resource_key: "home".to_string(),
            file_name: "home.png".to_string(),
            mime_type: "image/png".to_string(),
            file_path: "themes/7/home.png".to_string(),
            sha256: "digest".to_string(),
            byte_size: 42,
        };
        let summary = theme_summary_with_resources(
            theme(
                "dark",
                serde_json::to_value(ThemeDocument::default()).unwrap(),
            ),
            vec![resource.clone()],
        )
        .expect("valid theme projects");
        assert_eq!(summary.resources[0].resource_key, "home");
        assert_eq!(resource.sha256, "digest");
    }
}
