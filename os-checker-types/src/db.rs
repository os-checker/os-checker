use crate::{CacheLayout, InfoKey};
use redb::TableDefinition;

// const DATA: TableDefinition<CacheRepoKey, CacheValue> = TableDefinition::new("data");
// const INFO: TableDefinition<InfoKey, Info> = TableDefinition::new("info");
const LAYOUT: TableDefinition<InfoKey, CacheLayout> = TableDefinition::new("layout");
