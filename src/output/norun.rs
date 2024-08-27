use super::RustToolchains;
use crate::Result;
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
        let list = self.targets.iter().map(|s| s.as_str());
        duct::cmd("rustup", ["target", "add"].into_iter().chain(list)).run()?;
        self.toolchains.setup()
    }
}

fn update_set(set: &mut IndexSet<String>, val: &str) {
    if set.get(val).is_none() {
        set.insert(val.to_owned());
    }
}
