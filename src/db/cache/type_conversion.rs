use crate::{config::CheckerTool, output::Kind};
use os_checker_types::cache as out;

impl From<super::CacheRepoKeyCmd> for out::CacheRepoKeyCmd {
    fn from(value: super::CacheRepoKeyCmd) -> Self {
        let super::CacheRepoKeyCmd {
            pkg_name,
            checker,
            cmd,
        } = value;
        out::CacheRepoKeyCmd {
            pkg_name,
            checker: checker.into(),
            cmd: cmd.into(),
        }
    }
}

impl From<super::CacheRepoKey> for out::CacheRepoKey {
    fn from(value: super::CacheRepoKey) -> Self {
        let super::CacheRepoKey { repo, cmd } = value;
        out::CacheRepoKey {
            repo: repo.into(),
            cmd: cmd.into(),
        }
    }
}

impl From<super::CacheRepo> for out::CacheRepo {
    fn from(value: super::CacheRepo) -> Self {
        let super::CacheRepo {
            user,
            repo,
            sha,
            branch,
        } = value;
        out::CacheRepo {
            user,
            repo,
            sha,
            branch,
        }
    }
}

impl From<super::CacheChecker> for out::CacheChecker {
    fn from(value: super::CacheChecker) -> Self {
        let super::CacheChecker {
            checker,
            version,
            sha,
        } = value;
        out::CacheChecker {
            checker: checker.into(),
            version,
            sha,
        }
    }
}

impl From<super::CacheCmd> for out::CacheCmd {
    fn from(value: super::CacheCmd) -> Self {
        let super::CacheCmd {
            cmd,
            target,
            channel,
            features,
            flags,
        } = value;
        out::CacheCmd {
            cmd,
            target,
            channel,
            features,
            flags,
        }
    }
}

impl From<super::OutputData> for out::OutputData {
    fn from(value: super::OutputData) -> Self {
        let super::OutputData { duration_ms, data } = value;
        let data: Vec<_> = data.into_iter().map(|d| d.into()).collect();
        out::OutputData { duration_ms, data }
    }
}

impl From<super::OutputDataInner> for out::OutputDataInner {
    fn from(value: super::OutputDataInner) -> Self {
        let super::OutputDataInner { file, kind, raw } = value;
        let kind = kind.into();
        out::OutputDataInner { file, kind, raw }
    }
}

impl From<super::CacheValue> for out::CacheValue {
    fn from(value: super::CacheValue) -> Self {
        let super::CacheValue {
            unix_timestamp_milli,
            cmd,
            diagnostics,
        } = value;
        out::CacheValue {
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
