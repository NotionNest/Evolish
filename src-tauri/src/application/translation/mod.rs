//! Runtime-agnostic translation use cases and ports.
//!
//! Concrete desktop, persistence, and provider adapters are assembled by the
//! infrastructure and runtime layers.

pub mod chunker;
pub mod classifier;
pub mod language_direction;
pub mod ports;
pub mod prompt_compiler;
pub mod provider_selection;
pub mod result_validator;
pub mod service;
pub mod supervisor;
