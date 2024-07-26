use crate::Result;
use cargo_metadata::camino::Utf8PathBuf;

#[derive(Debug)]
pub enum Uri {
    Github(String),
    Url(String),
    Local(Utf8PathBuf),
}

pub fn uri(key: &str) -> Result<Uri> {
    Ok(match key.strip_prefix("file://") {
        Some(local) => Uri::Local(local.into()),
        None => match key.matches('/').count() {
            0 => bail!(
                "{key} 不是正确的代码库来源；请指定以下一种格式：\
                     `file://localpath`；github 的 `user/repo`；完整的 git 仓库地址"
            ),
            1 => Uri::Github(key.into()),
            _ => Uri::Url(key.into()),
        },
    })
}
