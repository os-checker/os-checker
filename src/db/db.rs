use super::{CacheKey, CacheValue};
use crate::Result;
use camino::Utf8Path;
use eyre::Context;
use redb::{Database, TableDefinition};
use std::sync::Arc;

const TABLE: TableDefinition<CacheKey, CacheValue> = TableDefinition::new("data");

#[derive(Clone)]
struct Db {
    db: Arc<Database>,
}

impl Db {
    #[instrument(level = "info")]
    pub fn new(path: &Utf8Path) -> Result<Db> {
        let db = Database::create(path).with_context(|| "无法创建或者打开 redb 数据库文件")?;
        let db = Arc::new(db);
        Ok(Db { db })
    }

    pub fn get_or_replace(
        &self,
        key: &CacheKey,
        f: impl FnOnce(Option<CacheValue>) -> Result<CacheValue>,
    ) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            let opt_value = table.remove(key)?.map(|guard| guard.value());
            let mut value = f(opt_value)?;
            value.update_unix_timestamp();
            table.insert(key, value)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

#[test]
fn db() -> crate::Result<()> {
    let (key, value) = super::types::new_cache();

    let db = Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?;
    let db = Db { db: Arc::new(db) };

    db.get_or_replace(&key, move |opt| {
        assert!(opt.is_none());
        Ok(value)
    })?;

    db.get_or_replace(&key, move |opt| {
        let value = opt.unwrap();
        dbg!(&value);
        Ok(value)
    })?;

    Ok(())
}
