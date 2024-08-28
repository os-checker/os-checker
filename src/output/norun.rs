use super::RustToolchains;
use crate::Result;
use duct::cmd;
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

    pub fn setup(&self) -> Result<()> {
        let list = self.targets.iter().map(|s| s.as_str()).collect_vec();

        // install detected targets for host toolchain
        rustup_target_add(&list).run()?;

        // install detected targets for toolchain required by lockbud
        setup_lockbud(&list)?;

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

fn setup_lockbud(targets: &[&str]) -> Result<()> {
    let url = "https://github.com/BurtonQin/lockbud.git";
    let dir = "repos/lockbud";
    cmd!("git", "clone", url, dir).run()?;
    rustup_target_add(targets).dir(dir).run()?;
    cmd!("rustup", "show").dir(dir).run()?;
    cmd!("cargo", "install", "--path", ".", "--force")
        .dir(dir)
        .run()?;
    Ok(())
}
