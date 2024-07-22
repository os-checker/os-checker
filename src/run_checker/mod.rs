use crate::{
    layout::Layout,
    repo::{Config, Resolve},
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

    pub fn resolve(&self) -> Result<Vec<Resolve>> {
        self.config.resolve(self.layout.packages())
    }
}

#[test]
fn repo() -> Result<()> {
    let yaml = "
arceos:
  all: true
  miri: false
";
    let repo = &Repo::new("tmp/test-fmt", &[], Config::from_yaml(yaml)?.pop().unwrap())?;
    // let repo = &Repo::new("repos/arceos", &[], Config::from_yaml(yaml)?.pop().unwrap())?;
    let resolve = repo.resolve()?;
    for res in resolve.iter().take(4) {
        let out = res
            .expr
            .stderr_capture()
            .stdout_capture()
            .unchecked()
            .run()?;
        let stdout = std::str::from_utf8(&out.stdout)?;
        let stderr = std::str::from_utf8(&out.stderr)?;
        println!(
            "[{} with {:?} checking] exit status{}\nstdout={stdout}\nstderr={stderr}",
            res.package.name, res.checker, out.status
        );
    }

    Ok(())
}
