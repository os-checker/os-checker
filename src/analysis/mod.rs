use cargo_metadata::diagnostic::DiagnosticLevel;
use std::time::Duration;

pub struct Statistics {
    count: Vec<Count>,
    time: Duration,
}

pub struct Count {
    level: Kind,
    count: usize,
}

/// The kind a checker reports.
pub enum Kind {
    /// fmt
    Unformatted,
    /// clippy
    Clippy(DiagnosticLevel),
    /// miri
    UndefinedBehavior,
    /// semver-checks
    SemverViolation,
    /// TODO
    Lockbud,
}
