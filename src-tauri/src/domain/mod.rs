//! Pure translation and learning domain contracts.
//!
//! This layer contains value objects and business rules. Runtime integration,
//! persistence, and network concerns belong outside this module.

pub mod language;
pub mod query_intent;
pub mod translation;
