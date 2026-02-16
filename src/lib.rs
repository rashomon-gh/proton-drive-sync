//! Proton Drive Sync - Rust implementation
//!
//! A CLI tool to sync local directories to Proton Drive cloud storage.

pub mod auth;
pub mod cli;
pub mod config;
pub mod dashboard;
pub mod db;
pub mod error;
pub mod logger;
pub mod paths;
pub mod processor;
pub mod proton;
pub mod queue;
pub mod sync;
pub mod types;
pub mod watcher;

pub use error::{Error, Result};
pub use types::*;
