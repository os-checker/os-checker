use super::*;
use ahash::{HashMap, HashMapExt, RandomState};
use cargo_metadata::camino::{Utf8Component, Utf8Path};
use color_eyre::owo_colors::OwoColorize;
use compact_str::{format_compact, ToCompactString};
use serde::Serialize;
use std::{borrow::Cow, collections::BTreeMap, iter::once, path::Path, sync::Arc};
use tabled::{
    builder::Builder,
    settings::{object::Rows, Alignment, Modify, Style},
};

type IndexMap<K, V> = indexmap::map::IndexMap<K, V, RandomState>;

pub struct Statistics {
    /// package name
    pkg_name: XString,
    /// 所有检查工具的输出结果
    outputs: Arc<[Output]>,
    /// 检查工具报告的不通过的数量（基于文件）
    count: Count,
    /// 总计
    total: Total,
}

impl Statistics {
    pub fn new(outputs: Vec<Output>) -> Vec<Statistics> {
        outputs
            .into_iter()
            .chunk_by(|out| out.package_name.clone())
            .into_iter()
            .map(|(pkg_name, outputs)| {
                //  outputs from each checker
                let outputs: Arc<[_]> = outputs.collect();
                let mut count = Count::default();
                let mut total = Total::default();
                for out in &*outputs {
                    total.duration_ms += out.duration_ms;

                    // 由于路径的唯一性在这变得重要，需要提前归一化路径；两条思路：
                    // * package_name 暗含了库的根目录，因此需要把路径的根目录去掉（选择了这条）
                    // * 如果能保证都是绝对路径，那么不需要处理路径
                    let root = out.package_root.as_std_path();
                    match &out.parsed {
                        OutputParsed::Fmt(v) => count.push_unformatted(v, root),
                        OutputParsed::Clippy(v) => count.push_clippy(v, root),
                    }
                }
                count.update_on_kind_and_file();
                Statistics {
                    pkg_name,
                    outputs,
                    count,
                    total,
                }
            })
            .collect()
    }

    /// 无任何不良检查结果
    pub fn check_fine(&self) -> bool {
        self.count.inner.is_empty()
    }

    fn vec_of_count_on_kind(&self) -> Vec<(Kind, usize)> {
        self.count
            .count_on_kind
            .iter()
            .map(|(&k, &c)| (k, c))
            .sorted_by_key(|val| val.0)
            .collect()
    }

    pub fn json_node_children(
        &self,
        key: &mut usize,
        user: &XString,
        repo: &XString,
        raw_reports: &mut Vec<(usize, RawReports)>,
    ) -> TreeNode {
        *key += 1;
        raw_reports.push((*key, RawReports::Package(self.outputs.clone())));
        // 保持 Kind 的顺序（虽然这在 JSON 中没什么用，但如果人工检查的话，会更方便）
        let kinds: IndexMap<_, _> = self
            .vec_of_count_on_kind()
            .into_iter()
            // 不统计为格式化的行数：因为 `Clippy(Error|Warn)` 和 `Unformatted(File)`
            // 都统计地点数量，这在合计时处于同一维度，加入行数维度到合计会很奇怪。
            // 此外，行数报告可以单独放在 repo 详情数据里，目前这里为总览数据。
            .filter(|(k, _)| !matches!(k, Kind::Unformatted(Unformatted::Line)))
            .map(|(kind, count)| (format_compact!("{kind:?}"), count))
            .collect();
        TreeNode {
            key: key.to_compact_string(),
            data: Data {
                user: user.clone(),
                repo: repo.clone(),
                package: self.pkg_name.clone(),
                total_count: kinds.values().sum(), // 由于筛选了 Kind，这里不能直接使用 total_count
                kinds,
            },
            children: None,
        }
    }

    pub fn table_of_count_of_kind(&self) -> String {
        let sorted = self.vec_of_count_on_kind().into_iter().enumerate();
        let row = sorted.map(|(i, (k, v))| [(i + 1).to_string(), format!("{k:?}"), v.to_string()]);
        let header = once([String::new(), "kind".into(), "count".into()]);
        let builder: Builder = header.chain(row).collect();

        let header = &self.pkg_name;
        #[cfg(not(test))]
        let header = header.bold().black().on_bright_yellow().to_string();

        format!(
            "{header} counts on kind\n{}",
            builder.build().with(Style::modern_rounded())
        )
    }

    pub fn table_of_count_of_file(&self) -> String {
        // outer 时出现依赖机器的绝对路径，应该在测试情况下想办法消除
        fn outer_path(path: &Utf8Path) -> String {
            #[cfg(test)]
            {
                let paths = path.components().collect_vec();
                let len = paths.len();
                if len < 3 {
                    path.to_string()
                } else {
                    once(Utf8Component::Normal("OUTER"))
                        .chain(paths[len - 3..].iter().copied())
                        .collect::<Utf8PathBuf>()
                        .to_string()
                }
            }
            #[cfg(not(test))]
            path.to_string()
        }

        let mut outer = 0;
        let iter = self.count.count_on_file.iter();
        let sorted = iter.sorted_by_key(|a| a.0).enumerate();
        let row = sorted.map(|(i, (k, v))| {
            let (path, inside) = if k.is_relative() {
                (k.to_string(), String::from("true"))
            } else {
                outer += 1;
                (outer_path(k), String::from("false"))
            };
            [(i + 1).to_string(), path, inside, v.to_string()]
        });
        let header = once([
            String::new(),
            "file".into(),
            "inside".into(),
            "count".into(),
        ]);
        let builder: Builder = header.chain(row).collect();

        let header = &self.pkg_name;
        #[cfg(not(test))]
        let header = header.bold().black().on_bright_yellow().to_string();

        let outside = if outer == 0 {
            String::new()
        } else {
            let ratio = outer as f32 / self.count.count_on_file.len() as f32 * 100.0;
            format!(
                " ({outer} outer file{}: {ratio:.0}%)",
                if outer == 1 { "" } else { "s" }
            )
        };
        format!(
            "{header} counts on file{outside}\n{}",
            builder.build().with(Style::modern_rounded())
        )
    }
}

/// 如果可能地话，缩短绝对路径到相对路径。
fn strip_prefix<'f>(file: &'f Utf8Path, root: &Path) -> &'f Utf8Path {
    file.strip_prefix(root).unwrap_or(file)
}

#[derive(Debug, Default)]
pub struct Total {
    duration_ms: u64,
}

#[derive(Debug, Default)]
pub struct Count {
    inner: HashMap<CountKey, usize>,
    // based on inner
    count_on_kind: HashMap<Kind, usize>,
    // based on inner
    count_on_file: HashMap<Utf8PathBuf, usize>,
}

impl Count {
    fn update_on_kind_and_file(&mut self) {
        let additional = self.inner.len();
        self.count_on_kind.reserve(additional);
        self.count_on_file.reserve(additional);

        // 或许可以统计不同维度的总计？不过暂时还无法确定需要哪些维度总计，
        // 比如文件数量总计、报告地点数量总计、涉及源码行数总计。
        for (key, &count) in &self.inner {
            *self.count_on_kind.entry(key.kind).or_insert(0) += count;

            if let Some(get) = self.count_on_file.get_mut(&key.file) {
                *get += count;
            } else {
                self.count_on_file.insert(key.file.clone(), count);
            }
        }
    }

    fn push_unformatted(&mut self, v: &[FmtMessage], root: &Path) {
        for file in v {
            // NOTE: 该路径似乎是绝对路径
            let fpath = strip_prefix(&file.name, root);
            let count: usize = file
                .mismatches
                .iter()
                .map(|ele| (ele.original_end_line + 1 - ele.original_begin_line) as usize)
                .sum();
            let key_line = CountKey::unformatted_line(fpath);
            *self.inner.entry(key_line).or_insert(0) += count;

            let key_file = CountKey::unformatted_file(fpath);
            let len = file.mismatches.len();
            *self.inner.entry(key_file).or_insert(0) += len;
        }
    }

    fn push_clippy(&mut self, v: &[ClippyMessage], root: &Path) {
        for mes in v {
            // NOTE: 该路径似乎是相对路径，但为了防止意外的绝对路径，统一去除前缀。
            // 虽然指定了 --no-deps，但如果错误发生在依赖中，那么这个路径为绝对路径，并且可能无法缩短，
            // 因为它们不处于同一个前缀。因此，我们需要根据处理后的路径是绝对还是相对路径来判断该文件位于
            // package 内部还是外部。
            match &mes.tag {
                ClippyTag::WarnDetailed(paths) => {
                    for file in paths {
                        let fpath = strip_prefix(file, root);
                        let key = CountKey::clippy_warning(fpath);
                        *self.inner.entry(key).or_insert(0) += 1;
                    }
                }
                ClippyTag::ErrorDetailed(paths) => {
                    for file in paths {
                        let fpath = strip_prefix(file, root);
                        let key = CountKey::clippy_error(fpath);
                        *self.inner.entry(key).or_insert(0) += 1;
                    }
                }
                _ => (),
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct CountKey {
    file: Utf8PathBuf,
    kind: Kind,
}

impl CountKey {
    /// 表明一个文件中未格式化的地点数量
    fn unformatted_file(file: &Utf8Path) -> Self {
        Self {
            file: file.into(),
            kind: Kind::Unformatted(Unformatted::File),
        }
    }

    /// 表明一个文件中未格式化的总行数
    fn unformatted_line(file: &Utf8Path) -> Self {
        Self {
            file: file.into(),
            kind: Kind::Unformatted(Unformatted::Line),
        }
    }

    fn clippy_warning(file: &Utf8Path) -> Self {
        Self {
            file: file.into(),
            kind: Kind::Clippy(Rustc::Warn),
        }
    }

    fn clippy_error(file: &Utf8Path) -> Self {
        Self {
            file: file.into(),
            kind: Kind::Clippy(Rustc::Error),
        }
    }
}

/// The kind a checker reports.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Kind {
    /// fmt
    Unformatted(Unformatted),
    /// clippy
    Clippy(Rustc),
    /// miri
    UndefinedBehavior(Rustc),
    /// semver-checks
    SemverViolation,
    /// TODO
    Lockbud,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Unformatted {
    File,
    Line,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Rustc {
    Warn,
    Error,
}

#[derive(Serialize)]
pub struct Data {
    user: XString,
    repo: XString,
    package: XString,
    total_count: usize,
    #[serde(flatten)]
    kinds: IndexMap<XString, usize>,
}

#[derive(Serialize)]
pub struct TreeNode {
    key: XString,
    data: Data,
    children: Option<Vec<TreeNode>>,
}

impl TreeNode {
    /// Unlike [`Statistics::json_node_children`], this will compute a complete NodeTree with
    /// children.
    pub fn json_node(
        stat: &[Statistics],
        key: &mut usize,
        user: XString,
        repo: XString,
        raw_reports: &mut Vec<(usize, RawReports)>,
    ) -> TreeNode {
        let key_str = key.to_compact_string();
        raw_reports.push((
            *key,
            RawReports::Repo(stat.iter().map(|s| s.outputs.clone()).collect()),
        ));
        let children = stat
            .iter()
            .map(|s| s.json_node_children(key, &user, &repo, raw_reports))
            .collect_vec();
        let mut kinds = HashMap::with_capacity(8);
        for (k, c) in stat
            .iter()
            .flat_map(|s| s.count.count_on_kind.iter().map(|(&k, &c)| (k, c)))
        {
            kinds.entry(k).and_modify(|val| *val += c).or_insert(c);
        }
        let kinds: IndexMap<_, _> = kinds
            .into_iter()
            .sorted_by_key(|k| k.0)
            .map(|(kind, count)| (format_compact!("{kind:?}"), count))
            .collect();
        let total_count = children.iter().map(|c| c.data.total_count).sum();
        TreeNode {
            key: key_str,
            data: Data {
                user,
                repo,
                package: XString::default(),
                total_count,
                kinds,
            },
            children: Some(children),
        }
    }
}

/// Original outputs captured.
pub enum RawReports {
    /// Outputs for a package.
    Package(Arc<[Output]>),
    /// Outputs for a repo.
    Repo(Vec<Arc<[Output]>>),
}

impl RawReports {
    pub fn to_serialization(&self) -> RawReportsSerialization {
        let mut ser = RawReportsSerialization::new();
        let f = |out| ser.push(out);
        match self {
            RawReports::Package(p) => p.iter().for_each(f),
            RawReports::Repo(v) => v.iter().flat_map(|p| p.iter()).for_each(f),
        };
        ser
    }
}

#[derive(Serialize)]
pub struct RawReportsSerialization<'s> {
    fmt: BTreeMap<&'s Utf8Path, Vec<Cow<'s, str>>>,
    clippy_warn: BTreeMap<&'s Utf8Path, Vec<Cow<'s, str>>>,
    clippy_error: BTreeMap<&'s Utf8Path, Vec<Cow<'s, str>>>,
}

impl<'s> RawReportsSerialization<'s> {
    fn new() -> Self {
        Self {
            fmt: BTreeMap::new(),
            clippy_warn: BTreeMap::new(),
            clippy_error: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, out: &'s Output) {
        use std::fmt::Write;

        match &out.parsed {
            OutputParsed::Fmt(v) => {
                let add = "+";
                let minus = "-";
                for mes in v {
                    for mis in mes.mismatches.iter() {
                        let mut buf = String::with_capacity(128);
                        _ = writeln!(
                            &mut buf,
                            "file: {} (original lines from {} to {})",
                            mes.name, mis.original_begin_line, mis.original_end_line
                        );
                        for diff in prettydiff::diff_lines(&mis.original, &mis.expected).diff() {
                            match diff {
                                prettydiff::basic::DiffOp::Insert(s) => {
                                    for line in s {
                                        _ = writeln!(&mut buf, "{add}{line}");
                                    }
                                }
                                prettydiff::basic::DiffOp::Replace(a, b) => {
                                    for line in a {
                                        _ = writeln!(&mut buf, "{minus}{line}");
                                    }
                                    for line in b {
                                        _ = writeln!(&mut buf, "{add}{line}");
                                    }
                                    // println!("~{a:?}#{b:?}")
                                }
                                prettydiff::basic::DiffOp::Remove(s) => {
                                    for line in s {
                                        _ = writeln!(&mut buf, "{minus}{line}");
                                    }
                                }
                                prettydiff::basic::DiffOp::Equal(s) => {
                                    for line in s {
                                        _ = writeln!(&mut buf, " {line}");
                                    }
                                }
                            }
                        }
                        if let Some(v) = self.fmt.get_mut(&*mes.name) {
                            v.push(buf.into());
                        } else {
                            let mut v = Vec::with_capacity(mes.mismatches.len());
                            v.push(buf.into());
                            self.fmt.insert(&mes.name, v);
                        }
                    }
                }
            }
            OutputParsed::Clippy(v) => {
                for mes in v {
                    match &mes.tag {
                        ClippyTag::WarnDetailed(filepaths) => {
                            for f in filepaths {
                                if let CargoMessage::CompilerMessage(cmes) = &mes.inner {
                                    if let Some(render) = &cmes.message.rendered {
                                        if let Some(v) = self.clippy_warn.get_mut(&**f) {
                                            v.push(render.as_str().into());
                                        } else {
                                            let mut v = Vec::with_capacity(v.len());
                                            v.push(render.as_str().into());
                                            self.clippy_warn.insert(f, v);
                                        }
                                    }
                                }
                            }
                        }
                        ClippyTag::ErrorDetailed(filepaths) => {
                            for f in filepaths {
                                if let CargoMessage::CompilerMessage(cmes) = &mes.inner {
                                    if let Some(render) = &cmes.message.rendered {
                                        if let Some(v) = self.clippy_error.get_mut(&**f) {
                                            v.push(render.as_str().into());
                                        } else {
                                            let mut v = Vec::with_capacity(v.len());
                                            v.push(render.as_str().into());
                                            self.clippy_error.insert(f, v);
                                        }
                                    }
                                }
                            }
                        }

                        _ => (),
                    }
                }
            }
        };
    }
}
