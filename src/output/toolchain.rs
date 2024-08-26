use crate::{layout::RustToolchain, Result};
use eyre::ContextCompat;
use indexmap::IndexMap;
use regex::Regex;
use serde::Serialize;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Serialize)]
#[serde(rename = "rust_toolchain")]
pub struct RustToochains {
    host: &'static Rustc,
    installed: Vec<RustToolchain>,
}

impl RustToochains {
    fn new() -> Self {
        RustToochains {
            host: &GLOBAL.host,
            installed: {
                let map = GLOBAL.installed.lock().unwrap();
                // FIXME: 其实不需要 Global，但如果那样做的话，需要自定义 RustToochains
                // 的 Serialize 实现。
                map.keys().cloned().collect()
            },
        }
    }
}

static GLOBAL: LazyLock<Global> = LazyLock::new(Global::new);

struct Global {
    host: Rustc,
    // NOTE: 必须保持 Key 的顺序不变化，因为索引已经分发出去了。
    installed: Mutex<IndexMap<RustToolchain, usize>>,
}

impl Global {
    fn new() -> Self {
        let mut map = IndexMap::with_capacity(16);
        Global {
            host: Rustc::new().unwrap(),
            installed: Mutex::new(map),
        }
    }
}

/// 将工具链放入全局“数组”，并返回其索引。
/// 如果该工具链信息已经存在，则不会推入到数组，并返回已存在的那个索引。
pub fn push_toolchain(val: RustToolchain) -> usize {
    let map = &mut *GLOBAL.installed.lock().unwrap();
    let index = map.len();
    *map.entry(val).or_insert(index)
}

pub fn host_target_triple() -> &'static str {
    &GLOBAL.host.host
}

#[derive(Debug, serde::Serialize)]
struct Rustc {
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
            let f = || format!("`{src:?}` doesn't contain `{pat}` pattern to get a value");
            Ok(Regex::new(pat)
                .unwrap()
                .captures(src)
                .with_context(f)?
                .get(1)
                .with_context(f)?
                .as_str()
                .to_owned())
        }

        let src = &duct::cmd!("rustc", "-vV").read()?;
        Ok(Rustc {
            version: parse("(?m)^rustc (.*)$", src)?,
            commit_hash: parse("(?m)^commit-hash: (.*)$", src)?,
            commit_date: parse("(?m)^commit-date: (.*)$", src)?,
            host: parse("(?m)^host: (.*)$", src)?,
            release: parse("(?m)^release: (.*)$", src)?,
            llvm_version: parse("(?m)^LLVM version: (.*)$", src)?,
        })
    }
}

#[test]
fn rustc_verbose() -> Result<()> {
    expect_test::expect![[r#"
        Rustc {
            version: "1.82.0-nightly (91376f416 2024-08-12)",
            commit_hash: "91376f416222a238227c84a848d168835ede2cc3",
            commit_date: "2024-08-12",
            host: "x86_64-unknown-linux-gnu",
            release: "1.82.0-nightly",
            llvm_version: "19.1.0",
        }
    "#]]
    .assert_debug_eq(&Rustc::new()?);
    Ok(())
}
