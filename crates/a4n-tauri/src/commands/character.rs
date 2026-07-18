//! 角色管理命令。
//!
//! Tauri 层负责 command 签名和模块组织；角色档案、快照时间线等业务逻辑统一在
//! `a4n_runtime::host_api`。

pub(crate) mod attribute_events;
pub(crate) mod behavior;
pub(crate) mod deletion_mark;
pub(crate) mod implicit_review;
pub mod profile;
mod timeline;

pub use self::timeline::*;
