#[macro_use]
extern crate eyre;

#[macro_use]
extern crate tracing;

use compact_str::CompactString as XString;
use eyre::Result;

/// cli argument parsing
mod cli;
/// figure out the codebase layout
mod layout;
/// parse yaml file for repo configuration
mod repo;
/// run checker tools based on codebase layout and configuration
mod run_checker;

fn main() {
    logger_init();
}

fn logger_init() {
    if let Err(err) = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
    {
        eprintln!("Logger already init: {err}");
    };
}

#[cfg(test)]
#[allow(dead_code)]
fn test_logger_init(log_file: &str) {
    let is_file = std::env::var("LOG_FILE").is_ok();
    let fmt = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time();
    let init = if is_file {
        fmt.with_writer(std::fs::File::create(log_file).unwrap())
            .with_ansi(false)
            .try_init()
    } else {
        fmt.try_init()
    };
    if let Err(err) = init {
        eprintln!("Logger already init: {err}");
    };
}
