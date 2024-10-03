use super::{
    gh::{Info, InfoKey},
    CacheLayout, CacheRepoKey, CacheValue,
};
use crate::Result;
use camino::Utf8Path;
use eyre::Context;
use os_checker_types::db as out;
use redb::{Database, Key, Table, TableDefinition, Value};
use std::sync::Arc;

use out::DATA;
const INFO: TableDefinition<InfoKey, Info> = TableDefinition::new("info");
const LAYOUT: TableDefinition<InfoKey, CacheLayout> = TableDefinition::new("layout");

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
            let _span = key.span();
            info!("Successfully cached repo layout.");
            Ok(())
        })
    }

    pub fn set_info(&self, key: &InfoKey, value: &Info) -> Result<()> {
        self.write(INFO, |table| {
            table.insert(key, value)?;
            let _span = key.span();
            info!("Successfully cached repo infomation.");
            Ok(())
        })
    }

    pub fn set_cache(&self, key: &CacheRepoKey, value: &CacheValue) -> Result<()> {
        self.write(DATA, |table| {
            let out_key = out::CacheRepoKey::from(key.clone());
            let out_value = out::CacheValue::from(value.clone());
            table.insert(&out_key, &out_value)?;
            let _span = key.span();
            info!("Successfully cached a checking result.");
            Ok(())
        })
    }

    // // TODO: remove this
    // #[cfg(test)]
    // pub fn set_or_replace(
    //     &self,
    //     key: &CacheRepoKey,
    //     f: impl FnOnce(Option<CacheValue>) -> Result<CacheValue>,
    // ) -> Result<()> {
    //     self.write(DATA, |table| {
    //         let opt_value = table.remove(key)?.map(|guard| guard.value());
    //         let mut value = f(opt_value)?;
    //         value.update_unix_timestamp();
    //         table.insert(key, value)?;
    //         Ok(())
    //     })
    // }

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

// #[test]
// fn test_db() -> crate::Result<()> {
//     let (key, value) = super::cache::new_cache();
//
//     let db = Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?;
//     let db = Db {
//         db: Arc::new(db),
//         path: Utf8Path::new("memory").into(),
//     };
//
//     db.set_or_replace(&key, move |opt| {
//         assert!(opt.is_none());
//         Ok(value)
//     })?;
//
//     db.set_or_replace(&key, move |opt| {
//         let value = opt.unwrap();
//         dbg!(&value);
//         Ok(value)
//     })?;
//
//     let (info_key, info) = super::gh::os_checker()?;
//     db.set_info(dbg!(&info_key), &info)?;
//     dbg!(db.get_info(&info_key)?);
//
//     Ok(())
// }
//
// // use crate::table::*;
// // use redb::*;
// //
// type StdResult<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
//
// #[test]
// fn cache_redb() -> StdResult<()> {
//     use redb::*;
//     const CACHE_REDB: &str = "cache.redb";
//     fn stats<K, V>(def: TableDefinition<K, V>, txn: &ReadTransaction) -> StdResult<()>
//     where
//         K: Key + 'static,
//         V: Value + 'static,
//     {
//         let table: ReadOnlyTable<K, V> = txn.open_table(def)?;
//         println!(
//             "{def} table [len={}]:\nstats: {:#?}",
//             table.len()?,
//             table.stats()?
//         );
//         for (idx, item) in table.iter()?.enumerate() {
//             let (guard_k, guard_v) = item?;
//             print!("{idx}k ");
//             _ = guard_k.value();
//             print!("{idx}v ");
//             _ = guard_v.value();
//         }
//         println!("{def} table all good!\n");
//         Ok(())
//     }
//
//     let db = redb::Database::open(CACHE_REDB)?;
//
//     let read_txn = db.begin_read()?;
//
//     // stats(DATA, &read_txn)?;
//     // stats(INFO, &read_txn)?;
//     stats(LAYOUT, &read_txn)?;
//
//     Ok(())
// }
