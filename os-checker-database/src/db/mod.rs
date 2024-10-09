use crate::{
    utils::{new_map_with_cap, IndexMap},
    Result,
};
use eyre::ContextCompat;
use os_checker_types::db::*;
use redb::{
    Key, ReadOnlyTable, ReadTransaction, ReadableTable, ReadableTableMetadata, TableDefinition,
    Value,
};
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

pub fn read_last_checks(txn: &ReadTransaction) -> Result<(u32, CheckValue)> {
    let table = txn.open_table(CHECKS)?;
    let last_checks = table
        .last()?
        .with_context(|| format!("{CHECKS} has no check item."))?;
    let idx = last_checks.0.value();
    info!(idx, %CHECKS, "Read last check item.");
    Ok((idx, last_checks.1.value()))
}

pub struct LastChecks<'txn> {
    txn: &'txn ReadTransaction,
    checks: CheckValue,
    layout: ReadOnlyTable<InfoKey, CacheLayout>,
    cache: ReadOnlyTable<CacheRepoKey, CacheValue>,
}

impl<'txn> LastChecks<'txn> {
    pub fn new(txn: &'txn ReadTransaction) -> Result<Self> {
        let (_, checks) = read_last_checks(txn)?;
        let layout = txn.open_table(LAYOUT)?;
        let cache = txn.open_table(DATA)?;
        Ok(Self {
            txn,
            checks,
            layout,
            cache,
        })
    }

    pub fn repo_counts(&self) -> usize {
        self.checks.keys.len()
    }

    pub fn with_layout_cache(
        &self,
        mut f: impl FnMut(&InfoKey, &[CacheRepoKey]) -> Result<()>,
    ) -> Result<()> {
        for key in &self.checks.keys {
            f(&key.info, &key.cache)?;
        }
        Ok(())
    }

    pub fn read_layout(&self, info_key: &InfoKey) -> Result<CacheLayout> {
        let _span = error_span!("read_layout", ?info_key).entered();
        let guard = self.layout.get(info_key)?;
        Ok(guard
            .with_context(|| "Info key refers to None value.")?
            .value())
    }

    pub fn read_cache(&self, cache_key: &CacheRepoKey) -> Result<CacheValue> {
        let _span = error_span!("read_cache", ?cache_key).entered();
        let guard = self.cache.get(cache_key)?;
        Ok(guard
            .with_context(|| "Cache key refers to None value.")?
            .value())
    }
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
    let invalid: Vec<_> = count.iter().filter(|(k, c)| **c != 1u8).collect();
    ensure!(invalid.is_empty(), "invalid = {invalid:#?}");
    Ok(())
}
