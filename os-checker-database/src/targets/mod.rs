use crate::{
    db::{check_key_uniqueness, read_table},
    utils::{new_map_with_cap, IndexMap},
    Result,
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

    let mut layouts = Vec::with_capacity(128);
    read_table(&txn, LAYOUT, |key, layout| {
        layouts.push((key, layout));
        Ok(())
    })?;
    {
        let _span = error_span!("do_resolves_layout", table = %LAYOUT).entered();
        check_key_uniqueness(layouts.iter().map(|(key, _)| key.user_repo()))?;
    }

    let mut data = new_map_with_cap(1024);
    read_table(&txn, DATA, |key, cache| {
        let diag = &cache.diagnostics;
        data.insert(key, (diag.data.len(), diag.duration_ms));
        Ok(())
    })?;
    // {
    //     let _span = error_span!("do_resolves_data", table = %DATA).entered();
    //     check_key_uniqueness(data.iter().map(|(user, repo, _, _)| (&**user, &**repo)))?;
    // }

    // let (data_len, layouts_len) = (data.len(), layouts.len());
    // ensure!(
    //     data_len == layouts_len,
    //     "data_len {data_len} ≠ layouts_len {layouts_len}"
    // );
    //
    // let mut map: LayoutData = new_map_with_cap(data_len);
    // for (user, repo, count, duration) in &data {
    //     map.insert((&**user, &**repo), (None, *count, *duration));
    // }
    // for (user, repo, layout) in &layouts {
    //     if let Some((value, _, _)) = map.get_mut(&(&**user, &**repo)) {
    //         *value = Some(layout);
    //     } else {
    //         bail!("{user}/{repo} exsits in DATA but not in LAYOUT table.");
    //     }
    // }

    table_resolves(&layouts, &data)?;

    Ok(())
}

type LayoutData<'a> = IndexMap<(&'a str, &'a str), (Option<&'a CacheLayout>, usize, u64)>;

fn table_resolves(
    layouts: &[(InfoKey, CacheLayout)],
    data: &IndexMap<CacheRepoKey, (usize, u64)>,
) -> Result<()> {
    for (key, layout) in layouts {
        let [user, repo] = key.user_repo();
        let CacheLayout {
            root_path,
            packages_info: pkgs,
            resolves,
            ..
        } = layout;

        // (pkg_name, target) => at least target_overridden once
        let mut pkg_tar_specified = new_map_with_cap(resolves.len());
        let mut sources = Vec::with_capacity(64);
        let mut resolved = Vec::with_capacity(64);

        for resolve in resolves {
            let pkg_target = (&*resolve.pkg_name, &*resolve.target);
            pkg_tar_specified
                .entry(pkg_target)
                .and_modify(|b| *b |= resolve.target_overridden)
                .or_insert(resolve.target_overridden);

            let data_key = CacheRepoKey {
                repo: key.repo.clone(),
                cmd: CacheRepoKeyCmd {
                    pkg_name: resolve.pkg_name.clone(),
                    checker: CacheChecker {
                        checker: resolve.checker,
                        version: None,
                        sha: None,
                    },
                    cmd: CacheCmd {
                        cmd: resolve.cmd.clone(),
                        target: resolve.target.clone(),
                        channel: resolve.channel.clone(),
                        features: vec![],
                        flags: vec![],
                    },
                },
            };
            let &(count, ms) = data.get(&data_key).unwrap();
            resolved.push(Resolve::new(resolve, count, ms));
        }

        resolved.sort_unstable();
        let dir = format!("targets/{user}/{repo}");
        crate::write_to_file(&dir, "resolved", &resolved)?;

        for info in pkgs {
            Source::push(info, &pkg_tar_specified, root_path, &mut sources);
        }
        sources
            .sort_unstable_by(|a, b| (a.pkg, a.target, a.source).cmp(&(b.pkg, b.target, b.source)));
        crate::write_to_file(&dir, "sources", &sources)?;
    }

    let map_user_repo = user_repo(
        layouts.len(),
        layouts.iter().map(|(key, _)| key.user_repo()),
    );
    crate::write_to_file("", "user_repo", &map_user_repo)?;

    Ok(())
}

// type Table = redb::ReadOnlyTable<InfoKey, CacheLayout>;
//
// fn read_layout(table: &Table, mut f: impl FnMut(CacheRepo, CacheLayout)) -> Result<()> {
//     use redb::{ReadableTable, ReadableTableMetadata};
//
//     for ele in table.iter()? {
//         let (guard_k, guard_v) = ele?;
//         let key = guard_k.value().repo;
//         let value = guard_v.value();
//         f(key, value);
//     }
//     Ok(())
// }

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
