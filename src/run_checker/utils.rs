use super::{
    CargoMessage, CargoSource, FmtMessage, Output as RawOutput, OutputParsed, RustcMessage,
    RustcTag,
};
use crate::{
    config::{CheckerTool, Resolve},
    db::{CacheRepo, CacheRepoKey, CacheValue, Db, InfoKeyValue, OutputDataInner},
    output::{Cmd, Data, Kind},
    Result,
};
use cargo_metadata::camino::Utf8Path;
use std::fmt::Write;

/// 将一次工具的检查命令推入一次 `Vec<Idx>`，并把原始输出全部推入 `Vec<Data>`。
pub fn push_idx_and_data(
    package_idx: usize,
    cache: &CacheValue,
    cmds: &mut Vec<Cmd>,
    data: &mut Vec<Data>,
) {
    let cmd_item = cache.to_cmd(package_idx);
    let cmd_idx = cmds.len();
    cmds.push(cmd_item);
    cache.append_to_data(cmd_idx, data);
}

impl RawOutput {
    pub fn to_cache(&self, db_repo: Option<DbRepo>) -> CacheValue {
        let root = &self.resolve.pkg_dir;

        // 由于路径的唯一性在这变得重要，需要提前归一化路径；两条思路：
        // * package_name 暗含了库的根目录，因此需要把路径的根目录去掉（选择了这条）
        // * 如果能保证都是绝对路径，那么不需要处理路径
        let data = match &self.parsed {
            OutputParsed::Fmt(v) => data_unformatted(v, root),
            OutputParsed::Clippy(v) => data_rustc(CheckerTool::Clippy, v, root),
            OutputParsed::Mirai(v) => data_rustc(CheckerTool::Mirai, v, root),
            OutputParsed::Lockbud(s) => data_lockbud(s),
            OutputParsed::Cargo { source, stderr } => data_cargo(source, stderr),
        };

        let cache = CacheValue::new(&self.resolve, self.duration_ms, data);
        if let Some(db_repo @ DbRepo { db, info, .. }) = db_repo {
            let key = db_repo.key(&self.resolve);
            // 写入命令缓存
            if let Err(err) = db.set_cache(&key, &cache) {
                error!(%err, ?key, "Unable to save the cache.");
            }
            // 写入键缓存
            if let Err(err) = info.append_cache_key(&key, db) {
                error!(%err, ?key, "Unable to save the cache.");
            }
        }
        cache
    }
}

#[derive(Clone, Copy)]
pub struct DbRepo<'a> {
    db: &'a Db,
    repo: &'a CacheRepo,
    info: &'a InfoKeyValue,
}

impl<'a> DbRepo<'a> {
    pub fn new(db: &'a Db, repo: &'a CacheRepo, info: &'a InfoKeyValue) -> DbRepo<'a> {
        DbRepo { db, repo, info }
    }

    pub fn key(self, resolve: &Resolve) -> CacheRepoKey {
        CacheRepoKey::new(self.repo, resolve)
    }

    pub fn cache(self, resolve: &Resolve) -> Result<Option<CacheValue>> {
        let key = self.key(resolve);
        self.db.get_cache(&key)
    }
}

fn data_cargo(source: &CargoSource, stderr: &str) -> Vec<OutputDataInner> {
    let file = match source {
        CargoSource::Checker(checker) => format!("(virtual) {}", checker.name()).into(),
        CargoSource::LayoutParseError(repo_root) => (&**repo_root).into(),
    };
    let data = OutputDataInner::new(file, Kind::Cargo, stderr.to_owned());
    vec![data]
}

fn data_lockbud(s: &str) -> Vec<OutputDataInner> {
    if s.is_empty() {
        Vec::new()
    } else {
        // FIXME: 目前 lockbud 无法良好地解析，需要等它实现 JSON 输出才能更可靠地区分哪种
        let kind = if s.contains(r#""possibility": "Possibly","#) {
            Kind::LockbudPossibly
        } else {
            Kind::LockbudProbably
        };
        let data = OutputDataInner::new("Not supported to display yet.".into(), kind, s.to_owned());
        vec![data]
    }
}

/// 尽可能缩短绝对路径到相对路径
fn strip_prefix<'f>(file: &'f Utf8Path, root: &Utf8Path) -> &'f Utf8Path {
    file.strip_prefix(root).unwrap_or(file)
}

fn raw_message_fmt(mes: &FmtMessage) -> impl '_ + ExactSizeIterator<Item = String> {
    mes.mismatches.iter().map(|mis| {
        let add = "+";
        let minus = "-";
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
        buf
    })
}

fn data_unformatted(v: &[FmtMessage], root: &Utf8Path) -> Vec<OutputDataInner> {
    let mut res = Vec::with_capacity(v.iter().map(|mes| mes.mismatches.len()).sum());
    for mes in v {
        // NOTE: 该路径似乎是绝对路径
        let file = strip_prefix(&mes.name, root);
        let iter = raw_message_fmt(mes)
            .map(|raw| OutputDataInner::new(file.to_owned(), Kind::Unformatted, raw));
        res.extend(iter);
    }
    res
}

fn data_rustc(checker: CheckerTool, v: &[RustcMessage], root: &Utf8Path) -> Vec<OutputDataInner> {
    fn raw_message_clippy(mes: &RustcMessage) -> Option<String> {
        if let CargoMessage::CompilerMessage(cmes) = &mes.inner {
            if let Some(render) = &cmes.message.rendered {
                return Some(render.clone());
            }
        }
        None
    }

    let mut res = Vec::with_capacity(128);

    for mes in v {
        // NOTE: 该路径似乎是相对路径，但为了防止意外的绝对路径，统一去除前缀。
        // 虽然指定了 --no-deps，但如果错误发生在依赖中，那么这个路径为绝对路径，并且可能无法缩短，
        // 因为它们不处于同一个前缀。因此，我们需要根据处理后的路径是绝对还是相对路径来判断该文件位于
        // package 内部还是外部。
        // NOTE: --no-deps 目前有 bug，见 https://github.com/os-checker/bug-MRE-clippy-no-deps
        match &mes.tag {
            RustcTag::WarnDetailed(paths) => {
                for path in paths {
                    let file = strip_prefix(path, root);
                    if let Some(raw) = raw_message_clippy(mes) {
                        let kind = match checker {
                            CheckerTool::Clippy => Kind::ClippyWarn,
                            CheckerTool::Mirai => Kind::Mirai,
                            _ => unreachable!("该函数只针对 rustc 风格的诊断"),
                        };
                        res.push(OutputDataInner::new(file.to_owned(), kind, raw));
                    };
                }
            }
            RustcTag::ErrorDetailed(paths) => {
                for path in paths {
                    let file = strip_prefix(path, root);
                    if let Some(raw) = raw_message_clippy(mes) {
                        let kind = match checker {
                            CheckerTool::Clippy => Kind::ClippyError,
                            CheckerTool::Mirai => Kind::Mirai,
                            _ => unreachable!("该函数只针对 rustc 风格的诊断"),
                        };
                        res.push(OutputDataInner::new(file.to_owned(), kind, raw));
                    };
                }
            }
            _ => (),
        }
    }
    res
}
