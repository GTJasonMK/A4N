use crate::generation::{
    GenerationDbError,
    types::{
        GENERATION_ENTITY_KINDS, GENERATION_ENTITY_ROLES, GENERATION_MEDIA_REF_ROLES,
        GENERATION_TEXT_ANCHOR_ROLES,
    },
    validation::normalize_one_of,
};

pub(in crate::generation) fn normalize_entity_kind(value: &str) -> Result<&str, GenerationDbError> {
    normalize_one_of(value, "entity_kind", GENERATION_ENTITY_KINDS)
}

pub(in crate::generation) fn normalize_text_anchor_role(
    value: &str,
) -> Result<&str, GenerationDbError> {
    normalize_one_of(value, "text_anchor_role", GENERATION_TEXT_ANCHOR_ROLES)
}

pub(in crate::generation) fn normalize_entity_role(value: &str) -> Result<&str, GenerationDbError> {
    normalize_one_of(value, "entity_role", GENERATION_ENTITY_ROLES)
}

pub(in crate::generation) fn normalize_media_ref_role(
    value: &str,
) -> Result<&str, GenerationDbError> {
    normalize_one_of(value, "media_ref_role", GENERATION_MEDIA_REF_ROLES)
}
