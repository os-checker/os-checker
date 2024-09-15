use crate::{write_to_file, Result};
use os_checker_types::JsonOutput;
use serde::{Deserialize, Serialize};

use crate::utils::{group_by, repo_cmdidx};

#[derive(Debug, Serialize, Deserialize)]
pub struct PassCountRepo {
    /// 无诊断的仓库数量
    pass: usize,
    /// 总仓库数量
    total: usize,
}

impl PassCountRepo {
    pub fn zero() -> Self {
        PassCountRepo { pass: 0, total: 0 }
    }

    /// NOTE: 调用此函数的前提是每批 json 输出不存在相同的仓库。
    /// 违反这个前提，需要内部存储 set 去重。
    /// os-checker 的配置解析或者 batch 能保证不存在相同的仓库。
    /// 但如果做缓存，则不能保证这一条。
    pub fn update(&mut self, json: &JsonOutput) {
        let Self { pass, total } = pass_count_repo(json);
        self.pass += pass;
        self.total += total;
    }

    /// 只在获取所有数据之后调用此函数。
    pub fn write_to_file(&self) -> Result<()> {
        write_to_file("", "pass_count_repo", self)?;
        info!(pass_count_repo = ?self, "写入 pass_count_repo.json 成功");
        Ok(())
    }
}

fn pass_count_repo(json: &JsonOutput) -> PassCountRepo {
    let total = json.env.repos.len();
    let mut set = ahash::AHashSet::with_capacity(total);
    for d in &json.data {
        set.insert(repo_cmdidx(json, d.cmd_idx).repo);
    }
    let defect = set.len();
    if defect > total {
        panic!("出现诊断的仓库数量 {defect} 不应该大于总检查仓库数量 {total}");
    }
    let pass = total - defect;
    PassCountRepo { pass, total }
}
