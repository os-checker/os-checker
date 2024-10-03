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

    let txn = db.begin_read()?;

    stats(DATA, &txn)?;
    stats(INFO, &txn)?;
    stats(LAYOUT, &txn)?;

    let table = txn.open_table(LAYOUT)?;
    for item in table.iter()? {
        let layout = item?.1.value();
        for ws in layout.workspaces.values() {
            // test Metadata deserialization
            assert!(ws.meta_data().is_ok());
        }
    }

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

    for (idx, item) in table.iter()?.enumerate() {
        let (guard_k, guard_v) = item?;
        print!("{idx}k ");
        _ = guard_k.value();
        print!("{idx}v ");
        _ = guard_v.value();
    }
    println!("\n{def} table all good!\n");
    Ok(())
}
