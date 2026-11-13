//! CLIBM Core Library Crate (`clibm_core`)
//!
//! Exposes the interface-agnostic storage, canonical hashing, indexing,
//! versioning, plugin contract, and domain engine modules.

pub mod canonical;
pub mod cas;
pub mod cli;
pub mod engine;
pub mod index;
pub mod install;
pub mod models;
pub mod plugin;
pub mod plugin_sdk;
pub mod update;
pub mod versioning;
