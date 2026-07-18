#[derive(Debug, Clone, Default)]
pub struct MangaReferenceContext {
    pub character_portraits: Vec<MangaCharacterPortraitRef>,
}

#[derive(Debug, Clone)]
pub struct MangaCharacterPortraitRef {
    pub profile_id: i64,
    pub portrait_id: i64,
    pub character_name: String,
    pub image_path: String,
    pub prompt: String,
    pub portrait_type: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub is_primary: bool,
}
