use super::{UserRepo, UserRepoPkg};
use crate::{prelude::*, Kind};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileTreeRepo {
    // 此字段没有在前端使用
    pub repo: UserRepo,
    pub data: Vec<Data>,
    pub kinds_order: Vec<Kind>,
}

impl FileTreeRepo {
    pub fn dir(&self) -> Utf8PathBuf {
        Utf8PathBuf::from_iter(["repos", &self.repo.user, &self.repo.repo])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Data {
    #[serde(flatten)]
    pub pkg: UserRepoPkg,
    pub count: usize,
    pub raw_reports: Vec<RawReport>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawReport {
    pub file: Utf8PathBuf,
    pub features: String,
    pub count: usize,
    pub kinds: IndexMap<Kind, Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileTree {
    pub data: Vec<Data>,
    pub kinds_order: Vec<Kind>,
}
