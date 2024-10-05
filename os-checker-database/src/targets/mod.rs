use crate::{
    utils::{new_map_with_cap, IndexMap},
    Result,
};
use camino::{Utf8Path, Utf8PathBuf};
use os_checker_types::{db::*, CheckerTool, XString};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Resolve<'a> {
    pkg: &'a str,
    toolchain: &'a str,
    checker: CheckerTool,
    target: &'a str,
    cmd: &'a str,
}

impl<'a> Resolve<'a> {
    fn new(resolve: &'a CacheResolve) -> Self {
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
        }
    }
}

pub fn do_resolves() -> Result<()> {
    let db = redb::Database::open(crate::CACHE_REDB)?;
    let txn = db.begin_read()?;
    let table = txn.open_table(LAYOUT)?;
    table_resolves(&table)?;

    Ok(())
}

fn table_resolves(table: &Table) -> Result<()> {
    let mut v = Vec::with_capacity(128);
    read_layout(table, |repo, layout| {
        v.push((repo.user, repo.repo, layout.resolves, layout.packages_info));
    })?;
    for (user, repo, resolves, pkgs) in v {
        // (pkg_name, target) => at least target_overridden once
        let mut pkg_tar_specified = new_map_with_cap(resolves.len());
        let mut sources = Vec::with_capacity(64);
        let mut resolved = Vec::with_capacity(64);

        for resolve in &resolves {
            let key = (&*resolve.pkg_name, &*resolve.target);
            pkg_tar_specified
                .entry(key)
                .and_modify(|b| *b |= resolve.target_overridden)
                .or_insert(resolve.target_overridden);

            resolved.push(Resolve::new(resolve));
        }

        resolved.sort_unstable();
        let dir = format!("targets/{user}/{repo}");
        crate::write_to_file(&dir, "resolved", &resolved)?;

        for info in &pkgs {
            Source::push(info, &pkg_tar_specified, &mut sources);
        }
        sources
            .sort_unstable_by(|a, b| (a.pkg, a.target, a.source).cmp(&(b.pkg, b.target, b.source)));
        crate::write_to_file(&dir, "sources", &sources)?;
    }
    Ok(())
}

type Table = redb::ReadOnlyTable<InfoKey, CacheLayout>;

fn read_layout(table: &Table, mut f: impl FnMut(CacheRepo, CacheLayout)) -> Result<()> {
    use redb::{ReadableTable, ReadableTableMetadata};

    for ele in table.iter()? {
        let (guard_k, guard_v) = ele?;
        let key = guard_k.value().repo;
        let value = guard_v.value();
        f(key, value);
    }
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
                v.push(Source {
                    pkg,
                    source,
                    target,
                    src: desc,
                    path: path.unwrap_or("".into()),
                    used,
                    specified,
                });
            }
        }
    }
}
