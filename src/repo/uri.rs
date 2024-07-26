use crate::Result;
use cargo_metadata::camino::Utf8PathBuf;
use duct::cmd;
use eyre::ContextCompat;

#[derive(Debug)]
pub enum UriTag {
    Github(String),
    Url(String),
    Local(Utf8PathBuf),
}

#[derive(Debug)]
pub struct Uri {
    tag: UriTag,
    local: Utf8PathBuf,
    key: String,
}

impl Uri {
    /// 获取该代码库的本地路径：如果指定 Github 或者 Url，则调用 git 命令下载
    pub fn local_root_path(&self) -> Result<Utf8PathBuf> {
        let url = match &self.tag {
            UriTag::Github(user_repo) => format!("https://github.com/{user_repo}.git"),
            UriTag::Url(url) => url.clone(),
            UriTag::Local(p) => return Ok(p.clone()),
        };

        // FIXME: 这里的目标目录处于 repos 下，基本功能完善之后需要改动
        let target_dir = Utf8PathBuf::from("repos").join(&self.local);
        let output = cmd!("git", "clone", "--recursive", url, &target_dir).run()?;
        ensure!(
            output.status.success(),
            "git 获取 {:?} 失败\nstderr={}\nstdout={}",
            self.tag,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );

        Ok(target_dir)
    }
}

pub fn uri(key: String) -> Result<Uri> {
    let (local, tag) = match key.strip_prefix("file://") {
        Some(path) => {
            let path = Utf8PathBuf::from(path);
            let last = path.components().next_back();
            let name = last.with_context(|| format!("无法在路径 `{path}` 中找到最后的目录名"))?;
            (name.as_str().into(), UriTag::Local(path))
        }
        None => match key.matches('/').count() {
            0 => bail!(
                "{key} 不是正确的代码库来源；请指定以下一种格式：\
                 `file://localpath`；github 的 `user/repo`；完整的 git 仓库地址"
            ),
            1 => (
                key[key.rfind('/').unwrap() + 1..].into(),
                UriTag::Github(key.as_str().into()),
            ),
            _ => {
                let strip_git = key.trim_end_matches(".git");
                (
                    strip_git[strip_git.rfind('/').unwrap() + 1..].into(),
                    UriTag::Url(key.as_str().into()),
                )
            }
        },
    };
    Ok(Uri { tag, local, key })
}
