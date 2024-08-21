//! os-checker 会启发式搜索一些常见的脚本来猜测额外的目标架构信息，这些被查询脚本为：
//! .github 文件夹内的任何文件、递归查找 Makefile、makefile、py、sh、just 后缀的文件
//!
//! 此外，一个可能的改进是查找 Cargo.toml 中的条件编译中包含架构的信息（见 `layout::parse`）。
//!
//! 除了标准的 Makefile 文件外，还有一些其他文件名可能会与 Makefile 一起使用，这些文件通常用于定义特定目标或条件的规则：
//!
//! 1. **GNUmakefile**: 这是 GNU make 的标准文件名，用于区分其他 make 版本。
//! 2. **makefile**: 这是 BSD make 的标准文件名。
//! 3. **Makefile.am** 或 **Makefile.in**: 这些文件通常由 autotools 生成，用于自动配置 makefile。
//! 4. **Makefile.\***: 有时，项目可能会有多个 Makefile 文件，用于不同的平台或配置，例如 Makefile.linux 或 Makefile.debug。
//! 5. **.mk**: 这是 Makefile 的另一种扩展名，用于包含在其他 Makefile 中的 makefile 片段。
//!
//! 请注意，尽管有多种可能的文件名，但大多数 make 工具默认寻找的文件名是 "Makefile" 或 "makefile"。如果你使用不同的文件名，可能需要在调用 `make` 命令时指定文件名。

use super::cargo_check_verbose::Targets;
use crate::{
    utils::{scan_scripts_for_target, walk_dir},
    Result, XString,
};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    Message,
};
use indexmap::{IndexMap, IndexSet};

type TargetsMap = IndexMap<(Utf8PathBuf, XString), Vec<String>>;

/// Default cargo target triple list got by `cargo check -vv` which compiles all targets.
#[derive(Debug)]
pub struct WorkspaceTargetTriples {
    // NOTE: this can be empty if no target is specified in .cargo/config.toml,
    // in which case the host target should be implied when the empty value is handled.
    pub targets: TargetsMap,
    /// The first time `cargo check` takes.
    pub first_check_duration_ms: u64,
}

impl WorkspaceTargetTriples {
    pub fn new(workspce_root: &Utf8Path, members: &IndexSet<(&str, &str)>) -> Result<Self> {
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
        _ = duct::cmd!("cargo", "clean").dir(workspce_root).run()?;
        let (duration_ms, output) = crate::utils::execution_time_ms(|| {
            duct::cmd!("cargo", "check", "-vv", "--workspace")
                .dir(workspce_root)
                .stderr_capture()
                .unchecked()
                .run()
        });

        let mut targets = TargetsMap::with_capacity(members.len());

        for parsed in Message::parse_stream(output?.stderr.as_slice()) {
            if let Ok(Message::TextLine(mes)) = &parsed {
                // 只需要当前 package 的 target triple：
                // * 需要 pkg_name 和 manifest_dir 是因为输出会产生依赖项的信息，仅有
                //   pkg_name 会造成可能的冲突（尤其 cargo check 最后才会编译当前 pkg）
                // * 实际的编译命令示例，见 https://github.com/os-checker/os-checker/commit/de95f5928a25f6b64bcf5f1964870351899f85c3
                if RE.running_cargo.is_match(mes) {
                    // trick to use ? in small scope
                    let mut f = || {
                        debug!(mes);
                        let crate_name = RE.pkg_name.captures(mes)?.get(1)?.as_str();
                        debug!(crate_name);
                        let manifest_dir = RE.manifest_dir.captures(mes)?.get(1)?.as_str();
                        debug!(manifest_dir);
                        let target_triple = RE.target_triple.captures(mes)?.get(1)?.as_str();
                        debug!(target_triple);
                        if members.contains(&(manifest_dir, crate_name)) {
                            let target = target_triple.to_owned();
                            let key = (Utf8PathBuf::from(manifest_dir), XString::new(crate_name));
                            if let Some(v) = targets.get_mut(&key) {
                                v.push(target);
                            } else {
                                targets.insert(key, vec![target]);
                            };
                        }
                        None::<()>
                    };
                    f();
                }
            }
        }

        Ok(WorkspaceTargetTriples {
            targets,
            first_check_duration_ms: duration_ms,
        })
    }
}

pub fn in_repo(repo_root: &str) -> Result<Targets> {
    let mut targets = Targets::new();
    let scripts = walk_dir(repo_root, 1, &[".github"], |file_path| {
        let file_stem = file_path.file_stem()?;

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

    Ok(targets)
}

pub fn in_pkg_dir(pkg_dir: &Utf8Path, targets: &mut Targets) -> Result<()> {
    let scripts = walk_dir(pkg_dir, 4, &[".github"], |file_path| {
        let file_stem = file_path.file_stem()?;

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
    debug!(?pkg_dir, ?scripts);

    scan_scripts_for_target(&scripts, |target, path| {
        targets.detected_by_pkg_scripts(target, path);
    })?;
    Ok(())
}
