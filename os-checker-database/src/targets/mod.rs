use crate::Result;
use camino::{Utf8Path, Utf8PathBuf};
use os_checker_types::{db::*, CheckerTool, XString};
use serde::Serialize;

// #[derive(Debug, Serialize)]
// struct TargetRow {
//     user: XString,
//     repo: XString,
//     pkg: XString,
//     pkg_dir: Utf8PathBuf,
//     triple: String,
//     #[serde(flatten)]
//     source_cmd: TargetSourceCmd,
// }
//
// #[derive(Debug, Serialize, Default)]
// struct TargetSourceCmd {
//     source: &'static str,
//     // only two valid values: None or Some("true")
//     source_specified: Option<&'static str>,
//     source_path: Option<Utf8PathBuf>,
//     checker: Option<CheckerTool>,
//     cmd: Option<String>,
//     // count: Option<u64>
// }
//
// impl TargetSourceCmd {
//     fn new_source(src: &TargetSource, repo_root: &Utf8Path) -> Self {
//         let mut res = Self::default();
//         let (source, path) = match src {
//             TargetSource::RustToolchainToml(p) => ("RustToolchainToml", Some(p.to_owned())),
//             TargetSource::CargoConfigToml(p) => ("CargoConfigToml", Some(p.to_owned())),
//             TargetSource::CargoTomlDocsrsInPkgDefault(p) => {
//                 ("CargoTomlDocsrsInPkgDefault", Some(p.to_owned()))
//             }
//             TargetSource::CargoTomlDocsrsInWorkspaceDefault(p) => {
//                 ("CargoTomlDocsrsInWorkspaceDefault", Some(p.to_owned()))
//             }
//             TargetSource::CargoTomlDocsrsInPkg(p) => ("CargoTomlDocsrsInPkg", Some(p.to_owned())),
//             TargetSource::CargoTomlDocsrsInWorkspace(p) => {
//                 ("CargoTomlDocsrsInWorkspace", Some(p.to_owned()))
//             }
//             TargetSource::UnspecifiedDefaultToHostTarget => {
//                 ("UnspecifiedDefaultToHostTarget", None)
//             }
//             TargetSource::DetectedByPkgScripts(p) => ("DetectedByPkgScripts", Some(p.to_owned())),
//             TargetSource::DetectedByRepoGithub(p) => ("DetectedByRepoGithub", Some(p.to_owned())),
//             TargetSource::DetectedByRepoScripts(p) => ("DetectedByRepoScripts", Some(p.to_owned())),
//         };
//         res.source_path = path;
//         res.source = source;
//         res
//     }
//
//     fn new_cmd(src: &'static str, specifed: bool, checker: CheckerTool, cmd: &str) -> Self {
//         Self {
//             source: src,
//             checker: Some(checker),
//             cmd: Some(cmd.to_owned()),
//             source_specified: specifed.then_some("true"),
//             ..Default::default()
//         }
//     }
// }
//
// type TargetRows = Vec<TargetRow>;
//
// impl TargetRow {
//     fn push(
//         key: &CacheRepo,
//         repo_root: &Utf8Path,
//         v_info: &[CachePackageInfo],
//         v_row: &mut TargetRows,
//     ) {
//         let user = &key.user;
//         let repo = &key.repo;
//         for info in v_info {
//             let pkg = &info.pkg_name;
//             let pkg_dir = info
//                 .pkg_dir
//                 .strip_prefix(repo_root)
//                 .unwrap_or(&info.pkg_dir);
//
//             for (triple, sources) in &info.targets.map {
//                 for src in sources {
//                     let value = Self {
//                         user: user.clone(),
//                         repo: repo.clone(),
//                         pkg: pkg.clone(),
//                         pkg_dir: pkg_dir.into(),
//                         triple: triple.into(),
//                         source_cmd: TargetSourceCmd::new_source(src, repo_root),
//                     };
//                     v_row.push(value);
//                 }
//             }
//
//             for resolve in &info.resolves {
//                 let value = Self {
//                     user: user.clone(),
//                     repo: repo.clone(),
//                     pkg: pkg.clone(),
//                     pkg_dir: pkg_dir.into(),
//                     triple: resolve.target.clone(),
//                     source_cmd: TargetSourceCmd::new_cmd(
//                         "",
//                         resolve.target_overriden,
//                         resolve.checker,
//                         &resolve.cmd,
//                     ),
//                 };
//                 v_row.push(value);
//             }
//         }
//     }
// }
//
// fn rows(table: &redb::ReadOnlyTable<InfoKey, CacheLayout>) -> Result<TargetRows> {
//     use redb::{ReadableTable, ReadableTableMetadata};
//
//     let mut v = Vec::with_capacity(table.len()? as usize * 16);
//     for ele in table.iter()?.take(1) {
//         let (guard_k, guard_v) = ele?;
//         let key = guard_k.value().repo;
//         let value = guard_v.value();
//         TargetRow::push(&key, dbg!(&value.root_path), &value.packages_info, &mut v);
//     }
//     Ok(v)
// }
//
// // 如果生成一个大的 JSON 数据，需要几十兆，那么可以考虑通过 xz 格式传输，在前端使用
// // https://github.com/httptoolkit/xz-decompress 解压。
// #[test]
// fn targets() -> Result<()> {
//     let db = os_checker_types::table::test_database("..");
//     let txn = db.begin_read()?;
//     let table = txn.open_table(LAYOUT)?;
//     let res = rows(&table)?;
//     dbg!(res.len());
//
//     let file = Utf8PathBuf::from_iter(["TargetRows.json"]);
//     let writer = std::io::BufWriter::new(std::fs::File::create(file)?);
//     serde_json::to_writer_pretty(writer, &res)?;
//
//     Ok(())
// }

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
    for (user, repo, resolves, _) in v {
        let mut v = Vec::with_capacity(64);
        for resolve in &resolves {
            v.push(Resolve::new(resolve));
        }
        v.sort_unstable();
        let dir = format!("targets/{user}/{repo}");
        crate::write_to_file(&dir, "resolved", &v)?;
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
