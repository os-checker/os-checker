use crate::Result;
use redb::{Key, ReadTransaction, ReadableTable, ReadableTableMetadata, TableDefinition, Value};

// TODO: move this to os_checker_types crate
// FIXME: this should be to read the latest checking results, not all results.
pub fn read_table<K, V>(
    txn: &ReadTransaction,
    table: TableDefinition<K, V>,
    mut f: impl FnMut(K, V) -> Result<()>,
) -> Result<()>
where
    K: for<'a> Key<SelfType<'a> = K>,
    V: for<'a> Value<SelfType<'a> = V>,
{
    let table = txn.open_table(table)?;

    for ele in table.iter()? {
        let (guard_k, guard_v) = ele?;
        let key = guard_k.value();
        let value = guard_v.value();
        f(key, value);
    }

    Ok(())
}
