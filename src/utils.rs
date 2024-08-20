use crate::Result;
use duct::cmd;
use eyre::ContextCompat;
use regex::Regex;
use std::{sync::LazyLock, time::Instant};

/// Perform an operation and get the execution time.
pub fn execution_time_ms<T>(op: impl FnOnce() -> T) -> (u64, T) {
    let now = Instant::now();
    let value = op();
    let duration_ms = now.elapsed().as_millis() as u64;
    (duration_ms, value)
}

pub struct GlobalInfo {
    pub rustc: Rustc,
}

pub static GLOBAL_INFO: LazyLock<GlobalInfo> = LazyLock::new(|| GlobalInfo {
    rustc: Rustc::new().unwrap(),
});

pub fn host_target_triple() -> &'static str {
    &GLOBAL_INFO.rustc.host
}

#[allow(dead_code)]
pub struct Rustc {
    version: String,
    commit_hash: String,
    commit_date: String,
    /// host target triple, usually as a default target
    host: String,
    release: String,
    llvm_version: String,
}

impl Rustc {
    // $ rustc -vV
    // rustc 1.82.0-nightly (91376f416 2024-08-12)
    // binary: rustc
    // commit-hash: 91376f416222a238227c84a848d168835ede2cc3
    // commit-date: 2024-08-12
    // host: x86_64-unknown-linux-gnu
    // release: 1.82.0-nightly
    // LLVM version: 19.1.0
    fn new() -> Result<Rustc> {
        fn parse(pat: &str, src: &str) -> Result<String> {
            let f = || format!("`{src}` doesn't contain `{pat}` pattern to get a value");
            Ok(Regex::new(&format!("^{pat}(.*)$"))
                .unwrap()
                .captures(src)
                .with_context(f)?
                .get(1)
                .with_context(f)?
                .as_str()
                .to_owned())
        }

        let src = &cmd!("rustc", "-vV").read()?;
        Ok(Rustc {
            version: parse("^rustc (.*)$", src)?,
            commit_hash: parse("^commit-hash: (.*)$", src)?,
            commit_date: parse("^commit-date: (.*)$", src)?,
            host: parse("^host: (.*)$", src)?,
            release: parse("^release: (.*)$", src)?,
            llvm_version: parse("^LLVM version: (.*)$", src)?,
        })
    }
}
