use crate::db::CacheLayout;
use crate::db::CachePackageInfo;
use crate::db::CacheResolve;
use crate::prelude::*;
use crate::table::*;
use indexmap::indexmap;
use redb::*;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const CACHE_REDB: &str = "cache.redb";

#[test]
fn cache_redb() -> Result<()> {
    std::env::set_current_dir("..")?;
    let db = redb::Database::open(CACHE_REDB)?;

    let read_txn = db.begin_read()?;

    stats(DATA, &read_txn)?;
    stats(INFO, &read_txn)?;
    stats(LAYOUT, &read_txn)?;

    Ok(())
}

fn stats<K, V>(def: TableDefinition<K, V>, txn: &ReadTransaction) -> Result<()>
where
    K: Key + 'static,
    V: Value + 'static,
{
    let table = txn.open_table(def)?;
    println!(
        "{def} table [len={}]:\nstats: {:#?}",
        table.len()?,
        table.stats()?
    );

    let idx = 0;
    let (guard_k, guard_v) = table.first()?.unwrap();
    print!("{idx}k ");
    _ = guard_k.value();
    print!("{idx}v ");
    _ = guard_v.value();

    for (idx, item) in table.iter()?.rev().enumerate() {
        let (guard_k, guard_v) = item?;
        print!("{idx}k ");
        _ = guard_k.value();
        print!("{idx}v ");
        _ = guard_v.value();
    }
    println!("{def} table all good!\n");
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct A {
    // #[serde(default)]
    // #[serde(skip_serializing_if = "Option::is_none")]
    a: A2,
}

#[derive(Encode, Decode, Debug)]
struct B {
    #[musli(with = musli::serde)]
    a: A,
}

#[derive(Serialize, Deserialize, Debug)]
struct A2 {
    // #[serde(default)]
    // #[serde(skip_serializing_if = "Option::is_none")]
    a: IndexMap<u8, ()>,
}
//
// #[derive(Encode, Decode, Debug)]
// struct B2 {
//     #[musli(with = musli::serde)]
//     a: A,
// }

#[test]
fn test_musli() {
    let a = A {
        a: A2 {
            a: indexmap! { 1 => () },
        },
    };
    let bytes = musli::storage::to_vec(&B { a }).unwrap();
    let b: B = musli::storage::from_slice(&bytes).unwrap();
    dbg!(b);

    let bytes = musli::storage::to_vec(&CacheLayout::default()).unwrap();
    let b2: CacheLayout = musli::storage::from_slice(&bytes).unwrap();
    dbg!(b2);

    let layout = CacheLayout {
        root_path: Utf8PathBuf::from("a"),
        cargo_tomls: Box::new([Utf8PathBuf::from("b")]),
        workspaces: IndexMap::new(),
        packages_info: Box::new([CachePackageInfo {
            pkg_name: XString::from("c"),
            pkg_dir: Utf8PathBuf::from("d"),
            targets: Default::default(),
            channel: String::from("e"),
            resolves: Box::new([CacheResolve {
                target: String::from("d"),
                target_overriden: false,
                channel: String::from("g"),
                checker: crate::CheckerTool::Clippy,
                cmd: String::from("h"),
            }]),
        }]),
    };
    let bytes = musli::storage::to_vec(&layout).unwrap();
    let b3: CacheLayout = musli::storage::from_slice(&bytes).unwrap();
    dbg!(b3);
}
