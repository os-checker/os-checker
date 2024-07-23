#[macro_use]
extern crate eyre;

#[macro_use]
extern crate tracing;

use eyre::Result;

/// 分析检查工具的结果
mod analysis;
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
    if let Err(err) = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::fs::File::create(log_file).unwrap())
        .with_ansi(false)
        .without_time()
        .try_init()
    {
        eprintln!("Logger already init: {err}");
    };
}
