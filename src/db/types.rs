use crate::config::CheckerTool;
use musli::{storage, Decode, Encode};

#[derive(Debug, Encode, Decode)]
pub struct CacheKey {
    repo: CacheRepo,
    checker: CacheChecker,
    cmd: CacheCmd,
}

impl redb::Value for CacheKey {
    type SelfType<'a> = Self
    where
        Self: 'a;

    type AsBytes<'a> = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        storage::from_slice(data).expect("Not a valid cache key.")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        storage::to_vec(value).expect("Cache key can't be encoded to bytes.")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("OsCheckerCacheKey")
    }
}

impl redb::Key for CacheKey {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        data1.cmp(data2)
    }
}

#[derive(Debug, Encode, Decode)]
struct CacheRepo {
    user: String,
    repo: String,
    pkg_name: String,
    sha: String,
    branch: String,
}

#[derive(Debug, Encode, Decode)]
struct CacheChecker {
    checker: CheckerTool,
    // If we don't care about the version, use None.
    version: Option<String>,
    sha: Option<String>,
}

#[derive(Debug, Encode, Decode)]
struct CacheCmd {
    cmd: String,
    target: String,
    // Below is not necessary, and currently not implemented.
    features: Vec<String>,
    rustflags: Vec<String>,
}

#[derive(Encode, Decode)]
pub struct CacheValue {
    unix_timestamp_milli: u64,
    diagnostics: Vec<String>,
}

impl std::fmt::Debug for CacheValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheValue")
            .field(
                "unix_timestamp_milli",
                &parse_now(self.unix_timestamp_milli)
                    .to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap()),
            )
            .field("diagnostics.len", &self.diagnostics.len())
            .finish()
    }
}

impl redb::Value for CacheValue {
    type SelfType<'a> = Self
    where
        Self: 'a;

    type AsBytes<'a> = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        storage::from_slice(data).expect("Not a valid cache value.")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        storage::to_vec(value).expect("Cache value can't be encoded to bytes.")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("OsCheckerCacheResults")
    }
}

impl CacheValue {
    pub fn new(diagnostics: Vec<String>) -> Self {
        CacheValue {
            unix_timestamp_milli: now(),
            diagnostics,
        }
    }
}

/// Returns the current unix timestamp in milliseconds.
pub fn now() -> u64 {
    let t = time::OffsetDateTime::from(std::time::SystemTime::now());
    let milli = t.millisecond() as u64;
    let unix_t_secs = t.unix_timestamp() as u64;
    unix_t_secs * 1000 + milli
}

pub fn parse_now(ts: u64) -> time::OffsetDateTime {
    match time::OffsetDateTime::from_unix_timestamp((ts / 1000) as i64) {
        Ok(t) => t,
        Err(err) => panic!("{ts} 无法转回时间：{err}"),
    }
}

#[test]
fn db() -> crate::Result<()> {
    use redb::{Database, TableDefinition};
    const TABLE: TableDefinition<CacheKey, CacheValue> = TableDefinition::new("test");

    let key = CacheKey {
        repo: CacheRepo {
            user: "user".to_owned(),
            repo: "repo".to_owned(),
            pkg_name: "pkg".to_owned(),
            sha: "abc".to_owned(),
            branch: "main".to_owned(),
        },
        checker: CacheChecker {
            checker: CheckerTool::Clippy,
            version: None,
            sha: None,
        },
        cmd: CacheCmd {
            cmd: "cargo clippy".to_owned(),
            target: "x86".to_owned(),
            features: vec![],
            rustflags: vec![],
        },
    };

    let value = CacheValue::new(vec!["warning: xxx".to_owned()]);

    let db = Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(TABLE)?;
        table.insert(&key, &value)?;
    }
    write_txn.commit()?;

    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(TABLE)?;
    dbg!(table.get(&key)?.unwrap().value());

    Ok(())
}
