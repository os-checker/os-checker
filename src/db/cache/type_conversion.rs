use super::{
    CacheChecker, CacheCmd, CacheRepo, CacheRepoKey, CacheRepoKeyCmd, CacheValue, OutputData,
    OutputDataInner,
};
use crate::{config::CheckerTool, output::Kind};
use os_checker_types::cache as out;

// ********** CLI => os_checker_types **********

impl From<CacheRepoKeyCmd> for out::CacheRepoKeyCmd {
    fn from(value: CacheRepoKeyCmd) -> Self {
        let CacheRepoKeyCmd {
            pkg_name,
            checker,
            cmd,
        } = value;
        Self {
            pkg_name,
            checker: checker.into(),
            cmd: cmd.into(),
        }
    }
}

impl From<CacheRepoKey> for out::CacheRepoKey {
    fn from(value: CacheRepoKey) -> Self {
        let CacheRepoKey { repo, cmd } = value;
        Self {
            repo: repo.into(),
            cmd: cmd.into(),
        }
    }
}

impl From<CacheRepo> for out::CacheRepo {
    fn from(value: CacheRepo) -> Self {
        let CacheRepo {
            user,
            repo,
            sha,
            branch,
        } = value;
        Self {
            user,
            repo,
            sha,
            branch,
        }
    }
}

impl From<CacheChecker> for out::CacheChecker {
    fn from(value: CacheChecker) -> Self {
        let CacheChecker {
            checker,
            version,
            sha,
        } = value;
        Self {
            checker: checker.into(),
            version,
            sha,
        }
    }
}

impl From<CacheCmd> for out::CacheCmd {
    fn from(value: CacheCmd) -> Self {
        let CacheCmd {
            cmd,
            target,
            channel,
            features,
            flags,
        } = value;
        Self {
            cmd,
            target,
            channel,
            features,
            flags,
        }
    }
}

impl From<OutputData> for out::OutputData {
    fn from(value: OutputData) -> Self {
        let OutputData { duration_ms, data } = value;
        let data: Vec<_> = data.into_iter().map(|d| d.into()).collect();
        Self { duration_ms, data }
    }
}

impl From<OutputDataInner> for out::OutputDataInner {
    fn from(value: OutputDataInner) -> Self {
        let OutputDataInner { file, kind, raw } = value;
        let kind = kind.into();
        Self { file, kind, raw }
    }
}

impl From<CacheValue> for out::CacheValue {
    fn from(value: CacheValue) -> Self {
        let CacheValue {
            unix_timestamp_milli,
            cmd,
            diagnostics,
        } = value;
        Self {
            unix_timestamp_milli,
            cmd: cmd.into(),
            diagnostics: diagnostics.into(),
        }
    }
}

impl From<CheckerTool> for os_checker_types::CheckerTool {
    fn from(value: CheckerTool) -> Self {
        match value {
            CheckerTool::Fmt => Self::Fmt,
            CheckerTool::Clippy => Self::Clippy,
            CheckerTool::Miri => Self::Miri,
            CheckerTool::SemverChecks => Self::SemverChecks,
            CheckerTool::Audit => Self::Audit,
            CheckerTool::Mirai => Self::Mirai,
            CheckerTool::Lockbud => Self::Lockbud,
            CheckerTool::Cargo => Self::Cargo,
        }
    }
}

impl From<Kind> for os_checker_types::Kind {
    fn from(value: Kind) -> Self {
        match value {
            Kind::Unformatted => Self::Unformatted,
            Kind::ClippyWarn => Self::ClippyWarn,
            Kind::ClippyError => Self::ClippyError,
            Kind::Miri => Self::Miri,
            Kind::SemverViolation => Self::SemverViolation,
            Kind::Mirai => Self::Mirai,
            Kind::LockbudProbably => Self::LockbudProbably,
            Kind::LockbudPossibly => Self::LockbudPossibly,
            Kind::Cargo => Self::Cargo,
        }
    }
}

// ********** os_checker_types => CLI **********

impl From<out::CacheRepoKeyCmd> for CacheRepoKeyCmd {
    fn from(value: out::CacheRepoKeyCmd) -> Self {
        let out::CacheRepoKeyCmd {
            pkg_name,
            checker,
            cmd,
        } = value;
        Self {
            pkg_name,
            checker: checker.into(),
            cmd: cmd.into(),
        }
    }
}

impl From<out::CacheRepoKey> for CacheRepoKey {
    fn from(value: out::CacheRepoKey) -> Self {
        let out::CacheRepoKey { repo, cmd } = value;
        Self {
            repo: repo.into(),
            cmd: cmd.into(),
        }
    }
}

impl From<out::CacheRepo> for CacheRepo {
    fn from(value: out::CacheRepo) -> Self {
        let out::CacheRepo {
            user,
            repo,
            sha,
            branch,
        } = value;
        Self {
            user,
            repo,
            sha,
            branch,
        }
    }
}

impl From<out::CacheChecker> for CacheChecker {
    fn from(value: out::CacheChecker) -> Self {
        let out::CacheChecker {
            checker,
            version,
            sha,
        } = value;
        Self {
            checker: checker.into(),
            version,
            sha,
        }
    }
}

impl From<out::CacheCmd> for CacheCmd {
    fn from(value: out::CacheCmd) -> Self {
        let out::CacheCmd {
            cmd,
            target,
            channel,
            features,
            flags,
        } = value;
        Self {
            cmd,
            target,
            channel,
            features,
            flags,
        }
    }
}

impl From<out::OutputData> for OutputData {
    fn from(value: out::OutputData) -> Self {
        let out::OutputData { duration_ms, data } = value;
        let data: Vec<_> = data.into_iter().map(|d| d.into()).collect();
        Self { duration_ms, data }
    }
}

impl From<out::OutputDataInner> for OutputDataInner {
    fn from(value: out::OutputDataInner) -> Self {
        let out::OutputDataInner { file, kind, raw } = value;
        let kind = kind.into();
        Self { file, kind, raw }
    }
}

impl From<out::CacheValue> for CacheValue {
    fn from(value: out::CacheValue) -> Self {
        let out::CacheValue {
            unix_timestamp_milli,
            cmd,
            diagnostics,
        } = value;
        Self {
            unix_timestamp_milli,
            cmd: cmd.into(),
            diagnostics: diagnostics.into(),
        }
    }
}

impl From<os_checker_types::CheckerTool> for CheckerTool {
    fn from(value: os_checker_types::CheckerTool) -> Self {
        match value {
            os_checker_types::CheckerTool::Fmt => Self::Fmt,
            os_checker_types::CheckerTool::Clippy => Self::Clippy,
            os_checker_types::CheckerTool::Miri => Self::Miri,
            os_checker_types::CheckerTool::SemverChecks => Self::SemverChecks,
            os_checker_types::CheckerTool::Audit => Self::Audit,
            os_checker_types::CheckerTool::Mirai => Self::Mirai,
            os_checker_types::CheckerTool::Lockbud => Self::Lockbud,
            os_checker_types::CheckerTool::Cargo => Self::Cargo,
        }
    }
}

impl From<os_checker_types::Kind> for Kind {
    fn from(value: os_checker_types::Kind) -> Self {
        match value {
            os_checker_types::Kind::Unformatted => Self::Unformatted,
            os_checker_types::Kind::ClippyWarn => Self::ClippyWarn,
            os_checker_types::Kind::ClippyError => Self::ClippyError,
            os_checker_types::Kind::Miri => Self::Miri,
            os_checker_types::Kind::SemverViolation => Self::SemverViolation,
            os_checker_types::Kind::Mirai => Self::Mirai,
            os_checker_types::Kind::LockbudProbably => Self::LockbudProbably,
            os_checker_types::Kind::LockbudPossibly => Self::LockbudPossibly,
            os_checker_types::Kind::Cargo => Self::Cargo,
        }
    }
}
