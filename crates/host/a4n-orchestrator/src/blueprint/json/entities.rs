mod characters;
mod relationships;

pub(in crate::blueprint::json) use characters::{parse_characters, parse_sparse_characters};
pub(in crate::blueprint::json) use relationships::{
    parse_relationships, parse_sparse_relationships,
};
