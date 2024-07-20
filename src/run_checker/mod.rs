use crate::{
    layout::{Layout, Package},
    repo::{Action, CheckerTool, Config},
    Result,
};
use eyre::Context;

#[derive(Debug)]
pub struct Repo {
    layout: Layout,
    config: Config,
}

impl Repo {
    pub fn new(repo_root: &str, dirs_excluded: &[&str], config: Config) -> Result<Repo> {
        let layout = Layout::parse(repo_root, dirs_excluded)
            .with_context(|| eyre!("无法解析 `{repo_root}` 内的 Rust 项目布局"))?;
        Ok(Self { layout, config })
    }

    pub fn resolve(&self) -> Resolve {
        todo!()
    }
}

struct Resolve<'a> {
    package: Package<'a>,
    actions: Vec<(CheckerTool, &'a Action)>,
}

#[test]
fn repo() -> Result<()> {
    let yaml = "
arceos:
  all: true
  miri: false
";
    dbg!(Repo::new(
        "repos/arceos",
        &[],
        Config::from_yaml(yaml)?.pop().unwrap()
    )?);
    Ok(())
}
