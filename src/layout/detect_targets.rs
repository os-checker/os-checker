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
    Result,
};
use cargo_metadata::camino::Utf8Path;

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
