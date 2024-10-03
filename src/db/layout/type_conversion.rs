use super::{CacheLayout, CachePackageInfo, CacheResolve};
use os_checker_types::db as out;

// ********** CLI => os_checker_types **********

impl From<CacheLayout> for out::CacheLayout {
    fn from(value: CacheLayout) -> Self {
        let CacheLayout {
            root_path,
            cargo_tomls,
            workspaces,
            packages_info,
        } = value;
        Self {
            root_path,
            cargo_tomls,
            workspaces,
            packages_info: Vec::from(packages_info)
                .into_iter()
                .map(|p| p.into())
                .collect(),
        }
    }
}

impl From<CachePackageInfo> for out::CachePackageInfo {
    fn from(value: CachePackageInfo) -> Self {
        let CachePackageInfo {
            pkg_name,
            pkg_dir,
            targets,
            channel,
            resolves,
        } = value;
        Self {
            pkg_name,
            pkg_dir,
            targets: targets.into(),
            channel,
            resolves: Vec::from(resolves).into_iter().map(|r| r.into()).collect(),
        }
    }
}

impl From<CacheResolve> for out::CacheResolve {
    fn from(value: CacheResolve) -> Self {
        let CacheResolve {
            target,
            target_overriden,
            channel,
            checker,
            cmd,
        } = value;
        Self {
            target,
            target_overriden,
            channel,
            checker: checker.into(),
            cmd,
        }
    }
}

// ********** os_checker_types => CLI **********

impl From<out::CacheLayout> for CacheLayout {
    fn from(value: out::CacheLayout) -> Self {
        let out::CacheLayout {
            root_path,
            cargo_tomls,
            workspaces,
            packages_info,
        } = value;
        Self {
            root_path,
            cargo_tomls,
            workspaces,
            packages_info: Vec::from(packages_info)
                .into_iter()
                .map(|p| p.into())
                .collect(),
        }
    }
}

impl From<out::CachePackageInfo> for CachePackageInfo {
    fn from(value: out::CachePackageInfo) -> Self {
        let out::CachePackageInfo {
            pkg_name,
            pkg_dir,
            targets,
            channel,
            resolves,
        } = value;
        Self {
            pkg_name,
            pkg_dir,
            targets: targets.into(),
            channel,
            resolves: Vec::from(resolves).into_iter().map(|r| r.into()).collect(),
        }
    }
}

impl From<out::CacheResolve> for CacheResolve {
    fn from(value: out::CacheResolve) -> Self {
        let out::CacheResolve {
            target,
            target_overriden,
            channel,
            checker,
            cmd,
        } = value;
        Self {
            target,
            target_overriden,
            channel,
            checker: checker.into(),
            cmd,
        }
    }
}
