use crate::{
    utils::{scan_scripts_for_target, walk_dir},
    Result, XString,
};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    CompilerMessage, Message,
};
use indexmap::IndexMap;

fn detected_targets(repo_root: &str, targets: &mut Targets) -> Result<()> {
    // os-checker 会启发式搜索一些常见的脚本来猜测额外的目标架构信息，这些被查询脚本为：
    // .github 文件夹内的任何文件、递归查找 Makefile、makefile、py、sh、just 后缀的文件
    //
    // 此外，一个可能的改进是查找 Cargo.toml 中的条件编译中包含架构的信息（见 `layout::parse`）。
    //
    // 更实际的一个改进是，此函数目前考虑在 repo_root 中查找，如果增加在 pkg_dir 中查找，那么会
    // 更好；不过，这意味着需要区分 TargetSource 中的 DetectedBy 是从仓库还是库得到的。
    let scripts = walk_dir(repo_root, 4, &[".github"], |file_path| {
        let file_stem = file_path.file_stem()?;

        // 除了标准的 Makefile 文件外，还有一些其他文件名可能会与 Makefile 一起使用，这些文件通常用于定义特定目标或条件的规则：
        //
        // 1. **GNUmakefile**: 这是 GNU make 的标准文件名，用于区分其他 make 版本。
        // 2. **makefile**: 这是 BSD make 的标准文件名。
        // 3. **Makefile.am** 或 **Makefile.in**: 这些文件通常由 autotools 生成，用于自动配置 makefile。
        // 4. **Makefile.\***: 有时，项目可能会有多个 Makefile 文件，用于不同的平台或配置，例如 Makefile.linux 或 Makefile.debug。
        // 5. **.mk**: 这是 Makefile 的另一种扩展名，用于包含在其他 Makefile 中的 makefile 片段。
        //
        // 请注意，尽管有多种可能的文件名，但大多数 make 工具默认寻找的文件名是 "Makefile" 或 "makefile"。如果你使用不同的文件名，可能需要在调用 `make` 命令时指定文件名。
        if file_stem.starts_with("Makefile")
            || file_stem.starts_with("makefile")
            || file_stem == "GNUmakefile"
        {
            return Some(file_path);
        }
        if let "mk" | "sh" | "py" | "just" = file_path.extension()? {
            return Some(file_path);
        }
        None
    });
    let github_dir = Utf8Path::new(repo_root).join(".github");
    let github_files = walk_dir(&github_dir, 4, &[], Some);
    debug!(repo_root, ?scripts, ?github_files);

    scan_scripts_for_target(&scripts, |target, path| {
        targets.detected_by_repo_scripts(target, path);
    })?;
    scan_scripts_for_target(&github_files, |target, path| {
        targets.detected_by_repo_github(target, path);
    })?;
    Ok(())
}

/// Refer to https://github.com/os-checker/os-checker/issues/26 for more info.
#[derive(Debug)]
pub enum TargetSource {
    SpecifiedDefault,
    UnspecifiedDefault,
    DetectedByRepoGithub(Utf8PathBuf),
    DetectedByRepoScripts(Utf8PathBuf),
    // DetectedBy(Utf8PathBuf),
    OverriddenInYaml,
}

/// A list of target triples obtained from multiple sources.
/// The orders in key and value demonstrates how they shape.
#[derive(Debug)]
pub struct Targets {
    map: IndexMap<String, Vec<TargetSource>>,
}

impl std::ops::Deref for Targets {
    type Target = IndexMap<String, Vec<TargetSource>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl std::ops::DerefMut for Targets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl Targets {
    fn new() -> Targets {
        Targets {
            map: IndexMap::with_capacity(4),
        }
    }

    fn specified_default(&mut self, target: &str) {
        if let Some(source) = self.get_mut(target) {
            source.push(TargetSource::SpecifiedDefault);
        } else {
            self.insert(target.to_owned(), vec![TargetSource::SpecifiedDefault]);
        }
    }

    fn unspecified_default(&mut self) {
        let target = crate::utils::host_target_triple();
        if let Some(source) = self.get_mut(target) {
            source.push(TargetSource::UnspecifiedDefault);
        } else {
            self.insert(target.to_owned(), vec![TargetSource::UnspecifiedDefault]);
        }
    }

    fn detected_by_repo_github(&mut self, target: &str, path: Utf8PathBuf) {
        match self.get_mut(target) {
            Some(v) => v.push(TargetSource::DetectedByRepoGithub(path)),
            None => {
                _ = self.insert(
                    target.to_owned(),
                    vec![TargetSource::DetectedByRepoGithub(path)],
                )
            }
        }
    }

    fn detected_by_repo_scripts(&mut self, target: &str, path: Utf8PathBuf) {
        match self.get_mut(target) {
            Some(v) => v.push(TargetSource::DetectedByRepoScripts(path)),
            None => {
                _ = self.insert(
                    target.to_owned(),
                    vec![TargetSource::DetectedByRepoScripts(path)],
                )
            }
        }
    }
}

/// Default cargo target triple list got by `cargo check -vv` which compiles all targets.
#[derive(Debug)]
pub struct TargetTriples {
    pub targets: Targets,
    /// The first time `cargo check` takes.
    pub first_check_duration_ms: u64,
}

impl TargetTriples {
    pub fn new(pkg_dir: &Utf8Path, pkg_name: &str) -> Result<TargetTriples> {
        use regex::Regex;
        use std::sync::LazyLock;

        struct ExtractTriplePattern {
            pkg_name: Regex,
            manifest_dir: Regex,
            target_triple: Regex,
            running_cargo: Regex,
        }
        static RE: LazyLock<ExtractTriplePattern> = LazyLock::new(|| ExtractTriplePattern {
            pkg_name: regex::Regex::new(r#"CARGO_PKG_NAME=(\S+)"#).unwrap(),
            manifest_dir: regex::Regex::new(r#"CARGO_MANIFEST_DIR=(\S+)"#).unwrap(),
            target_triple: regex::Regex::new(r#"--target\s+(\S+)"#).unwrap(),
            running_cargo: regex::Regex::new(r#"^\s+Running `CARGO="#).unwrap(),
        });

        // NOTE: 似乎只有第一次运行 cargo check 才会强制编译所有 target triples，
        // 第二次开始运行 cargo check 之后，如果在某个 triple 上编译失败，不会编译其他 triple，
        // 这导致无法全部获取 triples 列表。因此为了避免缓存影响，清除 target dir。
        _ = duct::cmd!("cargo", "clean").dir(pkg_dir).run()?;
        let (duration_ms, output) = crate::utils::execution_time_ms(|| {
            duct::cmd!("cargo", "check", "-vv")
                .dir(pkg_dir)
                .stderr_capture()
                .unchecked()
                .run()
        });

        let mut targets = Targets::new();

        for parsed in Message::parse_stream(output?.stderr.as_slice()) {
            if let Ok(Message::TextLine(mes)) = &parsed {
                // 只需要当前 package 的 target triple：
                // * 需要 pkg_name 和 manifest_dir 是因为输出会产生依赖项的信息，仅有
                //   pkg_name 会造成可能的冲突（尤其 cargo check 最后才会编译当前 pkg）
                // * 实际的编译命令示例，见 https://github.com/os-checker/os-checker/commit/de95f5928a25f6b64bcf5f1964870351899f85c3
                if RE.running_cargo.is_match(mes) {
                    // trick to use ? in small scope
                    let mut f = || {
                        let crate_name = RE.pkg_name.captures(mes)?.get(1)?.as_str();
                        let manifest_dir = RE.manifest_dir.captures(mes)?.get(1)?.as_str();
                        let target_triple = RE.target_triple.captures(mes)?.get(1)?.as_str();
                        if crate_name == pkg_name && manifest_dir == pkg_dir {
                            targets.specified_default(target_triple);
                        }
                        None::<()>
                    };
                    f();
                }
            }
        }

        // NOTE: this can be empty if no target is specified in .cargo/config.toml,
        // in which case the host target should be implied when the empty value is handled.
        if targets.is_empty() {
            targets.unspecified_default();
        }

        Ok(TargetTriples {
            targets,
            first_check_duration_ms: duration_ms,
        })
    }
}

pub struct CargoCheckDiagnostics {
    pub target_triple: String,
    pub compiler_messages: Box<[CompilerMessage]>,
    pub duration_ms: u64,
}

impl CargoCheckDiagnostics {
    pub fn new(pkg_dir: &Utf8Path, pkg_name: &str, target_triple: &str) -> Result<Self> {
        let (duration_ms, out) = crate::utils::execution_time_ms(|| {
            duct::cmd!(
                "cargo",
                "check",
                "--message-format=json",
                "--target",
                target_triple
            )
            .dir(pkg_dir)
            .stdout_capture()
            .unchecked()
            .run()
        });

        Ok(CargoCheckDiagnostics {
            target_triple: target_triple.to_owned(),
            compiler_messages: Message::parse_stream(out?.stdout.as_slice())
                .filter_map(|mes| match mes.ok()? {
                    Message::CompilerMessage(mes) if mes.target.name == pkg_name => Some(mes),
                    _ => None,
                })
                .collect(),
            duration_ms,
        })
    }
}

impl std::fmt::Debug for CargoCheckDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(test)]
        {
            f.debug_struct("CargoCheckDiagnostics")
                .field("target_triple", &self.target_triple)
                .field(
                    "compiler_messages",
                    &self
                        .compiler_messages
                        .iter()
                        .map(|d| d.message.to_string())
                        .collect::<Vec<_>>(),
                )
                .finish()
        }
        #[cfg(not(test))]
        f.debug_struct("CargoCheckDiagnostics")
            .field("target_triple", &self.target_triple)
            .field("duration_ms", &self.duration_ms)
            .field("compiler_messages.len", &self.compiler_messages.len())
            .finish()
    }
}

#[derive(Debug)]
pub struct PackageInfo {
    pub pkg_name: XString,
    /// i.e. manifest_dir
    pub pkg_dir: Utf8PathBuf,
    pub target_triples: TargetTriples,
    pub cargo_check_diagnostics: Box<[CargoCheckDiagnostics]>,
}

impl PackageInfo {
    pub fn new(pkg_dir: &Utf8Path, pkg_name: &str) -> Result<Self> {
        let default_target_triples = TargetTriples::new(pkg_dir, pkg_name)?;
        let cargo_check_diagnostics = default_target_triples
            .targets
            .keys()
            .map(|target| CargoCheckDiagnostics::new(pkg_dir, pkg_name, target))
            .collect::<Result<_>>()?;
        Ok(PackageInfo {
            pkg_name: pkg_name.into(),
            pkg_dir: pkg_dir.to_owned(),
            cargo_check_diagnostics,
            target_triples: default_target_triples,
        })
    }

    pub fn detected_targets_by_scripts(&mut self, repo_root: &str) -> Result<()> {
        detected_targets(repo_root, &mut self.target_triples.targets)
    }
}
