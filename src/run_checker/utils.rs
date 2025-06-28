use super::{
    CargoMessage, CargoSource, FmtMessage, Output as RawOutput, OutputParsed, RustcMessage,
    RustcTag,
};
use crate::{
    config::{CheckerTool, Resolve},
    db::{
        out::CacheLayout, CacheRepo, CacheRepoKey, CacheValue, Db, InfoKeyValue, OutputDataInner,
    },
    layout::Audit,
    output::{Cmd, Data, Kind},
    Result,
};
use cargo_metadata::camino::Utf8Path;
use std::{fmt::Write, sync::LazyLock};

struct Global {
    force_repo_check: bool,
    force_run_check: bool,
}

fn var(env: &str) -> Option<bool> {
    std::env::var(env)
        .map(|val| matches!(&*val, "true" | "1"))
        .ok()
}

static GLOBAL: LazyLock<Global> = LazyLock::new(|| Global {
    force_repo_check: var("FORCE_REPO_CHECK").unwrap_or(false),
    force_run_check: var("FORCE_RUN_CHECK").unwrap_or(false),
});

/// 当 os-checker 内部支持新检查时，将这个值设置为 true，
/// 来强制运行仓库检查（不影响已有的检查缓存结果）。
/// NOTE: cargo error 的检查结果总是在强制运行仓库检查时更新。
pub fn force_repo_check() -> bool {
    GLOBAL.force_repo_check
}

/// 当运行到 run_check 时，是否强制运行检查，不检查是否有缓存。
/// force_repo_check 会控制是否运行 run_check，而 force_run_check 会控制是否照顾缓存。
pub fn force_run_check() -> bool {
    GLOBAL.force_run_check
}

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
            OutputParsed::Audit(a) => data_audit(a, root),
            OutputParsed::Mirai(v) => data_rustc(CheckerTool::Mirai, v, root),
            OutputParsed::Lockbud(s) => data_lockbud(s),
            OutputParsed::AtomVChecker(s) => data_atomvchecker(s),
            OutputParsed::Rap(s) => data_rap(s),
            OutputParsed::Rudra(s) => data_rudra(s),
            OutputParsed::Outdated(s) => data_outdated(s),
            OutputParsed::Geiger(s) => data_geiger(s),
            OutputParsed::SemverChecks(s) => data_semver_checks(s),
            OutputParsed::Udeps(s) => data_udeps(s),
            OutputParsed::Cargo { source, stderr } => data_cargo(source, stderr),
        };

        let cache = CacheValue::new(&self.resolve, self.duration_ms, data);
        if let Some(db_repo) = db_repo {
            let key = &db_repo.key(&self.resolve);
            let _span = key.span();
            // 写入命令缓存
            db_repo.set_cache(key, &cache);
            db_repo.set_info_cache(key);
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

    pub fn read_cache(self, key: &CacheRepoKey) -> Result<Option<CacheValue>> {
        self.db
            .get_cache(&key.to_db_key())
            .map(|c| c.map(CacheValue::from))
    }

    /// 写入命令缓存
    pub fn set_cache(&self, key: &CacheRepoKey, cache: &CacheValue) {
        if let Err(err) = self.db.set_cache(&key.to_db_key(), &cache.to_db_value()) {
            error!(%err, ?key, "Unable to save the cache.");
        }
    }

    /// 写入键缓存
    pub fn set_info_cache(&self, key: &CacheRepoKey) {
        if let Err(err) = self.info.append_cache_key(key, self.db) {
            error!(%err, ?key, "Unable to save the info cache.");
        }
    }

    /// 写入 layout 缓存
    pub fn set_layout_cache(&self, layout: CacheLayout) {
        if let Err(err) = self.info.set_layout_cache(layout, self.db) {
            error!(%err, "Unable to save the layout cache.");
        }
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
        let data = OutputDataInner::new("[Lockbud] deadlock detection".into(), kind, s.to_owned());
        vec![data]
    }
}

fn data_atomvchecker(s: &str) -> Vec<OutputDataInner> {
    if s.is_empty() {
        Vec::new()
    } else {
        // FIXME: 目前 atomvchecker 无法良好地解析，需要等它实现 JSON 输出才能更可靠地区分哪种
        let data = OutputDataInner::new(
            "[AtomVChecker] memory ordering misuse detection".into(),
            Kind::Rapx,
            s.to_owned(),
        );
        vec![data]
    }
}

fn data_rap(s: &str) -> Vec<OutputDataInner> {
    if s.is_empty() {
        Vec::new()
    } else {
        // FIXME: 目前 rap 无法良好地解析，需要等它实现 JSON 输出才能更可靠地区分哪种
        let data = OutputDataInner::new(
            "[rap] Not supported to display yet.".into(),
            Kind::Rapx,
            s.to_owned(),
        );
        vec![data]
    }
}

fn data_rudra(s: &str) -> Vec<OutputDataInner> {
    if s.is_empty() {
        Vec::new()
    } else {
        // FIXME: 目前 rudra 无法良好地解析，需要等它实现 JSON 输出才能更可靠地区分哪种
        let data = OutputDataInner::new(
            "[rudra] Not supported to display yet.".into(),
            Kind::Rudra,
            s.to_owned(),
        );
        vec![data]
    }
}

fn data_outdated(s: &str) -> Vec<OutputDataInner> {
    if s.is_empty() {
        Vec::new()
    } else {
        let data = OutputDataInner::new(
            "[outdated direct dependencies]".into(),
            Kind::Outdated,
            s.to_owned(),
        );
        vec![data]
    }
}

fn data_semver_checks(s: &str) -> Vec<OutputDataInner> {
    if s.is_empty() {
        Vec::new()
    } else {
        let data = OutputDataInner::new(
            "[semver checks]".into(),
            Kind::SemverViolation,
            s.to_owned(),
        );
        vec![data]
    }
}

fn data_geiger(s: &str) -> Vec<OutputDataInner> {
    if s.is_empty() {
        Vec::new()
    } else {
        let data = OutputDataInner::new(
            "[geiger] Unsafe Code statistics".into(),
            Kind::Geiger,
            s.to_owned(),
        );
        vec![data]
    }
}

fn data_udeps(s: &str) -> Vec<OutputDataInner> {
    if s.is_empty() {
        Vec::new()
    } else {
        let data = OutputDataInner::new(
            "[udeps] Unused dependencies".into(),
            Kind::Udeps,
            s.to_owned(),
        );
        vec![data]
    }
}

fn data_audit(a: &Audit, root: &Utf8Path) -> Vec<OutputDataInner> {
    if let Some(audit) = a {
        let file = strip_prefix(audit.lock_file(), root).to_owned();
        let raw = audit.output().to_owned();
        vec![OutputDataInner::new(file, Kind::Audit, raw)]
    } else {
        vec![]
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
