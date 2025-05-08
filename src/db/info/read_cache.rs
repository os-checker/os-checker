use super::{Info, InfoKey};
use indexmap::IndexMap;
use os_checker_types::out_json::UserRepo;
use std::rc::Rc;

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

    pub fn push(&mut self, key: InfoKey, val: Info) {
        let user_repo = key.user_repo();
        let rc = RcCachedInfoKeyValue::new(key, val);

        if let Some(old) = self.latest_commit.get(&user_repo) {
            if old.committer_datetime() < rc.committer_datetime() {
                // the added cache is from a newer commit, so replace the old
                self.latest_commit.insert(user_repo, rc.clone());
            }
        } else {
            // the first time to add a cache for this user_repo
            self.latest_commit.insert(user_repo, rc.clone());
        }
        self.all.push(rc);
    }
}

#[derive(Debug)]
struct InfoKeyValuePair {
    key: InfoKey,
    val: Info,
}

#[derive(Clone, Debug)]
pub struct RcCachedInfoKeyValue {
    inner: Rc<InfoKeyValuePair>,
}

impl RcCachedInfoKeyValue {
    fn new(key: InfoKey, val: Info) -> Self {
        Self {
            inner: Rc::new(InfoKeyValuePair { key, val }),
        }
    }

    fn committer_datetime(&self) -> u64 {
        self.inner.val.latest_commit.committer.datetime
    }
}

#[test]
fn size() {
    use std::mem::size_of;
    dbg!(
        size_of::<InfoKeyValuePair>(),
        size_of::<InfoKey>(),
        size_of::<Info>(),
    );

    let db = crate::db::Db::new("tmp/cache.redb".into()).unwrap();
    let cached = db.get_all_cached_info_key_and_value().unwrap();
    dbg!(cached);
}
