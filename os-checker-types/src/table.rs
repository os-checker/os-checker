use crate::db::*;
use redb::{Database, TableDefinition};

pub const DATA: TableDefinition<CacheRepoKey, CacheValue> = TableDefinition::new("data");
pub const INFO: TableDefinition<InfoKey, Info> = TableDefinition::new("info");
pub const LAYOUT: TableDefinition<InfoKey, CacheLayout> = TableDefinition::new("layout");

const CACHE_REDB: &str = "cache.redb";

pub fn test_database(dir: &str) -> Database {
    std::env::set_current_dir(dir).unwrap();
    Database::open(CACHE_REDB).unwrap()
}
