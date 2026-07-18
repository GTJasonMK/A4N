mod completion;
mod embedding;
mod events;
mod identity;
mod registration;
mod reporters;

pub(in crate::routes::commands) use completion::*;
pub(in crate::routes::commands) use embedding::*;
pub(in crate::routes::commands) use events::*;
pub(in crate::routes::commands) use identity::*;
pub(in crate::routes::commands) use registration::*;
pub(in crate::routes::commands) use reporters::*;

#[cfg(test)]
mod tests;
