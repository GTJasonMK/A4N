#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::config::settings_all) struct DefaultStateRow {
    pub(in crate::config::settings_all) name: String,
    pub(in crate::config::settings_all) is_default: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::config::settings_all) enum DefaultImportPolicy {
    Required,
    PreserveIfConfigured,
}
