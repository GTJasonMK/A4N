use super::*;
use std::{
    net::TcpListener,
    process::{Child, Command},
};

include!("tests/readiness.rs");
include!("tests/core_reload.rs");
include!("tests/profile_reload.rs");
include!("tests/reader_reload.rs");
