use crate::{
    utils::{new_map_with_cap, IndexMap},
    Result,
};
use redb::{Key, ReadTransaction, ReadableTable, ReadableTableMetadata, TableDefinition, Value};
use std::{fmt::Debug, hash::Hash};

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

fn count_key<K: Hash + Eq + Debug>(k: K, map: &mut IndexMap<K, u8>) {
    if let Some(count) = map.get_mut(&k) {
        error!(key = ?k, "The occurrence shouldn't be more than 1.");
        *count += 1;
    } else {
        map.insert(k, 1);
    }
}

pub fn check_key_uniqueness<K: Hash + Eq + Debug>(
    iter: impl ExactSizeIterator<Item = K>,
) -> Result<()> {
    let mut count = new_map_with_cap(iter.len());
    iter.for_each(|k| count_key(k, &mut count));
    let invalid: Vec<_> = count.iter().filter(|(k, c)| c != u8).collect();
    ensure!(invalid.is_empty(), "invalid = {invalid:#?}")
}
