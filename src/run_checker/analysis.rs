use super::*;
use ahash::{HashMap, HashMapExt};
use cargo_metadata::camino::Utf8Path;
use color_eyre::owo_colors::OwoColorize;
use std::path::Path;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Alignment, Modify, Style},
};

#[derive(Debug)]
pub struct Statistics {
    pkg: XString,
    /// 检查工具报告的不通过的数量（基于文件）
    count: Count,
    /// 总计
    total: Total,
}

impl Statistics {
    pub fn new(outputs: &[Output]) -> Vec<Statistics> {
        outputs
            .iter()
            .chunk_by(|out| out.package_name.clone())
            .into_iter()
            .map(|(pkg, outputs)| {
                // iterate over outputs from each checker
                let mut count = Count::default();
                let mut total = Total::default();
                for out in outputs {
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
                Statistics { pkg, count, total }
            })
            .collect()
    }

    /// 无任何不良检查结果
    pub fn check_fine(&self) -> bool {
        self.count.inner.is_empty()
    }

    pub fn table_of_count_of_kind(&self) -> String {
        let iter = self.count.count_on_kind.iter();
        let sorted = iter.sorted_by_key(|a| a.0).enumerate();
        let row = sorted.map(|(i, (k, v))| [(i + 1).to_string(), format!("{k:?}"), v.to_string()]);
        let header = std::iter::once([String::new(), "kind".into(), "count".into()]);
        let builder: Builder = header.chain(row).collect();

        let header = &self.pkg;
        #[cfg(not(test))]
        let header = header.bold().black().on_bright_yellow().to_string();

        format!(
            "{header} counts on kind\n{}",
            builder.build().with(Style::modern_rounded())
        )
    }

    pub fn table_of_count_of_file(&self) -> String {
        let mut outer = 0;
        let iter = self.count.count_on_file.iter();
        let sorted = iter.sorted_by_key(|a| a.0).enumerate();
        let row = sorted.map(|(i, (k, v))| {
            let inside = if k.is_relative() {
                String::from("true")
            } else {
                outer += 1;
                String::from("false")
            };
            [(i + 1).to_string(), k.to_string(), inside, v.to_string()]
        });
        let header = std::iter::once([
            String::new(),
            "file".into(),
            "inside".into(),
            "count".into(),
        ]);
        let builder: Builder = header.chain(row).collect();

        let header = &self.pkg;
        #[cfg(not(test))]
        let header = header.bold().black().on_bright_yellow().to_string();

        let outside = if outer == 0 {
            String::new()
        } else {
            let ratio = outer as f32 / self.count.count_on_file.len() as f32 * 100.0;
            format!(" ({outer} outside files: {ratio:.0}%)")
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
