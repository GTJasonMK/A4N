use super::super::types::AllConfigsExport;

pub(super) fn validate_all_configs_export_header(
    incoming: &AllConfigsExport,
) -> Result<(), String> {
    if incoming.version != "1.0" {
        return Err(format!(
            "version 不匹配：期望 '1.0'，收到 '{}'",
            incoming.version
        ));
    }
    if incoming.export_type != "all" {
        return Err(format!(
            "export_type 不匹配：期望 'all'，收到 '{}'",
            incoming.export_type
        ));
    }
    Ok(())
}
