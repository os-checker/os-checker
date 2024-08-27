use crate::{Result, XString};
use indexmap::IndexMap;
use serde::Serialize;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct Targets {
    inner: Vec<TargetInner>,
}

impl Targets {
    pub fn new() -> Self {
        let map = &*GLOBAL.targets.lock().unwrap();
        // FIXME: 自定义 Serialize 实现，就无需定义 Global 和这里的类型转换
        let inner = map
            .iter()
            .map(|(triple, info)| TargetInner {
                triple: triple.to_owned(),
                arch: info.arch.to_owned(),
            })
            .collect();
        Self { inner }
    }
}

#[derive(Debug, Serialize)]
struct TargetInner {
    triple: String,
    arch: XString,
}

static GLOBAL: LazyLock<Global> = LazyLock::new(Global::new);

struct Global {
    // NOTE: 必须保持 Key 的顺序不变化，因为索引已经分发出去了。
    // Key 为 target triple。
    targets: Mutex<IndexMap<String, TargetInfo>>,
}

pub fn push_target(target: String) -> usize {
    let map = &mut *GLOBAL.targets.lock().unwrap();
    if let Some(info) = map.get(&*target) {
        info.index
    } else {
        let index = map.len();
        let info = TargetInfo::new(index, &target);
        map.insert(target, info);
        index
    }
}

/// 通过索引获取 target。
pub fn get_target(index: usize, f: impl FnOnce(&str)) {
    let map = &mut *GLOBAL.targets.lock().unwrap();
    if let Some((target, _)) = map.get_index(index) {
        f(target);
    }
}

struct TargetInfo {
    /// triple 在 IndexMap 中第一次插入的位置索引（也是唯一的索引）
    index: usize,
    arch: XString,
}

impl TargetInfo {
    fn new(index: usize, target: &str) -> Self {
        TargetInfo {
            index,
            arch: arch(target),
        }
    }
}

/// 有多个 arch 信息。最简单的从 triple 中截取，但这会导致同一个架构出现不同细分的信息。
/// 因此如果将来要调整成标准架构信息，需要等 target_spec 可用。
fn arch(target: &str) -> XString {
    target
        .split_once("-")
        .map(|(arch, _)| arch.into())
        .unwrap_or_default()
}

impl Global {
    fn new() -> Self {
        let mut map = IndexMap::with_capacity(16);
        let host_target = super::host_target_triple();
        // NOTE: 第 0 个是 host target
        map.insert(host_target.to_owned(), TargetInfo::new(0, host_target));
        Self {
            targets: Mutex::new(map),
        }
    }
}
