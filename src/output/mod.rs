#![allow(unused)]
use crate::XString;
use cargo_metadata::camino::Utf8PathBuf;
use serde::Serialize;

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
    pub user: XString,
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
    pub targets: Vec<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Data {
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
