use crate::{layout::RustToolchain, Result};
use color_eyre::owo_colors::OwoColorize;
use duct::cmd;
use eyre::ContextCompat;
use indexmap::IndexMap;
use regex::Regex;
use serde::Serialize;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Serialize)]
pub struct RustToolchains {
    host: &'static Rustc,
    installed: Vec<RustToolchain>,
}

impl RustToolchains {
    /// NOTE: 该函数应该在得到所有 repo::Repo 之后调用。
    pub fn new() -> Self {
        RustToolchains {
            host: &GLOBAL.host,
            installed: {
                let map = GLOBAL.installed.lock().unwrap();
                // FIXME: 其实不需要 Global，但如果那样做的话，需要自定义 RustToochains
                // 的 Serialize 实现。
                map.keys().cloned().collect()
            },
        }
    }

    /// Components required by all repos except host.
    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.installed[1..]
            .iter()
            .flat_map(|val| val.components.as_deref())
            .flatten()
            .map(|s| &**s)
    }

    /// 进入每个 installed 搜集的目录，运行 `rustup show` 来安装仓库指定的工具链
    // NOTE: rustup show 可能将来改为 rustup ensure 之类的命令来明确安装工具链。
    pub fn setup(&self) -> Result<()> {
        for toolchain in &self.installed[1..] {
            // toml_path 带 rust-toolchain.toml，应去除
            let dir = toolchain.toml_path.parent().unwrap();
            let out = cmd!("rustup", "show").dir(dir).read()?;
            println!("\n{}:\n{out}\n", dir.yellow());
        }

        Ok(())
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
        // NOTE: 第 0 个是 host 工具链
        map.insert(host_rust_toolchain().unwrap(), 0);
        Global {
            host: Rustc::new().unwrap(),
            installed: Mutex::new(map),
        }
    }
}

/// 此函数打印主机工具链，尤其校验默认是否为 nightly，应尽可能早调用。
pub fn init_toolchain_info() {
    let global = &*GLOBAL;
    let host = &global.host;
    let index_map = &*global.installed.lock().unwrap();
    let installed = &index_map.get_index(0).unwrap().0;
    info!("host = {host:#?}\ninstalled = {installed:#?}");
}

/// 将工具链放入全局“数组”，并返回其索引。
/// 如果该工具链信息已经存在，则不会推入到数组，并返回已存在的那个索引。
pub fn push_toolchain(val: RustToolchain) -> usize {
    let map = &mut *GLOBAL.installed.lock().unwrap();
    let index = map.len();
    *map.entry(val).or_insert(index)
}

/// 通过索引获取工具链信息。
pub fn get_toolchain(index: usize, f: impl FnOnce(&RustToolchain)) {
    let map = &mut *GLOBAL.installed.lock().unwrap();
    if let Some((toolchain, _)) = map.get_index(index) {
        f(toolchain);
    }
}

/// 此函数为 +host_toolchain，而不是单纯的 host_toolchain。
/// 目前主要用于传递给 cargo，在主机的 nightly 工具链上使用 fmt。
pub fn host_toolchain() -> String {
    let mut channel = String::new();
    get_toolchain(0, |t| channel = format!("+{}", t.channel));
    channel
}

pub fn host_target_triple() -> &'static str {
    &GLOBAL.host.host
}

#[derive(Debug, Serialize)]
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

        let src = &cmd!("rustc", "-vV").read()?;
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

enum RustupList {
    Target,
    Component,
}

impl RustupList {
    fn name(self) -> &'static str {
        match self {
            RustupList::Target => "target",
            RustupList::Component => "component",
        }
    }
}

/// arg: target or component
fn get_installed(arg: RustupList) -> Result<Vec<String>> {
    let list = cmd!("rustup", arg.name(), "list").read()?;
    Ok(list
        .lines()
        .filter(|&l| l.ends_with("(installed)"))
        .filter_map(|l| Some(l.split_once(" ")?.0.to_owned()))
        .collect())
}

fn host_rust_toolchain() -> Result<RustToolchain> {
    let channel = cmd!("rustup", "default").read()?;
    // e.g. nightly-x86_64-unknown-linux-gnu (default)
    // nightly-2024-09-09-x86_64-unknown-linux-gnu (default)
    ensure!(
        channel.starts_with("nightly"),
        "host toolchain {channel:?} is not a nightly toolchain"
    );
    let mut toolchain = RustToolchain {
        channel: channel.trim_end_matches(" (default)").to_owned(),
        profile: None,
        targets: Some(get_installed(RustupList::Target)?),
        components: Some(get_installed(RustupList::Target)?),
        toml_path: Default::default(),
        install_clippy: false,
    };
    toolchain.check_components()?;
    toolchain.install_rustfmt()?;
    Ok(toolchain)
}

#[test]
fn test_host_rust_toolchain() -> Result<()> {
    dbg!(host_rust_toolchain()?);
    Ok(())
}
