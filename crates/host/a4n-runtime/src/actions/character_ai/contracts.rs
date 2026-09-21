pub use a4n_api_contract::character::{
    DiscoverCharacterProfilesInput, SyncAllCharacterProfilesInput, SyncAllCharacterProfilesResult,
    SyncCharacterProfileInput, SyncCharacterProfileResult,
};

#[derive(Debug, Clone, Copy)]
pub struct CharacterAiTaskScope {
    pub project_id: i64,
    pub total: u32,
}
