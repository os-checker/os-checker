use crate::{
    db::{check_key_uniqueness, read_table, LastChecks},
    utils::{new_map_with_cap, IndexMap},
    write_to_file, Result,
};
use camino::{Utf8Path, Utf8PathBuf};
use eyre::ensure;
use os_checker_types::{db::*, CheckerTool, XString};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Resolve<'a> {
    pkg: &'a str,
    toolchain: &'a str,
    checker: CheckerTool,
    target: &'a str,
    cmd: &'a str,
    count: usize,
    ms: u64,
}

impl<'a> Resolve<'a> {
    fn new(resolve: &'a CacheResolve, count: usize, ms: u64) -> Self {
        let CacheResolve {
            pkg_name,
            target,
            channel,
            checker,
            cmd,
            ..
        } = resolve;
        Self {
            pkg: pkg_name,
            toolchain: channel,
            checker: *checker,
            target,
            cmd,
            count,
            ms,
        }
    }
}

pub fn do_resolves() -> Result<()> {
    let db = redb::Database::open(crate::CACHE_REDB)?;
    let txn = db.begin_read()?;

    let checks = LastChecks::new(&txn)?;

    let mut v_user_repo = Vec::with_capacity(checks.repo_counts());

    checks.with_layout_cache(|info_key, cache_keys| {
        let user = info_key.repo.user.clone();
        let repo = info_key.repo.repo.clone();

        let _span = error_span!("do_resolves", ?user, ?repo).entered();

        let layout = checks.read_layout(info_key)?;
        let CacheLayout {
            root_path,
            packages_info: pkgs,
            resolves,
            ..
        } = &layout;

        #[derive(Hash, PartialEq, Eq)]
        struct CmdKeyRef<'a> {
            pkg: &'a str,
            target: &'a str,
            channel: &'a str,
            checker: CheckerTool,
            cmd: &'a str,
        }

        let caches = cache_keys
            .iter()
            .map(|k| checks.read_cache(k))
            .collect::<Result<Vec<_>>>()?;
        let mut map_cmd = new_map_with_cap(caches.len());
        for cache in &caches {
            map_cmd.insert(
                CmdKeyRef {
                    pkg: &cache.cmd.pkg_name,
                    target: &cache.cmd.cmd.target,
                    channel: &cache.cmd.cmd.channel,
                    checker: cache.cmd.checker.checker,
                    cmd: &cache.cmd.cmd.cmd,
                },
                (cache.diagnostics.data.len(), cache.diagnostics.duration_ms),
            );
        }

        let capacity = resolves.len();

        // (pkg_name, target) => at least target_overridden once
        let mut pkg_tar_specified = new_map_with_cap(capacity);

        let mut resolved = Vec::with_capacity(capacity);
        for resolve in resolves {
            let pkg_target = (&*resolve.pkg_name, &*resolve.target);
            pkg_tar_specified
                .entry(pkg_target)
                .and_modify(|b| *b |= resolve.target_overridden)
                .or_insert(resolve.target_overridden);

            let cmd_key = CmdKeyRef {
                pkg: &resolve.pkg_name,
                target: &resolve.target,
                channel: &resolve.channel,
                checker: resolve.checker,
                cmd: &resolve.cmd,
            };
            let (count, ms) = map_cmd.get(&cmd_key).unwrap();
            resolved.push(Resolve::new(resolve, *count, *ms));
        }
        resolved.sort_unstable();
        let dir = format!("targets/{user}/{repo}");
        write_to_file(&dir, "resolved", &resolved)?;

        let mut sources = Vec::with_capacity(pkgs.len());
        for info in pkgs {
            Source::push(info, &pkg_tar_specified, root_path, &mut sources);
        }
        sources
            .sort_unstable_by(|a, b| (a.pkg, a.target, a.source).cmp(&(b.pkg, b.target, b.source)));
        write_to_file(&dir, "sources", &sources)?;

        v_user_repo.push((user, repo));

        Ok(())
    })?;

    let map_user_repo = user_repo(
        v_user_repo.len(),
        v_user_repo
            .iter()
            .map(|(user, repo)| [user.as_str(), repo.as_str()]),
    );
    write_to_file("", "user_repo", &map_user_repo)?;

    Ok(())
}

#[derive(Debug, Serialize)]
struct Source<'a> {
    pkg: &'a str,
    #[serde(skip)]
    source: &'a TargetSource,
    target: &'a str,
    src: &'a str,
    path: &'a Utf8Path,
    used: bool,
    specified: bool,
}

impl<'a> Source<'a> {
    // NOTE: here we assume there is unique pkg name in repo
    pub fn push(
        info: &'a CachePackageInfo,
        pkg_tar_specified: &IndexMap<(&str, &str), bool>,
        repo_root: &Utf8Path,
        v: &mut Vec<Source<'a>>,
    ) {
        for (target, sources) in &info.targets.map {
            let pkg = &*info.pkg_name;
            let target = &**target;
            let (used, specified) = match pkg_tar_specified.get(&(pkg, target)) {
                Some(true) => (true, true),
                Some(false) => (true, false),
                None => (false, false),
            };

            for source in sources {
                let (desc, path) = source.descibe();
                let path = match path {
                    Some(p) => p.strip_prefix(repo_root).unwrap_or(p),
                    None => "".into(),
                };
                v.push(Source {
                    pkg,
                    source,
                    target,
                    src: desc,
                    path,
                    used,
                    specified,
                });
            }
        }
    }
}

fn user_repo<'a>(
    len: usize,
    iter: impl IntoIterator<Item = [&'a str; 2]>,
) -> IndexMap<&'a str, Vec<&'a str>> {
    let mut map = new_map_with_cap::<&'a str, Vec<&'a str>>(len);

    for [user, repo] in iter {
        map.entry(user)
            .and_modify(|v| v.push(repo))
            .or_insert_with(|| vec![repo]);
    }

    map.values_mut().for_each(|v| v.sort_unstable());
    map
}
