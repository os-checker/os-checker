use crate::db::*;
use redb::TableDefinition;

pub const DATA: TableDefinition<CacheRepoKey, CacheValue> = TableDefinition::new("data");
pub const INFO: TableDefinition<InfoKey, Info> = TableDefinition::new("info");
pub const LAYOUT: TableDefinition<InfoKey, CacheLayout> = TableDefinition::new("layout");
