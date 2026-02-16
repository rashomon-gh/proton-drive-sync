//! Logging setup

use crate::error::Result;
use std::path::PathBuf;
use tracing_subscriber::{
    fmt, prelude::*, registry, EnvFilter,
};

/// Initialize logging
pub fn init(debug: bool) -> Result<()> {
    let env_filter = if debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env()
            .add_directive(tracing::level_filters::LevelFilter::INFO.into())
    };

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

/// Initialize logging with file output
pub fn init_with_file(log_dir: PathBuf, debug: bool) -> Result<()> {
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "proton-drive-sync.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = if debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env()
            .add_directive(tracing::level_filters::LevelFilter::INFO.into())
    };

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(file_layer)
        .init();

    // Keep guard in scope to flush logs
    std::mem::forget(_guard);

    Ok(())
}
