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
    /// Recompute all counts, and sort.
    pub fn recount_and_sort(&mut self) {
        recount_and_sort(&mut self.data);
    }
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

/// Sort a vec of Data by count, pkg, and then file.
pub fn recount_and_sort(v: &mut Vec<Data>) {
    for data in &mut *v {
        let mut count = 0;
        for raw_reports in &data.raw_reports {
            for reports in raw_reports.kinds.values() {
                count += reports.len();
            }
        }
        data.count = count;
    }

    // FIXME: sort by features?

    // 对 pkg 的计数排序
    v.sort_unstable_by(|a, b| (b.count, &*a.pkg.pkg).cmp(&(a.count, &*b.pkg.pkg)));
    // 对文件的计数和文件名排序
    for pkg in v {
        pkg.raw_reports
            .sort_unstable_by(|a, b| (b.count, &*a.file).cmp(&(a.count, &*b.file)));
    }
}
