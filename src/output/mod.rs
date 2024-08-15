#![allow(unused)]
use crate::XString;
use cargo_metadata::camino::Utf8PathBuf;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Env {
    repos: Vec<Repo>,
    packages: Vec<Package>,
}

#[derive(Debug, Serialize)]
struct Repo {
    user: XString,
    repo: XString,
}

#[derive(Debug, Serialize)]
struct Package {
    user: XString,
    repo: PackageRepo,
}

#[derive(Debug, Serialize)]
struct PackageRepo {
    idx: usize,
    user: XString,
    repo: XString,
}

#[derive(Debug, Serialize)]
struct PackageCargo {
    targets: Vec<String>,
    features: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Data {
    idx: usize,
    file: Utf8PathBuf,
    kind: Kind,
    raw: String,
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
