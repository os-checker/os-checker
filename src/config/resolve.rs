//! 校验 JSON 配置文件：
//! * 校验自定义命令：
//!     * 每条自定义命令必须包含工具名称
//!     * 如果指定 target，则校验是否与 rustc 的 target triple 匹配：需要存储 rustc target triple 列表
//! * 校验 package name：
//!     * 如果指定包名，则校验是否定义于仓库内：需要 repo layout 信息
//!     * 如果指定 features，则校验是否定义于 package 内：需要 cargo metadata 信息

use super::{cargo_clippy, cargo_fmt, cargo_lockbud, cargo_mirai, checker::CheckerTool, custom};
use crate::{
    layout::{Audit, Pkg},
    output::{get_toolchain, host_target_triple},
    Result, XString,
};
use cargo_metadata::camino::Utf8PathBuf;
use duct::Expression;

/// 一个 package 待运行的检查命令（含 package 和 target triple）
#[derive(Debug)]
pub struct Resolve {
    pub pkg_name: XString,
    pub pkg_dir: Utf8PathBuf,
    pub target: String,
    /// 仅当自定义检查命令出现 --target 时为 true
    pub target_overridden: bool,
    pub toolchain: Option<usize>,
    pub checker: CheckerTool,
    /// 完整的检查命令字符串（一定包含 --target）：
    /// 来自 os-checker 生成或者配置文件自定义
    pub cmd: String,
    /// 待运行的检查命令
    pub expr: Expression,
    /// The result of cargo-audit has already been there.
    pub audit: Audit,
}

impl Resolve {
    /// 来自 os-checker 生成
    pub fn new(pkg: &Pkg, checker: CheckerTool, cmd: String, expr: Expression) -> Self {
        Self {
            pkg_name: pkg.name.into(),
            pkg_dir: pkg.dir.to_owned(),
            target: pkg.target.to_owned(),
            target_overridden: false,
            toolchain: pkg.toolchain,
            checker,
            cmd,
            expr,
            audit: None,
        }
    }

    /// 配置文件自定义
    pub fn new_overrriden(
        pkg: &Pkg,
        target: String,
        checker: CheckerTool,
        cmd: String,
        expr: Expression,
    ) -> Self {
        Self {
            pkg_name: pkg.name.into(),
            pkg_dir: pkg.dir.to_owned(),
            target,
            target_overridden: true,
            toolchain: pkg.toolchain,
            checker,
            cmd,
            expr,
            audit: None,
        }
    }

    /// 由于 CheckerTool::Cargo 是虚拟的，因此有些字段并不具备实际的含义
    pub fn new_cargo(&self) -> Self {
        Resolve {
            pkg_name: self.pkg_name.clone(),
            pkg_dir: self.pkg_dir.clone(),
            target: self.target.clone(),
            target_overridden: self.target_overridden, // 无实际含义
            toolchain: self.toolchain,
            checker: CheckerTool::Cargo,
            cmd: format!("VRITUAL={} cargo", self.checker.name()),
            expr: duct::cmd!("false"), // 无实际含义
            audit: None,
        }
    }

    /// 此函数仅表示 Layout 解析错误，不与 checker 对于。所有字段并不具备实际的含义。
    pub fn new_cargo_layout_parse_error(pkg_name: &str, repo_root: Utf8PathBuf) -> Self {
        Resolve {
            pkg_name: pkg_name.into(), // 无实际含义
            pkg_dir: repo_root,        // 无实际含义
            target: host_target_triple().to_owned(),
            target_overridden: false, // 无实际含义
            toolchain: None,
            checker: CheckerTool::Cargo,
            cmd: "VRITUAL=LayoutParseError cargo".to_owned(),
            expr: duct::cmd!("false"), // 无实际含义
            audit: None,
        }
    }

    pub fn fmt(pkgs: &[Pkg], resolved: &mut Vec<Self>) {
        resolved.extend(pkgs.iter().map(cargo_fmt));
    }

    pub fn clippy(pkgs: &[Pkg], resolved: &mut Vec<Self>) {
        resolved.extend(pkgs.iter().map(cargo_clippy));
    }

    pub fn lockbud(pkgs: &[Pkg], resolved: &mut Vec<Self>) {
        resolved.extend(pkgs.iter().map(cargo_lockbud));
    }

    /// 由于 mirai 检查需要配备高规格的机器，Github Action 的机器不太能满足，
    /// 因此只在某些条件下开启。
    pub fn mirai(pkgs: &[Pkg], resolved: &mut Vec<Self>) {
        // 暂时只在 x86_64-unknown-linux-gnu 上检查
        let iter = pkgs
            .iter()
            .filter(|pkg| pkg.target == "x86_64-unknown-linux-gnu");
        resolved.extend(iter.map(cargo_mirai));
    }

    pub fn audit(pkgs: &[Pkg], resolved: &mut Vec<Self>) {
        resolved.reserve(pkgs.len());
        for pkg in pkgs {
            if let Some(audit) = pkg.audit {
                let mut val = Self::new(pkg, CheckerTool::Audit, audit.cmd(), audit.cmd_expr());
                val.audit = Some(audit.clone());
                resolved.push(val);
            }
        }
    }

    #[instrument(level = "trace")]
    pub fn custom(
        pkgs: &[Pkg],
        lines: &[String],
        checker: CheckerTool,
        resolved: &mut Vec<Self>,
    ) -> Result<()> {
        resolved.reserve(pkgs.len() * lines.len());
        'line: for line in lines {
            for pkg in pkgs {
                let value = custom(line, pkg, checker)?;
                let target_overriden = value.target_overridden;
                resolved.push(value);
                if target_overriden {
                    // 已经从自定义命令中覆盖了所有搜索到的 targets，因此无需继续
                    // NOTE:这也意味着，自定义命令的 --target 仅作用于那一行，而不是这一批
                    continue 'line;
                }
            }
        }
        Ok(())
    }

    pub fn toolchain(&self) -> String {
        // 0 表示 host toolchain
        get_toolchain(self.toolchain.unwrap_or(0))
    }
}