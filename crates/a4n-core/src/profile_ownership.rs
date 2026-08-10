pub const PROFILE_OWNERSHIP_USER: &str = "user";
pub const PROFILE_OWNERSHIP_AI_SYNC: &str = "ai_sync";
pub const PROFILE_OWNERSHIP_LEGACY_UNKNOWN: &str = "legacy_unknown";

pub fn is_valid_profile_ownership(value: &str) -> bool {
    matches!(
        value,
        PROFILE_OWNERSHIP_USER | PROFILE_OWNERSHIP_AI_SYNC | PROFILE_OWNERSHIP_LEGACY_UNKNOWN
    )
}

pub fn legacy_unknown_profile_ownership() -> String {
    PROFILE_OWNERSHIP_LEGACY_UNKNOWN.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_ownership_accepts_only_supported_sources() {
        assert!(is_valid_profile_ownership(PROFILE_OWNERSHIP_USER));
        assert!(is_valid_profile_ownership(PROFILE_OWNERSHIP_AI_SYNC));
        assert!(is_valid_profile_ownership(PROFILE_OWNERSHIP_LEGACY_UNKNOWN));
        assert!(!is_valid_profile_ownership("generated"));
    }
}
