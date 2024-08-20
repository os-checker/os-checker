use crate::{Result, XString};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    CompilerMessage, Message,
};
use itertools::Itertools;

/// A target triple with precise source.
#[derive(Debug)]
pub struct TargetTriple {
    target: String,
    source: Vec<TargetSource>,
}

impl TargetTriple {
    fn specified_default(target: &str) -> Self {
        TargetTriple {
            target: target.to_owned(),
            source: vec![TargetSource::SpecifiedDefault],
        }
    }

    fn unspecified_default() -> Self {
        TargetTriple {
            target: crate::utils::host_target_triple().to_owned(),
            source: vec![TargetSource::SpecifiedDefault],
        }
    }

    fn triple(&self) -> &str {
        &self.target
    }
}

/// Refer to https://github.com/os-checker/os-checker/issues/26 for more info.
#[derive(Debug)]
pub enum TargetSource {
    SpecifiedDefault,
    UnspecifiedDefault,
    DetectedBy(Utf8PathBuf),
    OverriddenInYaml,
}

/// Default cargo target triple list got by `cargo check -vv` which compiles all targets.
#[derive(Debug)]
pub struct DefaultTargetTriples {
    pub targets: Vec<TargetTriple>,
    /// The first time `cargo check` takes.
    pub first_check_duration_ms: u64,
}

impl DefaultTargetTriples {
    pub fn new(pkg_dir: &Utf8Path, pkg_name: &str) -> Result<DefaultTargetTriples> {
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

        let mut targets = Message::parse_stream(output?.stderr.as_slice())
            .filter_map(|parsed| {
                if let Message::TextLine(mes) = &parsed.ok()? {
                    // 只需要当前 package 的 target triple：
                    // * 需要 pkg_name 和 manifest_dir 是因为输出会产生依赖项的信息，仅有
                    //   pkg_name 会造成可能的冲突（尤其 cargo check 最后才会编译当前 pkg）
                    // * 实际的编译命令示例，见 https://github.com/os-checker/os-checker/commit/de95f5928a25f6b64bcf5f1964870351899f85c3
                    if RE.running_cargo.is_match(mes) {
                        let crate_name = RE.pkg_name.captures(mes)?.get(1)?.as_str();
                        let manifest_dir = RE.manifest_dir.captures(mes)?.get(1)?.as_str();
                        let target_triple = RE.target_triple.captures(mes)?.get(1)?.as_str();
                        if crate_name == pkg_name && manifest_dir == pkg_dir {
                            return Some(TargetTriple::specified_default(target_triple));
                        }
                    }
                }
                None
            })
            .collect_vec();

        // NOTE: this can be empty if no target is specified in .cargo/config.toml,
        // in which case the host target should be implied when the empty value is handled.
        if targets.is_empty() {
            targets = vec![TargetTriple::unspecified_default()];
        }

        Ok(DefaultTargetTriples {
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
                        .collect_vec(),
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
    pub default_target_triples: DefaultTargetTriples,
    pub cargo_check_diagnostics: Box<[CargoCheckDiagnostics]>,
}

impl PackageInfo {
    pub fn new(pkg_dir: &Utf8Path, pkg_name: &str) -> Result<Self> {
        let default_target_triples = DefaultTargetTriples::new(pkg_dir, pkg_name)?;
        let cargo_check_diagnostics = default_target_triples
            .targets
            .iter()
            .map(|target| CargoCheckDiagnostics::new(pkg_dir, pkg_name, target.triple()))
            .collect::<Result<_>>()?;
        Ok(PackageInfo {
            pkg_name: pkg_name.into(),
            pkg_dir: pkg_dir.to_owned(),
            cargo_check_diagnostics,
            default_target_triples,
        })
    }
}
