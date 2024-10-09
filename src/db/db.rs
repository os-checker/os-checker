use crate::Result;
use camino::Utf8Path;
use eyre::Context;
use os_checker_types::db::{
    CacheLayout, CacheRepoKey, CacheValue, CheckValue, Info, InfoKey, CHECKS, DATA, INFO, LAYOUT,
};
use redb::{Database, Key, ReadableTable, Table, TableDefinition, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct Db {
    db: Arc<Database>,
    path: Box<Utf8Path>,
}

impl std::fmt::Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Db").field("path", &self.path).finish()
    }
}

impl Db {
    #[instrument(level = "info")]
    pub fn new(path: &Utf8Path) -> Result<Db> {
        let db = Database::create(path).with_context(|| "无法创建或者打开 redb 数据库文件")?;
        let db = Db {
            db: Arc::new(db),
            path: path.into(),
        };

        // 如果这个表不存在，那么创建它
        db.write(DATA, |_| Ok(()))?;
        db.write(INFO, |_| Ok(()))?;

        Ok(db)
    }

    pub fn get_info(&self, key: &InfoKey) -> Result<Option<Info>> {
        self.read(INFO, key)
    }

    pub fn get_cache(&self, key: &CacheRepoKey) -> Result<Option<CacheValue>> {
        self.read(DATA, key)
    }

    pub fn read<K, V>(&self, table: TableDefinition<K, V>, key: &K) -> Result<Option<V>>
    where
        K: for<'a> Key<SelfType<'a> = K>,
        V: for<'a> Value<SelfType<'a> = V>,
    {
        let table = self.db.begin_read()?.open_table(table)?;
        Ok(table.get(key)?.map(|guard| guard.value()))
    }

    fn write<K: Key, V: Value>(
        &self,
        table: TableDefinition<K, V>,
        f: impl for<'a> FnOnce(&mut Table<'a, K, V>) -> Result<()>,
    ) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        f(&mut write_txn.open_table(table)?)?;
        write_txn.commit()?;
        Ok(())
    }

    pub fn set_layout(&self, key: &InfoKey, value: &CacheLayout) -> Result<()> {
        self.write(LAYOUT, |table| {
            table.insert(key, value)?;
            info!("Successfully cached repo layout.");
            Ok(())
        })
    }

    pub fn set_info(&self, key: &InfoKey, value: &Info) -> Result<()> {
        self.write(INFO, |table| {
            table.insert(key, value)?;
            info!("Successfully cached repo infomation.");
            Ok(())
        })
    }

    pub fn set_cache(&self, key: &CacheRepoKey, value: &CacheValue) -> Result<()> {
        self.write(DATA, |table| {
            table.insert(key, value)?;
            info!("Successfully cached a checking result.");
            Ok(())
        })
    }

    pub fn compact(self) {
        let _span = error_span!("compact", db_path = %self.path).entered();
        if let Some(mut db) = Arc::into_inner(self.db) {
            match db.compact() {
                Ok(true) => info!("compacted"),
                Ok(false) => warn!("not compacted"),
                Err(err) => error!(?err, "failed to compact"),
            }
        } else {
            error!("Unable to get the unique db handler");
        }
    }
}

impl Db {
    fn check_write_last(&self, f: impl FnOnce(&mut CheckValue)) -> Result<()> {
        self.write(CHECKS, |t| {
            let (key, mut value) = match t.last()? {
                Some((guard_k, guard_v)) => (guard_k.value(), guard_v.value()),
                None => bail!(
                    "The last check item is not available in {CHECKS}. \
                     A new check should be created."
                ),
            };
            f(&mut value);
            t.insert(&key, &value)?;
            Ok(())
        })
    }

    /// Create a new check item only with key set.
    pub fn new_check(&self) -> Result<()> {
        self.write(CHECKS, |t| {
            let key = match t.last()? {
                Some((guard_k, _)) => guard_k.value() + 1,
                None => 0,
            };
            t.insert(key, &Default::default())?;
            info!(key, "Successfully create a new check item.");
            Ok(())
        })
    }

    // push info key
    pub fn check_push_info_key(&self, info: InfoKey) -> Result<()> {
        self.check_write_last(|check| check.push_info_key(info))
    }

    // set check complete + merge
    pub fn check_set_complete(&self) -> Result<()> {
        self.check_write_last(|check| check.set_complete())?;

        let txn = self.db.begin_write()?;
        let mut table = txn.open_table(CHECKS)?;

        let last = table
            .iter()?
            .rev()
            .take(2)
            .map(|res| res.map(|(k, v)| (k.value(), v.value())))
            .collect::<Result<Vec<_>, _>>()?;
        if let [(_, last1_v), (last2_k, last2_v)] = last.as_slice() {
            if last1_v.is_same_keys(last2_v) {
                // use the second to last id, and remove the item
                table.pop_last()?;
                table.pop_last()?;
                table.insert(last2_k, last1_v)?;
            }
        }

        drop(table);
        txn.commit()?;

        Ok(())
    }
}
