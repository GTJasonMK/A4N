//! 项目级联删除与相关清理（阶段 11 收口：从 a4n-services::project 迁入）。

use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait,
};

use crate::generation;
use crate::project as project_db;

mod types;
pub use types::*;

mod chapter_generation;
pub(crate) use chapter_generation::*;

mod profile_scene;
pub(crate) use profile_scene::*;

mod cascade;
pub use cascade::*;

#[cfg(test)]
mod tests;
