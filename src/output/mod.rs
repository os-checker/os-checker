#![allow(unused)]
use crate::{repo::CheckerTool, XString};
use cargo_metadata::camino::Utf8PathBuf;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub env: Env,
    pub cmd: Vec<Cmd>,
    pub data: Vec<Data>,
}

impl JsonOutput {
    pub fn new() -> Self {
        Self {
            env: Env {
                kinds: Kinds::new(),
                repos: vec![],
                packages: vec![],
            },
            cmd: vec![],
            data: vec![],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Env {
    kinds: Kinds,
    pub repos: Vec<Repo>,
    pub packages: Vec<Package>,
}

#[derive(Debug, Serialize)]
pub struct Repo {
    pub user: XString,
    pub repo: XString,
}

#[derive(Debug, Serialize)]
pub struct Package {
    pub name: XString,
    pub repo: PackageRepo,
}

#[derive(Debug, Serialize)]
pub struct PackageRepo {
    pub repo_idx: usize,
    pub user: XString,
    pub repo: XString,
}

#[derive(Debug, Serialize)]
pub struct PackageCargo {
    pub targets: Vec<XString>,
    pub features: Vec<XString>,
}

#[derive(Debug, Serialize)]
pub struct Cmd {
    pub package_idx: usize,
    pub tool: CheckerTool,
    pub cmd: String,
    pub count: usize,
    pub duration_ms: u64,
    pub arch: XString,
    pub target_triple: String,
    pub features: Vec<XString>,
    pub flags: Vec<XString>,
}

#[derive(Debug, Serialize)]
pub struct Data {
    /// idx referring to `Vec<Idx>`
    pub cmd_idx: usize,
    pub file: Utf8PathBuf,
    pub kind: Kind,
    pub raw: String,
}

/// The kind a checker reports.
#[derive(Debug, Serialize)]
pub enum Kind {
    /// fmt
    Unformatted,
    // FIXME: 带括号的键存在诸多不变，为了编程方便，使用 camel-case；
    // 面向 UI 时，前端会转换成所需的文字。
    #[serde(rename = "Clippy(Warn)")]
    ClippyWarn,
    #[serde(rename = "Clippy(Error)")]
    ClippyError,
    /// miri
    Miri,
    /// semver-checks
    SemverViolation,
    /// TODO
    Lockbud,
}

#[derive(Debug, Serialize)]
struct Kinds {
    order: Vec<Kind>,
    mapping: serde_json::Value,
}

impl Kinds {
    fn new() -> Kinds {
        use Kind::*;
        Kinds {
            order: vec![ClippyError, ClippyWarn, Unformatted],
            mapping: serde_json::json!({
                "clippy": [ClippyError, ClippyWarn],
                "fmt": [Unformatted]
            }),
        }
    }
}
