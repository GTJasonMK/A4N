#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalgameTargetContext {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub scene_id: i64,
    pub line_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalgamePortraitSpecTargetInput {
    pub profile_id: i64,
    pub expression: String,
    pub pose: String,
    pub position_samples: Vec<String>,
    pub source_line_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalgamePortraitSpecTargetProfile {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
}
