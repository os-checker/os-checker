#![allow(unused)]
use crate::{repo::CheckerTool, XString};
use cargo_metadata::camino::Utf8PathBuf;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub env: Env,
    pub idx: Vec<Idx>,
    pub data: Vec<Data>,
}

impl JsonOutput {
    pub fn new() -> Self {
        Self {
            env: Env {
                repos: vec![],
                packages: vec![],
            },
            idx: vec![],
            data: vec![],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Env {
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
    pub idx: usize,
    pub user: XString,
    pub repo: XString,
}

#[derive(Debug, Serialize)]
pub struct PackageCargo {
    pub targets: Vec<XString>,
    pub features: Vec<XString>,
}

#[derive(Debug, Serialize)]
pub struct Idx {
    pub package: usize,
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
    pub idx: usize,
    pub file: Utf8PathBuf,
    pub kind: Kind,
    pub raw: String,
}

/// The kind a checker reports.
#[allow(unused)]
#[derive(Debug, Serialize, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
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

#[derive(Debug, Serialize, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Unformatted {
    File,
    Line,
}

#[derive(Debug, Serialize, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Rustc {
    Warn,
    Error,
}
