use crate::table::*;
use redb::*;

type Result<T, E = Box<dyn Send + Sync + std::error::Error>> = std::result::Result<T, E>;

#[test]
fn cache_redb() -> Result<()> {
    let db = test_database("..");

    let txn = db.begin_read()?;

    stats(CHECKS, &txn)?;
    let table = txn.open_table(CHECKS)?;
    let last_checks = table.last()?.unwrap();
    let id = last_checks.0.value();
    let checks = last_checks.1.value();
    dbg!(id, checks);

    stats(DATA, &txn)?;
    stats(INFO, &txn)?;
    stats(LAYOUT, &txn)?;

    let table = txn.open_table(LAYOUT)?;
    for (idx, item) in table.iter()?.enumerate() {
        let layout = item?.1.value();
        for ws in layout.workspaces.values() {
            assert!(
                ws.meta_data().is_ok(),
                "[idx={idx}] Metadata deserialization failure."
            );
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
