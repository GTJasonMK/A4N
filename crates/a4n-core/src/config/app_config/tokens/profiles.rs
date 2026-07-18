use super::contracts::LlmMaxTokensConfig;

pub mod llm_token_profiles {
    pub const CONNECTION_TEST: &str = "connection_test";
    pub const SHORT: &str = "short";
    pub const STANDARD: &str = "standard";
    pub const JSON_STANDARD: &str = "json_standard";
    pub const LONG: &str = "long";
    pub const EXTRA_LONG: &str = "extra_long";
    pub const CHAPTER_GENERATION: &str = "chapter_generation";
    pub const CHAPTER_ANALYSIS: &str = "chapter_analysis";
    pub const OUTLINE: &str = "outline";
    pub const BLUEPRINT: &str = "blueprint";
    pub const CHARACTER_DISCOVERY: &str = "character_discovery";
    pub const CHARACTER_SYNC: &str = "character_sync";
    pub const MEDIA_PROMPT: &str = "media_prompt";
}

impl LlmMaxTokensConfig {
    pub fn profile_values(&self) -> [(&'static str, usize); 13] {
        use llm_token_profiles::*;

        [
            (CONNECTION_TEST, self.connection_test),
            (SHORT, self.short),
            (STANDARD, self.standard),
            (JSON_STANDARD, self.json_standard),
            (LONG, self.long),
            (EXTRA_LONG, self.extra_long),
            (CHAPTER_GENERATION, self.chapter_generation),
            (CHAPTER_ANALYSIS, self.chapter_analysis),
            (OUTLINE, self.outline),
            (BLUEPRINT, self.blueprint),
            (CHARACTER_DISCOVERY, self.character_discovery),
            (CHARACTER_SYNC, self.character_sync),
            (MEDIA_PROMPT, self.media_prompt),
        ]
    }

    pub fn for_profile(&self, profile: &str) -> Option<usize> {
        use llm_token_profiles::*;

        Some(match profile {
            CONNECTION_TEST => self.connection_test,
            SHORT => self.short,
            STANDARD => self.standard,
            JSON_STANDARD => self.json_standard,
            LONG => self.long,
            EXTRA_LONG => self.extra_long,
            CHAPTER_GENERATION => self.chapter_generation,
            CHAPTER_ANALYSIS => self.chapter_analysis,
            OUTLINE => self.outline,
            BLUEPRINT => self.blueprint,
            CHARACTER_DISCOVERY => self.character_discovery,
            CHARACTER_SYNC => self.character_sync,
            MEDIA_PROMPT => self.media_prompt,
            _ => return None,
        })
    }
}
