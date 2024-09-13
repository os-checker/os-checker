use super::RustToolchains;
use crate::{
    utils::{git_clone, BASE_DIR_CHECKERS},
    Result,
};
use cargo_metadata::camino::Utf8PathBuf;
use duct::cmd;
use eyre::Context;
use indexmap::IndexSet;
use itertools::Itertools;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Norun {
    targets: IndexSet<String>,
    components: IndexSet<String>,
    toolchains: RustToolchains,
}

impl Norun {
    /// 此函数应在生成所有 repo::Repo 之后调用
    pub fn new() -> Self {
        let toolchains = RustToolchains::new();
        let mut components = IndexSet::<String>::with_capacity(16);
        for component in toolchains.components() {
            update_set(&mut components, component);
        }
        Self {
            targets: IndexSet::with_capacity(16),
            components,
            toolchains,
        }
    }

    pub fn update_target(&mut self, target: &str) {
        update_set(&mut self.targets, target);
    }

    #[instrument(level = "trace")]
    pub fn setup(&self) -> Result<()> {
        let list = self.targets.iter().map(|s| s.as_str()).collect_vec();

        if list.is_empty() {
            info!("不需要 target");
            return Ok(());
        }

        // install detected targets for host toolchain
        rustup_target_add(&list).run()?;

        // install detected targets for toolchain required by lockbud
        setup_lockbud(&list)?;

        // install toolchain and detected targets for mirai
        setup_mirai(&list)?;

        // install toolchains required by all repos
        self.toolchains.setup()
    }
}

fn update_set(set: &mut IndexSet<String>, val: &str) {
    if set.get(val).is_none() {
        set.insert(val.to_owned());
    }
}

fn rustup_target_add(targets: &[&str]) -> duct::Expression {
    cmd("rustup", ["target", "add"].iter().chain(targets))
}

#[instrument(level = "trace")]
fn setup_lockbud(targets: &[&str]) -> Result<()> {
    let url = "https://github.com/BurtonQin/lockbud.git";
    let dir = &Utf8PathBuf::from_iter([BASE_DIR_CHECKERS, "lockbud"]);
    git_clone(dir, url)?;
    rustup_target_add(targets).dir(dir).run()?;
    cmd!("rustup", "show").dir(dir).run()?;
    cmd!("cargo", "install", "--path", ".", "--force")
        .dir(dir)
        .run()?;
    Ok(())
}

#[instrument(level = "trace")]
fn setup_mirai(targets: &[&str]) -> Result<()> {
    const TOOLCHAIN: &str = "nightly-2023-12-30";
    const URL: &str =
        "https://github.com/os-checker/MIRAI/releases/download/v1.1.9/mirai-installer.sh";
    cmd!("curl", "--proto", "=https", "--tlsv1.2", "-LsSf", URL)
        .pipe(cmd!("sh"))
        .run()
        .with_context(|| "安装 mirai 失败")?;
    cmd!("rustup", "toolchain", "install", TOOLCHAIN).run()?;
    rustup_toolchain_add_target(&format!("+{TOOLCHAIN}"), targets)
        .run()
        .with_context(|| format!("在 {TOOLCHAIN} 上安装 {targets:?} 失败"))?;
    Ok(())
}

fn rustup_toolchain_add_target(toolchain: &str, targets: &[&str]) -> duct::Expression {
    cmd("rustup", [toolchain, "target", "add"].iter().chain(targets))
}
