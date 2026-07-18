use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(super) struct WebgalExportAsset {
    pub(super) script_name: String,
    pub(super) zip_path: String,
    pub(super) data_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct WebgalFigureKey {
    pub(super) profile_id: i64,
    pub(super) expression: Option<String>,
    pub(super) pose: Option<String>,
}

#[derive(Default)]
pub(super) struct WebgalExportAssets {
    pub(super) scene_backgrounds: BTreeMap<i64, WebgalExportAsset>,
    pub(super) line_cgs: BTreeMap<i64, WebgalExportAsset>,
    pub(super) line_bgms: BTreeMap<i64, WebgalExportAsset>,
    pub(super) line_sfx: BTreeMap<i64, WebgalExportAsset>,
    pub(super) figures: BTreeMap<WebgalFigureKey, WebgalExportAsset>,
}
