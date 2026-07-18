mod header;
mod providers;
mod settings;
mod shared;

use super::types::AllConfigsExport;

#[cfg(test)]
fn validate_all_configs_export_header(incoming: &AllConfigsExport) -> Result<(), String> {
    header::validate_all_configs_export_header(incoming)
}

pub(super) fn validate_all_configs_export(incoming: &AllConfigsExport) -> Result<(), String> {
    header::validate_all_configs_export_header(incoming)?;
    providers::validate_provider_rows(incoming)?;
    settings::validate_setting_rows(incoming)
}

pub(super) fn normalize_all_configs_export(incoming: &mut AllConfigsExport) -> Result<(), String> {
    providers::normalize_provider_rows(incoming)?;
    settings::normalize_setting_rows(incoming);
    Ok(())
}

#[cfg(test)]
mod tests;
