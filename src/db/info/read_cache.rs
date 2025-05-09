use super::{Info, InfoKey, InfoKeyValue};
use indexmap::IndexMap;
use os_checker_types::{out_json::UserRepo, parse_unix_timestamp_milli};
use std::{cell::RefCell, fmt, rc::Rc};

/// Extract all info from db and compute necessary data
/// to identify the last cache for each repo.
#[derive(Debug, Default)]
pub struct CachedAllInfoKeyValue {
    /// All (InfoKey, Info) from local db.
    /// NOTE: this field is initialized only once and never updated
    /// in single os-checker run.
    all: Vec<RcCachedInfoKeyValue>,
    /// Extract the last cache for each user/repo.
    latest_commit: IndexMap<UserRepo, RcCachedInfoKeyValue>,
}

impl CachedAllInfoKeyValue {
    pub fn with_capacity(len: usize) -> Self {
        Self {
            all: Vec::with_capacity(len),
            latest_commit: Default::default(),
        }
    }

    pub fn push(&mut self, key: InfoKey, val: Info, max_cache_value_timestamp: Option<u64>) {
        let user_repo = key.user_repo();
        let rc = RcCachedInfoKeyValue::new(key, val, max_cache_value_timestamp);

        if let Some(old) = self.latest_commit.get(&user_repo) {
            // When comparing Some and None, Some > None, which is good for our case.
            // This means if an old non-empty result is chosen over a newer empty result.
            if old.committer_datetime() < rc.committer_datetime()
                && old.max_cache_value_timestamp < rc.max_cache_value_timestamp
            {
                // the added cache is from a newer commit and newer checks, so replace the old
                self.latest_commit.insert(user_repo, rc.clone());
            }
        } else {
            // the first time to add a cache for this user_repo
            self.latest_commit.insert(user_repo, rc.clone());
        }
        self.all.push(rc);
    }

    pub fn get(&self, user: &str, repo: &str) -> Option<&InfoKeyValuePair> {
        let key = UserRepo {
            user: user.into(),
            repo: repo.into(),
        };
        self.latest_commit.get(&key).map(|pair| &*pair.inner)
    }
}

#[derive(Debug)]
pub struct InfoKeyValuePair {
    pub key: InfoKey,
    pub val: Info,
}

impl InfoKeyValuePair {
    pub fn to_info_key_value(&self) -> InfoKeyValue {
        InfoKeyValue {
            key: self.key.clone(),
            val: RefCell::new(self.val.clone()),
        }
    }
}

#[derive(Clone)]
pub struct RcCachedInfoKeyValue {
    inner: Rc<InfoKeyValuePair>,
    /// The max unix_timestamp_milli among CacheValues through info.caches
    max_cache_value_timestamp: Option<u64>,
}

impl fmt::Debug for RcCachedInfoKeyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RcCachedInfoKeyValue")
            .field("inner", &self.inner)
            .field(
                "max_cache_value_timestamp",
                &self
                    .max_cache_value_timestamp
                    .map(parse_unix_timestamp_milli),
            )
            .finish()
    }
}

impl RcCachedInfoKeyValue {
    fn new(key: InfoKey, val: Info, max_cache_value_timestamp: Option<u64>) -> Self {
        Self {
            inner: Rc::new(InfoKeyValuePair { key, val }),
            max_cache_value_timestamp,
        }
    }

    fn committer_datetime(&self) -> u64 {
        self.inner.val.latest_commit.committer.datetime
    }
}

#[test]
fn get_all_cached_info_key_and_value() {
    use std::mem::size_of;
    dbg!(
        size_of::<InfoKeyValuePair>(),
        size_of::<InfoKey>(),
        size_of::<Info>(),
    );

    let db = crate::db::Db::new("tmp/cache.redb".into()).unwrap();
    // let cached = db.get_all_cached_info_key_and_value().unwrap();
    // dbg!(cached);
}
