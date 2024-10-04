use crate::Result;
use camino::Utf8Path;
use eyre::Context;
use os_checker_types::db::{
    CacheLayout, CacheRepoKey, CacheValue, Info, InfoKey, DATA, INFO, LAYOUT,
};
use redb::{Database, Key, Table, TableDefinition, Value};
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
